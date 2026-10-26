//! Set-up orders: a `Place` puts a pending piece on a hex its placement
//! allows (59.2), under the stacking ceiling (9.14) and clear of enemy
//! units where the placement says so.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::{GameState, Pending};
use cna_data::map::HexId;
use cna_data::oa::Nation;
use cna_data::scenario::{Locator, Placement, Side, StackedWith};

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
        // Placed with its parent, or its own units with it, it starts
        // attached (4.25).
        self.attach_where_stacked(Some(std::slice::from_ref(piece)));
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

    /// `Order::Unplace`: the side setting up takes back a piece it placed
    /// (59.2), which waits again with its placement, its motorisation
    /// points and nothing attached to it.
    pub(crate) fn unplace(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        // The set-up as the scenario gives it: what the piece waited with.
        let fresh = GameState::with_modules(data, 0, self.modules)
            .map_err(|e| Illegal::new("59.2", format!("the set-up does not build: {e:#}")))?;
        let refuse = |why: &str| Err(Illegal::new("59.2", format!("{piece}: {why}")));
        let (Some(p), Some(f)) = (self.pieces.get(piece), fresh.pieces.get(piece)) else {
            return refuse("no such piece");
        };
        if p.side != player {
            return refuse("not this side's");
        }
        let (Some(hex), Location::Pending(_)) = (p.hex().cloned(), &f.location) else {
            return refuse("not placed at set-up");
        };
        let unit = piece.unit();
        for q in self.pieces.values_mut() {
            if unit.is_some() && q.attached_to == unit {
                q.attached_to = None;
            }
        }
        let p = self.pieces.get_mut(piece).expect("checked");
        p.location = f.location.clone();
        p.attached_to = None;
        if let Some(&points) = fresh.pending_motorisation.get(piece) {
            if let Some(m) = self
                .motorisation
                .get_mut(&player)
                .and_then(|m| m.get_mut(hex.as_str()))
            {
                *m = m.saturating_sub(points);
            }
            self.pending_motorisation.insert(piece.clone(), points);
        }
        Ok(vec![Event::Unplaced {
            piece: piece.clone(),
        }])
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
        if placement
            .locators
            .contains(&Locator::StackedWith(StackedWith::GermanUnit))
            && !self.pieces.values().any(|q| {
                q.side == player
                    && q.nation == Some(Nation::De)
                    && matches!(q.kind, PieceKind::Combat | PieceKind::Hq)
                    && q.hex() == Some(hex)
            })
        {
            return Err(Illegal::new(
                "R-135",
                format!("{piece} starts with a German unit; {hex} holds none"),
            ));
        }
        // `together`: the first of the block placed chooses the hex.
        if let Some(g) = p.group
            && let Some(mate) = self
                .pieces
                .values()
                .find(|q| q.group == Some(g) && q.id != p.id && q.hex().is_some())
            && mate.hex() != Some(hex)
        {
            return Err(Illegal::new(
                "59.2",
                format!(
                    "{piece}'s block goes into one hex: {} is at {}",
                    mate.id,
                    mate.hex().expect("placed")
                ),
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
        if let (Some(n), Some(entry)) = (placement.apart, p.entry) {
            let near = data.map.within(hex, n.saturating_sub(1));
            if let Some(other) = self.pieces.values().find(|q| {
                q.entry
                    .is_some_and(|e| e.block == entry.block && e.index != entry.index)
                    && q.hex().is_some_and(|qh| near.contains(qh))
            }) {
                return Err(Illegal::new(
                    "R-136",
                    format!(
                        "{piece}'s block goes {n} hexes apart: {} is at {}",
                        other.id,
                        other.hex().expect("on a hex")
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
        // Needs the state: `check_place` looks at the hex's units.
        Locator::StackedWith(_) => true,
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

    // 60.92 via cna's `together`: a block's supply units share one hex,
    // the first one placed choosing it.
    #[test]
    fn a_together_block_goes_into_one_hex() {
        let data = data();
        let mut state = supply_game(&data);
        let g = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.group.is_some())
            .and_then(|p| p.group)
            .expect("an Axis group");
        let group: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.group == Some(g))
            .map(|p| p.id.clone())
            .collect();
        assert_eq!(group.len(), 3);
        let offered = state.placements(&data, &group[0]);
        let (first, other) = (offered[0].clone(), offered[1].clone());
        state
            .apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: group[0].clone(),
                    hex: first.clone(),
                },
            )
            .unwrap();
        assert_eq!(state.placements(&data, &group[1]), vec![first.clone()]);
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: group[1].clone(),
                    hex: other,
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "59.2");
        assert!(err.reason.contains("one hex"), "{}", err.reason);
        state
            .apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: group[1].clone(),
                    hex: first,
                },
            )
            .unwrap();
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

    /// R-135: Rommel starts on a hex holding a German unit, and only
    /// there; his counter is a commander, not a combat unit.
    #[test]
    fn rommel_sets_up_with_a_german_unit() {
        let data = Data::load("operation-crusader").unwrap();
        let mut state = GameState::new(&data, 7).unwrap();
        let rommel = PieceId::from(cna_data::oa::UnitId::parse("unit:de:rommel").unwrap());
        assert_eq!(state.pieces[&rommel].kind, PieceKind::Commander);
        assert!(state.pieces[&rommel].is_pending());
        let german = |s: &GameState, h: &HexId| {
            s.pieces.values().any(|q| {
                q.nation == Some(cna_data::oa::Nation::De)
                    && q.kind != PieceKind::Commander
                    && q.hex() == Some(h)
            })
        };
        let offered = state.placements(&data, &rommel);
        assert!(!offered.is_empty());
        assert!(offered.iter().all(|h| german(&state, h)), "{offered:?}");
        let italian_only = state
            .pieces
            .values()
            .filter_map(|q| q.hex().cloned())
            .find(|h| !german(&state, h))
            .expect("a hex without Germans");
        let refused = state
            .apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: rommel.clone(),
                    hex: italian_only,
                },
            )
            .unwrap_err();
        assert_eq!(refused.case, "R-135");
        let hex = offered[0].clone();
        state
            .apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: rommel.clone(),
                    hex: hex.clone(),
                },
            )
            .unwrap();
        assert_eq!(state.pieces[&rommel].hex(), Some(&hex));
    }

    /// R-136 (61.41): no Italian division within two hexes of another;
    /// a division's own units may stand together.
    #[test]
    fn the_italian_divisions_set_up_two_hexes_apart() {
        let data = Data::load("rommels-arrival").unwrap();
        let mut state = GameState::new(&data, 7).unwrap();
        let unit = |s: &str| PieceId::from(cna_data::oa::UnitId::parse(s).unwrap());
        let (pavia, bologna) = (
            unit("unit:it:17th-pavia-div-hq"),
            unit("unit:it:25th-bologna-div-hq"),
        );
        let at = HexId::parse("A2307").unwrap();
        let place = |state: &mut GameState, piece: &PieceId, hex: &HexId| {
            state.apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: piece.clone(),
                    hex: hex.clone(),
                },
            )
        };
        let area = state.placements(&data, &bologna);
        place(&mut state, &pavia, &at).unwrap();
        let next = data.map.neighbours(&at);
        let two = data
            .map
            .within(&at, 2)
            .into_iter()
            .find(|h| {
                !next.contains(h) && h != &at && state.placements(&data, &bologna).contains(h)
            })
            .expect("a hex two away in the area");
        let offered = state.placements(&data, &bologna);
        assert!(
            offered.iter().all(|h| h != &at && !next.contains(h)),
            "{offered:?}"
        );
        let touching = next.iter().find(|h| area.contains(h)).unwrap();
        let refused = place(&mut state, &bologna, touching).unwrap_err();
        assert_eq!(refused.case, "R-136");
        // Pavia's own units may join or touch its HQ.
        let own = state
            .pieces
            .values()
            .find(|q| q.is_pending() && q.id != pavia && q.entry == state.pieces[&pavia].entry)
            .map(|q| q.id.clone())
            .expect("a Pavia unit");
        assert!(state.placements(&data, &own).contains(&at));
        place(&mut state, &bologna, &two).unwrap();
    }

    /// 59.2: a side setting up may take back a piece it placed, which
    /// waits again with its placement, so a crowded `apart` band (R-136)
    /// never strands the set-up.
    #[test]
    fn a_placed_piece_can_be_taken_back_during_set_up() {
        let data = Data::load("rommels-arrival").unwrap();
        let mut state = GameState::new(&data, 7).unwrap();
        let unit = |s: &str| PieceId::from(cna_data::oa::UnitId::parse(s).unwrap());
        let (pavia, bologna) = (
            unit("unit:it:17th-pavia-div-hq"),
            unit("unit:it:25th-bologna-div-hq"),
        );
        let fresh = state.clone();
        let at = HexId::parse("A2307").unwrap();
        let next = data.map.neighbours(&at);
        let area = state.placements(&data, &bologna);
        let touching = next.iter().find(|h| area.contains(h)).unwrap().clone();
        state
            .apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: pavia.clone(),
                    hex: at.clone(),
                },
            )
            .unwrap();
        assert!(!state.placements(&data, &bologna).contains(&touching));
        state
            .apply(
                &data,
                Side::Axis,
                Order::Unplace {
                    piece: pavia.clone(),
                },
            )
            .unwrap();
        assert_eq!(state.pieces[&pavia], fresh.pieces[&pavia]);
        assert_eq!(state.placements(&data, &bologna), area);
        // Only a piece placed at set-up, and only one's own.
        let refused = state
            .apply(
                &data,
                Side::Axis,
                Order::Unplace {
                    piece: pavia.clone(),
                },
            )
            .unwrap_err();
        assert_eq!(refused.case, "59.2");
        let cw = state
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.hex().is_some())
            .unwrap()
            .id
            .clone();
        assert_eq!(
            state
                .apply(&data, Side::Axis, Order::Unplace { piece: cw })
                .unwrap_err()
                .case,
            "59.2"
        );
    }
}
