//! The session store: tower-sessions records in our own Postgres pool.
//! `tower-sessions-sqlx-store` is built on sqlx 0.8 and this crate on
//! 0.9, so the store is ours; it also copies the record's account into a
//! column, so suspending an account can delete every session it holds.

use crate::games::AccountId;
use async_trait::async_trait;
use sqlx::PgPool;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{Error, Result, SessionStore};

/// The data key a signed-in session keeps its account id under.
pub const ACCOUNT: &str = "account";

#[derive(Debug, Clone)]
pub struct PgStore {
    pool: PgPool,
}

fn backend(e: sqlx::Error) -> Error {
    Error::Backend(e.to_string())
}

fn account_of(record: &Record) -> Option<AccountId> {
    record.data.get(ACCOUNT).and_then(|v| v.as_i64())
}

impl PgStore {
    pub fn new(pool: PgPool) -> PgStore {
        PgStore { pool }
    }

    /// Ends every session `account` holds.
    pub async fn delete_account(&self, account: AccountId) -> sqlx::Result<u64> {
        let r = sqlx::query("DELETE FROM sessions WHERE account_id = $1")
            .bind(account)
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected())
    }

    /// Drops the sessions past their expiry.
    pub async fn delete_expired(&self) -> sqlx::Result<u64> {
        let r = sqlx::query("DELETE FROM sessions WHERE expiry_date < now()")
            .execute(&self.pool)
            .await?;
        Ok(r.rows_affected())
    }
}

#[async_trait]
impl SessionStore for PgStore {
    async fn create(&self, record: &mut Record) -> Result<()> {
        let data = serde_json::to_value(&record.data).map_err(|e| Error::Encode(e.to_string()))?;
        // A fresh id until one is free (a clash of 128 random bits).
        loop {
            let done = sqlx::query(
                "INSERT INTO sessions (id, account_id, data, expiry_date) \
                 VALUES ($1, $2, $3, to_timestamp($4)) ON CONFLICT (id) DO NOTHING",
            )
            .bind(record.id.to_string())
            .bind(account_of(record))
            .bind(&data)
            .bind(record.expiry_date.unix_timestamp() as f64)
            .execute(&self.pool)
            .await
            .map_err(backend)?;
            if done.rows_affected() == 1 {
                return Ok(());
            }
            record.id = Id::default();
        }
    }

    async fn save(&self, record: &Record) -> Result<()> {
        let data = serde_json::to_value(&record.data).map_err(|e| Error::Encode(e.to_string()))?;
        sqlx::query(
            "INSERT INTO sessions (id, account_id, data, expiry_date) \
             VALUES ($1, $2, $3, to_timestamp($4)) ON CONFLICT (id) DO UPDATE \
             SET account_id = EXCLUDED.account_id, data = EXCLUDED.data, \
             expiry_date = EXCLUDED.expiry_date",
        )
        .bind(record.id.to_string())
        .bind(account_of(record))
        .bind(&data)
        .bind(record.expiry_date.unix_timestamp() as f64)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn load(&self, id: &Id) -> Result<Option<Record>> {
        let row: Option<(serde_json::Value, i64)> = sqlx::query_as(
            "SELECT data, extract(epoch FROM expiry_date)::bigint FROM sessions \
             WHERE id = $1 AND expiry_date > now()",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.map(|(data, expiry)| {
            Ok(Record {
                id: *id,
                data: serde_json::from_value(data).map_err(|e| Error::Decode(e.to_string()))?,
                expiry_date: time::OffsetDateTime::from_unix_timestamp(expiry)
                    .map_err(|e| Error::Decode(e.to_string()))?,
            })
        })
        .transpose()
    }

    async fn delete(&self, id: &Id) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}
