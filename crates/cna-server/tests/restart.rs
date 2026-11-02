//! Loading a game back: after a restart, from snapshots or the seed,
//! under other rules, and when its orders no longer replay; eviction.
//! Needs `DATABASE_URL` and `--features db-tests`.

mod common;

use cna_data::scenario::Side;
use cna_embed::RULES_COMMIT;
use cna_server::accounts;
use cna_server::games::{GameError, Games};
use common::*;
use sqlx::PgPool;
use std::time::Duration;

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_restart_reads_the_game_back_from_snapshots_or_by_replay(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    play(&games, id, axis, cw, 120).await;
    let want = expected(120);
    assert_eq!(games.get(id).await.unwrap().state.to_json().unwrap(), want);
    let snapshots = "SELECT count(*) FROM snapshots WHERE game_id = $1";
    assert_eq!(count(&pool, snapshots, id).await, 2, "at 50 and 100");
    drop(games);

    // From the snapshot at 100 and 20 orders.
    let state = Games::new(pool.clone())
        .get(id)
        .await
        .unwrap()
        .state
        .to_json()
        .unwrap();
    assert_eq!(state, want);

    // A snapshot is trusted only while its log is the stored orders'
    // prefix: with order 10 changed, it is set aside and the replay
    // from the seed finds the order refused.
    let flip = "UPDATE orders SET side = CASE side WHEN 'axis' THEN 'cw' ELSE 'axis' END \
                WHERE seq = 10";
    sqlx::query(flip).execute(&pool).await.unwrap();
    let e = Games::new(pool.clone()).get(id).await.err();
    assert!(
        matches!(e, Some(GameError::Stale)),
        "a snapshot whose log differs from the stored orders is not trusted: {e:?}"
    );
    sqlx::query(flip).execute(&pool).await.unwrap();
    sqlx::query("UPDATE games SET status = 'active'")
        .execute(&pool)
        .await
        .unwrap();

    // From the seed.
    sqlx::query("DELETE FROM snapshots")
        .execute(&pool)
        .await
        .unwrap();
    let state = Games::new(pool.clone())
        .get(id)
        .await
        .unwrap()
        .state
        .to_json()
        .unwrap();
    assert_eq!(state, want);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_game_made_under_other_rules_is_replayed_and_moved(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    play(&games, id, axis, cw, 120).await;
    drop(games);
    let other = "0".repeat(40);
    for sql in [
        "UPDATE games SET rules_commit = $1",
        "UPDATE snapshots SET rules_commit = $1",
    ] {
        sqlx::query(sql).bind(&other).execute(&pool).await.unwrap();
    }
    let state = Games::new(pool.clone())
        .get(id)
        .await
        .unwrap()
        .state
        .to_json()
        .unwrap();
    assert_eq!(state, expected(120));
    let commit: String = sqlx::query_scalar("SELECT rules_commit FROM games WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(commit, RULES_COMMIT);
    assert_eq!(status(&pool, id).await, "active");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_game_whose_orders_no_longer_replay_is_stale(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    play(&games, id, axis, cw, 120).await;
    drop(games);
    // Order 110 (after the last snapshot) given by the side not to act.
    sqlx::query(
        "UPDATE orders SET side = CASE side WHEN 'axis' THEN 'cw' ELSE 'axis' END \
         WHERE seq = 110",
    )
    .execute(&pool)
    .await
    .unwrap();
    let games = Games::new(pool.clone());
    let e = games.get(id).await.err();
    assert!(matches!(e, Some(GameError::Stale)), "{e:?}");
    assert_eq!(status(&pool, id).await, "stale");
    let e = games
        .apply(id, axis, Side::Axis, &golden().orders[120].order, 120)
        .await;
    assert!(matches!(e, Err(GameError::Stale)), "{e:?}");

    // An order row that is not an order at all: stale too, not a panic.
    let (games, id, axis, cw) = {
        let games = Games::new(pool.clone());
        let a = accounts::insert(&pool, "3", "C").await.unwrap();
        let b = accounts::insert(&pool, "4", "D").await.unwrap();
        let id = games
            .create_seeded("grazianis-offensive", a, false, 7)
            .await
            .unwrap();
        games.seat(id, Side::Axis, a).await.unwrap();
        games.seat(id, Side::Cw, b).await.unwrap();
        (games, id, a, b)
    };
    play(&games, id, axis, cw, 3).await;
    drop(games);
    sqlx::query(
        r#"UPDATE orders SET "order" = '{"no-such-order": 1}' WHERE game_id = $1 AND seq = 1"#,
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    let e = Games::new(pool.clone()).get(id).await.err();
    assert!(matches!(e, Some(GameError::Stale)), "{e:?}");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn idle_games_are_evicted_unless_streamed(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    play(&games, id, axis, cw, 1).await;
    assert_eq!(games.live_count(), 1);
    assert_eq!(games.evict_idle(Duration::from_secs(60)), 0, "not idle yet");

    // A held stream keeps it.
    let rx = games.get(id).await.unwrap().tx.subscribe();
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(games.evict_idle(Duration::from_millis(10)), 0);
    drop(rx);

    let evictor = games.spawn_evictor(Duration::from_millis(10), Duration::from_millis(20));
    tokio::time::sleep(Duration::from_millis(100)).await;
    evictor.abort();
    assert_eq!(games.live_count(), 0);
    // And it comes back as it was.
    assert_eq!(
        games.get(id).await.unwrap().state.to_json().unwrap(),
        expected(1)
    );
}
