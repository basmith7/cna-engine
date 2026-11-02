use anyhow::{Context, Result};
use cna_server::{Config, migrate, router};
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,cna_server=debug".into()),
        )
        .init();
    let config = Config::from_env()?;
    let pool = PgPoolOptions::new()
        .connect(&config.database_url)
        .await
        .context("connecting to the database")?;
    migrate(&pool).await?;
    let app = router(pool).layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    tracing::info!("listening on {}", config.bind);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
