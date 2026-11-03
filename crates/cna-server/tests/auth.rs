//! Sign-in with Discord, driven through the fake Discord in process.
//! Needs `DATABASE_URL` and `--features db-tests,fake`.
#![cfg(feature = "fake")]

mod common;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use cna_server::fake_discord;
use cna_server::games::Games;
use common::*;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

/// The fake Discord on a free port: its base URL.
pub async fn fake_discord() -> String {
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

/// The server pointed at the fake at `base`, with `vars` on top.
fn server(pool: &PgPool, base: &str, vars: &[(&str, &str)]) -> Router {
    let api = format!("{base}/api");
    let authorize = format!("{base}/oauth2/authorize");
    let mut all = vec![
        ("CNA_DISCORD_API", api.as_str()),
        ("CNA_DISCORD_AUTHORIZE", authorize.as_str()),
    ];
    all.extend_from_slice(vars);
    router_with(Games::new(pool.clone()), &config(&all))
}

/// A response's status, `Location`, `cna_session` `Set-Cookie` and body.
struct Res {
    status: StatusCode,
    location: Option<String>,
    set_cookie: Option<String>,
    body: String,
}

async fn send(app: &Router, method: &str, uri: &str, cookie: Option<&str>) -> Res {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("origin", PUBLIC_URL)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let h = |name: header::HeaderName| {
        res.headers()
            .get_all(&name)
            .iter()
            .map(|v| v.to_str().unwrap().to_owned())
            .find(|v| name != header::SET_COOKIE || v.starts_with("cna_session="))
    };
    let (location, set_cookie) = (h(header::LOCATION), h(header::SET_COOKIE));
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    Res {
        status,
        location,
        set_cookie,
        body: String::from_utf8_lossy(&body).into_owned(),
    }
}

/// `name=value` of a `Set-Cookie`.
fn pair(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap().to_owned()
}

/// Starts a sign-in: the authorize URL and the session cookie.
async fn start(app: &Router) -> (url::Url, String) {
    let r = send(app, "GET", "/auth/discord", None).await;
    assert_eq!(r.status, StatusCode::FOUND);
    let to = url::Url::parse(&r.location.unwrap()).unwrap();
    (to, pair(&r.set_cookie.expect("a session for the sign-in")))
}

/// The callback link the fake's consent page offers for `user`.
async fn consent(authorize: &url::Url, user: &str) -> url::Url {
    let page = reqwest::get(authorize.as_str())
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let link = page
        .split("<a href=\"")
        .skip(1)
        .find(|a| a.contains(&format!(">{user}</a>")))
        .unwrap_or_else(|| panic!("{user} on {page}"));
    url::Url::parse(&link[..link.find('"').unwrap()].replace("&amp;", "&")).unwrap()
}

fn path_and_query(u: &url::Url) -> String {
    format!("{}?{}", u.path(), u.query().unwrap_or(""))
}

async fn accounts(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM accounts")
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Signs `user` in through the fake: the signed-in session cookie.
pub async fn sign_in(app: &Router, user: &str) -> String {
    let (authorize, cookie) = start(app).await;
    let back = consent(&authorize, user).await;
    let r = send(app, "GET", &path_and_query(&back), Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::FOUND, "{}", r.body);
    pair(&r.set_cookie.unwrap())
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn signing_in_through_discord_makes_the_account_and_a_session(pool: PgPool) {
    let base = fake_discord().await;
    let app = server(&pool, &base, &[]);

    let (authorize, cookie) = start(&app).await;
    assert!(
        authorize
            .as_str()
            .starts_with(&format!("{base}/oauth2/authorize?"))
    );
    let q: std::collections::HashMap<_, _> = authorize.query_pairs().into_owned().collect();
    assert_eq!(q["client_id"], "test-client");
    assert_eq!(q["scope"], "identify guilds.join");
    assert_eq!(q["response_type"], "code");
    assert_eq!(q["code_challenge_method"], "S256");
    assert!(!q["code_challenge"].is_empty() && !q["state"].is_empty());
    assert_eq!(
        q["redirect_uri"],
        format!("{PUBLIC_URL}/auth/discord/callback")
    );
    assert_eq!(accounts(&pool).await, 0);
    // A sign-in nobody finishes lives minutes, not the signed-in month.
    let short: bool = sqlx::query_scalar(
        "SELECT bool_and(expiry_date < now() + interval '1 hour') FROM sessions",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(short, "a pending sign-in's session expires within the hour");

    let back = consent(&authorize, "alice").await;
    let r = send(&app, "GET", &path_and_query(&back), Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::FOUND, "{}", r.body);
    assert_eq!(r.location.as_deref(), Some("/"));
    let set = r.set_cookie.unwrap();
    for attr in [
        "HttpOnly",
        "SameSite=Lax",
        "Secure",
        "Path=/",
        "Max-Age=2592000",
    ] {
        assert!(set.split("; ").any(|a| a == attr), "{attr} in {set}");
    }
    let signed_in = pair(&set);
    assert_ne!(signed_in, cookie, "the session id is rotated at sign-in");
    assert_eq!(accounts(&pool).await, 1);

    let me = send(&app, "GET", "/api/me", Some(&signed_in)).await;
    assert_eq!(me.status, StatusCode::OK);
    let me: Value = serde_json::from_str(&me.body).unwrap();
    assert_eq!(me["discord_id"], "1001");
    assert_eq!(me["name"], "alice");
    assert_eq!(me["admin"], false);
    let old = send(&app, "GET", "/api/me", Some(&cookie)).await;
    assert_eq!(
        old.status,
        StatusCode::UNAUTHORIZED,
        "the pre-sign-in id is gone"
    );

    // Signing in again finds the same account.
    let again = sign_in(&app, "alice").await;
    assert_eq!(
        send(&app, "GET", "/api/me", Some(&again)).await.status,
        StatusCode::OK
    );
    assert_eq!(accounts(&pool).await, 1);

    let out = send(&app, "POST", "/auth/sign-out", Some(&signed_in)).await;
    assert_eq!(out.status, StatusCode::NO_CONTENT);
    let me = send(&app, "GET", "/api/me", Some(&signed_in)).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        send(&app, "GET", "/api/me", Some(&again)).await.status,
        StatusCode::OK
    );
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn a_wrong_state_is_refused_and_makes_no_account(pool: PgPool) {
    let base = fake_discord().await;
    let app = server(&pool, &base, &[]);
    let (authorize, cookie) = start(&app).await;
    let mut back = consent(&authorize, "alice").await;
    let code = back
        .query_pairs()
        .find(|(k, _)| k == "code")
        .unwrap()
        .1
        .into_owned();
    back.query_pairs_mut()
        .clear()
        .append_pair("code", &code)
        .append_pair("state", "forged");
    let r = send(&app, "GET", &path_and_query(&back), Some(&cookie)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(accounts(&pool).await, 0);
    // Without the session that started it, the real state fails too.
    let real = consent(&authorize, "alice").await;
    let r = send(&app, "GET", &path_and_query(&real), None).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(accounts(&pool).await, 0);
}

#[sqlx::test(migrator = "cna_server::MIGRATOR")]
#[cfg_attr(
    not(feature = "db-tests"),
    ignore = "needs Postgres: --features db-tests"
)]
async fn dev_cookies_are_not_secure(pool: PgPool) {
    let base = fake_discord().await;
    let app = server(&pool, &base, &[("CNA_DEV", "1")]);
    let r = send(&app, "GET", "/auth/discord", None).await;
    let set = r.set_cookie.unwrap();
    assert!(set.contains("HttpOnly") && !set.contains("Secure"), "{set}");
}
