//! Retreats: before assault (SPI 13.x) and forced (15.82, 10.36).

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::PieceId;
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::scenario::Side;

impl GameState {
    /// `Order::RetreatBeforeAssault` (13.2x).
    pub(crate) fn retreat_before_assault(
        &mut self,
        _data: &Data,
        _player: Side,
        _pieces: &[PieceId],
        _path: &[HexId],
    ) -> Result<Vec<Event>, Illegal> {
        Err(Illegal::new(
            "13.1",
            "retreat before assault is not built yet",
        ))
    }

    /// `Order::Retreat` or `Pass` while a forced retreat is pending.
    pub(crate) fn forced_retreat(
        &mut self,
        _data: &Data,
        _player: Side,
        _path: Option<&[HexId]>,
    ) -> Result<Vec<Event>, Illegal> {
        Err(Illegal::new("15.82", "forced retreats are not built yet"))
    }
}
