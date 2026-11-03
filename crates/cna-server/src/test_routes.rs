//! `/test/*`: what the two-player Playwright run asks the server that no
//! player may: the seed of the next lobby game, a game's state hash and
//! the leak predicate. Behind `test-auth`, so never in a default or
//! release build.

use crate::App;
use crate::api::ApiError;
use crate::games::{GameId, Games};
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use cna_data::scenario::Side;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/next-seed", post(next_seed))
        .route("/games/{id}/hash", get(hash))
        .route("/games/{id}/leaks", post(leaks))
}

#[derive(Deserialize)]
struct SeedBody {
    seed: u64,
}

/// The next game `create_as` makes takes this seed instead of the OS's.
async fn next_seed(State(games): State<Games>, Json(b): Json<SeedBody>) -> &'static str {
    games.set_next_seed(b.seed);
    "ok"
}

/// The SHA-256 of the live state's JSON, as `replay`'s state hashes.
async fn hash(State(games): State<Games>, Path(id): Path<GameId>) -> Result<String, ApiError> {
    let live = games.get(id).await?;
    let json = live.state.to_json().map_err(ApiError::internal)?;
    Ok(format!("{:x}", Sha256::digest(json.as_bytes())))
}

#[derive(Deserialize)]
struct About {
    about: Side,
}

/// What `json` (a response or SSE message sent to `about`'s enemy) leaks
/// about `about`'s pieces in the game's current state: `[]` when clean.
async fn leaks(
    State(games): State<Games>,
    Path(id): Path<GameId>,
    Query(q): Query<About>,
    Json(json): Json<Value>,
) -> Result<Json<Vec<String>>, ApiError> {
    let live = games.get(id).await?;
    Ok(Json(cna_game::view::leaks(&live.state, &json, q.about)))
}
