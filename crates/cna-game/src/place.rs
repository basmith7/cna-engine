//! Set-up orders: a `Place` puts a pending piece on a hex its placement
//! allows (59.2), under the stacking ceiling (9.14) and clear of enemy
//! units where the placement says so.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
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
        if let Some(points) = self.pending_motorisation.remove(piece) {
            *self
                .motorisation
                .entry(player)
                .or_default()
                .entry(hex.to_string())
                .or_default() += points;
        }
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
        if let Some(e) = self.supply_stack_refusal(data, p.kind, hex, 1) {
            return Err(e);
        }
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

    /// 32.1x: with the abstract supply module on, at most five supply
    /// units (dummies look real, 3.61) stand in a major city hex and three
    /// anywhere else; `adding` more of kind `kind` would break it. The
    /// off-map boxes take any number.
    pub(crate) fn supply_stack_refusal(
        &self,
        data: &Data,
        kind: PieceKind,
        hex: &HexId,
        adding: usize,
    ) -> Option<Illegal> {
        if !self.modules.abstract_supply
            || !matches!(kind, PieceKind::SupplyUnit | PieceKind::DummySupplyUnit)
        {
            return None;
        }
        let n = adding + self.supply_units_at(hex);
        let terrain = data.map.hex(hex)?.terrain;
        let cap = if terrain.name() == "major-city" { 5 } else { 3 };
        (n > cap).then(|| {
            Illegal::new(
                "32.1x",
                format!(
                    "{hex} ({}) would hold {n} supply units, over {cap}",
                    terrain.name()
                ),
            )
        })
    }

    /// The supply units, real or dummy, on a hex.
    pub(crate) fn supply_units_at(&self, hex: &HexId) -> usize {
        self.pieces
            .values()
            .filter(|p| {
                matches!(p.kind, PieceKind::SupplyUnit | PieceKind::DummySupplyUnit)
                    && p.hex() == Some(hex)
            })
            .count()
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

/// Whether `hex` is in the placement's area (59.2): inside every one of
/// its locators.
fn allowed(data: &Data, placement: &Placement, hex: &HexId) -> bool {
    placement.locators.iter().all(|l| in_locator(data, l, hex))
}

fn in_locator(data: &Data, locator: &Locator, hex: &HexId) -> bool {
    match locator {
        Locator::Hexes(list) => list.contains(hex),
        Locator::Within { of, hexes } => data.map.within(of, *hexes).contains(hex),
        Locator::Region(c) => data.map.country(hex) == Some(*c),
        Locator::Sheets(s) => s.contains(&hex.sheet()),
        Locator::Place(ids) => ids
            .iter()
            .any(|id| data.map.place(id).is_some_and(|p| p.hexes.contains(hex))),
        // Sheets run A to E west to east; within a sheet the first two
        // digits (cna's column CC, `HexId::row` here) grow eastward.
        Locator::WestOf(bound) => {
            hex.sheet() < bound.sheet() || (hex.sheet() == bound.sheet() && hex.row() < bound.row())
        }
        // Resolved at expansion; never pending.
        Locator::Box(_) | Locator::AirFacility => false,
    }
}

/// The placement's whole area (59.2): every hex `allowed` says yes to,
/// in map order, before stacking, `not_within` and `together`.
pub(crate) fn area(data: &Data, placement: &Placement) -> Vec<HexId> {
    let map = &data.map;
    // A short list to filter where a locator gives one; else the map.
    let seed: Option<Vec<HexId>> = placement.locators.iter().find_map(|l| match l {
        Locator::Hexes(h) => Some(h.clone()),
        Locator::Within { of, hexes } => Some(map.within(of, *hexes).into_iter().collect()),
        Locator::Place(ids) => Some(
            ids.iter()
                .filter_map(|id| map.place(id))
                .flat_map(|p| p.hexes.iter().cloned())
                .collect(),
        ),
        _ => None,
    });
    if placement
        .locators
        .iter()
        .any(|l| matches!(l, Locator::Box(_) | Locator::AirFacility))
    {
        return Vec::new();
    }
    match seed {
        Some(hexes) => hexes
            .into_iter()
            .filter(|h| allowed(data, placement, h))
            .collect(),
        None => map
            .hexes()
            .filter(|h| allowed(data, placement, &h.id))
            .map(|h| h.id.clone())
            .collect(),
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

    fn supply_game(data: &Data) -> GameState {
        GameState::with_modules(data, 1, crate::state::Modules::ALL).unwrap()
    }

    /// Places `ids` in `at` in turn, returning the first refusal.
    fn place_all(
        data: &Data,
        state: &mut GameState,
        ids: &[PieceId],
        at: &HexId,
    ) -> Option<Illegal> {
        ids.iter().find_map(|id| {
            let order = Order::Place {
                piece: id.clone(),
                hex: at.clone(),
            };
            state.apply(data, Side::Axis, order).err()
        })
    }

    // 32.1x: no more than three supply units, dummies included, in a hex
    // that is not a major city.
    #[test]
    fn a_fourth_supply_unit_in_an_ordinary_hex_is_refused() {
        let data = data();
        let mut state = supply_game(&data);
        let first = PieceId::supply(Side::Axis, 16);
        let at = state
            .placements(&data, &first)
            .into_iter()
            .find(|h| {
                data.map.hex(h).unwrap().terrain.name() != "major-city"
                    && state.stacked(&data, h) == 0
            })
            .unwrap();
        let ids = [
            first,
            PieceId::supply(Side::Axis, 17),
            PieceId::dummy_supply(Side::Axis, 1),
            PieceId::dummy_supply(Side::Axis, 2),
        ];
        let err = place_all(&data, &mut state, &ids, &at).expect("the fourth is refused");
        assert_eq!(err.case, "32.1x");
        assert!(state.pieces[&ids[3]].is_pending());
        // Without the module the stack is free.
        let mut free = GameState::new(&data, 1).unwrap();
        assert!(place_all(&data, &mut free, &ids, &at).is_none());
    }

    // 32.1x: a major city takes five.
    #[test]
    fn a_sixth_supply_unit_in_a_major_city_is_refused() {
        let data = data();
        let mut state = supply_game(&data);
        let tobruk = HexId::parse("C4807").unwrap();
        assert_eq!(data.map.hex(&tobruk).unwrap().terrain.name(), "major-city");
        // Three real ones already stand there.
        let ids = [
            PieceId::supply(Side::Axis, 16),
            PieceId::supply(Side::Axis, 17),
            PieceId::supply(Side::Axis, 18),
        ];
        let err = place_all(&data, &mut state, &ids, &tobruk).expect("the sixth is refused");
        assert_eq!(err.case, "32.1x");
        assert_eq!(state.pieces[&ids[0]].hex(), Some(&tobruk));
        assert_eq!(state.pieces[&ids[1]].hex(), Some(&tobruk));
        assert!(state.pieces[&ids[2]].is_pending());
        // `placements` agrees with `apply`.
        assert!(!state.placements(&data, &ids[2]).contains(&tobruk));
    }

    // 32.1x binds what is added, not 60.92's own set-up: it stacks five
    // supply units at Mersa Matruh, a village, so the cap is no load error.
    #[test]
    fn the_scenarios_own_stack_at_a_village_stands_but_takes_no_more() {
        let data = data();
        let state = supply_game(&data);
        let matruh = HexId::parse("D3714").unwrap();
        assert_eq!(state.supply_units_at(&matruh), 5);
        state.check_invariants(&data).unwrap();
        let err = state
            .supply_stack_refusal(&data, PieceKind::SupplyUnit, &matruh, 1)
            .unwrap();
        assert_eq!(err.case, "32.1x");
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
