//! `GET /api/games/{id}/stream`: the live channel. Each accepted order
//! goes to every open stream as one server-sent event, `id: <cursor>`
//! (the order's random name) and data `{version, events, view}` as the
//! stream's seat may see them; an order of the other side that leaves
//! the seat no events and its view as it was is not sent at all, so that
//! nothing (not a `seq`, not an empty message) counts the orders the
//! seat's view hides. A client reconnecting with `Last-Event-ID` (or
//! `?after=`, a cursor) first gets one catch-up message: the events of
//! every order since, and the view now.
//! When a player joins or leaves, every open stream gets `event: seats`
//! with data `{}` and no id, so the client reloads the game's status.

use crate::api::{ApiError, after_cursor};
use crate::auth::CurrentAccount;
use crate::games::{AccountId, Applied, GameId, Games};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event as Sse, KeepAlive};
use axum::response::{IntoResponse, Response};
use cna_data::scenario::Side;
use cna_game::view::events_for;
use futures_util::stream::{self, Stream};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Open streams one account may hold on one game.
pub const MAX_PER_GAME: usize = 4;
/// Open streams one account may hold in all.
pub const MAX_PER_ACCOUNT: usize = 32;

/// One account's open streams: how many per game, and the token that
/// closes them all.
#[derive(Default)]
struct AccountStreams {
    games: HashMap<GameId, usize>,
    close: CancellationToken,
}

/// The open streams per account and game. Cloning shares it.
#[derive(Clone, Default)]
pub struct Streams(Arc<Mutex<HashMap<AccountId, AccountStreams>>>);

/// One open stream, counted until it drops.
pub struct Ticket {
    streams: Streams,
    account: AccountId,
    game: GameId,
    /// Cancelled when the account's streams are closed.
    pub closed: CancellationToken,
}

impl Streams {
    /// A ticket for a new stream, or `None` past a cap.
    pub fn open(&self, account: AccountId, game: GameId) -> Option<Ticket> {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let mine = map.entry(account).or_default();
        let total: usize = mine.games.values().sum();
        let here = mine.games.entry(game).or_default();
        if *here >= MAX_PER_GAME || total >= MAX_PER_ACCOUNT {
            return None;
        }
        *here += 1;
        Some(Ticket {
            streams: self.clone(),
            account,
            game,
            closed: mine.close.clone(),
        })
    }

    /// Whether `account` has a stream open on `game` (no alert then).
    pub fn watching(&self, account: AccountId, game: GameId) -> bool {
        let map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.get(&account)
            .is_some_and(|mine| mine.games.contains_key(&game))
    }

    /// Ends every stream `account` holds open (it was suspended).
    pub fn close_account(&self, account: AccountId) {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mine) = map.get_mut(&account) {
            mine.close.cancel();
            // Streams opened from now on get a fresh token.
            mine.close = CancellationToken::new();
        }
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        let mut map = self.streams.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(mine) = map.get_mut(&self.account) {
            if let Some(n) = mine.games.get_mut(&self.game) {
                *n -= 1;
                if *n == 0 {
                    mine.games.remove(&self.game);
                }
            }
            if mine.games.is_empty() {
                map.remove(&self.account);
            }
        }
    }
}

#[derive(Deserialize)]
pub struct StreamQuery {
    after: Option<String>,
}

/// What a stream holds between messages.
struct Open {
    games: Games,
    id: GameId,
    side: Side,
    rx: broadcast::Receiver<Arc<Applied>>,
    /// Join and leave signals; `None` once their sender is gone.
    seats: Option<broadcast::Receiver<()>>,
    /// Orders up to this seq have been sent (or predate the stream).
    sent: i64,
    catch_up: Option<(i64, String)>,
    /// The version of the view last sent.
    version: Option<String>,
    ticket: Ticket,
}

/// One message: its event id, its version, its data.
struct Message {
    cursor: String,
    version: String,
    data: Value,
}

/// One message: `applied` as `side` may see it, with the view now; its
/// id is the last order's cursor, else `cursor` (nothing since it).
async fn message(
    games: &Games,
    id: GameId,
    side: Side,
    applied: &[Applied],
    cursor: &str,
) -> Result<Message, ApiError> {
    let live = games.get(id).await?;
    let events: Vec<Value> = applied
        .iter()
        .flat_map(|a| events_for(&live.state, &live.aliases, &a.events, Some(side), live.open))
        .collect();
    let cursor = applied.last().map_or(cursor, |a| &a.cursor).to_owned();
    let (view, version) = live.view(side).map_err(ApiError::internal)?;
    let data = json!({ "version": version, "events": events, "view": view });
    Ok(Message {
        cursor,
        version,
        data,
    })
}

fn sse(m: &Message) -> Sse {
    Sse::default().id(m.cursor.clone()).data(m.data.to_string())
}

/// The next message of `open`, or `None` to end the stream: its
/// account's streams were closed, the game failed, or the stream fell
/// behind (the client reconnects with its `Last-Event-ID` and catches
/// up).
async fn next(open: Open) -> Option<(Result<Sse, Infallible>, Open)> {
    let closed = open.ticket.closed.clone();
    tokio::select! {
        biased;
        _ = closed.cancelled() => None,
        n = next_message(open) => n,
    }
}

async fn next_message(mut open: Open) -> Option<(Result<Sse, Infallible>, Open)> {
    if let Some((after, cursor)) = open.catch_up.take() {
        let applied = match open.games.applied_after(open.id, Some(after)).await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("stream of game {}: {e}", open.id);
                return None;
            }
        };
        let m = message(&open.games, open.id, open.side, &applied, &cursor)
            .await
            .ok()?;
        open.sent = open.sent.max(applied.last().map_or(after, |a| a.seq));
        open.version = Some(m.version.clone());
        return Some((Ok(sse(&m)), open));
    }
    loop {
        let seats = async {
            match open.seats.as_mut() {
                Some(rx) => rx.recv().await,
                None => std::future::pending().await,
            }
        };
        let applied = tokio::select! {
            a = open.rx.recv() => match a {
                Ok(a) => a,
                Err(_) => return None,
            },
            s = seats => match s {
                // Missed signals say no more than one does.
                Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {
                    return Some((Ok(Sse::default().event("seats").data("{}")), open));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    open.seats = None;
                    continue;
                }
            },
        };
        if applied.seq <= open.sent {
            continue; // in the catch-up already
        }
        open.sent = applied.seq;
        let m = message(
            &open.games,
            open.id,
            open.side,
            std::slice::from_ref(&*applied),
            "",
        )
        .await
        .ok()?;
        // The other side's order that changed nothing this seat sees is
        // not sent: a message would count it.
        let silent = m.data["events"].as_array().is_some_and(|e| e.is_empty());
        if applied.side != open.side && silent && open.version.as_ref() == Some(&m.version) {
            continue;
        }
        open.version = Some(m.version.clone());
        return Some((Ok(sse(&m)), open));
    }
}

pub async fn stream(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
    Query(q): Query<StreamQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let last = headers
        .get("last-event-id")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty());
    let side = games
        .seat_of(id, account.0)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::FORBIDDEN, "not your seat"))?;
    let ticket = games
        .streams()
        .open(account.0, id)
        .ok_or_else(|| ApiError::new(StatusCode::TOO_MANY_REQUESTS, "too many open streams"))?;
    // Suspension marks the account before it closes its streams, so a
    // stream opened while a suspension lands is caught here or closed.
    let suspended: bool =
        sqlx::query_scalar("SELECT suspended_at IS NOT NULL FROM accounts WHERE id = $1")
            .bind(account.0)
            .fetch_one(games.pool())
            .await?;
    if suspended {
        drop(ticket);
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "suspended"));
    }
    // Subscribed under the game's lock, so no order falls between the
    // catch-up and the first live message.
    let (rx, seats, sent, version) = {
        let live = games.get(id).await?;
        let (_, version) = live.view(side).map_err(ApiError::internal)?;
        (
            live.tx.subscribe(),
            live.seats.subscribe(),
            live.seq() - 1,
            version,
        )
    };
    let catch_up = match last.or(q.after) {
        Some(c) => after_cursor(&games, id, Some(&c))
            .await?
            .map(|seq| (seq, c)),
        None => None,
    };
    let open = Open {
        games,
        id,
        side,
        rx,
        seats: Some(seats),
        sent: catch_up.as_ref().map_or(sent, |_| -1),
        catch_up,
        version: Some(version),
        ticket,
    };
    let s: std::pin::Pin<Box<dyn Stream<Item = Result<Sse, Infallible>> + Send>> =
        Box::pin(stream::unfold(open, next));
    Ok(axum::response::sse::Sse::new(s)
        .keep_alive(KeepAlive::default())
        .into_response())
}
