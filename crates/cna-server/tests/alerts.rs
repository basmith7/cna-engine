//! Turn alerts: the outbox `Games::apply` writes, and the sender that
//! DMs it through the fake Discord. Needs `DATABASE_URL` and
//! `--features db-tests,fake`.
#![cfg(feature = "fake")]

mod common;

use cna_data::scenario::Side;
use cna_game::state::GameState;
use cna_server::alerts::Sender;
use cna_server::games::{AccountId, GameError, GameId, Games};
use common::*;
use serde_json::{Value, json};
use sqlx::PgPool;

/// The golden orders after which the move passes to `side`: their
/// indices, in order.
fn handovers(side: Side) -> Vec<usize> {
    let log = golden();
    let data = cna_embed::data(&log.scenario).unwrap();
    let mut s = GameState::with_modules(data, log.seed, log.modules).unwrap();
    let mut out = Vec::new();
    for (i, o) in log.orders.iter().enumerate() {
        let before = s.to_act();
        s.apply(data, o.player, o.order.clone()).unwrap();
        if s.to_act() == Some(side) && before != Some(side) {
            out.push(i);
        }
        if out.len() == 2 {
            break;
        }
    }
    out
}

/// The alerts rows for `account`: `(game, seq)`, oldest first.
async fn alerts_for(pool: &PgPool, account: AccountId) -> Vec<(GameId, i64)> {
    sqlx::query_as("SELECT game_id, seq FROM alerts WHERE account_id = $1 ORDER BY id")
        .bind(account)
        .fetch_all(pool)
        .await
        .unwrap()
}

/// Plays golden orders `from..to` (`seated` games, each order as its side).
async fn play_range(
    games: &Games,
    id: GameId,
    axis: AccountId,
    cw: AccountId,
    from: usize,
    to: usize,
) {
    for (i, o) in golden().orders.iter().enumerate().take(to).skip(from) {
        let account = if o.player == Side::Axis { axis } else { cw };
        games
            .apply(id, account, o.player, &o.order, i as i64)
            .await
            .unwrap_or_else(|e| panic!("order {i}: {e}"));
    }
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_order_handing_over_the_move_writes_one_alert(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    let n = handovers(Side::Cw)[0];
    play(&games, id, axis, cw, n).await;
    assert!(
        alerts_for(&pool, cw).await.is_empty(),
        "before the handover"
    );
    play_range(&games, id, axis, cw, n, n + 1).await;
    assert_eq!(alerts_for(&pool, cw).await, vec![(id, n as i64)]);
    assert!(alerts_for(&pool, axis).await.is_empty());
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn an_illegal_order_writes_none(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    let n = handovers(Side::Cw)[0];
    play(&games, id, axis, cw, n).await;
    // The order before the handover again: already done, so refused.
    let again = &golden().orders[n - 1];
    let account = if again.player == Side::Axis { axis } else { cw };
    let r = games
        .apply(id, account, again.player, &again.order, n as i64)
        .await;
    assert!(matches!(r, Err(GameError::Illegal(_))), "{r:?}");
    let all: i64 = sqlx::query_scalar("SELECT count(*) FROM alerts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(all, 0);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_rolled_back_order_leaves_no_alert_and_a_failed_alert_no_order(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    let n = handovers(Side::Cw)[0];
    play(&games, id, axis, cw, n).await;
    // The alert's INSERT fails: the order goes with it.
    sqlx::query(
        "CREATE FUNCTION refuse() RETURNS trigger LANGUAGE plpgsql AS \
         $$ BEGIN RAISE EXCEPTION 'refused'; END $$",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER refuse BEFORE INSERT ON alerts FOR EACH ROW EXECUTE FUNCTION refuse()",
    )
    .execute(&pool)
    .await
    .unwrap();
    let o = &golden().orders[n];
    let r = games.apply(id, axis, o.player, &o.order, n as i64).await;
    assert!(matches!(r, Err(GameError::Db(_))), "{r:?}");
    assert_eq!(
        count(&pool, "SELECT count(*) FROM orders WHERE game_id = $1", id).await,
        n as i64
    );
    sqlx::query("DROP TRIGGER refuse ON alerts")
        .execute(&pool)
        .await
        .unwrap();
    assert!(alerts_for(&pool, cw).await.is_empty());
    // Retried, the order and its alert land together.
    play_range(&games, id, axis, cw, n, n + 1).await;
    assert_eq!(alerts_for(&pool, cw).await, vec![(id, n as i64)]);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_second_handover_within_15_minutes_writes_none(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    let h = handovers(Side::Cw);
    assert_eq!(h.len(), 2, "the golden game hands Cw the move twice");
    play(&games, id, axis, cw, h[1] + 1).await;
    assert_eq!(alerts_for(&pool, cw).await, vec![(id, h[0] as i64)]);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_second_handover_after_15_minutes_writes_another(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    let h = handovers(Side::Cw);
    play(&games, id, axis, cw, h[0] + 1).await;
    sqlx::query("UPDATE alerts SET created_at = now() - interval '16 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    play_range(&games, id, axis, cw, h[0] + 1, h[1] + 1).await;
    assert_eq!(
        alerts_for(&pool, cw).await,
        vec![(id, h[0] as i64), (id, h[1] as i64)]
    );
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn none_while_the_seat_watches_the_game_or_has_alerts_off(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    let h = handovers(Side::Cw);
    play(&games, id, axis, cw, h[0]).await;
    {
        let _watching = games.streams().open(cw, id).unwrap();
        play_range(&games, id, axis, cw, h[0], h[0] + 1).await;
    }
    assert!(alerts_for(&pool, cw).await.is_empty());
    // The stream closed: the next handover alerts, then alerts off, none.
    play_range(&games, id, axis, cw, h[0] + 1, h[1] + 1).await;
    assert_eq!(alerts_for(&pool, cw).await, vec![(id, h[1] as i64)]);

    sqlx::query("UPDATE accounts SET alerts = false WHERE id = $1")
        .bind(cw)
        .execute(&pool)
        .await
        .unwrap();
    let again = games
        .create_seeded(&golden().scenario, axis, false, golden().seed)
        .await
        .unwrap();
    games.seat(again, Side::Axis, axis).await.unwrap();
    games.seat(again, Side::Cw, cw).await.unwrap();
    play(&games, again, axis, cw, h[0] + 1).await;
    assert_eq!(alerts_for(&pool, cw).await.len(), 1);
}

// The sender.

/// A sender against the fake at `base` with `token`.
fn sender(pool: &PgPool, base: &str, token: &str) -> Sender {
    Sender::new(
        pool.clone(),
        format!("{base}/api"),
        token.to_owned(),
        PUBLIC_URL.to_owned(),
    )
    .unwrap()
}

async fn sent(base: &str) -> Vec<Value> {
    reqwest::get(format!("{base}/_sent"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn fail(base: &str, fault: Value) {
    let r = reqwest::Client::new()
        .post(format!("{base}/_fail"))
        .json(&fault)
        .send()
        .await
        .unwrap();
    assert!(r.status().is_success());
}

/// One alert for Cw's seat (Discord id `2`), handed over by golden play.
async fn one_alert(pool: &PgPool) -> (GameId, AccountId) {
    let (games, id, axis, cw) = seated(pool).await;
    play(&games, id, axis, cw, handovers(Side::Cw)[0] + 1).await;
    (id, cw)
}

type Row = (bool, i32, Option<String>, bool, f64);

/// The one alert: sent_at, attempts, last_error, given_up_at, and
/// seconds from now to next_attempt_at.
async fn the_alert(pool: &PgPool) -> Row {
    sqlx::query_as(
        "SELECT sent_at IS NOT NULL, attempts, last_error, given_up_at IS NOT NULL, \
         extract(epoch FROM next_attempt_at - now())::float8 FROM alerts",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_sender_dms_the_link_once(pool: PgPool) {
    let token = bot_token();
    let base = fake_discord_with(Some(token.clone())).await;
    let (id, _) = one_alert(&pool).await;
    let s = sender(&pool, &base, &token);
    assert_eq!(s.send_due().await.unwrap(), 1);
    assert_eq!(s.send_due().await.unwrap(), 0);
    let dms = sent(&base).await;
    assert_eq!(dms.len(), 1, "{dms:?}");
    assert_eq!(dms[0]["channel"], "dm-2");
    let text = dms[0]["content"].as_str().unwrap();
    assert!(
        text.starts_with("Your move in Graziani's Offensive against Axis, Game-Turn "),
        "{text}"
    );
    assert!(
        text.ends_with(&format!(": {PUBLIC_URL}/game/{id}")),
        "{text}"
    );
    assert!(the_alert(&pool).await.0, "sent_at set");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn two_senders_at_once_send_an_alert_once(pool: PgPool) {
    let token = bot_token();
    let base = fake_discord_with(Some(token.clone())).await;
    one_alert(&pool).await;
    let (a, b) = (sender(&pool, &base, &token), sender(&pool, &base, &token));
    let (x, y) = tokio::join!(a.send_due(), b.send_due());
    assert_eq!(x.unwrap() + y.unwrap(), 1);
    assert_eq!(sent(&base).await.len(), 1);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_429_waits_retry_after_without_counting_an_attempt(pool: PgPool) {
    let token = bot_token();
    let base = fake_discord_with(Some(token.clone())).await;
    one_alert(&pool).await;
    fail(
        &base,
        json!({"user": "2", "status": 429, "retry_after": 42.5}),
    )
    .await;
    let s = sender(&pool, &base, &token);
    assert_eq!(s.send_due().await.unwrap(), 0);
    let (sent_at, attempts, _, given_up, wait) = the_alert(&pool).await;
    assert_eq!((sent_at, attempts, given_up), (false, 0, false));
    assert!((41.0..=43.0).contains(&wait), "{wait}");
    // Not due yet; once due, it goes.
    assert_eq!(s.send_due().await.unwrap(), 0);
    sqlx::query("UPDATE alerts SET next_attempt_at = now()")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(s.send_due().await.unwrap(), 1);
    assert_eq!(sent(&base).await.len(), 1);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn other_errors_back_off_doubling_from_30_s(pool: PgPool) {
    let token = bot_token();
    let base = fake_discord_with(Some(token.clone())).await;
    one_alert(&pool).await;
    fail(&base, json!({"user": "2", "status": 500, "times": 2})).await;
    let s = sender(&pool, &base, &token);
    assert_eq!(s.send_due().await.unwrap(), 0);
    let (_, attempts, err, _, wait) = the_alert(&pool).await;
    assert_eq!(attempts, 1);
    assert!(err.unwrap().contains("500"));
    assert!((29.0..=31.0).contains(&wait), "{wait}");
    sqlx::query("UPDATE alerts SET next_attempt_at = now()")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(s.send_due().await.unwrap(), 0);
    let (_, attempts, _, given_up, wait) = the_alert(&pool).await;
    assert_eq!((attempts, given_up), (2, false));
    assert!((59.0..=61.0).contains(&wait), "{wait}");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_wrong_bot_token_fails_and_after_24_hours_it_gives_up(pool: PgPool) {
    let base = fake_discord_with(Some(bot_token())).await;
    one_alert(&pool).await;
    sqlx::query("UPDATE alerts SET created_at = now() - interval '25 hours'")
        .execute(&pool)
        .await
        .unwrap();
    let s = sender(&pool, &base, &bot_token());
    assert_eq!(s.send_due().await.unwrap(), 0);
    let (sent_at, _, err, given_up, _) = the_alert(&pool).await;
    assert!(!sent_at && given_up);
    assert!(err.unwrap().contains("401"));
    assert!(sent(&base).await.is_empty());
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_403_marks_the_account_unreachable_and_stops_until_a_dm_lands(pool: PgPool) {
    let token = bot_token();
    let base = fake_discord_with(Some(token.clone())).await;
    let (_, cw) = one_alert(&pool).await;
    fail(&base, json!({"user": "2", "status": 403})).await;
    let s = sender(&pool, &base, &token);
    assert_eq!(s.send_due().await.unwrap(), 0);
    let unreachable = || async {
        sqlx::query_scalar::<_, bool>(
            "SELECT alerts_unreachable_at IS NOT NULL FROM accounts WHERE id = $1",
        )
        .bind(cw)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    assert!(unreachable().await);
    assert!(the_alert(&pool).await.3, "given up");
    sqlx::query("UPDATE alerts SET next_attempt_at = now()")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(s.send_due().await.unwrap(), 0, "no retry");
    assert!(sent(&base).await.is_empty());

    // A later alert that lands clears the mark.
    sqlx::query("INSERT INTO alerts (account_id, game_id, seq) SELECT account_id, game_id, seq + 1 FROM alerts")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(s.send_due().await.unwrap(), 1);
    assert!(!unreachable().await);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn no_dm_once_the_game_is_no_longer_active_or_the_move_has_passed(pool: PgPool) {
    let token = bot_token();
    let base = fake_discord_with(Some(token.clone())).await;
    let (id, _) = one_alert(&pool).await;
    sqlx::query("UPDATE games SET status = 'abandoned' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let s = sender(&pool, &base, &token);
    assert_eq!(s.send_due().await.unwrap(), 0);
    assert!(
        sent(&base).await.is_empty(),
        "an abandoned game sends nothing"
    );

    // Active again, but an order after the alert's: the move has passed.
    sqlx::query("UPDATE games SET status = 'active' WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE alerts SET given_up_at = NULL, seq = -1 WHERE game_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(s.send_due().await.unwrap(), 0);
    assert!(
        sent(&base).await.is_empty(),
        "a move already answered sends nothing"
    );
}
