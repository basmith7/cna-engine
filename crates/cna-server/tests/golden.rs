//! The module-on golden game played to its end over the HTTP API, each
//! order posted from its own side's seat; the finished game's save must
//! hash to the state `cna-game` pins for the same orders.
//! Needs `DATABASE_URL` and `--features db-tests,test-auth`.
#![cfg(feature = "test-auth")]

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use cna_data::scenario::Side;
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
async fn golden_game_over_http(pool: PgPool) {
    let (games, id, _, _) = seated(&pool).await;
    let app = router_for_play(games);
    let send = |method: &str, uri: String, who: &str, body: Body| {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("x-cna-test-account", who)
            .header("origin", PUBLIC_URL)
            .header("content-type", "application/json")
            .body(body)
            .unwrap();
        app.clone().oneshot(req)
    };
    let save = format!("/api/games/{id}/save");

    let log = golden();
    for (seq, o) in log.orders.iter().enumerate() {
        if seq == 0 {
            let res = send("GET", save.clone(), "1", Body::empty()).await.unwrap();
            assert_eq!(res.status(), StatusCode::CONFLICT, "a save before the end");
        }
        let who = if o.player == Side::Axis { "1" } else { "2" };
        let body = json!({"side": o.player, "order": o.order, "seq": seq});
        let res = send(
            "POST",
            format!("/api/games/{id}/orders"),
            who,
            Body::from(body.to_string()),
        )
        .await
        .unwrap();
        let status = res.status();
        let text = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "order {seq}: {}",
            String::from_utf8_lossy(&text)
        );
    }

    let pinned =
        std::fs::read_to_string(cna_data::repo_root().join("tests/games/graziani-m5.sha256"))
            .unwrap();
    for who in ["1", "2"] {
        let res = send("GET", save.clone(), who, Body::empty()).await.unwrap();
        assert_eq!(
            res.status(),
            StatusCode::OK,
            "the save once the game is over"
        );
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            pinned.trim(),
            "the save as {who} sees it"
        );
    }
}
