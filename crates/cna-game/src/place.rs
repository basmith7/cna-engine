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
    /// The hexes `place` would accept for `piece` now, in id order: its
    /// placement area, less the hexes stacking or `not_within` refuse.
    /// Empty for a piece that is not waiting to be placed, or whose side
    /// is not the one setting up. The placement's `constraint` prose is
    /// not applied: the owner honours it (59.2; spec, *Placements*).
    pub fn placements(&self, data: &Data, piece: &PieceId) -> Vec<HexId> {
        let Some(p) = self.pieces.get(piece) else {
            return Vec::new();
        };
        let Location::Pending(placement) = &p.location else {
            return Vec::new();
        };
        if self.pending != Some(Pending::SetUp { side: p.side }) || self.to_act() != Some(p.side) {
            return Vec::new();
        }
        let mut v: Vec<HexId> = area(data, placement)
            .into_iter()
            .filter(|hex| self.check_place(data, p.side, piece, hex).is_ok())
            .collect();
        v.sort();
        v
    }

    pub(crate) fn place(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        hex: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        self.check_place(data, player, piece, hex)?;
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

    /// Everything `place` checks, and nothing it changes: the one owner
    /// of 59.2 for `apply` and `placements` alike.
    fn check_place(
        &self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        hex: &HexId,
    ) -> Result<(), Illegal> {
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
        Ok(())
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

/// The placement's whole area (59.2): every hex `allowed` says yes to,
/// in map order, before stacking and `not_within`.
pub(crate) fn area(data: &Data, placement: &Placement) -> Vec<HexId> {
    let map = &data.map;
    match &placement.locator {
        Locator::Hexes(h) => h.clone(),
        Locator::Within { of, hexes } => map.within(of, *hexes).into_iter().collect(),
        Locator::Region(_) | Locator::Sheets(_) => map
            .hexes()
            .filter(|h| allowed(data, placement, &h.id))
            .map(|h| h.id.clone())
            .collect(),
        Locator::Place(_) | Locator::Box(_) | Locator::AirFacility => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::Log;
    use crate::order::Order;
    use crate::piece::Piece;

    fn data() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// The golden game's first `n` orders applied: `n = 36` is the Axis
    /// set-up done, the Commonwealth to place.
    fn after(data: &Data, n: usize) -> GameState {
        let path = cna_data::repo_root().join("tests/games/graziani-short.json");
        let log: Log = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut state = GameState::new(data, log.seed).unwrap();
        for o in &log.orders[..n] {
            state.apply(data, o.player, o.order.clone()).unwrap();
        }
        state
    }

    /// `placements` is exactly the hexes `apply` accepts: every returned
    /// hex places the piece on a clone, every other hex of its area is
    /// refused; the other side's pending pieces, which may not act yet,
    /// get none.
    fn placements_match_apply(data: &Data, state: &GameState) {
        let side = state.to_act().unwrap();
        let pending: Vec<&Piece> = state.pieces.values().filter(|p| p.is_pending()).collect();
        assert!(pending.iter().any(|p| p.side == side));
        for p in pending {
            if p.side != side {
                assert!(state.placements(data, &p.id).is_empty(), "{}", p.id);
                continue;
            }
            let Location::Pending(placement) = &p.location else {
                unreachable!()
            };
            let offered = state.placements(data, &p.id);
            assert!(!offered.is_empty(), "{} has nowhere to go", p.id);
            assert!(
                offered.windows(2).all(|w| w[0] < w[1]),
                "sorted, no duplicates"
            );
            for hex in area(data, placement) {
                let mut clone = state.clone();
                let r = clone.apply(
                    data,
                    p.side,
                    Order::Place {
                        piece: p.id.clone(),
                        hex: hex.clone(),
                    },
                );
                assert_eq!(
                    r.is_ok(),
                    offered.binary_search(&hex).is_ok(),
                    "{} at {hex}: apply {r:?}",
                    p.id
                );
            }
        }
    }

    #[test]
    fn a_new_games_placements_are_what_apply_accepts() {
        let data = data();
        let state = GameState::new(&data, 7).unwrap();
        placements_match_apply(&data, &state);
    }

    #[test]
    fn the_commonwealths_placements_after_the_axis_set_up_are_what_apply_accepts() {
        let data = data();
        let state = after(&data, 36);
        assert_eq!(state.pending, Some(Pending::SetUp { side: Side::Cw }));
        placements_match_apply(&data, &state);
    }

    #[test]
    fn placements_are_the_area_less_what_the_engine_refuses() {
        let data = data();
        let state = after(&data, 36);
        // supply:cw:7 may go anywhere on sheet D but the sea (8.37).
        let supply = PieceId::supply(Side::Cw, 7);
        let offered = state.placements(&data, &supply);
        assert!(offered.iter().all(|h| h.sheet() == 'D'), "{offered:?}");
        assert!(offered.contains(&HexId::parse("D3613").unwrap()));
        let Location::Pending(placement) = &state.pieces[&supply].location else {
            unreachable!()
        };
        let sea = area(&data, placement)
            .iter()
            .filter(|h| {
                data.terrain
                    .ceiling(data.map.hex(h).unwrap().terrain.name())
                    .is_none()
            })
            .count();
        assert!(sea > 0);
        assert_eq!(area(&data, placement).len(), offered.len() + sea);
    }

    #[test]
    fn a_piece_that_is_not_pending_has_no_placements() {
        let data = data();
        let state = GameState::new(&data, 7).unwrap();
        let placed = state.pieces.values().find(|p| p.hex().is_some()).unwrap();
        assert!(state.placements(&data, &placed.id).is_empty());
        assert!(
            state
                .placements(
                    &data,
                    &serde_json::from_value(serde_json::json!("unit:it:no-such-unit")).unwrap()
                )
                .is_empty()
        );
    }
}
