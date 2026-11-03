//! The lobby over HTTP: making, listing, joining and leaving games, and
//! the sweep of idle ones. Needs `DATABASE_URL` and
//! `--features db-tests,test-auth`.
#![cfg(feature = "test-auth")]

mod common;

use axum::Router;
use axum::http::StatusCode;
use cna_server::accounts;
use cna_server::games::{GameId, Games};
use common::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

/// Alice, Bob and Carol, signed up; the router over their games.
async fn lobby(pool: &PgPool) -> Router {
    for (d, n) in [("10", "Alice"), ("11", "Bob"), ("12", "Carol")] {
        accounts::insert(pool, d, n).await.unwrap();
    }
    router(Games::new(pool.clone()))
}

const ALICE: &str = "10";
const BOB: &str = "11";
const CAROL: &str = "12";

async fn create(app: &Router, who: &str, body: Value) -> (GameId, Value) {
    let (s, made) = call(app, "POST", "/api/games", Some(who), Some(body)).await;
    assert_eq!(s, StatusCode::CREATED, "{made}");
    (made["id"].as_i64().unwrap(), made)
}

async fn join(app: &Router, who: &str, id: GameId, body: Value) -> (StatusCode, Value) {
    call(
        app,
        "POST",
        &format!("/api/games/{id}/join"),
        Some(who),
        Some(body),
    )
    .await
}

async fn mine(app: &Router, who: &str) -> Value {
    let (s, b) = call(app, "GET", "/api/games?mine", Some(who), None).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    b
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn either_side_is_decided_at_join(pool: PgPool) {
    let app = lobby(&pool).await;
    for (joiner_side, creator_side) in [("cw", "axis"), ("axis", "cw")] {
        let (id, _) = create(&app, ALICE, json!({"side": "either", "open": true})).await;
        assert_eq!(status(&pool, id).await, "open");
        let m = mine(&app, ALICE).await;
        let g = m
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == id)
            .unwrap();
        assert_eq!(g["my_side"], Value::Null, "undecided until a join: {g}");
        assert_eq!(g["my_move"], false);

        let (s, b) = join(&app, BOB, id, json!({"side": joiner_side})).await;
        assert_eq!(s, StatusCode::OK, "{b}");
        assert_eq!(status(&pool, id).await, "active");
        let m = mine(&app, ALICE).await;
        let g = m
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["id"] == id)
            .unwrap();
        assert_eq!(g["my_side"], creator_side, "{g}");
        assert_eq!(g["opponent"], "Bob");
    }
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_invite_token_is_shown_once_and_stored_hashed(pool: PgPool) {
    let app = lobby(&pool).await;
    let (id, made) = create(&app, ALICE, json!({"side": "axis", "open": false})).await;
    let token = made["invite"].as_str().expect("an invite token").to_owned();
    assert!(token.len() >= 32, "{token}");
    let stored: Vec<u8> = sqlx::query_scalar("SELECT invite_token_hash FROM games WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, Sha256::digest(token.as_bytes()).to_vec());

    // No later answer carries it.
    let m = mine(&app, ALICE).await;
    assert!(!m.to_string().contains(&token), "{m}");
    let (_, g) = call(&app, "GET", &format!("/api/games/{id}"), Some(ALICE), None).await;
    assert!(!g.to_string().contains(&token));
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn open_lists_public_waiting_games_without_tokens(pool: PgPool) {
    let app = lobby(&pool).await;
    let (public, made) = create(&app, ALICE, json!({"side": "axis", "open": true})).await;
    let token = made["invite"].as_str().unwrap().to_owned();
    let (private, _) = create(&app, ALICE, json!({"side": "cw", "open": false})).await;
    let (either, _) = create(&app, ALICE, json!({"side": "either", "open": true})).await;

    let (s, open) = call(&app, "GET", "/api/games?open", Some(BOB), None).await;
    assert_eq!(s, StatusCode::OK);
    let ids: Vec<i64> = open
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["id"].as_i64().unwrap())
        .collect();
    assert!(ids.contains(&public) && ids.contains(&either), "{open}");
    assert!(!ids.contains(&private), "a private game is not listed");
    assert!(!open.to_string().contains(&token));
    assert!(!open.to_string().contains("invite"), "{open}");
    let e = open
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == either)
        .unwrap();
    assert_eq!(e["creator"], "Alice");
    assert_eq!(e["creator_side"], "either");

    let (_, own) = call(&app, "GET", "/api/games?open", Some(ALICE), None).await;
    assert_eq!(
        own,
        json!([]),
        "not one's own games, the either one included"
    );
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn joins_are_refused_for_ones_own_game_a_taken_side_or_no_token(pool: PgPool) {
    let app = lobby(&pool).await;
    let (id, _) = create(&app, ALICE, json!({"side": "axis", "open": true})).await;
    let (s, _) = join(&app, ALICE, id, json!({"side": "cw"})).await;
    assert_eq!(s, StatusCode::CONFLICT, "one's own game");
    let (s, _) = join(&app, BOB, id, json!({"side": "axis"})).await;
    assert_eq!(s, StatusCode::CONFLICT, "a taken side");

    let (either, _) = create(&app, ALICE, json!({"side": "either", "open": true})).await;
    let (s, _) = join(&app, ALICE, either, json!({"side": "cw"})).await;
    assert_eq!(s, StatusCode::CONFLICT, "one's own either game");

    let (private, made) = create(&app, ALICE, json!({"side": "axis", "open": false})).await;
    let token = made["invite"].as_str().unwrap();
    let (s, _) = join(&app, BOB, private, json!({"side": "cw"})).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "no token");
    let (s, _) = join(&app, BOB, private, json!({"side": "cw", "invite": "x"})).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "a wrong token");
    let (s, b) = join(&app, BOB, private, json!({"side": "cw", "invite": token})).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(status(&pool, private).await, "active");
    let (s, _) = join(&app, CAROL, private, json!({"side": "cw", "invite": token})).await;
    assert_eq!(s, StatusCode::CONFLICT, "the game is full");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn mine_says_whose_move_the_clock_and_the_opponent(pool: PgPool) {
    let app = lobby(&pool).await;
    let (id, _) = create(&app, ALICE, json!({"side": "axis", "open": true})).await;
    let m = mine(&app, ALICE).await;
    assert_eq!(m[0]["opponent"], Value::Null, "{m}");
    assert_eq!(m[0]["status"], "open");
    assert_eq!(
        m[0]["my_move"], false,
        "nobody moves before the game starts"
    );

    join(&app, BOB, id, json!({"side": "cw"})).await;
    let (a, b) = (mine(&app, ALICE).await, mine(&app, BOB).await);
    let (a, b) = (&a[0], &b[0]);
    assert_eq!((a["id"].as_i64(), b["id"].as_i64()), (Some(id), Some(id)));
    assert_eq!(
        (&a["my_side"], &b["my_side"]),
        (&json!("axis"), &json!("cw"))
    );
    assert_eq!(
        (&a["opponent"], &b["opponent"]),
        (&json!("Bob"), &json!("Alice"))
    );
    assert_eq!(a["status"], "active");
    let to_act = a["to_act"].as_str().expect("someone acts first");
    assert_eq!(a["my_move"], to_act == "axis");
    assert_eq!(b["my_move"], to_act == "cw");
    assert_eq!(a["clock"]["game_turn"], 1, "{a}");
    assert!(a["clock"]["stage"].is_number(), "{a}");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn leaving_abandons_the_game(pool: PgPool) {
    let app = lobby(&pool).await;
    let (id, _) = create(&app, ALICE, json!({"side": "axis", "open": true})).await;
    join(&app, BOB, id, json!({"side": "cw"})).await;
    let leave = format!("/api/games/{id}/leave");
    let (s, _) = call(&app, "POST", &leave, Some(CAROL), Some(json!({}))).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "not a player");
    assert_eq!(status(&pool, id).await, "active");
    let (s, b) = call(&app, "POST", &leave, Some(BOB), Some(json!({}))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(status(&pool, id).await, "abandoned");
    let (s, _) = call(&app, "POST", &leave, Some(ALICE), Some(json!({}))).await;
    assert_eq!(s, StatusCode::CONFLICT, "already over");

    // The creator of an either game, seatless, may withdraw it.
    let (either, _) = create(&app, ALICE, json!({"side": "either"})).await;
    let leave = format!("/api/games/{either}/leave");
    let (s, b) = call(&app, "POST", &leave, Some(ALICE), Some(json!({}))).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(status(&pool, either).await, "abandoned");
    let (s, _) = join(&app, BOB, either, json!({"side": "cw"})).await;
    assert_eq!(s, StatusCode::CONFLICT, "an abandoned game takes no one");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_sweep_abandons_active_games_idle_for_60_days(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    play(&games, id, axis, cw, 1).await;
    let now: i64 = sqlx::query_scalar("SELECT extract(epoch FROM now())::bigint")
        .fetch_one(&pool)
        .await
        .unwrap();
    let day = 24 * 60 * 60;

    assert_eq!(games.sweep_idle(now + 59 * day).await.unwrap(), 0);
    assert_eq!(status(&pool, id).await, "active");
    assert_eq!(games.sweep_idle(now + 61 * day).await.unwrap(), 1);
    assert_eq!(status(&pool, id).await, "abandoned");

    // An open game waiting for a player is not swept.
    let open = games
        .create_seeded(&golden().scenario, axis, false, 1)
        .await
        .unwrap();
    assert_eq!(games.sweep_idle(now + 61 * day).await.unwrap(), 0);
    assert_eq!(status(&pool, open).await, "open");
}
