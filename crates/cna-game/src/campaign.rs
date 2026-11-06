//! The campaign games' victory (SPI 64.71–64.73, R-109) under the
//! `campaign_victory` module, with §32's supply units standing in for the
//! Logistics Game's Supply Dumps (60.92): a unit "traces supply" when a
//! friendly supply unit with ammunition or fuel left lies within the
//! printed truck Movement Points (90 for 64.71, 60 for 64.72), traced
//! from the supply unit as a medium truck moves (32.17), and that unit has an unbroken
//! route (R-110) to Tobruk or to Nofilia, the map's end of the road to
//! Tripoli (8.89). Occupation's end-of-game test (64.73) is a unit of at
//! least one TOE point that may draw on a supply unit (32.17).
//!
//! Both automatic wins are judged at each OpStage's end, as phase A of
//! the next opens: the Axis's when it has held every hex of the
//! scenario's Axis level (Alexandria and Cairo) at every OpStage end from
//! the start of a Game-Turn to its last OpStage; the Commonwealth's from
//! the end of Game-Turn 34 (so from OpStage 1 of Game-Turn 35 on).

use crate::data::Data;
use crate::event::Event;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use crate::victory::{VictoryReport, objective_hexes};
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::{Side, Victory, VictoryKind, VictoryLevel};
use cna_data::time::GameTime;
use cna_rules::movement::Mobility;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

/// The Axis automatic win's supply line, in truck Movement Points (64.71).
const AXIS_LINE: u32 = 90;
/// The Commonwealth automatic win's line (64.72; R-109 keeps both).
const CW_LINE: u32 = 60;
/// The Commonwealth's test applies from this Game-Turn on (64.72).
const CW_FROM_TURN: u8 = 35;

/// The campaign games' victory block: a points victory ranked by ratio
/// (64.76 is theirs alone).
pub(crate) fn campaign(data: &Data) -> Option<&Victory> {
    data.scenario
        .victory
        .as_ref()
        .filter(|v| v.kind == VictoryKind::Points && !v.ratios.is_empty())
}

impl GameState {
    /// Whether 64.71–64.73 are judged in this game: the module, §32's
    /// supply units to trace to, and a campaign game.
    pub(crate) fn campaign_rules(&self, data: &Data) -> bool {
        self.modules.campaign_victory && self.modules.abstract_supply && campaign(data).is_some()
    }

    /// The hexes of `side`'s supply units with ammunition or fuel left
    /// that have a route, as a medium truck, from Tobruk or Nofilia
    /// (R-110: no hex in `blocked`).
    fn fed_supply_hexes(
        &self,
        data: &Data,
        side: Side,
        blocked: &BTreeSet<HexId>,
    ) -> BTreeSet<HexId> {
        let mut sources: Vec<HexId> = Vec::new();
        for id in ["tobruk", "nofilia"] {
            if let Some(p) = data.map.place(id) {
                sources.extend(p.hexes.iter().cloned());
            }
        }
        let mut seen: BTreeSet<HexId> = sources
            .iter()
            .filter(|h| !blocked.contains(*h))
            .cloned()
            .collect();
        let mut queue: VecDeque<HexId> = seen.iter().cloned().collect();
        while let Some(at) = queue.pop_front() {
            for to in data.map.neighbours(&at) {
                if seen.contains(&to) {
                    continue;
                }
                if self
                    .trace_step(data, Mobility::Vehicle, blocked, &at, &to)
                    .is_some()
                {
                    seen.insert(to.clone());
                    queue.push_back(to);
                }
            }
        }
        self.pieces
            .values()
            .filter(|q| q.side == side && q.kind == PieceKind::SupplyUnit)
            .filter(|q| q.load.is_some_and(|l| l.ammo + l.fuel > 0))
            .filter_map(|q| q.hex().cloned())
            .filter(|h| seen.contains(h))
            .collect()
    }

    /// The hexes within `line` Movement Points of a fed supply unit of
    /// `side`, traced as a medium truck moves from it (32.17's trace at
    /// the printed length).
    fn within_line(&self, data: &Data, side: Side, line: u32) -> BTreeSet<HexId> {
        let blocked = self.trace_blocked(data, side);
        let budget = Cp::whole(line);
        let mut best: BTreeMap<HexId, Cp> = self
            .fed_supply_hexes(data, side, &blocked)
            .into_iter()
            .map(|h| (h, Cp::ZERO))
            .collect();
        let mut queue: BinaryHeap<Reverse<(Cp, HexId)>> = best
            .keys()
            .map(|h| Reverse((Cp::ZERO, h.clone())))
            .collect();
        while let Some(Reverse((cost, at))) = queue.pop() {
            if best.get(&at).is_some_and(|b| *b < cost) {
                continue;
            }
            for to in data.map.neighbours(&at) {
                let Some(step) = self.trace_step(data, Mobility::Vehicle, &blocked, &at, &to)
                else {
                    continue;
                };
                let total = cost + step;
                if total <= budget && best.get(&to).is_none_or(|b| total < *b) {
                    best.insert(to.clone(), total);
                    queue.push(Reverse((total, to)));
                }
            }
        }
        best.into_keys().collect()
    }

    /// `side`'s combat units on the map that trace supply within `line`
    /// (the module's reading of 64.71–64.72).
    fn traced(&self, data: &Data, side: Side, line: u32) -> BTreeSet<PieceId> {
        let reach = self.within_line(data, side, line);
        self.pieces
            .values()
            .filter(|p| p.side == side && p.kind == PieceKind::Combat)
            .filter(|p| p.hex().is_some_and(|h| reach.contains(h)))
            .map(|p| p.id.clone())
            .collect()
    }

    /// Whether every hex of the Axis level's objectives holds an Axis
    /// combat unit tracing the 90-point line, and no Commonwealth one.
    fn axis_holds_cities(&self, data: &Data, victory: &Victory) -> bool {
        let hexes: Vec<HexId> = victory
            .levels
            .iter()
            .filter(|l| l.side == Side::Axis)
            .flat_map(|l| l.all_of.iter())
            .flat_map(|o| objective_hexes(data, o).1)
            .collect();
        if hexes.is_empty() {
            return false;
        }
        let traced = self.traced(data, Side::Axis, AXIS_LINE);
        hexes.iter().all(|h| {
            let here: Vec<_> = self
                .pieces
                .values()
                .filter(|p| p.kind == PieceKind::Combat && p.hex() == Some(h))
                .collect();
            here.iter().all(|p| p.side == Side::Axis) && here.iter().any(|p| traced.contains(&p.id))
        })
    }

    /// The automatic win due as phase A of `now` opens, if any: the
    /// Axis's after a full Game-Turn held (64.71), else the
    /// Commonwealth's from Game-Turn 35 (64.72). Updates the Axis's
    /// holding record.
    pub(crate) fn automatic_win(&mut self, data: &Data, now: GameTime) -> Option<VictoryReport> {
        if !self.campaign_rules(data) {
            return None;
        }
        let victory = campaign(data)?;
        if self.axis_holds_cities(data, victory) {
            let since = *self.axis_held_since.get_or_insert(now);
            // Held at the start of a Game-Turn's first OpStage and at the
            // end of its last (R-109).
            if now.opstage == 1 && since <= GameTime::new(now.game_turn.saturating_sub(1), 1) {
                let level = victory
                    .levels
                    .iter()
                    .filter(|l| l.side == Side::Axis)
                    .map(|l| l.level)
                    .max()
                    .unwrap_or(VictoryLevel::Strategic);
                return Some(VictoryReport::automatic(
                    Side::Axis,
                    level,
                    format!(
                        "64.71: the Axis held every hex of Alexandria and Cairo, \
                         supplied, from Game-Turn {} OpStage {} through Game-Turn {}",
                        since.game_turn,
                        since.opstage,
                        now.game_turn - 1
                    ),
                ));
            }
        } else {
            self.axis_held_since = None;
        }
        if now >= GameTime::new(CW_FROM_TURN, 1)
            && self.traced(data, Side::Axis, CW_LINE).is_empty()
        {
            return Some(VictoryReport::automatic(
                Side::Cw,
                VictoryLevel::Strategic,
                format!(
                    "64.72: no Axis combat unit on the map traced supply \
                     within {CW_LINE} truck Movement Points at the end of \
                     Game-Turn {} OpStage {}",
                    if now.opstage == 1 {
                        now.game_turn - 1
                    } else {
                        now.game_turn
                    },
                    if now.opstage == 1 { 3 } else { now.opstage - 1 }
                ),
            ));
        }
        None
    }

    /// Ends the game on an automatic win due as phase A opens.
    pub(crate) fn end_by_automatic_win(&mut self, data: &Data, events: &mut Vec<Event>) -> bool {
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        if now <= data.scenario.start {
            return false;
        }
        let Some(report) = self.automatic_win(data, now) else {
            return false;
        };
        self.over = Some(report.clone());
        events.push(Event::GameOver { report });
        true
    }

    /// Whether `piece` passes 64.73's end-of-game supply test: at least
    /// one TOE point and a supply unit to draw on (32.17), in range or,
    /// for a unit with no CPA to trace with, in its own hex.
    pub(crate) fn passes_end_supply(&self, data: &Data, piece: &PieceId) -> bool {
        let Some(p) = self.pieces.get(piece).filter(|p| p.toe >= 1) else {
            return false;
        };
        let fed_here = p.hex().is_some_and(|hex| {
            self.pieces.values().any(|q| {
                q.side == p.side
                    && q.kind == PieceKind::SupplyUnit
                    && q.hex() == Some(hex)
                    && q.load.is_some_and(|l| l.ammo + l.fuel > 0)
            })
        });
        fed_here || !self.supply_in_range(data, piece).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use crate::piece::{Load, Location};
    use crate::state::Modules;
    use crate::state::test_support::auto_place;

    /// The campaign's set-up, every piece placed, with `side` swept off.
    fn campaign_without(data: &Data, side: Side) -> GameState {
        let mut state = GameState::with_modules(data, 5, Modules::ALL).unwrap();
        auto_place(data, &mut state);
        for p in state.pieces.values_mut() {
            if p.side == side && p.hex().is_some() {
                p.location = Location::Eliminated;
            }
        }
        state
    }

    fn combat(state: &GameState, side: Side, skip: usize) -> PieceId {
        state
            .pieces
            .values()
            .filter(|p| p.side == side && p.kind == PieceKind::Combat && p.toe >= 1)
            .nth(skip)
            .unwrap()
            .id
            .clone()
    }

    fn supply_unit(state: &mut GameState, side: Side, at: &HexId) -> PieceId {
        let id = state
            .pieces
            .values()
            .find(|p| p.side == side && p.kind == PieceKind::SupplyUnit)
            .expect("a supply unit")
            .id
            .clone();
        for p in state.pieces.values_mut() {
            if p.side == side && p.kind == PieceKind::SupplyUnit {
                p.location = Location::Eliminated;
            }
        }
        state.put(&id, Location::Hex(at.clone()));
        state.pieces.get_mut(&id).unwrap().load = Some(Load { ammo: 10, fuel: 10 });
        id
    }

    fn city_hexes(data: &Data) -> Vec<HexId> {
        let v = campaign(data).unwrap();
        v.levels
            .iter()
            .flat_map(|l| l.all_of.iter())
            .flat_map(|o| objective_hexes(data, o).1)
            .collect()
    }

    /// The Axis in every hex of Alexandria and Cairo, a loaded supply unit
    /// in the first, nothing of the Commonwealth's on the map.
    fn axis_in_the_cities(data: &Data) -> GameState {
        let mut state = campaign_without(data, Side::Cw);
        let hexes = city_hexes(data);
        for (i, h) in hexes.iter().enumerate() {
            let id = combat(&state, Side::Axis, i);
            state.put(&id, Location::Hex(h.clone()));
        }
        supply_unit(&mut state, Side::Axis, &hexes[0]);
        state
    }

    fn at(state: &mut GameState, data: &Data, gt: u8, st: u8) -> Option<VictoryReport> {
        state.clock = Clock::opening(GameTime::new(gt, st));
        let mut events = Vec::new();
        let ended = state.end_by_automatic_win(data, &mut events);
        assert_eq!(ended, state.is_over());
        if ended {
            assert!(matches!(events.as_slice(), [Event::GameOver { .. }]));
        }
        state.over.clone()
    }

    #[test]
    fn a_full_game_turn_in_alexandria_and_cairo_wins_for_the_axis_64_71() {
        for slug in ["campaign-game", "desert-fox-campaign"] {
            let data = Data::load(slug).unwrap();
            let mut state = axis_in_the_cities(&data);
            // First held as Game-Turn 40 OpStage 2 opens: not a full turn
            // until Game-Turn 42 opens.
            assert_eq!(at(&mut state, &data, 40, 2), None, "{slug}");
            assert_eq!(at(&mut state, &data, 40, 3), None);
            assert_eq!(
                at(&mut state, &data, 41, 1),
                None,
                "41 is the first full turn"
            );
            assert_eq!(at(&mut state, &data, 41, 2), None);
            assert_eq!(at(&mut state, &data, 41, 3), None);
            let report = at(&mut state, &data, 42, 1).expect("the Axis wins");
            assert_eq!(report.axis, Some(VictoryLevel::Strategic));
            assert_eq!(report.cw, None);
            assert!(
                report.automatic.as_deref().unwrap().starts_with("64.71"),
                "{report:?}"
            );
            assert!(state.is_over());
        }
    }

    #[test]
    fn a_break_in_the_hold_or_its_supply_starts_the_count_again_64_71() {
        let data = Data::load("campaign-game").unwrap();
        let mut state = axis_in_the_cities(&data);
        assert_eq!(at(&mut state, &data, 20, 1), None);
        // The supply unit runs dry: the hold is unsupplied.
        let su = state
            .pieces
            .values()
            .find(|p| p.kind == PieceKind::SupplyUnit && p.hex().is_some())
            .unwrap()
            .id
            .clone();
        state.pieces.get_mut(&su).unwrap().load = Some(Load { ammo: 0, fuel: 0 });
        assert_eq!(at(&mut state, &data, 20, 2), None);
        assert_eq!(state.axis_held_since, None);
        state.pieces.get_mut(&su).unwrap().load = Some(Load { ammo: 5, fuel: 0 });
        assert_eq!(at(&mut state, &data, 20, 3), None);
        assert_eq!(at(&mut state, &data, 21, 1), None, "held only since 20/3");
        assert_eq!(at(&mut state, &data, 21, 2), None);
        assert_eq!(at(&mut state, &data, 21, 3), None);
        let r = at(&mut state, &data, 22, 1).expect("Game-Turn 21 held whole");
        assert_eq!(r.axis, Some(VictoryLevel::Strategic));
    }

    #[test]
    fn the_commonwealth_wins_when_no_axis_unit_traces_but_not_before_game_turn_35_64_72() {
        let data = Data::load("campaign-game").unwrap();
        let mut state = campaign_without(&data, Side::Axis);
        assert_eq!(at(&mut state, &data, 34, 2), None);
        assert_eq!(
            at(&mut state, &data, 34, 3),
            None,
            "OpStage 3 of 34 is still before 35"
        );
        let report = at(&mut state, &data, 35, 1).expect("the Commonwealth wins");
        assert_eq!(report.cw, Some(VictoryLevel::Strategic));
        assert!(
            report.automatic.as_deref().unwrap().starts_with("64.72"),
            "{report:?}"
        );
        assert!(
            report
                .automatic
                .as_deref()
                .unwrap()
                .contains("Game-Turn 34 OpStage 3")
        );
    }

    #[test]
    fn one_supplied_axis_unit_holds_off_the_commonwealth_win_64_72() {
        let data = Data::load("campaign-game").unwrap();
        let mut state = campaign_without(&data, Side::Cw);
        for p in state.pieces.values_mut() {
            if p.side == Side::Axis && p.kind == PieceKind::Combat {
                p.location = Location::Eliminated;
            }
        }
        let tobruk = data.map.place("tobruk").unwrap().hex.clone();
        let id = combat(&state, Side::Axis, 0);
        state.put(&id, Location::Hex(tobruk.clone()));
        supply_unit(&mut state, Side::Axis, &tobruk);
        assert_eq!(at(&mut state, &data, 50, 1), None);
        // Out of supply, it no longer counts.
        for p in state.pieces.values_mut() {
            if p.kind == PieceKind::SupplyUnit {
                p.load = Some(Load { ammo: 0, fuel: 0 });
            }
        }
        assert!(at(&mut state, &data, 50, 2).is_some());
    }

    #[test]
    fn a_unit_with_no_cpa_draws_on_supply_in_its_own_hex_64_73() {
        let data = Data::load("campaign-game").unwrap();
        let mut state = GameState::with_modules(&data, 5, Modules::ALL).unwrap();
        auto_place(&data, &mut state);
        // Tobruk's coastal artillery prints no CPA (32.17's half of it is
        // nothing), but a loaded supply unit in its hex still feeds it.
        let tobruk = HexId::parse("C4807").unwrap();
        let guns = state
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Axis
                    && p.kind == PieceKind::Combat
                    && p.toe >= 1
                    && state.cpa(p).is_none()
            })
            .expect("an Axis combat unit with no CPA")
            .id
            .clone();
        state.put(&guns, Location::Hex(tobruk.clone()));
        let source = supply_unit(&mut state, Side::Axis, &tobruk);
        assert!(state.passes_end_supply(&data, &guns));
        state.pieces.get_mut(&source).unwrap().load = Some(Load { ammo: 0, fuel: 0 });
        assert!(
            !state.passes_end_supply(&data, &guns),
            "an empty supply unit feeds nothing"
        );
    }

    #[test]
    fn off_the_module_or_outside_a_campaign_nothing_ends_early() {
        let data = Data::load("campaign-game").unwrap();
        let mut state = campaign_without(&data, Side::Axis);
        state.modules.campaign_victory = false;
        assert_eq!(at(&mut state, &data, 40, 1), None);
        let data = Data::load("operation-crusader").unwrap();
        let mut state = campaign_without(&data, Side::Axis);
        assert_eq!(at(&mut state, &data, 70, 1), None);
    }
}
