//! Accounts. Slice 3 creates them at Discord sign-in; until then only
//! tests do, through `insert`.

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
