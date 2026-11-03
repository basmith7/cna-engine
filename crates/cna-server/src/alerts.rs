//! Turn alerts by Discord DM, through a transactional outbox: the order
//! that makes it a player's move writes an `alerts` row in its own
//! transaction (`queue`), so an alert is never sent for an order that
//! rolled back nor lost to a crash; the `Sender` DMs the rows.
//!
//! The sender runs only when `CNA_DISCORD_BOT_TOKEN` and
//! `CNA_DISCORD_GUILD_ID` are both set; without them alerts are off,
//! though rows still queue (and go out once a bot is configured, if
//! under 24 hours old).

use crate::games::{AccountId, GameId, side_str};
use crate::stream::Streams;
use anyhow::{Context, Result};
use axum::http::{StatusCode, header};
use cna_data::scenario::Side;
use cna_game::clock::Clock;
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::types::Json;
use std::time::Duration;

/// At most one alert per game per account in this long.
pub const QUIET_MINUTES: i64 = 15;
/// How often the sender looks for due rows.
pub const EVERY: Duration = Duration::from_secs(5);
/// The first retry's wait after an error; it doubles each time.
pub const BACKOFF_SECS: f64 = 30.0;
/// A row still unsent this long after it was written is given up.
pub const GIVE_UP_HOURS: i64 = 24;
/// Rows one `send_due` takes at most.
const BATCH: usize = 50;

/// Inside the order's transaction: queues an alert for `side`'s seat in
/// `game` (the order `seq` made it their move), unless that seat is
/// `mover` (hot-seat: they are here), has alerts off or is suspended,
/// had an alert for this game in the last `QUIET_MINUTES`, or has the
/// game's stream open.
pub(crate) async fn queue(
    tx: &mut sqlx::PgConnection,
    streams: &Streams,
    game: GameId,
    side: Side,
    seq: i64,
    mover: AccountId,
) -> sqlx::Result<()> {
    let to: Option<AccountId> = sqlx::query_scalar(
        "SELECT s.account_id FROM seats s JOIN accounts a ON a.id = s.account_id \
         WHERE s.game_id = $1 AND s.side = $2 AND a.alerts AND a.suspended_at IS NULL",
    )
    .bind(game)
    .bind(side_str(side))
    .fetch_optional(&mut *tx)
    .await?;
    let Some(to) = to.filter(|a| *a != mover) else {
        return Ok(());
    };
    if streams.watching(to, game) {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO alerts (account_id, game_id, seq) SELECT $1, $2, $3 \
         WHERE NOT EXISTS (SELECT 1 FROM alerts WHERE account_id = $1 AND game_id = $2 \
         AND created_at > now() - make_interval(mins => $4))",
    )
    .bind(to)
    .bind(game)
    .bind(seq)
    .bind(QUIET_MINUTES as i32)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Sends the due alerts. Several may run at once (rows are taken `FOR
/// UPDATE SKIP LOCKED`), each row sent once.
pub struct Sender {
    pool: PgPool,
    http: reqwest::Client,
    /// Discord's API base, without a trailing `/`.
    api: String,
    bot_token: String,
    /// Where the game links point, without a trailing `/`.
    public_url: String,
}

/// How one try went.
enum Outcome {
    Sent,
    /// 429: try again after this many seconds; not counted as an attempt.
    RateLimited(f64, String),
    /// 403: the user takes no DMs from us. The row is given up and the
    /// account marked.
    Unreachable(String),
    /// Anything else: back off and retry.
    Failed(String),
}

impl Sender {
    pub fn new(pool: PgPool, api: String, bot_token: String, public_url: String) -> Result<Sender> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .user_agent("DiscordBot (https://github.com/basmith7/cna-engine, 0.1)")
            .build()
            .context("building the HTTP client")?;
        Ok(Sender {
            pool,
            http,
            api: api.trim_end_matches('/').to_owned(),
            bot_token,
            public_url: public_url.trim_end_matches('/').to_owned(),
        })
    }

    /// Runs `send_due` every `EVERY` for ever.
    pub fn spawn(self) {
        tokio::spawn(async move {
            let mut every = tokio::time::interval(EVERY);
            loop {
                every.tick().await;
                if let Err(e) = self.send_due().await {
                    tracing::warn!("sending alerts: {e:#}");
                }
            }
        });
    }

    /// Tries every due row once (up to a batch): how many were sent.
    pub async fn send_due(&self) -> Result<usize> {
        let mut sent = 0;
        for _ in 0..BATCH {
            match self.send_one().await? {
                Some(true) => sent += 1,
                Some(false) => {}
                None => break,
            }
        }
        Ok(sent)
    }

    /// Takes one due row and tries it: `None` when there is none.
    async fn send_one(&self) -> Result<Option<bool>> {
        let mut tx = self.pool.begin().await?;
        type Due = (i64, AccountId, GameId, i32, i64);
        let due: Option<Due> = sqlx::query_as(
            "SELECT id, account_id, game_id, attempts, seq FROM alerts \
             WHERE sent_at IS NULL AND given_up_at IS NULL AND next_attempt_at <= now() \
             ORDER BY next_attempt_at, id LIMIT 1 FOR UPDATE SKIP LOCKED",
        )
        .fetch_optional(&mut *tx)
        .await?;
        let Some((id, account, game, attempts, seq)) = due else {
            return Ok(None);
        };
        type Detail = (
            String,
            bool,
            bool,
            String,
            Option<Json<Clock>>,
            Option<String>,
        );
        let (discord_id, wanted, current, scenario, clock, opponent): Detail = sqlx::query_as(
            "SELECT a.discord_id, a.alerts AND a.suspended_at IS NULL, \
             g.status = 'active' AND NOT EXISTS \
              (SELECT 1 FROM orders o WHERE o.game_id = g.id AND o.seq > $3), \
             g.scenario, g.clock, \
             (SELECT o.name FROM seats s JOIN accounts o ON o.id = s.account_id \
              WHERE s.game_id = g.id AND s.account_id <> a.id LIMIT 1) \
             FROM accounts a, games g WHERE a.id = $1 AND g.id = $2",
        )
        .bind(account)
        .bind(game)
        .bind(seq)
        .fetch_one(&mut *tx)
        .await?;
        if !wanted || !current {
            // Alerts turned off (or the account suspended) since, or the
            // game is no longer in play or has moved on: never sent.
            sqlx::query("UPDATE alerts SET given_up_at = now(), last_error = $2 WHERE id = $1")
                .bind(id)
                .bind(if wanted { "moved on" } else { "alerts off" })
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(Some(false));
        }
        let name = cna_embed::data(&scenario)
            .map(|d| d.scenario.name.clone())
            .unwrap_or(scenario);
        let when = clock
            .map(|Json(c)| format!(", Game-Turn {} OpStage {}", c.game_turn, c.stage))
            .unwrap_or_default();
        let against = opponent
            .map(|o| format!(" against {o}"))
            .unwrap_or_default();
        let content = format!(
            "Your move in {name}{against}{when}: {}/game/{game}",
            self.public_url
        );
        let outcome = self.dm(&discord_id, &content).await;
        let sent = matches!(outcome, Outcome::Sent);
        match outcome {
            Outcome::Sent => {
                sqlx::query("UPDATE alerts SET sent_at = now(), last_error = NULL WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("UPDATE accounts SET alerts_unreachable_at = NULL WHERE id = $1")
                    .bind(account)
                    .execute(&mut *tx)
                    .await?;
            }
            Outcome::Unreachable(e) => {
                tracing::info!("alert {id}: account {account} takes no DMs: {e}");
                sqlx::query(
                    "UPDATE alerts SET given_up_at = now(), attempts = attempts + 1, \
                     last_error = $2 WHERE id = $1",
                )
                .bind(id)
                .bind(e)
                .execute(&mut *tx)
                .await?;
                sqlx::query("UPDATE accounts SET alerts_unreachable_at = now() WHERE id = $1")
                    .bind(account)
                    .execute(&mut *tx)
                    .await?;
            }
            Outcome::RateLimited(wait, e) => {
                retry(&mut tx, id, attempts, wait, &e).await?;
            }
            Outcome::Failed(e) => {
                tracing::warn!("alert {id}: {e}");
                let wait = BACKOFF_SECS * 2f64.powi(attempts.min(16));
                retry(&mut tx, id, attempts + 1, wait, &e).await?;
            }
        }
        tx.commit().await?;
        Ok(Some(sent))
    }

    /// Opens the DM channel with `discord_id` and posts `content`.
    async fn dm(&self, discord_id: &str, content: &str) -> Outcome {
        let channel = match self
            .call(
                &format!("{}/users/@me/channels", self.api),
                json!({ "recipient_id": discord_id }),
            )
            .await
        {
            Ok(v) => match v.get("id").and_then(Value::as_str) {
                Some(c) => c.to_owned(),
                None => return Outcome::Failed(format!("a DM channel without an id: {v}")),
            },
            Err(o) => return o,
        };
        match self
            .call(
                &format!("{}/channels/{channel}/messages", self.api),
                json!({ "content": content }),
            )
            .await
        {
            Ok(_) => Outcome::Sent,
            Err(o) => o,
        }
    }

    /// POSTs `body` as the bot: the reply's JSON, or how it failed.
    async fn call(&self, url: &str, body: Value) -> Result<Value, Outcome> {
        let r = self
            .http
            .post(url)
            .header(header::AUTHORIZATION, format!("Bot {}", self.bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| Outcome::Failed(format!("{url}: {e}")))?;
        let status = r.status();
        let text = r.text().await.unwrap_or_default();
        let reply: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let error = || format!("{status}: {text}");
        match status {
            s if s.is_success() => Ok(reply),
            StatusCode::TOO_MANY_REQUESTS => {
                let wait = reply
                    .get("retry_after")
                    .and_then(Value::as_f64)
                    .unwrap_or(BACKOFF_SECS);
                Err(Outcome::RateLimited(wait, error()))
            }
            StatusCode::FORBIDDEN => Err(Outcome::Unreachable(error())),
            _ => Err(Outcome::Failed(error())),
        }
    }
}

/// Sets the row to be tried again in `wait` seconds, or gives it up if
/// it is `GIVE_UP_HOURS` old.
async fn retry(
    tx: &mut sqlx::PgConnection,
    id: i64,
    attempts: i32,
    wait: f64,
    error: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE alerts SET attempts = $2, last_error = $3, \
         next_attempt_at = now() + make_interval(secs => $4), \
         given_up_at = CASE WHEN created_at < now() - make_interval(hours => $5) \
                       THEN now() END \
         WHERE id = $1",
    )
    .bind(id)
    .bind(attempts)
    .bind(error)
    .bind(wait)
    .bind(GIVE_UP_HOURS as i32)
    .execute(&mut *tx)
    .await?;
    Ok(())
}
