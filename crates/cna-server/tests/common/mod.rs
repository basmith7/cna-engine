//! Helpers shared by the database tests.
#![allow(dead_code)]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use cna_data::scenario::Side;
use cna_game::log::Log;
use cna_game::state::{GameState, Modules};
use cna_server::accounts;
use cna_server::games::{AccountId, GameId, Games};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

/// The module-on golden game: the orders the server's games (every
/// module on) accept.
pub fn golden() -> Log {
    let path = cna_data::repo_root().join("tests/games/graziani-m5.json");
    let log: Log = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(log.modules, Modules::ALL);
    log
}

/// A game on the golden seed, Axis seated by `axis`, Commonwealth by `cw`.
pub async fn seated(pool: &PgPool) -> (Games, GameId, AccountId, AccountId) {
    let games = Games::new(pool.clone());
    let axis = accounts::insert(pool, "1", "Axis").await.unwrap();
    let cw = accounts::insert(pool, "2", "Commonwealth").await.unwrap();
    let id = games
        .create_seeded(&golden().scenario, axis, false, golden().seed)
        .await
        .unwrap();
    games.seat(id, Side::Axis, axis).await.unwrap();
    games.seat(id, Side::Cw, cw).await.unwrap();
    (games, id, axis, cw)
}

/// Plays the golden game's first `n` orders, each as the side it names.
pub async fn play(games: &Games, id: GameId, axis: AccountId, cw: AccountId, n: usize) {
    for (i, o) in golden().orders.iter().take(n).enumerate() {
        let account = if o.player == Side::Axis { axis } else { cw };
        games
            .apply(id, account, o.player, &o.order, i as i64)
            .await
            .unwrap_or_else(|e| panic!("order {i}: {e}"));
    }
}

/// The state the first `n` golden orders lead to, in process.
pub fn expected(n: usize) -> String {
    let log = golden();
    let data = cna_embed::data(&log.scenario).unwrap();
    let mut s = GameState::with_modules(data, log.seed, log.modules).unwrap();
    assert_eq!(
        s.ruleset, log.ruleset,
        "the golden game plays the default rules"
    );
    for o in &log.orders[..n] {
        s.apply(data, o.player, o.order.clone()).unwrap();
    }
    s.to_json().unwrap()
}

pub async fn count(pool: &PgPool, sql: &'static str, id: GameId) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

pub async fn status(pool: &PgPool, id: GameId) -> String {
    sqlx::query_scalar("SELECT status FROM games WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// The public URL the test servers are configured with: the `Origin`
/// their state-changing requests carry.
pub const PUBLIC_URL: &str = "http://localhost:3240";

/// A server config for tests, `vars` over the defaults (a client id, a
/// client secret made at run time, `PUBLIC_URL`).
pub fn config(vars: &[(&str, &str)]) -> cna_server::Config {
    let secret = format!(
        "{:016x}",
        rand_core::RngCore::next_u64(&mut rand_core::OsRng)
    );
    let defaults = [
        ("DATABASE_URL", "postgres://unused"),
        ("CNA_PUBLIC_URL", PUBLIC_URL),
        ("CNA_DISCORD_CLIENT_ID", "test-client"),
        ("CNA_DISCORD_CLIENT_SECRET", secret.as_str()),
    ];
    cna_server::Config::from_vars(|k| {
        vars.iter()
            .chain(defaults.iter())
            .find(|(n, _)| *n == k)
            .map(|(_, v)| (*v).to_owned())
    })
    .unwrap()
}

/// The routes over `games` with `config`, every request from 127.0.0.1.
pub fn router_with(games: Games, config: &cna_server::Config) -> axum::Router {
    cna_server::router(cna_server::App::new(games, config).unwrap()).layer(
        axum::extract::connect_info::MockConnectInfo(std::net::SocketAddr::from((
            [127, 0, 0, 1],
            40000,
        ))),
    )
}

/// The routes over `games` with the test config.
pub fn router(games: Games) -> axum::Router {
    router_with(games, &config(&[]))
}

/// `router`, for tests playing whole games: hundreds of orders in
/// seconds, past the per-account order limit.
pub fn router_for_play(games: Games) -> axum::Router {
    let mut config = config(&[]);
    config.limits.orders_per_minute = 10_000;
    router_with(games, &config)
}

/// One request to `app` as the `test-auth` account `who` (Discord id),
/// with the `Origin` and `Content-Type` the CSRF check wants: the
/// status and the JSON body (`null` if it is not JSON).
pub async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    who: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header("origin", PUBLIC_URL);
    if method != "GET" {
        req = req.header("content-type", "application/json");
    }
    if let Some(who) = who {
        req = req.header("x-cna-test-account", who);
    }
    let req = match body {
        Some(b) => req.body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}
