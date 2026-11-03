use anyhow::{Context, Result};
use cna_embed::RULES_COMMIT;
use cna_server::games::{Games, MAX_IDLE};
use cna_server::{App, Config, migrate, router, with_web};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::time::Duration;
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
    // At start, games under older rules are moved or
    // marked stale, and idle games leave memory.
    let games = Games::new(pool.clone());
    let (moved, stale) = games.recheck_rules().await?;
    tracing::info!("rules {RULES_COMMIT}: {moved} games moved, {stale} stale");
    games.spawn_evictor(Duration::from_secs(60), MAX_IDLE);
    games.spawn_sweeper();
    let app = App::new(games, &config)?;
    // Turn alerts: only with the bot configured (see `Config`).
    match &app.auth.bot {
        Some((token, _)) => cna_server::alerts::Sender::new(
            pool.clone(),
            app.auth.discord_api.clone(),
            token.clone(),
            app.auth.public_url.clone(),
        )?
        .spawn(),
        None => {
            tracing::info!("turn alerts off: no CNA_DISCORD_BOT_TOKEN and CNA_DISCORD_GUILD_ID")
        }
    }
    {
        let (limits, store) = (app.limits.clone(), app.auth.store.clone());
        tokio::spawn(async move {
            let mut every = tokio::time::interval(Duration::from_secs(600));
            loop {
                every.tick().await;
                limits.prune();
                if let Err(e) = store.delete_expired().await {
                    tracing::warn!("deleting expired sessions: {e}");
                }
            }
        });
    }
    let routes = router(app);
    let routes = match config.web_dist.clone() {
        Some(dist) => with_web(routes, dist),
        None => routes,
    };
    let app = routes.layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    tracing::info!("listening on {}", config.bind);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown())
    .await?;
    Ok(())
}

/// Ctrl-C, or SIGTERM: what `docker stop` sends, and as PID 1 in the
/// image the server would otherwise ignore it until the SIGKILL.
async fn shutdown() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("installing the SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}
