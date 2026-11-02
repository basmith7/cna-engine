//! The registry against Postgres: creating, seating, applying an order.
//! Needs `DATABASE_URL` and `--features db-tests`.

mod common;

use cna_data::scenario::Side;
use cna_embed::RULES_COMMIT;
use cna_server::accounts;
use cna_server::games::{GameError, Games};
use common::*;
use sqlx::PgPool;

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_new_game_records_its_rules_and_a_key_and_activates_when_seated(pool: PgPool) {
    let games = Games::new(pool.clone());
    let a = accounts::insert(&pool, "1", "A").await.unwrap();
    let b = accounts::insert(&pool, "2", "B").await.unwrap();
    let id = games.create("grazianis-offensive", a, false).await.unwrap();
    let (commit, key, status0): (String, Vec<u8>, String) =
        sqlx::query_as("SELECT rules_commit, alias_key, status FROM games WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(commit, RULES_COMMIT);
    assert_eq!(key.len(), 32);
    assert_eq!(status0, "open");
    games.seat(id, Side::Axis, a).await.unwrap();
    assert_eq!(status(&pool, id).await, "open");
    assert!(
        games.seat(id, Side::Cw, a).await.is_err(),
        "one seat per account"
    );
    games.seat(id, Side::Cw, b).await.unwrap();
    assert_eq!(status(&pool, id).await, "active");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn an_order_is_stored_only_from_its_seat_when_legal_and_current(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    let log = golden();
    let first = &log.orders[0];
    assert_eq!(first.player, Side::Axis);
    let mut rx = games.get(id).await.unwrap().tx.subscribe();

    // The Commonwealth's holder may not order as the Axis, nor the Axis's
    // as the Commonwealth.
    let e = games.apply(id, cw, Side::Axis, &first.order, 0).await;
    assert!(matches!(e, Err(GameError::NotYourSeat)), "{e:?}");
    let e = games.apply(id, axis, Side::Cw, &first.order, 0).await;
    assert!(matches!(e, Err(GameError::NotYourSeat)), "{e:?}");

    games
        .apply(id, axis, Side::Axis, &first.order, 0)
        .await
        .unwrap();
    let (seq, side): (i64, String) =
        sqlx::query_as("SELECT seq, side FROM orders WHERE game_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((seq, side.as_str()), (0, "axis"));
    let applied = rx.recv().await.unwrap();
    assert_eq!((applied.seq, applied.side), (0, Side::Axis));

    // Out of turn: the engine's own refusal, nothing stored.
    let cw_order = &log
        .orders
        .iter()
        .find(|o| o.player == Side::Cw)
        .unwrap()
        .order;
    let e = games.apply(id, cw, Side::Cw, cw_order, 1).await;
    assert!(
        matches!(&e, Err(GameError::Illegal(i)) if i.case == "5.2"),
        "{e:?}"
    );
    let sql = "SELECT count(*) FROM orders WHERE game_id = $1";
    assert_eq!(count(&pool, sql, id).await, 1);

    // A seq already taken.
    let e = games
        .apply(id, axis, Side::Axis, &log.orders[1].order, 0)
        .await;
    assert!(matches!(e, Err(GameError::Conflict { seq: 1 })), "{e:?}");
    assert_eq!(count(&pool, sql, id).await, 1);
    assert_eq!(games.get(id).await.unwrap().seq(), 1);
}
