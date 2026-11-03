//! `GET /api/games/{id}/stream`: the live channel. Each accepted order
//! goes to every open stream as one server-sent event, `id: <seq>` and
//! data `{seq, events, view}` as the stream's seat may see them. A
//! client reconnecting with `Last-Event-ID` (or `?after=`) first gets one
//! catch-up message: the events of every order since, and the view now.

use crate::api::ApiError;
use crate::auth::CurrentAccount;
use crate::games::{AccountId, Applied, GameId, Games};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event as Sse, KeepAlive};
use axum::response::{IntoResponse, Response};
use cna_data::scenario::Side;
use cna_game::view::{events_for, view_for};
use futures_util::stream::{self, Stream};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Open streams one account may hold on one game.
pub const MAX_PER_GAME: usize = 4;
/// Open streams one account may hold in all.
pub const MAX_PER_ACCOUNT: usize = 32;

/// The open streams per account and game. Cloning shares it.
#[derive(Clone, Default)]
pub struct Streams(Arc<Mutex<HashMap<AccountId, HashMap<GameId, usize>>>>);

/// One open stream, counted until it drops.
pub struct Ticket {
    streams: Streams,
    account: AccountId,
    game: GameId,
}

impl Streams {
    /// A ticket for a new stream, or `None` past a cap.
    pub fn open(&self, account: AccountId, game: GameId) -> Option<Ticket> {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let games = map.entry(account).or_default();
        let total: usize = games.values().sum();
        let here = games.entry(game).or_default();
        if *here >= MAX_PER_GAME || total >= MAX_PER_ACCOUNT {
            return None;
        }
        *here += 1;
        Some(Ticket {
            streams: self.clone(),
            account,
            game,
        })
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        let mut map = self.streams.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(games) = map.get_mut(&self.account) {
            if let Some(n) = games.get_mut(&self.game) {
                *n -= 1;
                if *n == 0 {
                    games.remove(&self.game);
                }
            }
            if games.is_empty() {
                map.remove(&self.account);
            }
        }
    }
}

#[derive(Deserialize)]
pub struct StreamQuery {
    after: Option<i64>,
}

/// What a stream holds between messages.
struct Open {
    games: Games,
    id: GameId,
    side: Side,
    rx: broadcast::Receiver<Arc<Applied>>,
    /// Orders up to this seq have been sent (or predate the stream).
    sent: i64,
    catch_up: Option<i64>,
    _ticket: Ticket,
}

/// One message: `applied` as `side` may see it, with the view now.
async fn message(
    games: &Games,
    id: GameId,
    side: Side,
    applied: &[Applied],
) -> Result<(i64, Value), ApiError> {
    let live = games.get(id).await?;
    let events: Vec<Value> = applied
        .iter()
        .flat_map(|a| events_for(&live.state, &live.aliases, &a.events, Some(side), live.open))
        .collect();
    let seq = applied.last().map_or(live.seq() - 1, |a| a.seq);
    let view = view_for(&live.state, live.data, &live.aliases, Some(side), live.open)
        .map_err(ApiError::internal)?;
    Ok((seq, json!({ "seq": seq, "events": events, "view": view })))
}

fn sse(seq: i64, data: &Value) -> Sse {
    Sse::default().id(seq.to_string()).data(data.to_string())
}

/// The next message of `open`, or `None` to end the stream (the game
/// failed, or the stream fell behind: the client reconnects with its
/// `Last-Event-ID` and catches up).
async fn next(mut open: Open) -> Option<(Result<Sse, Infallible>, Open)> {
    if let Some(after) = open.catch_up.take() {
        let applied = match open.games.applied_after(open.id, Some(after)).await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("stream of game {}: {e}", open.id);
                return None;
            }
        };
        let (seq, data) = message(&open.games, open.id, open.side, &applied)
            .await
            .ok()?;
        open.sent = open.sent.max(seq);
        return Some((Ok(sse(seq, &data)), open));
    }
    loop {
        let applied = match open.rx.recv().await {
            Ok(a) => a,
            Err(_) => return None,
        };
        if applied.seq <= open.sent {
            continue; // in the catch-up already
        }
        let (seq, data) = message(
            &open.games,
            open.id,
            open.side,
            std::slice::from_ref(&*applied),
        )
        .await
        .ok()?;
        open.sent = seq;
        return Some((Ok(sse(seq, &data)), open));
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
        .and_then(|v| v.trim().parse::<i64>().ok());
    let side = games
        .seat_of(id, account.0)
        .await?
        .ok_or_else(|| ApiError::new(StatusCode::FORBIDDEN, "not your seat"))?;
    let ticket = games
        .streams()
        .open(account.0, id)
        .ok_or_else(|| ApiError::new(StatusCode::TOO_MANY_REQUESTS, "too many open streams"))?;
    // Subscribed under the game's lock, so no order falls between the
    // catch-up and the first live message.
    let (rx, sent) = {
        let live = games.get(id).await?;
        (live.tx.subscribe(), live.seq() - 1)
    };
    let catch_up = last.or(q.after);
    let open = Open {
        games,
        id,
        side,
        rx,
        sent: catch_up.map_or(sent, |_| -1),
        catch_up,
        _ticket: ticket,
    };
    let s: std::pin::Pin<Box<dyn Stream<Item = Result<Sse, Infallible>> + Send>> =
        Box::pin(stream::unfold(open, next));
    Ok(axum::response::sse::Sse::new(s)
        .keep_alive(KeepAlive::default())
        .into_response())
}
