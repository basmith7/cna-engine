//! `fake-discord`: the fake Discord on `FAKE_DISCORD_BIND` (default
//! `127.0.0.1:3241`), its users from `FAKE_DISCORD_USERS` (`id:name,…`),
//! its bot token `FAKE_DISCORD_BOT_TOKEN` (any, when unset).

use anyhow::{Context, Result, anyhow};
use cna_server::fake_discord::{default_users, parse_users, router_with};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let bind: SocketAddr = match std::env::var("FAKE_DISCORD_BIND") {
        Ok(b) => b
            .parse()
            .with_context(|| format!("FAKE_DISCORD_BIND={b}"))?,
        Err(_) => SocketAddr::from(([127, 0, 0, 1], 3241)),
    };
    let users = match std::env::var("FAKE_DISCORD_USERS") {
        Ok(u) => parse_users(&u).map_err(|e| anyhow!("FAKE_DISCORD_USERS: {e}"))?,
        Err(_) => default_users(),
    };
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!("fake Discord on {bind} with {} users", users.len());
    axum::serve(
        listener,
        router_with(users, std::env::var("FAKE_DISCORD_BOT_TOKEN").ok()),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
