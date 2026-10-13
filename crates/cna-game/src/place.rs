//! Set-up orders: a `Place` puts a pending piece on a hex its placement
//! allows (59.2), under the stacking ceiling (9.14) and clear of enemy
//! units where the placement says so.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId};
use crate::state::{GameState, Pending};
use cna_data::map::HexId;
use cna_data::scenario::{Locator, Placement, Side};

impl GameState {
    pub(crate) fn place(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        hex: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        let p = self
            .pieces
            .get(piece)
            .ok_or_else(|| Illegal::new("59.2", format!("{piece}: no such piece")))?;
        if p.side != player {
            return Err(Illegal::new("59.2", format!("{piece} is not {player}'s")));
        }
        let Location::Pending(placement) = &p.location else {
            return Err(Illegal::new(
                "59.2",
                format!("{piece} is not waiting to be placed"),
            ));
        };
        let Some(h) = data.map.hex(hex) else {
            return Err(Illegal::new("59.2", format!("{hex}: not on the map")));
        };
        if !allowed(data, placement, hex) {
            return Err(Illegal::new(
                "59.2",
                format!("{hex} is outside {piece}'s placement area"),
            ));
        }
        if let Some(nw) = &placement.not_within {
            let area = data.map.within(hex, nw.hexes);
            if let Some(enemy) = self
                .pieces
                .values()
                .find(|e| e.side != player && e.hex().is_some_and(|eh| area.contains(eh)))
            {
                return Err(Illegal::new(
                    "59.2",
                    format!(
                        "{hex} is within {} hexes of the enemy unit {} at {}",
                        nw.hexes,
                        enemy.id,
                        enemy.hex().expect("on a hex")
                    ),
                ));
            }
        }
        let terrain = h.terrain.name();
        let Some(ceiling) = data.terrain.ceiling(terrain) else {
            return Err(Illegal::new(
                "8.37",
                format!("{hex} is {terrain}: no unit may be there"),
            ));
        };
        let stacked = self.stacked(data, hex);
        let sp = cna_rules::stacking::points(&data.stacking, p.equivalent);
        if stacked + sp > ceiling {
            return Err(Illegal::new(
                "9.14",
                format!(
                    "{hex} holds {stacked} SP of a ceiling of {ceiling}; {piece} would add {sp}"
                ),
            ));
        }
        self.pieces.get_mut(piece).expect("checked").location = Location::Hex(hex.clone());
        let mut events = vec![Event::Placed {
            piece: piece.clone(),
            hex: hex.clone(),
        }];
        self.pending = self.next_set_up();
        if self.pending.is_none() {
            events.push(Event::SetUpComplete);
            events.extend(self.start_play(data));
        }
        Ok(events)
    }

    pub(crate) fn next_set_up(&self) -> Option<Pending> {
        [Side::Axis, Side::Cw]
            .into_iter()
            .find(|&side| {
                self.pieces
                    .values()
                    .any(|p| p.side == side && p.is_pending())
            })
            .map(|side| Pending::SetUp { side })
    }
}

/// Whether `hex` is in the placement's area (59.2).
fn allowed(data: &Data, placement: &Placement, hex: &HexId) -> bool {
    match &placement.locator {
        Locator::Hexes(list) => list.contains(hex),
        Locator::Within { of, hexes } => data.map.within(of, *hexes).contains(hex),
        Locator::Region(c) => data.map.country(hex) == Some(*c),
        Locator::Sheets(s) => s.contains(&hex.sheet()),
        // Resolved at expansion; never pending.
        Locator::Place(_) | Locator::Box(_) | Locator::AirFacility => false,
    }
}
