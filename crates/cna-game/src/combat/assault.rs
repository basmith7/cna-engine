//! Steps 5 and 6 (SPI §14, §15): anti-armour fire, then the close
//! assaults in the phasing player's order; the must-attack rule (10.3x)
//! as the step ends.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::scenario::Side;

impl GameState {
    /// Step 5: every anti-armour attack rolled and its damage taken (14.0).
    pub(crate) fn resolve_anti_armour(&mut self, _data: &Data, _events: &mut Vec<Event>) {}

    /// `Order::Assault`: the assault assigned against `target` (15.2x).
    pub(crate) fn assault(
        &mut self,
        _data: &Data,
        _player: Side,
        _target: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        Err(Illegal::new("15.0", "close assault is not built yet"))
    }

    /// The close assault step is over: the must-attack rule (10.3x).
    /// True when a forced retreat is now pending and the clock waits.
    pub(crate) fn end_close_assault(&mut self, _data: &Data, _events: &mut Vec<Event>) -> bool {
        false
    }
}
