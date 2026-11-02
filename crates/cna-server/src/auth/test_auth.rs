//! Test sign-in: `X-Cna-Test-Account: <discord_id>` names the caller.
//! Anyone can claim any account with it, so it must never ship.

#[cfg(all(feature = "test-auth", not(debug_assertions)))]
compile_error!("the test-auth feature signs anyone in: it must not be built in release");

use crate::api::ApiError;
use crate::games::{AccountId, Games};
use axum::http::StatusCode;
use axum::http::request::Parts;

pub const HEADER: &str = "x-cna-test-account";

/// The account the header names; `None` without the header, 401 for
/// one naming no account.
pub async fn account(parts: &Parts, games: &Games) -> Result<Option<AccountId>, ApiError> {
    let Some(v) = parts.headers.get(HEADER) else {
        return Ok(None);
    };
    let discord_id = v
        .to_str()
        .map_err(|_| ApiError::new(StatusCode::UNAUTHORIZED, "unreadable test account"))?;
    let id: Option<AccountId> = sqlx::query_scalar(
        "SELECT id FROM accounts WHERE discord_id = $1 AND suspended_at IS NULL",
    )
    .bind(discord_id)
    .fetch_optional(games.pool())
    .await
    .map_err(ApiError::internal)?;
    id.map(Some)
        .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "no such account"))
}
