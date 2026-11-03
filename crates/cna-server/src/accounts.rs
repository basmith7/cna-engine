//! Accounts: made at the first Discord sign-in (`sign_in`), or by
//! tests through `insert`.

use crate::games::AccountId;
use sqlx::PgPool;

/// A new account for `discord_id`.
pub async fn insert(pool: &PgPool, discord_id: &str, name: &str) -> sqlx::Result<AccountId> {
    sqlx::query_scalar("INSERT INTO accounts (discord_id, name) VALUES ($1, $2) RETURNING id")
        .bind(discord_id)
        .bind(name)
        .fetch_one(pool)
        .await
}

/// The account for a Discord sign-in, made on the first and refreshed
/// (name, avatar) on each: its id, and whether it is suspended.
pub async fn sign_in(
    pool: &PgPool,
    discord_id: &str,
    name: &str,
    avatar: Option<&str>,
) -> sqlx::Result<(AccountId, bool)> {
    sqlx::query_as(
        "INSERT INTO accounts (discord_id, name, avatar) VALUES ($1, $2, $3) \
         ON CONFLICT (discord_id) DO UPDATE SET name = EXCLUDED.name, avatar = EXCLUDED.avatar \
         RETURNING id, suspended_at IS NOT NULL",
    )
    .bind(discord_id)
    .bind(name)
    .bind(avatar)
    .fetch_one(pool)
    .await
}
