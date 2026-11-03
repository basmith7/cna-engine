//! Admins (`CNA_ADMINS`) and suspension: a suspended account's sessions
//! and streams end, and it cannot sign in again until restored.
//! Needs `DATABASE_URL` and `--features db-tests,test-auth,fake`.
#![cfg(all(feature = "test-auth", feature = "fake"))]

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use cna_data::scenario::Side;
use cna_server::games::{AccountId, Games};
use cna_server::{accounts, fake_discord};
use common::*;
use futures_util::StreamExt;
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;

const ADMIN: &str = "9";

async fn fake() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            fake_discord::router(fake_discord::default_users()),
        )
        .await
        .unwrap()
    });
    format!("http://{addr}")
}

fn server(pool: &PgPool, base: &str) -> Router {
    let (api, authorize) = (format!("{base}/api"), format!("{base}/oauth2/authorize"));
    router_with(
        Games::new(pool.clone()),
        &config(&[
            ("CNA_DISCORD_API", &api),
            ("CNA_DISCORD_AUTHORIZE", &authorize),
            ("CNA_ADMINS", ADMIN),
        ]),
    )
}

enum Who<'a> {
    Test(&'a str),
    Cookie(&'a str),
    Nobody,
}

async fn send(app: &Router, method: &str, uri: &str, who: Who<'_>) -> axum::response::Response {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("origin", PUBLIC_URL)
        .header("content-type", "application/json");
    req = match who {
        Who::Test(d) => req.header("x-cna-test-account", d),
        Who::Cookie(c) => req.header(header::COOKIE, c),
        Who::Nobody => req,
    };
    app.clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn text(res: axum::response::Response) -> (StatusCode, String) {
    let s = res.status();
    let b = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (s, String::from_utf8_lossy(&b).into_owned())
}

fn cookie_of(res: &axum::response::Response) -> Option<String> {
    res.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap())
        .find(|v| v.starts_with("cna_session="))
        .map(|v| v.split(';').next().unwrap().to_owned())
}

/// Signs `user` in through the fake: the callback's response and the
/// cookie the sign-in left.
async fn sign_in(app: &Router, user: &str) -> (StatusCode, String, Option<String>) {
    let res = send(app, "GET", "/auth/discord", Who::Nobody).await;
    let started = cookie_of(&res).unwrap();
    let to = res.headers()[header::LOCATION].to_str().unwrap().to_owned();
    let page = reqwest::get(&to).await.unwrap().text().await.unwrap();
    let link = page
        .split("<a href=\"")
        .skip(1)
        .find(|a| a.contains(&format!(">{user}</a>")))
        .unwrap();
    let back = url::Url::parse(&link[..link.find('"').unwrap()].replace("&amp;", "&")).unwrap();
    let uri = format!("{}?{}", back.path(), back.query().unwrap());
    let res = send(app, "GET", &uri, Who::Cookie(&started)).await;
    let cookie = cookie_of(&res);
    let (s, body) = text(res).await;
    (s, body, cookie)
}

async fn account_of(pool: &PgPool, discord_id: &str) -> AccountId {
    sqlx::query_scalar("SELECT id FROM accounts WHERE discord_id = $1")
        .bind(discord_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn only_admins_see_the_accounts(pool: PgPool) {
    accounts::insert(&pool, ADMIN, "Admin").await.unwrap();
    accounts::insert(&pool, "2", "Player").await.unwrap();
    let app = server(&pool, "http://127.0.0.1:9");
    let (s, body) = text(send(&app, "GET", "/admin/accounts", Who::Test(ADMIN)).await).await;
    assert_eq!(s, StatusCode::OK);
    let list: Vec<Value> = serde_json::from_str(&body).unwrap();
    let names: Vec<_> = list.iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Admin", "Player"]);
    assert_eq!(list[0]["admin"], true);
    assert_eq!(list[1]["suspended"], false);
    let me: Value = serde_json::from_str(
        &text(send(&app, "GET", "/api/me", Who::Test(ADMIN)).await)
            .await
            .1,
    )
    .unwrap();
    assert_eq!(me["admin"], true);

    let s = send(&app, "GET", "/admin/accounts", Who::Test("2"))
        .await
        .status();
    assert_eq!(s, StatusCode::FORBIDDEN);
    let s = send(&app, "GET", "/admin/accounts", Who::Nobody)
        .await
        .status();
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let other = account_of(&pool, ADMIN).await;
    let s = send(
        &app,
        "POST",
        &format!("/admin/accounts/{other}/suspend"),
        Who::Test("2"),
    )
    .await
    .status();
    assert_eq!(s, StatusCode::FORBIDDEN, "a player cannot suspend");
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn suspending_ends_sessions_and_streams_and_refuses_sign_in(pool: PgPool) {
    accounts::insert(&pool, ADMIN, "Admin").await.unwrap();
    let base = fake().await;
    let app = server(&pool, &base);
    let (s, _, cookie) = sign_in(&app, "alice").await;
    assert_eq!(s, StatusCode::FOUND);
    let cookie = cookie.unwrap();
    let (_, _, second) = sign_in(&app, "alice").await;
    let second = second.unwrap();
    let alice = account_of(&pool, "1001").await;

    // Alice holds the Axis seat of a game and watches it.
    let games = Games::new(pool.clone());
    let game = games
        .create(cna_embed::DEFAULT_SCENARIO, alice, false)
        .await
        .unwrap();
    games.seat(game, Side::Axis, alice).await.unwrap();
    let res = send(
        &app,
        "GET",
        &format!("/api/games/{game}/stream"),
        Who::Cookie(&cookie),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let mut stream = res.into_body().into_data_stream();

    let (s, body) = text(
        send(
            &app,
            "POST",
            &format!("/admin/accounts/{alice}/suspend"),
            Who::Test(ADMIN),
        )
        .await,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{body}");

    let end = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(chunk) = stream.next().await {
            chunk.unwrap();
        }
    })
    .await;
    assert!(end.is_ok(), "the open stream is closed");
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions WHERE account_id = $1")
        .bind(alice)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sessions, 0);
    for c in [&cookie, &second] {
        let s = send(&app, "GET", "/api/me", Who::Cookie(c)).await.status();
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    let (s, page, signed_in) = sign_in(&app, "alice").await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert!(page.contains("suspended"), "{page}");
    let pending = signed_in.unwrap();
    let s = send(&app, "GET", "/api/me", Who::Cookie(&pending))
        .await
        .status();
    assert_eq!(
        s,
        StatusCode::UNAUTHORIZED,
        "a refused sign-in signs nobody in"
    );
    let list: Vec<Value> = serde_json::from_str(
        &text(send(&app, "GET", "/admin/accounts", Who::Test(ADMIN)).await)
            .await
            .1,
    )
    .unwrap();
    assert!(
        list.iter()
            .any(|a| a["id"] == alice && a["suspended"] == true)
    );
    assert_eq!(
        status(&pool, game).await,
        "open",
        "her games stay as they are"
    );
    assert_eq!(games.seat_of(game, alice).await.unwrap(), Some(Side::Axis));

    let s = send(
        &app,
        "POST",
        &format!("/admin/accounts/{alice}/restore"),
        Who::Test(ADMIN),
    )
    .await
    .status();
    assert_eq!(s, StatusCode::OK);
    let (s, _, cookie) = sign_in(&app, "alice").await;
    assert_eq!(s, StatusCode::FOUND);
    let s = send(&app, "GET", "/api/me", Who::Cookie(&cookie.unwrap()))
        .await
        .status();
    assert_eq!(s, StatusCode::OK);

    let s = send(
        &app,
        "POST",
        "/admin/accounts/999999/suspend",
        Who::Test(ADMIN),
    )
    .await
    .status();
    assert_eq!(s, StatusCode::NOT_FOUND);
}
