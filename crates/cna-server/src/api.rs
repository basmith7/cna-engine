//! The game routes under `/api`. Each mirrors a method of the wasm
//! `Game` (`cna-web`'s `api.rs`), run as the caller's seat: inputs name
//! enemy supply units by their aliases and are un-aliased here, and
//! every answer passes through `view_for`, `events_for` or
//! `refusal_for` for that seat, so a client swaps engines without
//! reshaping data and never sees what 3.61 hides from it.

use crate::App;
use crate::auth::CurrentAccount;
use crate::games::{GameError, GameId, Games, LiveGuard};
use crate::limits::Limiters;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use cna_data::scenario::Side;
use cna_game::event::Event;
use cna_game::order::Order;
use cna_game::piece::PieceId;
use cna_game::reach::Purpose;
use cna_game::view::{events_for, refusal_for, view_for};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

/// An error answer: a status and `{"error": message}`, or a body of
/// its own (`{"illegal": ...}`, a stale `seq`).
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: Value,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl std::fmt::Display) -> ApiError {
        ApiError {
            status,
            body: json!({ "error": message.to_string() }),
        }
    }

    pub fn internal(e: impl std::fmt::Display) -> ApiError {
        tracing::error!("{e}");
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
    }

    fn forbidden() -> ApiError {
        ApiError::new(StatusCode::FORBIDDEN, "not your seat")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<GameError> for ApiError {
    fn from(e: GameError) -> ApiError {
        match e {
            GameError::NotFound => ApiError::new(StatusCode::NOT_FOUND, e),
            GameError::NotYourSeat => ApiError::forbidden(),
            GameError::Conflict { seq } => ApiError {
                status: StatusCode::CONFLICT,
                body: json!({ "error": e.to_string(), "seq": seq }),
            },
            GameError::Unknown(_) => unknown_piece(),
            GameError::Stale => ApiError::new(StatusCode::GONE, e),
            GameError::Refused(_) => ApiError::new(StatusCode::CONFLICT, e),
            GameError::BadInvite => ApiError::new(StatusCode::FORBIDDEN, e),
            // `Illegal` carries real ids: the routes answer it through
            // `refusal_for` before it gets here.
            GameError::Illegal(_) | GameError::Panicked | GameError::Db(_) => ApiError::internal(e),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> ApiError {
        ApiError::internal(e)
    }
}

type ApiResult = Result<Json<Value>, ApiError>;

/// The largest request body taken (an order is a few hundred bytes).
pub const MAX_BODY: usize = 64 * 1024;

pub fn routes() -> Router<App> {
    Router::new()
        .route("/me", get(me))
        .route("/games", get(list).post(create))
        .route("/games/{id}", get(game))
        .route("/games/{id}/join", post(join))
        .route("/games/{id}/leave", post(leave))
        .route("/games/{id}/events", get(events))
        .route("/games/{id}/stream", get(crate::stream::stream))
        .route("/games/{id}/orders", post(orders))
        .route("/games/{id}/placements", get(placements))
        .route("/games/{id}/reachable", get(reachable))
        .route("/games/{id}/sealed-options", get(sealed_options))
        .route("/games/{id}/preview", post(preview))
        .route("/games/{id}/save", get(save))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY))
}

/// The caller's account: `{id, discord_id, name, avatar, alerts, admin}`.
async fn me(State(app): State<App>, account: CurrentAccount) -> ApiResult {
    let (discord_id, name, avatar, alerts): (String, String, Option<String>, bool) =
        sqlx::query_as("SELECT discord_id, name, avatar, alerts FROM accounts WHERE id = $1")
            .bind(account.0)
            .fetch_one(app.games.pool())
            .await?;
    let admin = app.auth.admins.contains(&discord_id);
    Ok(Json(json!({
        "id": account.0,
        "discord_id": discord_id,
        "name": name,
        "avatar": avatar,
        "alerts": alerts,
        "admin": admin,
    })))
}

/// The caller's seat in `id` (403 without one) and the game, locked.
async fn seated(
    games: &Games,
    id: GameId,
    account: CurrentAccount,
) -> Result<(Side, LiveGuard), ApiError> {
    // The seat first: a caller with none must not make us load the game.
    let side = games
        .seat_of(id, account.0)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    let live = games.get(id).await?;
    Ok((side, live))
}

/// 429: past a per-account rate limit.
fn too_many(what: &str) -> ApiError {
    ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        format!("too many {what}: wait and try again"),
    )
}

fn bad(e: impl std::fmt::Display) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, e)
}

/// An input naming a piece the caller cannot name, 422. The id is not
/// echoed: it may be the enemy's real supply id.
fn unknown_piece() -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "names a piece you cannot name",
    )
}

/// `{"illegal": ...}` as `side` may read it, 422.
fn refused(live: &LiveGuard, side: Side, illegal: &cna_game::event::Illegal) -> ApiError {
    ApiError {
        status: StatusCode::UNPROCESSABLE_ENTITY,
        body: json!({ "illegal": refusal_for(&live.aliases, Some(side), illegal, live.open) }),
    }
}

fn events_json(live: &LiveGuard, side: Side, events: &[Event]) -> Vec<Value> {
    events_for(&live.state, &live.aliases, events, Some(side), live.open)
}

#[derive(Deserialize)]
struct ListQuery {
    mine: Option<String>,
    open: Option<String>,
}

/// `?mine`: the caller's games (seated, or waiting as an "either"
/// creator), each with `my_side`, `to_act`, `my_move`, `clock`,
/// `status` and the `opponent`'s name. `?open`: public games waiting
/// for a player that the caller is not in. Neither holds an invite.
async fn list(
    State(games): State<Games>,
    account: CurrentAccount,
    Query(q): Query<ListQuery>,
) -> ApiResult {
    let mine = match (q.mine.is_some(), q.open.is_some()) {
        (true, false) => true,
        (false, true) => false,
        _ => return Err(bad("give one of ?mine or ?open")),
    };
    type Row = (
        GameId,
        String,
        String,
        Option<String>,
        Option<Value>,
        bool,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT g.id, g.scenario, g.status, g.to_act, g.clock, g.creator_either, \
         (SELECT a.name FROM accounts a WHERE a.id = g.created_by), \
         (SELECT a.name FROM seats s JOIN accounts a ON a.id = s.account_id \
          WHERE s.game_id = g.id AND s.side = 'axis'), \
         (SELECT a.name FROM seats s JOIN accounts a ON a.id = s.account_id \
          WHERE s.game_id = g.id AND s.side = 'cw'), \
         (SELECT s.side FROM seats s WHERE s.game_id = g.id AND s.account_id = $1) \
         FROM games g WHERE \
         (EXISTS (SELECT 1 FROM seats s WHERE s.game_id = g.id AND s.account_id = $1) \
          OR (g.creator_either AND g.created_by = $1)) = $2 \
         AND ($2 OR (g.status = 'open' AND g.public)) \
         ORDER BY g.updated_at DESC, g.id DESC LIMIT 200",
    )
    .bind(account.0)
    .bind(mine)
    .fetch_all(games.pool())
    .await?;
    let out = rows.into_iter().map(
        |(id, scenario, status, to_act, clock, either, creator, axis, cw, my_side)| {
            let mut g = json!({
                "id": id,
                "scenario": scenario,
                "status": status,
                "to_act": to_act,
                "clock": clock,
                "seats": { "axis": axis, "cw": cw },
            });
            if mine {
                let opponent = match my_side.as_deref() {
                    Some("axis") => cw,
                    Some("cw") => axis,
                    _ => None,
                };
                let my_move = status == "active" && my_side.is_some() && to_act == my_side;
                g["my_side"] = json!(my_side);
                g["my_move"] = json!(my_move);
                g["opponent"] = json!(opponent);
            } else {
                g["creator"] = json!(creator);
                g["creator_side"] = json!(if either {
                    "either"
                } else if axis.is_some() {
                    "axis"
                } else {
                    "cw"
                });
            }
            g
        },
    );
    Ok(Json(json!(out.collect::<Vec<_>>())))
}

/// The side a creator asks for: one of the two, or either.
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum CreateSide {
    Axis,
    Cw,
    Either,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
struct CreateBody {
    side: CreateSide,
    scenario: Option<String>,
    /// Public: listed in the open games. Private games are joined only
    /// with the invite.
    #[serde(default = "default_true")]
    open: bool,
    #[serde(default)]
    open_information: bool,
}

/// A new game with the caller in `side`'s seat (or, for "either", to
/// be seated at the join): `{"id", "invite"}`, 201. The invite token is
/// shown here once; only its hash is kept.
async fn create(
    State(games): State<Games>,
    State(limits): State<Arc<Limiters>>,
    account: CurrentAccount,
    Json(b): Json<CreateBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if !limits.create_game(account.0) {
        return Err(too_many("games made"));
    }
    let open: i64 =
        sqlx::query_scalar("SELECT count(*) FROM games WHERE created_by = $1 AND status = 'open'")
            .bind(account.0)
            .fetch_one(games.pool())
            .await?;
    if open >= limits.limits.open_games {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "you have too many games waiting for a player",
        ));
    }
    let scenario = b.scenario.as_deref().unwrap_or(cna_embed::DEFAULT_SCENARIO);
    if cna_embed::data(scenario).is_err() {
        return Err(bad(format!("no scenario {scenario:?}")));
    }
    let side = match b.side {
        CreateSide::Axis => Some(Side::Axis),
        CreateSide::Cw => Some(Side::Cw),
        CreateSide::Either => None,
    };
    let (id, invite) = games
        .create_as(scenario, account.0, side, b.open, b.open_information)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "id": id, "invite": invite })),
    ))
}

#[derive(Deserialize)]
struct JoinBody {
    side: Side,
    invite: Option<String>,
}

/// Takes the free `side` of an open game; 409 if it is held, the game
/// is not open or the caller is already in it; 403 for a private game
/// without its invite.
async fn join(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
    Json(b): Json<JoinBody>,
) -> ApiResult {
    match games.join(id, account.0, b.side, b.invite.as_deref()).await {
        Ok(()) => Ok(Json(json!({ "id": id, "side": b.side }))),
        // Two joins at once: the second meets the first's seat.
        Err(GameError::Db(e))
            if e.downcast_ref::<sqlx::Error>()
                .and_then(|e| e.as_database_error())
                .is_some_and(|d| d.is_unique_violation()) =>
        {
            Err(ApiError::new(StatusCode::CONFLICT, "that seat is taken"))
        }
        Err(e) => Err(e.into()),
    }
}

/// The caller leaves the game, which is then `abandoned`: 403 for one
/// not in it, 409 once it is over.
async fn leave(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
) -> ApiResult {
    games.leave(id, account.0).await?;
    Ok(Json(json!({ "id": id, "status": "abandoned" })))
}

/// `{"side", "seq", "view"}`: the caller's seat, the next order's
/// `seq`, and the game as that seat may see it.
async fn game(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
) -> ApiResult {
    let (side, live) = seated(&games, id, account).await?;
    let view = view_for(&live.state, live.data, &live.aliases, Some(side), live.open)
        .map_err(ApiError::internal)?;
    Ok(Json(
        json!({ "side": side, "seq": live.seq(), "open": live.open, "view": view }),
    ))
}

#[derive(Deserialize)]
struct EventsQuery {
    after: Option<i64>,
}

/// The orders after `?after=seq` (all without it), each `{"seq",
/// "side", "events"}` with the events the caller may see.
async fn events(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
    Query(q): Query<EventsQuery>,
) -> ApiResult {
    let side = games
        .seat_of(id, account.0)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    let applied = games.applied_after(id, q.after).await?;
    let live = games.get(id).await?;
    Ok(Json(json!(
        applied
            .iter()
            .map(|a| json!({
                "seq": a.seq,
                "side": a.side,
                "events": events_json(&live, side, &a.events),
            }))
            .collect::<Vec<_>>()
    )))
}

#[derive(Deserialize)]
struct OrderBody {
    side: Side,
    order: Order,
    seq: i64,
}

/// Applies an order from the caller's seat: `{"ok": events}`,
/// `{"illegal"}` (422), 409 on a stale `seq`, 403 for the other side.
async fn orders(
    State(games): State<Games>,
    State(limits): State<Arc<Limiters>>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
    Json(b): Json<OrderBody>,
) -> ApiResult {
    if !limits.order(account.0) {
        return Err(too_many("orders"));
    }
    match games.apply(id, account.0, b.side, &b.order, b.seq).await {
        Ok(events) => {
            let live = games.get(id).await?;
            Ok(Json(json!({ "ok": events_json(&live, b.side, &events) })))
        }
        Err(GameError::Illegal(illegal)) => {
            let live = games.get(id).await?;
            Err(refused(&live, b.side, &illegal))
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Deserialize)]
struct PlacementsQuery {
    piece: PieceId,
}

/// The hexes the caller's pending `piece` may be placed in; 403 for a
/// piece that is not the caller's.
async fn placements(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
    Query(q): Query<PlacementsQuery>,
) -> ApiResult {
    let (side, live) = seated(&games, id, account).await?;
    // A piece the caller cannot name (an enemy's real supply id, an
    // unknown alias) is as much not its own as an enemy's.
    let piece = live.aliases.unalias(&live.state, side, &q.piece).ok();
    let own = piece.as_ref().and_then(|p| live.state.pieces.get(p));
    let Some(piece) = piece.filter(|_| own.is_some_and(|p| p.side == side)) else {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "not your piece"));
    };
    Ok(Json(json!(live.state.placements(live.data, &piece))))
}

#[derive(Deserialize)]
struct ReachableQuery {
    /// A JSON array of piece ids, as the wasm method takes.
    pieces: String,
    purpose: Purpose,
}

/// Where the caller's stack can end a move: `{"ok": [Reach]}` or
/// `{"illegal"}`, as `apply` would say.
async fn reachable(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
    Query(q): Query<ReachableQuery>,
) -> ApiResult {
    let (side, live) = seated(&games, id, account).await?;
    let named: Vec<PieceId> = serde_json::from_str(&q.pieces).map_err(bad)?;
    let pieces = named
        .iter()
        .map(|p| live.aliases.unalias(&live.state, side, p))
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(|_| unknown_piece())?;
    match live.state.reachable(live.data, side, &pieces, q.purpose) {
        Ok(reaches) => Ok(Json(json!({ "ok": reaches }))),
        Err(illegal) => Err(refused(&live, side, &illegal)),
    }
}

/// What the caller may answer the pending sealed step with.
async fn sealed_options(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
) -> ApiResult {
    let (side, live) = seated(&games, id, account).await?;
    match live.state.sealed_options(live.data, side) {
        Ok(options) => Ok(Json(json!({ "ok": options }))),
        Err(illegal) => Err(refused(&live, side, &illegal)),
    }
}

#[derive(Deserialize)]
struct PreviewBody {
    order: Order,
}

/// What `orders` would answer, on a copy: `{"ok": events}` /
/// `{"illegal"}`, or `{"hidden": "dice"}` when the order rolls (the
/// dice are the game's next rolls), and `{"hidden": "sealed"}` for any
/// order while a sealed step is pending (completing the step would show
/// the other side's answer), as the wasm `preview` does.
async fn preview(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
    Json(b): Json<PreviewBody>,
) -> ApiResult {
    let (side, live) = seated(&games, id, account).await?;
    if matches!(
        live.state.pending,
        Some(cna_game::state::Pending::Sealed { .. })
    ) {
        return Ok(Json(json!({ "hidden": "sealed" })));
    }
    let real = live
        .aliases
        .unalias_order(&live.state, side, &b.order)
        .map_err(|_| unknown_piece())?;
    let mut copy = live.state.clone();
    let data = live.data;
    let (copy, r) = tokio::task::spawn_blocking(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            copy.apply(data, side, real)
        }));
        (copy, r)
    })
    .await
    .map_err(ApiError::internal)?;
    let r = r.map_err(|_| ApiError::internal("the engine panicked on a preview"))?;
    match r {
        Ok(_) if copy.rng.draws() != live.state.rng.draws() => {
            Ok(Json(json!({ "hidden": "dice" })))
        }
        Ok(events) => Ok(Json(json!({
            "ok": events_for(&copy, &live.aliases, &events, Some(side), live.open)
        }))),
        Err(illegal) => Err(refused(&live, side, &illegal)),
    }
}

/// The full state, once the game is over (it then hides nothing); 409
/// before.
async fn save(
    State(games): State<Games>,
    account: CurrentAccount,
    Path(id): Path<GameId>,
) -> Result<Response, ApiError> {
    let (_, live) = seated(&games, id, account).await?;
    if !live.state.is_over() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "the game is not over: its save would show the other side's secrets",
        ));
    }
    let json = live.state.to_json().map_err(ApiError::internal)?;
    Ok((
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json,
    )
        .into_response())
}
