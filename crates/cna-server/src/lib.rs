//! The multiplayer server: games kept in Postgres (the order log is the
//! record) and in memory (a `GameState` per live game), orders taken
//! over HTTP from the seat holders.

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

/// The routes, over a pool.
pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(pool)
}

/// 200 `ok` when the database answers, 503 when it does not.
async fn healthz(State(pool): State<PgPool>) -> (StatusCode, &'static str) {
    match sqlx::query("SELECT 1").execute(&pool).await {
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
        let res = router(pool)
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

    #[test]
    fn config_defaults_the_bind_and_needs_a_database() {
        let c =
            Config::from_vars(|k| (k == "DATABASE_URL").then(|| "postgres://x".into())).unwrap();
        assert_eq!(c.bind, "0.0.0.0:3240".parse().unwrap());
        assert_eq!(c.public_url, None);
        assert!(Config::from_vars(|_| None).is_err());
    }
}
