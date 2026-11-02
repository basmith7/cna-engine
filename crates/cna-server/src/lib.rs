//! The multiplayer server: games kept in Postgres (the order log is the
//! record) and in memory (a `GameState` per live game), orders taken
//! over HTTP from the seat holders.

pub mod accounts;
pub mod api;
pub mod auth;
pub mod games;

use anyhow::{Context, Result};
use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::path::PathBuf;

/// What the server reads from its environment (`CNA_` prefix, plus
/// the conventional `DATABASE_URL`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `CNA_BIND`, default `0.0.0.0:3240`.
    pub bind: SocketAddr,
    /// `DATABASE_URL`, required.
    pub database_url: String,
    /// `CNA_PUBLIC_URL`: where players reach the server (links, OAuth).
    pub public_url: Option<String>,
    /// `CNA_WEB_DIST`: the built `web/` to serve, if any.
    pub web_dist: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Config> {
        Config::from_vars(|k| std::env::var(k).ok())
    }

    /// `from_env` over any lookup, so tests need not touch the process env.
    pub fn from_vars(var: impl Fn(&str) -> Option<String>) -> Result<Config> {
        let bind = match var("CNA_BIND") {
            Some(b) => b.parse().with_context(|| format!("CNA_BIND={b}"))?,
            None => SocketAddr::from(([0, 0, 0, 0], 3240)),
        };
        Ok(Config {
            bind,
            database_url: var("DATABASE_URL").context("DATABASE_URL is not set")?,
            public_url: var("CNA_PUBLIC_URL"),
            web_dist: var("CNA_WEB_DIST").map(PathBuf::from),
        })
    }
}

/// The schema in `migrations/`, compiled in.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!();

/// Brings the database up to this build's schema; run at start.
pub async fn migrate(pool: &PgPool) -> Result<()> {
    MIGRATOR.run(pool).await.context("migrating the database")
}

/// The routes, over the game registry.
pub fn router(games: games::Games) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .nest("/api", api::routes())
        .with_state(games)
}

/// 200 `ok` when the database answers, 503 when it does not.
async fn healthz(State(games): State<games::Games>) -> (StatusCode, &'static str) {
    match sqlx::query("SELECT 1").execute(games.pool()).await {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(e) => {
            tracing::warn!("healthz: {e}");
            (StatusCode::SERVICE_UNAVAILABLE, "database unavailable")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn get_healthz(pool: PgPool) -> (StatusCode, String) {
        let res = router(games::Games::new(pool))
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[sqlx::test]
    #[cfg_attr(
        not(feature = "db-tests"),
        ignore = "needs Postgres: --features db-tests"
    )]
    async fn healthz_is_ok_with_a_database(pool: PgPool) {
        assert_eq!(get_healthz(pool).await, (StatusCode::OK, "ok".to_owned()));
    }

    #[sqlx::test]
    #[cfg_attr(
        not(feature = "db-tests"),
        ignore = "needs Postgres: --features db-tests"
    )]
    async fn healthz_is_503_when_the_database_is_gone(pool: PgPool) {
        pool.close().await;
        assert_eq!(get_healthz(pool).await.0, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[sqlx::test(migrations = false)]
    #[cfg_attr(
        not(feature = "db-tests"),
        ignore = "needs Postgres: --features db-tests"
    )]
    async fn migrating_twice_is_a_no_op(pool: PgPool) {
        async fn tables(pool: &PgPool) -> Vec<String> {
            sqlx::query_scalar(
                "SELECT table_name::text FROM information_schema.tables \
                 WHERE table_schema = 'public' ORDER BY 1",
            )
            .fetch_all(pool)
            .await
            .unwrap()
        }
        migrate(&pool).await.unwrap();
        let once = tables(&pool).await;
        for t in ["accounts", "games", "seats", "orders", "snapshots"] {
            assert!(once.contains(&t.to_owned()), "{t} in {once:?}");
        }
        migrate(&pool).await.unwrap();
        assert_eq!(tables(&pool).await, once);
        let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(applied as usize, MIGRATOR.iter().count());
    }

    #[test]
    fn config_defaults_the_bind_and_needs_a_database() {
        let c =
            Config::from_vars(|k| (k == "DATABASE_URL").then(|| "postgres://x".into())).unwrap();
        assert_eq!(c.bind, "0.0.0.0:3240".parse().unwrap());
        assert_eq!(c.public_url, None);
        assert!(Config::from_vars(|_| None).is_err());
    }
}
