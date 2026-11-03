//! The games in memory: a registry of live games, each behind its own
//! lock, loaded from Postgres on first use and evicted when idle. The
//! order log in Postgres is the record; a live `GameState` is a cache
//! that can always be rebuilt from it.

use anyhow::Context;
use cna_data::scenario::Side;
use cna_embed::RULES_COMMIT;
use cna_game::data::Data;
use cna_game::event::{Event, Illegal};
use cna_game::order::{LoggedOrder, Order};
use cna_game::state::{GameState, Modules};
use cna_game::view::Aliases;
use cna_rules::ruleset::Ruleset;
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::Row;
use sqlx::types::Json;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedMutexGuard, broadcast};

pub type GameId = i64;
pub type AccountId = i64;

/// A snapshot of the state is written every this many orders.
pub const SNAPSHOT_EVERY: usize = 50;

/// A game nobody has touched or streamed for this long leaves memory.
pub const MAX_IDLE: Duration = Duration::from_secs(30 * 60);

/// An accepted order, as sent to the game's subscribers. `events` are
/// the engine's, unredacted: each stream passes them through
/// `events_for` for its own side.
#[derive(Debug, Clone)]
pub struct Applied {
    /// The order's index in the log.
    pub seq: i64,
    pub side: Side,
    pub events: Vec<Event>,
}

/// Why a game could not be loaded or an order not applied.
#[derive(Debug)]
pub enum GameError {
    NotFound,
    /// The caller does not hold the seat for the side it ordered as.
    NotYourSeat,
    /// The caller's `seq` is not the game's next one (it is `seq`).
    Conflict {
        seq: i64,
    },
    /// The engine refused the order (real ids: pass it through
    /// `refusal_for` before a client sees it).
    Illegal(Illegal),
    /// The order names a piece the side cannot name (an unknown alias,
    /// or an enemy's real supply id).
    Unknown(String),
    /// The stored orders no longer replay under this build's data.
    Stale,
    /// The engine panicked on the order; the old state is kept.
    Panicked,
    /// A lobby request the game's state refuses: joining one's own
    /// game, a taken seat, a game no longer open, leaving one that is.
    Refused(&'static str),
    /// A private game joined without its invite token (or a wrong one).
    BadInvite,
    Db(anyhow::Error),
}

impl std::fmt::Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameError::NotFound => write!(f, "no such game"),
            GameError::NotYourSeat => write!(f, "not your seat"),
            GameError::Conflict { seq } => write!(f, "stale seq: the game is at {seq}"),
            GameError::Illegal(i) => write!(f, "{i}"),
            GameError::Unknown(e) => write!(f, "{e}"),
            GameError::Stale => write!(f, "the game no longer replays under these rules"),
            GameError::Panicked => write!(f, "the engine failed on the order"),
            GameError::Refused(why) => write!(f, "{why}"),
            GameError::BadInvite => write!(f, "a private game: the invite is missing or wrong"),
            GameError::Db(e) => write!(f, "{e:#}"),
        }
    }
}

impl std::error::Error for GameError {}

impl From<sqlx::Error> for GameError {
    fn from(e: sqlx::Error) -> GameError {
        GameError::Db(e.into())
    }
}

impl From<anyhow::Error> for GameError {
    fn from(e: anyhow::Error) -> GameError {
        GameError::Db(e)
    }
}

/// A game in memory.
pub struct Live {
    pub data: &'static Data,
    pub state: GameState,
    pub aliases: Aliases,
    /// Open information: both sides see everything.
    pub open: bool,
    pub tx: broadcast::Sender<Arc<Applied>>,
    /// Signalled when a player joins or leaves.
    pub seats: broadcast::Sender<()>,
    last_used: Instant,
}

impl Live {
    /// The `seq` the next order must carry.
    pub fn seq(&self) -> i64 {
        self.state.log.len() as i64
    }
}

/// One game's slot: empty until loaded (or after a failed load).
type Slot = Arc<Mutex<Option<Live>>>;

/// A loaded game, locked: the holder is the only one reading or
/// changing it until the guard drops.
pub struct LiveGuard(OwnedMutexGuard<Option<Live>>);

impl Deref for LiveGuard {
    type Target = Live;
    fn deref(&self) -> &Live {
        self.0.as_ref().expect("a LiveGuard holds a loaded game")
    }
}

impl DerefMut for LiveGuard {
    fn deref_mut(&mut self) -> &mut Live {
        self.0.as_mut().expect("a LiveGuard holds a loaded game")
    }
}

/// The registry. Cloning shares it.
#[derive(Clone)]
pub struct Games {
    pool: PgPool,
    live: Arc<std::sync::Mutex<HashMap<GameId, Slot>>>,
    streams: crate::stream::Streams,
    /// The seed `create_as` takes next, once (`/test/next-seed`).
    #[cfg(feature = "test-auth")]
    next_seed: Arc<std::sync::Mutex<Option<u64>>>,
}

pub(crate) fn side_str(side: Side) -> &'static str {
    match side {
        Side::Axis => "axis",
        Side::Cw => "cw",
    }
}

/// How a new game is listed and joined.
#[derive(Debug, Clone, Copy)]
struct Listing<'a> {
    /// Shown in the open games; else joined only with the invite.
    public: bool,
    /// The creator takes whichever side the joiner leaves.
    creator_either: bool,
    /// The SHA-256 of the invite token.
    invite_hash: Option<&'a [u8]>,
}

impl Listing<'_> {
    /// Public, no invite: the games tests and tools make directly.
    const PUBLIC: Listing<'static> = Listing {
        public: true,
        creator_either: false,
        invite_hash: None,
    };
}

/// A new invite token (shown to the creator once) and its SHA-256 (stored).
fn invite_token() -> (String, Vec<u8>) {
    let mut raw = [0u8; 24];
    OsRng.fill_bytes(&mut raw);
    let token: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    let hash = Sha256::digest(token.as_bytes()).to_vec();
    (token, hash)
}

/// Inserts a new game of `scenario`, waiting for its seats.
async fn insert_game(
    tx: &mut sqlx::PgConnection,
    scenario: &str,
    created_by: AccountId,
    open: bool,
    seed: u64,
    listing: Listing<'_>,
) -> Result<GameId, GameError> {
    let data = cna_embed::data(scenario)?;
    // Every module on, as the browser plays (`cna-web`'s new game).
    let state = GameState::with_modules(data, seed, Modules::ALL)?;
    // The alias key is its own random value, not the seed's: knowing
    // the dice must tell nothing about which alias is which.
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let id: GameId = sqlx::query_scalar(
        "INSERT INTO games (scenario, ruleset, modules, open_information, seed, \
         alias_key, rules_commit, created_by, to_act, clock, public, creator_either, \
         invite_token_hash) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id",
    )
    .bind(scenario)
    .bind(Json(state.ruleset))
    .bind(Json(state.modules))
    .bind(open)
    .bind(seed as i64)
    .bind(&key[..])
    .bind(RULES_COMMIT)
    .bind(created_by)
    .bind(state.to_act().map(side_str))
    .bind(Json(state.clock))
    .bind(listing.public)
    .bind(listing.creator_either)
    .bind(listing.invite_hash)
    .fetch_one(&mut *tx)
    .await?;
    Ok(id)
}

/// Seats `account` as `side` inside `tx`, the game row locked first so
/// two joins at once cannot both see one seat and leave it `open`.
async fn seat_in(
    tx: &mut sqlx::PgConnection,
    game: GameId,
    side: Side,
    account: AccountId,
) -> Result<(), GameError> {
    sqlx::query("SELECT id FROM games WHERE id = $1 FOR UPDATE")
        .bind(game)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO seats (game_id, side, account_id) VALUES ($1, $2, $3)")
        .bind(game)
        .bind(side_str(side))
        .bind(account)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE games SET status = 'active', updated_at = now() \
         WHERE id = $1 AND status = 'open' \
         AND (SELECT count(*) FROM seats WHERE game_id = $1) = 2",
    )
    .bind(game)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

fn parse_side(s: &str) -> anyhow::Result<Side> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .with_context(|| format!("side {s:?}"))
}

impl Games {
    pub fn new(pool: PgPool) -> Games {
        Games {
            pool,
            live: Default::default(),
            streams: Default::default(),
            #[cfg(feature = "test-auth")]
            next_seed: Default::default(),
        }
    }

    /// Makes the next lobby game's seed `seed` (tests replaying a known game).
    #[cfg(feature = "test-auth")]
    pub fn set_next_seed(&self, seed: u64) {
        *self.next_seed.lock().unwrap_or_else(|e| e.into_inner()) = Some(seed);
    }

    fn lobby_seed(&self) -> u64 {
        #[cfg(feature = "test-auth")]
        if let Some(s) = self
            .next_seed
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            return s;
        }
        OsRng.next_u64()
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// The open live streams, counted against their caps.
    pub fn streams(&self) -> &crate::stream::Streams {
        &self.streams
    }

    /// A new game of `scenario` with a seed from the OS, waiting for
    /// its seats.
    pub async fn create(
        &self,
        scenario: &str,
        created_by: AccountId,
        open: bool,
    ) -> Result<GameId, GameError> {
        self.create_seeded(scenario, created_by, open, OsRng.next_u64())
            .await
    }

    /// `create` with a given seed (tests, and replaying a known game).
    pub async fn create_seeded(
        &self,
        scenario: &str,
        created_by: AccountId,
        open: bool,
        seed: u64,
    ) -> Result<GameId, GameError> {
        let mut tx = self.pool.begin().await?;
        let id = insert_game(&mut tx, scenario, created_by, open, seed, Listing::PUBLIC).await?;
        tx.commit().await?;
        Ok(id)
    }

    /// A lobby game: its creator already in `side`'s seat (none for
    /// `None`, "either": seated at the join), in one transaction so the
    /// open-games list never shows it without them. Returns the id and
    /// the invite token, which only its hash outlives.
    pub async fn create_as(
        &self,
        scenario: &str,
        created_by: AccountId,
        side: Option<Side>,
        public: bool,
        open_information: bool,
    ) -> Result<(GameId, String), GameError> {
        let (token, hash) = invite_token();
        let listing = Listing {
            public,
            creator_either: side.is_none(),
            invite_hash: Some(&hash),
        };
        let mut tx = self.pool.begin().await?;
        let seed = self.lobby_seed();
        let id = insert_game(
            &mut tx,
            scenario,
            created_by,
            open_information,
            seed,
            listing,
        )
        .await?;
        if let Some(side) = side {
            seat_in(&mut tx, id, side, created_by).await?;
        }
        tx.commit().await?;
        Ok((id, token))
    }

    /// Seats `account` as `side` in an open game: refused for its own
    /// creator, a taken side, or a private game without its invite. An
    /// "either" creator takes the other side in the same transaction.
    pub async fn join(
        &self,
        game: GameId,
        account: AccountId,
        side: Side,
        invite: Option<&str>,
    ) -> Result<(), GameError> {
        let mut tx = self.pool.begin().await?;
        type Row = (String, bool, Option<Vec<u8>>, Option<AccountId>, bool);
        let row: Option<Row> = sqlx::query_as(
            "SELECT status, public, invite_token_hash, created_by, creator_either \
             FROM games WHERE id = $1 FOR UPDATE",
        )
        .bind(game)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((status, public, hash, creator, either)) = row else {
            return Err(GameError::NotFound);
        };
        let inside = creator == Some(account) || self.seat_in_tx(&mut tx, game, account).await?;
        // The invite before anything else: without it a stranger learns
        // nothing of a private game, not even whether it has started.
        if !public && !inside {
            let given = invite.map(|t| Sha256::digest(t.as_bytes()).to_vec());
            if hash.is_none() || given != hash {
                return Err(GameError::BadInvite);
            }
        }
        if inside {
            return Err(GameError::Refused("you are already in this game"));
        }
        if status != "open" {
            return Err(GameError::Refused("the game is not open"));
        }
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM seats WHERE game_id = $1 AND side = $2)",
        )
        .bind(game)
        .bind(side_str(side))
        .fetch_one(&mut *tx)
        .await?;
        if taken {
            return Err(GameError::Refused("that seat is taken"));
        }
        if either {
            let creator = creator.ok_or(GameError::Refused("the game has no creator"))?;
            seat_in(&mut tx, game, side.other(), creator).await?;
            sqlx::query("UPDATE games SET creator_either = false WHERE id = $1")
                .bind(game)
                .execute(&mut *tx)
                .await?;
        }
        seat_in(&mut tx, game, side, account).await?;
        tx.commit().await?;
        self.seats_changed(game).await;
        Ok(())
    }

    /// Tells the game's open streams its seats changed. A streamed game
    /// is in memory (`evict_idle` keeps it), so one not loaded has none.
    async fn seats_changed(&self, game: GameId) {
        let slot = self
            .live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&game)
            .cloned();
        if let Some(slot) = slot
            && let Some(live) = slot.lock().await.as_ref()
        {
            let _ = live.seats.send(());
        }
    }

    async fn seat_in_tx(
        &self,
        tx: &mut sqlx::PgConnection,
        game: GameId,
        account: AccountId,
    ) -> Result<bool, GameError> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM seats WHERE game_id = $1 AND account_id = $2)",
        )
        .bind(game)
        .bind(account)
        .fetch_one(&mut *tx)
        .await?)
    }

    /// `account` leaves `game`: it is `abandoned`, no winner declared.
    /// A player may leave an open or active game; the seatless creator
    /// of an "either" game may withdraw it while it waits.
    pub async fn leave(&self, game: GameId, account: AccountId) -> Result<(), GameError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(String, Option<AccountId>, bool)> = sqlx::query_as(
            "SELECT status, created_by, creator_either FROM games WHERE id = $1 FOR UPDATE",
        )
        .bind(game)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((status, creator, either)) = row else {
            return Err(GameError::NotFound);
        };
        let player =
            self.seat_in_tx(&mut tx, game, account).await? || (either && creator == Some(account));
        if !player {
            return Err(GameError::NotYourSeat);
        }
        if status != "open" && status != "active" {
            return Err(GameError::Refused("the game is already over"));
        }
        sqlx::query("UPDATE games SET status = 'abandoned', updated_at = now() WHERE id = $1")
            .bind(game)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.seats_changed(game).await;
        Ok(())
    }

    /// Marks `abandoned` every active game with no order (nor, with
    /// none, a start) in the 60 days before `now` (Unix seconds);
    /// returns how many. The clock is a parameter so tests can move it.
    pub async fn sweep_idle(&self, now: i64) -> Result<u64, GameError> {
        let done = sqlx::query(
            "UPDATE games g SET status = 'abandoned', updated_at = now() \
             WHERE g.status = 'active' AND COALESCE( \
               (SELECT max(o.at) FROM orders o WHERE o.game_id = g.id), g.updated_at) \
             < to_timestamp($1::double precision) - interval '60 days'",
        )
        .bind(now as f64)
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected())
    }

    /// Runs `sweep_idle` now and every day until the runtime stops.
    pub fn spawn_sweeper(&self) -> tokio::task::JoinHandle<()> {
        let games = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
            loop {
                tick.tick().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs() as i64);
                match games.sweep_idle(now).await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!("abandoned {n} games idle for 60 days"),
                    Err(e) => tracing::error!("the idle sweep failed: {e}"),
                }
            }
        })
    }

    /// Seats `account` as `side`; the game is `active` once both seats
    /// are held.
    pub async fn seat(
        &self,
        game: GameId,
        side: Side,
        account: AccountId,
    ) -> Result<(), GameError> {
        let mut tx = self.pool.begin().await?;
        seat_in(&mut tx, game, side, account).await?;
        tx.commit().await?;
        Ok(())
    }

    /// The game, loaded if it is not in memory, and locked.
    pub async fn get(&self, game: GameId) -> Result<LiveGuard, GameError> {
        let slot = self
            .live
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(game)
            .or_default()
            .clone();
        let mut guard = slot.lock_owned().await;
        if guard.is_none() {
            *guard = Some(self.load(game).await?);
        }
        let mut live = LiveGuard(guard);
        live.last_used = Instant::now();
        Ok(live)
    }

    /// Applies `order` from `account` as `side`. The order names enemy
    /// supply units by their aliases, as that side sees them; `seq` is
    /// the index the caller expects it to take.
    pub async fn apply(
        &self,
        game: GameId,
        account: AccountId,
        side: Side,
        order: &Order,
        seq: i64,
    ) -> Result<Vec<Event>, GameError> {
        // The seat first: a caller with none must not make us load the game.
        if self.seat_of(game, account).await? != Some(side) {
            return Err(GameError::NotYourSeat);
        }
        // Play is for active games: not before both seats are taken,
        // nor once the game is over, abandoned or stale.
        let status: Option<String> = sqlx::query_scalar("SELECT status FROM games WHERE id = $1")
            .bind(game)
            .fetch_optional(&self.pool)
            .await?;
        match status.as_deref() {
            Some("active") => {}
            Some("stale") => return Err(GameError::Stale),
            None => return Err(GameError::NotFound),
            Some(_) => return Err(GameError::Refused("the game is not in play")),
        }
        let mut live = self.get(game).await?;
        if seq != live.seq() {
            return Err(GameError::Conflict { seq: live.seq() });
        }
        let real = live
            .aliases
            .unalias_order(&live.state, side, order)
            .map_err(|e| GameError::Unknown(format!("{e:#}")))?;

        // On a clone, off the async threads, so a panicking order leaves
        // the live state as it was.
        let mut next = live.state.clone();
        let before = live.state.to_act();
        let data = live.data;
        let stored = real.clone();
        let (next, result) = tokio::task::spawn_blocking(move || {
            let r = catch_unwind(AssertUnwindSafe(|| next.apply(data, side, real)));
            (next, r)
        })
        .await
        .map_err(|_| GameError::Panicked)?;
        let events = match result {
            Ok(Ok(events)) => events,
            Ok(Err(illegal)) => return Err(GameError::Illegal(illegal)),
            Err(_) => {
                tracing::error!("game {game}: the engine panicked on order {seq}");
                return Err(GameError::Panicked);
            }
        };

        // Once the INSERT is sent, a failure may still have committed it
        // (the connection lost after COMMIT): drop the cached game so the
        // next request reloads it from the database instead of trying
        // this seq again for ever.
        let stored_ok: Result<(), GameError> = async {
            let mut tx = self.pool.begin().await?;
            // Checked again under the row lock a leave also takes: an
            // order racing a leave must not land in an abandoned game.
            let status: String =
                sqlx::query_scalar("SELECT status FROM games WHERE id = $1 FOR UPDATE")
                    .bind(game)
                    .fetch_one(&mut *tx)
                    .await?;
            if status != "active" {
                return Err(GameError::Refused("the game is not in play"));
            }
            sqlx::query(r#"INSERT INTO orders (game_id, seq, side, "order") VALUES ($1, $2, $3, $4)"#)
                .bind(game)
                .bind(seq)
                .bind(side_str(side))
                .bind(Json(&stored))
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "UPDATE games SET to_act = $2, updated_at = now(), clock = $4, \
                 status = CASE WHEN $3 THEN 'over' ELSE status END WHERE id = $1",
            )
            .bind(game)
            .bind(next.to_act().map(side_str))
            .bind(next.is_over())
            .bind(Json(next.clock))
            .execute(&mut *tx)
            .await?;
            // The move passing to the other seat is its alert, in this
            // transaction: none for a rolled-back order, none lost.
            if let Some(to) = next.to_act().filter(|s| Some(*s) != before)
                && !next.is_over()
            {
                crate::alerts::queue(&mut tx, &self.streams, game, to, seq, account).await?;
            }
            if next.log.len() % SNAPSHOT_EVERY == 0 {
                sqlx::query(
                    "INSERT INTO snapshots (game_id, seq, rules_commit, state) VALUES ($1, $2, $3, $4) \
                     ON CONFLICT DO NOTHING",
                )
                .bind(game)
                .bind(next.log.len() as i64)
                .bind(&next.rules_commit)
                .bind(next.to_json()?)
                .execute(&mut *tx)
                .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        if let Err(e) = stored_ok {
            *live.0 = None;
            return Err(e);
        }

        live.state = next;
        // No subscribers is not an error.
        let _ = live.tx.send(Arc::new(Applied {
            seq,
            side,
            events: events.clone(),
        }));
        Ok(events)
    }

    /// Reads a game back: the latest snapshot made under this build's
    /// rules plus the orders after it, else a full replay from the seed.
    /// A game made under other rules is replayed under these and moved
    /// to them if every order is still accepted; a game whose orders no
    /// longer replay is marked `stale`.
    async fn load(&self, game: GameId) -> Result<Live, GameError> {
        let Some(row) = sqlx::query(
            "SELECT scenario, ruleset, modules, open_information, seed, alias_key, \
             rules_commit, status FROM games WHERE id = $1",
        )
        .bind(game)
        .fetch_optional(&self.pool)
        .await?
        else {
            return Err(GameError::NotFound);
        };
        let status: String = row.try_get("status")?;
        if status == "stale" {
            return Err(GameError::Stale);
        }
        let scenario: String = row.try_get("scenario")?;
        let Json(ruleset): Json<Ruleset> = row.try_get("ruleset")?;
        let Json(modules): Json<Modules> = row.try_get("modules")?;
        let open: bool = row.try_get("open_information")?;
        let seed = row.try_get::<i64, _>("seed")? as u64;
        let key: Vec<u8> = row.try_get("alias_key")?;
        let key: [u8; 32] = key
            .try_into()
            .map_err(|_| anyhow::anyhow!("game {game}: the alias key is not 32 bytes"))?;
        let commit: String = row.try_get("rules_commit")?;
        let data = cna_embed::data(&scenario)?;

        let rows =
            sqlx::query(r#"SELECT seq, side, "order" FROM orders WHERE game_id = $1 ORDER BY seq"#)
                .bind(game)
                .fetch_all(&self.pool)
                .await?;
        let mut orders = Vec::with_capacity(rows.len());
        for (i, r) in rows.iter().enumerate() {
            let parsed = (|| -> anyhow::Result<LoggedOrder> {
                anyhow::ensure!(
                    r.try_get::<i64, _>("seq")? == i as i64,
                    "a gap in the orders"
                );
                let Json(order): Json<Order> = r.try_get("order")?;
                Ok(LoggedOrder {
                    player: parse_side(&r.try_get::<String, _>("side")?)?,
                    order,
                })
            })();
            match parsed {
                Ok(o) => orders.push(o),
                Err(e) => return self.mark_stale(game, &format!("order {i}: {e:#}")).await,
            }
        }
        let snapshot: Option<(i64, String)> = if commit == RULES_COMMIT {
            sqlx::query_as(
                "SELECT seq, state FROM snapshots WHERE game_id = $1 AND rules_commit = $2 \
                 ORDER BY seq DESC LIMIT 1",
            )
            .bind(game)
            .bind(RULES_COMMIT)
            .fetch_optional(&self.pool)
            .await?
        } else {
            None
        };

        let rebuilt = tokio::task::spawn_blocking(move || {
            catch_unwind(AssertUnwindSafe(|| {
                rebuild(data, ruleset, modules, seed, &orders, snapshot)
            }))
        })
        .await;
        let state = match rebuilt {
            Ok(Ok(Ok(state))) => state,
            Ok(Ok(Err(e))) => return self.mark_stale(game, &format!("{e:#}")).await,
            Ok(Err(_)) | Err(_) => return self.mark_stale(game, "the engine panicked").await,
        };
        if commit != RULES_COMMIT {
            sqlx::query("UPDATE games SET rules_commit = $2, updated_at = now() WHERE id = $1")
                .bind(game)
                .bind(RULES_COMMIT)
                .execute(&self.pool)
                .await?;
            tracing::info!("game {game}: replayed and moved from rules {commit} to {RULES_COMMIT}");
        }
        Ok(Live {
            data,
            state,
            aliases: Aliases::new(key),
            open,
            tx: broadcast::channel(64).0,
            seats: broadcast::channel(4).0,
            last_used: Instant::now(),
        })
    }

    async fn mark_stale<T>(&self, game: GameId, why: &str) -> Result<T, GameError> {
        tracing::warn!("game {game} is stale under rules {RULES_COMMIT}: {why}");
        sqlx::query("UPDATE games SET status = 'stale', updated_at = now() WHERE id = $1")
            .bind(game)
            .execute(&self.pool)
            .await?;
        Err(GameError::Stale)
    }

    /// Run at start: loads every active game made under other rules than
    /// this build's, so each is moved to them or marked stale before a
    /// player opens it. Returns (moved, stale). Nothing stays in memory.
    pub async fn recheck_rules(&self) -> Result<(usize, usize), GameError> {
        let ids: Vec<GameId> = sqlx::query_scalar(
            "SELECT id FROM games WHERE status = 'active' AND rules_commit <> $1 ORDER BY id",
        )
        .bind(RULES_COMMIT)
        .fetch_all(&self.pool)
        .await?;
        let (mut moved, mut stale) = (0, 0);
        for id in ids {
            match self.get(id).await {
                Ok(_) => moved += 1,
                Err(GameError::Stale) => stale += 1,
                Err(e) => return Err(e),
            }
            self.live
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id);
        }
        Ok((moved, stale))
    }

    /// The side `account` holds in `game`, if any.
    pub async fn seat_of(
        &self,
        game: GameId,
        account: AccountId,
    ) -> Result<Option<Side>, GameError> {
        let held: Option<String> =
            sqlx::query_scalar("SELECT side FROM seats WHERE game_id = $1 AND account_id = $2")
                .bind(game)
                .bind(account)
                .fetch_optional(&self.pool)
                .await?;
        Ok(held.as_deref().map(parse_side).transpose()?)
    }

    /// The orders after `after` (every order when `None`) with the
    /// engine's events for each, unredacted, rebuilt by replaying the
    /// game from its last snapshot before them (else its seed): events
    /// are not stored.
    pub async fn applied_after(
        &self,
        game: GameId,
        after: Option<i64>,
    ) -> Result<Vec<Applied>, GameError> {
        let seed: i64 = sqlx::query_scalar("SELECT seed FROM games WHERE id = $1")
            .bind(game)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(GameError::NotFound)?;
        let live = self.get(game).await?;
        let data = live.data;
        let (ruleset, modules) = (live.state.ruleset, live.state.modules);
        let orders = live.state.log.clone();
        drop(live);
        let first = after.map_or(0, |a| a.saturating_add(1).max(0)) as usize;
        // Start from the latest snapshot at or before `first`, so a
        // catch-up replays at most a snapshot's span before the orders
        // it returns, not the whole game.
        let snapshot: Option<(i64, String)> = sqlx::query_as(
            "SELECT seq, state FROM snapshots WHERE game_id = $1 AND rules_commit = $2 \
             AND seq <= $3 ORDER BY seq DESC LIMIT 1",
        )
        .bind(game)
        .bind(RULES_COMMIT)
        .bind(first as i64)
        .fetch_optional(&self.pool)
        .await?;
        let replayed = tokio::task::spawn_blocking(move || {
            catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<Vec<Applied>> {
                let from_snapshot = snapshot.and_then(|(n, json)| {
                    let s = GameState::from_json(&json, data).ok()?;
                    let n = usize::try_from(n).ok()?;
                    (s.log.len() == n && orders.get(..n) == Some(&s.log[..])).then_some(s)
                });
                let mut s = match from_snapshot {
                    Some(s) => s,
                    None => {
                        let mut s = GameState::with_modules(data, seed as u64, modules)?;
                        s.ruleset = ruleset;
                        s
                    }
                };
                let mut out = Vec::new();
                for (i, o) in orders.iter().enumerate().skip(s.log.len()) {
                    let events = s
                        .apply(data, o.player, o.order.clone())
                        .map_err(|e| anyhow::anyhow!("order {i} is refused: {e}"))?;
                    if i >= first {
                        out.push(Applied {
                            seq: i as i64,
                            side: o.player,
                            events,
                        });
                    }
                }
                Ok(out)
            }))
        })
        .await;
        match replayed {
            Ok(Ok(r)) => Ok(r?),
            _ => Err(GameError::Panicked),
        }
    }

    /// Games held in memory.
    pub fn live_count(&self) -> usize {
        self.live.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Drops every game idle for `max_idle` that nobody holds and
    /// nobody streams; returns how many went.
    pub fn evict_idle(&self, max_idle: Duration) -> usize {
        let mut map = self.live.lock().unwrap_or_else(|e| e.into_inner());
        let before = map.len();
        map.retain(|_, slot| {
            if Arc::strong_count(slot) > 1 {
                return true; // someone is loading it or holds its lock
            }
            match slot.try_lock() {
                Ok(g) => g
                    .as_ref()
                    .is_some_and(|l| l.tx.receiver_count() > 0 || l.last_used.elapsed() < max_idle),
                Err(_) => true,
            }
        });
        before - map.len()
    }

    /// Runs `evict_idle(max_idle)` every `every` until the runtime stops.
    pub fn spawn_evictor(
        &self,
        every: Duration,
        max_idle: Duration,
    ) -> tokio::task::JoinHandle<()> {
        let games = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(every);
            loop {
                tick.tick().await;
                let n = games.evict_idle(max_idle);
                if n > 0 {
                    tracing::debug!("evicted {n} idle games");
                }
            }
        })
    }
}

/// The state `orders` lead to: from `snapshot` (orders applied, state
/// JSON) if it reads back under `data` and its log is the stored one's
/// prefix, else from the seed.
fn rebuild(
    data: &Data,
    ruleset: Ruleset,
    modules: Modules,
    seed: u64,
    orders: &[LoggedOrder],
    snapshot: Option<(i64, String)>,
) -> anyhow::Result<GameState> {
    let from_snapshot = snapshot.and_then(|(n, json)| {
        let s = GameState::from_json(&json, data).ok()?;
        let n = usize::try_from(n).ok()?;
        (s.log.len() == n && orders.get(..n) == Some(&s.log[..])).then_some(s)
    });
    let mut state = match from_snapshot {
        Some(s) => s,
        None => {
            let mut s = GameState::with_modules(data, seed, modules)?;
            s.ruleset = ruleset;
            s
        }
    };
    for (i, o) in orders.iter().enumerate().skip(state.log.len()) {
        state
            .apply(data, o.player, o.order.clone())
            .map_err(|e| anyhow::anyhow!("order {i} is refused: {e}"))?;
    }
    Ok(state)
}
