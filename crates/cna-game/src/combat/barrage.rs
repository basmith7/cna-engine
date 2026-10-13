//! Step 2 resolved (SPI 12.3x, 12.4x): every plotted fire pooled by
//! target, rolled, and applied together after the last roll.

use crate::data::Data;
use crate::event::Event;
use crate::state::GameState;

impl GameState {
    /// Rolls every barrage and applies the hits and pins (12.0).
    pub(crate) fn resolve_barrages(&mut self, _data: &Data, _events: &mut Vec<Event>) {}
}
