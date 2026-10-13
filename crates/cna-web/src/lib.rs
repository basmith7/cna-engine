//! `cna-game` for the browser. The Graziani data is compiled in
//! (`embedded`), the API is plain JSON over Rust (`api`), and the
//! `wasm-bindgen` `Game` below only passes strings through, so the
//! native tests and the page see byte-identical output.

pub mod api;
pub mod embedded;

use wasm_bindgen::prelude::*;

/// An `anyhow` error with its full context chain, as JS sees it.
fn js(e: anyhow::Error) -> JsError {
    JsError::new(&format!("{e:#}"))
}

/// One game of Graziani's Offensive, driven by JSON strings.
#[wasm_bindgen]
pub struct Game(api::Game);

#[wasm_bindgen]
impl Game {
    /// A new game with this RNG seed.
    #[wasm_bindgen(js_name = newGame)]
    pub fn new_game(seed: u64) -> Result<Game, JsError> {
        api::Game::new(seed).map(Game).map_err(js)
    }

    /// A game from the JSON `save()` wrote; throws if it is of another
    /// scenario or `vendor/cna` commit.
    pub fn load(json: &str) -> Result<Game, JsError> {
        api::Game::load(json).map(Game).map_err(js)
    }

    /// The state as JSON: the save format.
    pub fn save(&self) -> Result<String, JsError> {
        self.0.save().map_err(js)
    }

    /// The state for drawing, as JSON (see `api::Game::view`).
    pub fn view(&self) -> Result<String, JsError> {
        let v = self.0.view().map_err(js)?;
        Ok(serde_json::to_string(&v)?)
    }

    /// The hexes a pending piece of the side setting up may be placed
    /// on, as a JSON array of hex ids; `[]` for any other piece.
    pub fn placements(&self, piece: &str) -> Result<String, JsError> {
        let v = self.0.placements(piece).map_err(js)?;
        Ok(serde_json::to_string(&v)?)
    }

    /// The hexes a stack of `side`'s (`pieces_json`, a JSON array of
    /// piece ids) can end a move of `purpose` (`move`, `react`,
    /// `retreat-before-assault`, `forced-retreat`) in, as
    /// `{"ok": [Reach]}` or `{"illegal": {...}}` JSON (see
    /// `api::Game::reachable`). Throws only on malformed input.
    pub fn reachable(
        &self,
        side: &str,
        pieces_json: &str,
        purpose: &str,
    ) -> Result<String, JsError> {
        let v = self.0.reachable(side, pieces_json, purpose).map_err(js)?;
        Ok(serde_json::to_string(&v)?)
    }

    /// What `side` may answer the sealed step now pending with, as
    /// `{"ok": SealedOptions}` or `{"illegal": {...}}` JSON (see
    /// `api::Game::sealed_options`). Throws only on malformed input.
    #[wasm_bindgen(js_name = sealedOptions)]
    pub fn sealed_options(&self, side: &str) -> Result<String, JsError> {
        let v = self.0.sealed_options(side).map_err(js)?;
        Ok(serde_json::to_string(&v)?)
    }

    /// One order from `side` (`axis` / `cw`) as `Order` JSON; the answer
    /// is `{"ok": [...]}` or `{"illegal": {...}}` JSON. Throws only on
    /// malformed input.
    pub fn apply(&mut self, side: &str, order_json: &str) -> Result<String, JsError> {
        let v = self.0.apply(side, order_json).map_err(js)?;
        Ok(serde_json::to_string(&v)?)
    }

    /// What `apply` would answer `order_json` from `side` with, the game
    /// untouched (see `api::Game::preview`). Throws only on malformed
    /// input.
    pub fn preview(&self, side: &str, order_json: &str) -> Result<String, JsError> {
        let v = self.0.preview(side, order_json).map_err(js)?;
        Ok(serde_json::to_string(&v)?)
    }

    /// The `vendor/cna` commit the embedded data came from.
    #[wasm_bindgen(js_name = rulesCommit)]
    pub fn rules_commit() -> String {
        embedded::RULES_COMMIT.to_owned()
    }
}
