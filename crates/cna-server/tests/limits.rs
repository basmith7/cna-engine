//! Open sign-up's abuse limits and CSRF: per-IP sign-in starts,
//! per-account orders and creations, the open-games cap, the `Origin`
//! and `Content-Type` check, the body cap.
//! Needs `DATABASE_URL` and `--features db-tests,test-auth`.
#![cfg(feature = "test-auth")]

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use cna_server::accounts;
use cna_server::games::Games;
use common::*;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

async fn status(app: &Router, req: axum::http::request::Builder, body: String) -> StatusCode {
    app.clone()
        .oneshot(req.body(Body::from(body)).unwrap())
        .await
        .unwrap()
        .status()
}

/// A JSON `method` to `uri` as `who`, from the public URL.
fn json_req(method: &str, uri: &str, who: &str) -> axum::http::request::Builder {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("x-cna-test-account", who)
        .header("origin", PUBLIC_URL)
        .header("content-type", "application/json")
}

fn start(forwarded: Option<&str>) -> axum::http::request::Builder {
    let r = Request::get("/auth/discord");
    match forwarded {
        Some(f) => r.header("x-forwarded-for", f),
        None => r,
    }
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_eleventh_sign_in_start_from_one_ip_in_a_minute_is_429(pool: PgPool) {
    let app = router(Games::new(pool.clone()));
    for i in 0..10 {
        // A forwarded address is not trusted by default: still 127.0.0.1.
        let s = status(&app, start(Some(&format!("10.0.0.{i}"))), String::new()).await;
        assert_eq!(s, StatusCode::FOUND, "start {i}");
    }
    let s = status(&app, start(Some("10.0.1.1")), String::new()).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn behind_a_trusted_proxy_the_forwarded_address_is_the_client(pool: PgPool) {
    let app = router_with(
        Games::new(pool.clone()),
        &config(&[("CNA_TRUST_PROXY", "1")]),
    );
    for _ in 0..10 {
        // The proxy appends the address it saw: the last one counts.
        let s = status(&app, start(Some("6.6.6.6, 10.0.0.1")), String::new()).await;
        assert_eq!(s, StatusCode::FOUND);
    }
    let s = status(&app, start(Some("7.7.7.7, 10.0.0.1")), String::new()).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    let s = status(&app, start(Some("6.6.6.6, 10.0.0.2")), String::new()).await;
    assert_eq!(s, StatusCode::FOUND, "another client");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_sixty_first_order_from_one_account_in_a_minute_is_429(pool: PgPool) {
    let (games, id, _, _) = seated(&pool).await;
    let app = router(games);
    let first = golden().orders[0].clone();
    let who = if first.player == cna_data::scenario::Side::Axis {
        "1"
    } else {
        "2"
    };
    let body = json!({
        "side": first.player,
        "order": first.order,
        "version": version(&app, id, who).await,
    })
    .to_string();
    let uri = format!("/api/games/{id}/orders");
    for i in 0..60 {
        let s = status(&app, json_req("POST", &uri, who), body.clone()).await;
        // The first is taken; the rest are stale.
        let want = if i == 0 {
            StatusCode::OK
        } else {
            StatusCode::CONFLICT
        };
        assert_eq!(s, want, "order {i}");
    }
    let s = status(&app, json_req("POST", &uri, who), body.clone()).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    let other = if who == "1" { "2" } else { "1" };
    let s = status(&app, json_req("POST", &uri, other), body).await;
    assert_ne!(
        s,
        StatusCode::TOO_MANY_REQUESTS,
        "the other account has its own budget"
    );
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn the_eleventh_game_created_in_an_hour_is_429(pool: PgPool) {
    accounts::insert(&pool, "10", "A").await.unwrap();
    let app = router(Games::new(pool.clone()));
    let body = json!({"side": "axis"}).to_string();
    for i in 0..10 {
        let s = status(&app, json_req("POST", "/api/games", "10"), body.clone()).await;
        assert_eq!(s, StatusCode::CREATED, "game {i}");
    }
    let s = status(&app, json_req("POST", "/api/games", "10"), body).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_twenty_first_open_game_is_409(pool: PgPool) {
    let a = accounts::insert(&pool, "10", "A").await.unwrap();
    let games = Games::new(pool.clone());
    for _ in 0..19 {
        games
            .create(cna_embed::DEFAULT_SCENARIO, a, false)
            .await
            .unwrap();
    }
    let app = router(games);
    let body = json!({"side": "axis"}).to_string();
    let s = status(&app, json_req("POST", "/api/games", "10"), body.clone()).await;
    assert_eq!(s, StatusCode::CREATED, "the twentieth");
    let s = status(&app, json_req("POST", "/api/games", "10"), body).await;
    assert_eq!(s, StatusCode::CONFLICT);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn state_changes_need_json_and_our_origin(pool: PgPool) {
    accounts::insert(&pool, "10", "A").await.unwrap();
    let app = router(Games::new(pool.clone()));
    let body = json!({"side": "axis"}).to_string();
    let base = || Request::post("/api/games").header("x-cna-test-account", "10");
    let cases = [
        ("no content type", base().header("origin", PUBLIC_URL)),
        (
            "a form",
            base()
                .header("origin", PUBLIC_URL)
                .header("content-type", "application/x-www-form-urlencoded"),
        ),
        (
            "a foreign origin",
            base()
                .header("origin", "https://evil.example")
                .header("content-type", "application/json"),
        ),
        (
            "no origin",
            base().header("content-type", "application/json"),
        ),
        (
            "a PATCH from elsewhere",
            Request::patch("/api/me")
                .header("x-cna-test-account", "10")
                .header("origin", "https://evil.example")
                .header("content-type", "application/json"),
        ),
        (
            "a sign-out without an origin",
            Request::post("/auth/sign-out").header("content-type", "application/json"),
        ),
    ];
    for (what, req) in cases {
        assert_eq!(
            status(&app, req, body.clone()).await,
            StatusCode::FORBIDDEN,
            "{what}"
        );
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM games")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);
    let s = status(&app, json_req("POST", "/api/games", "10"), body).await;
    assert_eq!(s, StatusCode::CREATED, "the same request done right");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_body_over_64_kb_is_413(pool: PgPool) {
    accounts::insert(&pool, "10", "A").await.unwrap();
    let app = router(Games::new(pool.clone()));
    let pad = "x".repeat(65 * 1024);
    let big = json!({"side": "axis", "pad": pad}).to_string();
    for uri in ["/api/games", "/auth/sign-out"] {
        // As a client sends it: with its length, which is refused
        // before a route that never reads the body runs.
        let req = json_req("POST", uri, "10").header("content-length", big.len());
        let s = status(&app, req, big.clone()).await;
        assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE, "{uri}");
    }
}
