//! The `test-auth` routes the two-player e2e leans on: a lobby game on a
//! chosen seed, a game's state hash, and the leak predicate.
//! Needs `DATABASE_URL` and `--features db-tests,test-auth`.
#![cfg(feature = "test-auth")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_next_lobby_game_takes_the_seed_once(pool: PgPool) {
    let games = cna_server::games::Games::new(pool.clone());
    cna_server::accounts::insert(&pool, "1", "Axis")
        .await
        .unwrap();
    let app = router(games);
    let (status, _) = call(
        &app,
        "POST",
        "/test/next-seed",
        None,
        Some(json!({"seed": 7})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut seeds = Vec::new();
    for _ in 0..2 {
        let (status, body) = call(
            &app,
            "POST",
            "/api/games",
            Some("1"),
            Some(json!({"side": "axis"})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let seed: i64 = sqlx::query_scalar("SELECT seed FROM games WHERE id = $1")
            .bind(body["id"].as_i64().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
        seeds.push(seed);
    }
    assert_eq!(seeds[0], 7);
    assert_ne!(seeds[1], 7, "the seed is for one game");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_hash_is_the_states_and_the_leak_check_finds_a_leak(pool: PgPool) {
    let (games, id, axis, cw) = seated(&pool).await;
    play(&games, id, axis, cw, 3).await;
    let app = router(games);

    let res = app
        .clone()
        .oneshot(
            Request::get(format!("/test/games/{id}/hash"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let hash = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&hash),
        format!("{:x}", Sha256::digest(expected(3).as_bytes()))
    );

    let leaks = format!("/test/games/{id}/leaks?about=axis");
    let (status, found) = call(&app, "POST", &leaks, None, Some(json!({"view": {}}))).await;
    assert_eq!((status, found), (StatusCode::OK, json!([])));
    let (_, found) = call(
        &app,
        "POST",
        &leaks,
        None,
        Some(json!({"combat": {"plots": {"axis": []}}})),
    )
    .await;
    assert_eq!(found, json!(["combat.plots.axis"]));
}
