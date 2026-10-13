//! Victory (SPI 60.8, 60.81; R-110): at the scenario's end, each side's
//! highest level whose every objective it holds with a combat unit that
//! can trace a route to the level's source. The report says what was
//! checked and what the data could only state in prose.

use crate::data::Data;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::scenario::{Level, Locator, Objective, Side, SupplyTrace, VictoryKind, VictoryLevel};
use cna_rules::movement::{Mobility, Step, Weather, step_cost};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

/// What the end of the game found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VictoryReport {
    pub axis: Option<VictoryLevel>,
    pub cw: Option<VictoryLevel>,
    /// Every objective of every level, as it stood.
    pub checked: Vec<Condition>,
    /// Conditions the data states only in prose, which nothing checked.
    pub unchecked: Vec<String>,
}

impl VictoryReport {
    pub fn level(&self, side: Side) -> Option<VictoryLevel> {
        match side {
            Side::Axis => self.axis,
            Side::Cw => self.cw,
        }
    }
}

/// One objective of one level at the end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    pub side: Side,
    pub level: VictoryLevel,
    /// The place id, or the hexes, as the scenario names them.
    pub objective: String,
    pub hexes: Vec<HexId>,
    pub any_hex: bool,
    /// The objective's hexes held by this side's combat units that trace
    /// to the level's source.
    pub held: Vec<HexId>,
    /// Holding combat units with no route to the source (R-110).
    pub unsupplied: Vec<PieceId>,
    pub met: bool,
}

/// Each side's level at the end of `state`, with everything checked.
pub fn evaluate(data: &Data, state: &GameState) -> VictoryReport {
    let mut report = VictoryReport {
        axis: None,
        cw: None,
        checked: Vec::new(),
        unchecked: Vec::new(),
    };
    let Some(victory) = &data.scenario.victory else {
        report
            .unchecked
            .push("the scenario has no victory block".into());
        return report;
    };
    if victory.kind != VictoryKind::Levels {
        report
            .unchecked
            .push("a points victory is not evaluated".into());
        return report;
    }
    for level in &victory.levels {
        let trace = Tracer::new(data, state, level.side);
        if level.supply_trace.is_none()
            && let Some(prose) = &level.supply
        {
            report
                .unchecked
                .push(format!("{} {}: {prose}", level.side, level.level));
        }
        let mut check = |objective: &Objective| -> bool {
            if let Some(c) = &objective.hold.constraint {
                report
                    .unchecked
                    .push(format!("{} {}: {c}", level.side, level.level));
            }
            let condition = trace.condition(level, objective);
            let met = condition.met;
            report.checked.push(condition);
            met
        };
        // Every objective is checked and reported, met or not.
        let all: Vec<bool> = level.all_of.iter().map(&mut check).collect();
        let any: Vec<bool> = level.any_of.iter().map(&mut check).collect();
        if all.iter().all(|m| *m) && (any.is_empty() || any.contains(&true)) {
            let slot = match level.side {
                Side::Axis => &mut report.axis,
                Side::Cw => &mut report.cw,
            };
            if slot.is_none_or(|l| l < level.level) {
                *slot = Some(level.level);
            }
        }
    }
    report
}

/// The route trace of R-110 for one side: as a medium truck in normal
/// weather, through no hex holding an enemy combat unit and no enemy-ZOC
/// hex without a friendly combat unit in it. Every enemy hex that could
/// exert a ZOC does: 10.16's declarations belong to a movement segment,
/// and the game is over. Supply units, real or dummy, and HQs are not
/// units here (R-119): they block, deny and lift nothing.
struct Tracer<'a> {
    data: &'a Data,
    state: &'a GameState,
    side: Side,
    blocked: BTreeSet<HexId>,
}

impl<'a> Tracer<'a> {
    fn new(data: &'a Data, state: &'a GameState, side: Side) -> Tracer<'a> {
        let enemy = side.other();
        let mut blocked = BTreeSet::new();
        let mut friendly = BTreeSet::new();
        for p in state.pieces.values() {
            if p.kind != PieceKind::Combat {
                continue;
            }
            if let Some(h) = p.hex() {
                if p.side == enemy {
                    blocked.insert(h.clone());
                } else {
                    friendly.insert(h.clone());
                }
            }
        }
        let enemy_hexes: Vec<HexId> = blocked.iter().cloned().collect();
        for src in enemy_hexes {
            if !state.could_exert(data, &src) {
                continue;
            }
            for n in data.map.neighbours(&src) {
                if !friendly.contains(&n) && state.reaches(data, &src, &n) {
                    blocked.insert(n);
                }
            }
        }
        Tracer {
            data,
            state,
            side,
            blocked,
        }
    }

    fn hexes_of(&self, objective: &Objective) -> (String, Vec<HexId>) {
        match &objective.hold.locator {
            Locator::Place(id) => (
                id.clone(),
                self.data
                    .map
                    .place(id)
                    .map(|p| p.hexes.clone())
                    .unwrap_or_default(),
            ),
            Locator::Hexes(h) => (
                h.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
                h.clone(),
            ),
            other => (format!("{other:?}"), Vec::new()),
        }
    }

    /// Combat units of this side in a hex, with no enemy combat unit
    /// there (R-119).
    fn holders(&self, hex: &HexId) -> Vec<PieceId> {
        let mut ids: Vec<PieceId> = Vec::new();
        for p in self.state.pieces.values() {
            if p.hex() != Some(hex) || p.kind != PieceKind::Combat {
                continue;
            }
            if p.side != self.side {
                return Vec::new();
            }
            ids.push(p.id.clone());
        }
        ids.sort();
        ids
    }

    fn condition(&self, level: &Level, objective: &Objective) -> Condition {
        let (name, hexes) = self.hexes_of(objective);
        let mut held = Vec::new();
        let mut unsupplied = Vec::new();
        for h in &hexes {
            let holders = self.holders(h);
            if holders.is_empty() {
                continue;
            }
            let mut supplied = false;
            for id in holders {
                match &level.supply_trace {
                    Some(t) if !self.traces(h, t) => unsupplied.push(id),
                    _ => supplied = true,
                }
            }
            if supplied {
                held.push(h.clone());
            }
        }
        let met = !hexes.is_empty()
            && if objective.any_hex {
                !held.is_empty()
            } else {
                held.len() == hexes.len()
            };
        Condition {
            side: level.side,
            level: level.level,
            objective: name,
            hexes,
            any_hex: objective.any_hex,
            held,
            unsupplied,
            met,
        }
    }

    fn is_source(&self, hex: &HexId, trace: &SupplyTrace) -> bool {
        trace.sheet == Some(hex.sheet())
            || trace.hexes.contains(hex)
            || trace
                .places
                .iter()
                .filter_map(|id| self.data.map.place(id))
                .any(|p| p.hexes.contains(hex))
    }

    /// Whether a route runs from `from` to any hex of `trace`.
    fn traces(&self, from: &HexId, trace: &SupplyTrace) -> bool {
        let map = &self.data.map;
        let mut seen = BTreeSet::from([from.clone()]);
        let mut queue = VecDeque::from([from.clone()]);
        while let Some(h) = queue.pop_front() {
            if self.is_source(&h, trace) {
                return true;
            }
            let Some(here) = map.hex(&h) else { continue };
            for n in map.neighbours(&h) {
                if seen.contains(&n) || self.blocked.contains(&n) {
                    continue;
                }
                let Some(to) = map.hex(&n) else { continue };
                let hexside = map.hexside(&h, &n);
                let step = Step {
                    from: here,
                    to,
                    hexside,
                    climbs: hexside.is_some_and(|hs| hs.climbs(&h, &n)),
                    weather: Weather::Normal,
                };
                if step_cost(&self.data.terrain, Mobility::Vehicle, &step).is_err() {
                    continue;
                }
                seen.insert(n.clone());
                queue.push_back(n);
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::Location;
    use crate::state::test_support::set_up;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn pid(s: &str) -> PieceId {
        PieceId::from(cna_data::oa::UnitId::parse(s).unwrap())
    }

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    /// The set-up with one side's pieces all taken off the map.
    fn without(data: &Data, side: Side) -> GameState {
        let mut state = set_up(data, 5);
        for p in state.pieces.values_mut() {
            if p.side == side {
                p.location = Location::Eliminated;
            }
        }
        state
    }

    fn put(state: &mut GameState, id: &str, h: &str) {
        state.pieces.get_mut(&pid(id)).unwrap().location = Location::Hex(hex(h));
    }

    fn condition<'a>(
        r: &'a VictoryReport,
        side: Side,
        level: VictoryLevel,
        obj: &str,
    ) -> &'a Condition {
        r.checked
            .iter()
            .find(|c| c.side == side && c.level == level && c.objective == obj)
            .unwrap_or_else(|| panic!("no condition {side} {level} {obj}"))
    }

    #[test]
    fn the_set_up_gives_neither_side_a_level() {
        let data = graziani();
        let state = set_up(&data, 5);
        let r = evaluate(&data, &state);
        assert_eq!((r.axis, r.cw), (None, None));
        assert_eq!(r.checked.len(), 1 + 2 + 4 + 3 + 5 + 1);
        assert!(r.unchecked.is_empty());
        // The Axis holds Fort Maddalena and Giarabub with a route to
        // Tobruk, but not Sidi Barrani (Commonwealth) nor Sollum (empty).
        let fm = condition(&r, Side::Axis, VictoryLevel::Tactical, "fort-maddalena");
        assert_eq!((fm.met, &fm.held[..]), (true, &[hex("C3019")][..]));
        assert!(fm.unsupplied.is_empty());
        assert!(condition(&r, Side::Axis, VictoryLevel::Tactical, "giarabub").met);
        let sb = condition(&r, Side::Axis, VictoryLevel::Tactical, "sidi-barrani");
        assert_eq!((sb.met, sb.held.len()), (false, 0));
        assert!(!condition(&r, Side::Axis, VictoryLevel::Tactical, "sollum").met);
        // The Commonwealth holds Tobruk's objective nowhere, Halfaya yes.
        assert!(!condition(&r, Side::Cw, VictoryLevel::Strategic, "tobruk").met);
        assert!(condition(&r, Side::Cw, VictoryLevel::Tactical, "C3922").met);
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<VictoryReport>(&json).unwrap(), r);
    }

    #[test]
    fn each_axis_level_is_the_highest_met() {
        let data = graziani();
        let mut state = without(&data, Side::Cw);
        let r = evaluate(&data, &state);
        assert_eq!(r.axis, None);
        // Tactical: Sidi Barrani and Sollum besides the garrisons.
        put(&mut state, "unit:it:xvii-libyan-bn", "C4131");
        put(&mut state, "unit:it:xix-libyan-bn", "C4021");
        let r = evaluate(&data, &state);
        assert_eq!(r.axis, Some(VictoryLevel::Tactical));
        assert_eq!(r.cw, None);
        // Decisive: Mersa Matruh and Siwa, traced to Tobruk.
        put(&mut state, "unit:it:ii-libyan-bn", "D3714");
        put(&mut state, "unit:it:vi-libyan-bn", "C0127");
        let r = evaluate(&data, &state);
        assert_eq!(r.axis, Some(VictoryLevel::Decisive));
        let mm = condition(&r, Side::Axis, VictoryLevel::Decisive, "mersa-matruh");
        assert_eq!(mm.held, [hex("D3714")]);
        // An HQ alone holds nothing (60.81: a combat unit).
        put(&mut state, "unit:it:ii-libyan-bn", "C0101");
        put(&mut state, "unit:it:1st-libyan-infantry-hq", "D3714");
        let r = evaluate(&data, &state);
        assert_eq!(r.axis, Some(VictoryLevel::Tactical));
        // A Commonwealth unit in Tobruk cuts the source (R-110).
        put(&mut state, "unit:it:ii-libyan-bn", "D3714");
        put(&mut state, "unit:cw:1-krrc", "C4807");
        let r = evaluate(&data, &state);
        assert_eq!(r.axis, None);
        let mm = condition(&r, Side::Axis, VictoryLevel::Decisive, "mersa-matruh");
        assert_eq!(mm.unsupplied, [pid("unit:it:ii-libyan-bn")]);
        assert!(mm.held.is_empty());
        put(&mut state, "unit:cw:1-krrc", "E0101");
        // Strategic: any one hex of Alexandria or Cairo, traced to sheet D.
        for h in ["E3613", "E3714", "E1730"] {
            let mut s = state.clone();
            put(&mut s, "unit:it:vii-libyan-bn", h);
            let r = evaluate(&data, &s);
            assert_eq!(r.axis, Some(VictoryLevel::Strategic), "{h}");
            let c = condition(
                &r,
                Side::Axis,
                VictoryLevel::Strategic,
                "E3613 E3714 E1730 E1829 E1830 E1930 E1931",
            );
            assert!(c.any_hex);
            assert_eq!(c.held, [hex(h)]);
        }
    }

    #[test]
    fn each_commonwealth_level_is_the_highest_met() {
        let data = graziani();
        let mut state = without(&data, Side::Axis);
        let r = evaluate(&data, &state);
        assert_eq!(r.cw, None);
        // Tactical: Sollum, Halfaya (held already) and Siwa.
        put(&mut state, "unit:cw:1-buffs", "C4021");
        put(&mut state, "unit:cw:1-essex", "C0127");
        let r = evaluate(&data, &state);
        assert_eq!((r.axis, r.cw), (None, Some(VictoryLevel::Tactical)));
        // Decisive: Bardia, Fort Maddalena and Sidi Omar as well.
        put(&mut state, "unit:cw:1-hamp", "C4321");
        put(&mut state, "unit:cw:1-sostff", "C3019");
        put(&mut state, "unit:cw:1-dur-lt", "C3618");
        let r = evaluate(&data, &state);
        assert_eq!(r.cw, Some(VictoryLevel::Decisive));
        // Strategic: Tobruk, traced to Cairo or Alexandria.
        put(&mut state, "unit:cw:1-rtr", "C4807");
        let r = evaluate(&data, &state);
        assert_eq!(r.cw, Some(VictoryLevel::Strategic));
        let t = condition(&r, Side::Cw, VictoryLevel::Strategic, "tobruk");
        assert_eq!(t.held, [hex("C4807")]);
        assert!(t.unsupplied.is_empty());
    }

    #[test]
    fn a_holder_cut_off_by_enemy_zoc_does_not_count_until_a_friend_opens_a_way() {
        let data = graziani();
        let mut state = without(&data, Side::Cw);
        put(&mut state, "unit:it:xvii-libyan-bn", "C4131");
        put(&mut state, "unit:it:xix-libyan-bn", "C4021");
        assert_eq!(evaluate(&data, &state).axis, Some(VictoryLevel::Tactical));
        // Two Commonwealth stacks north and south of Fort Maddalena
        // (C3019): their ZOCs cover every other neighbour.
        for (id, h) in [
            ("unit:cw:1-krrc", "C3018"),
            ("unit:cw:3-cldgds", "C3018"),
            ("unit:cw:1-buffs", "C3020"),
            ("unit:cw:1-essex", "C3020"),
        ] {
            put(&mut state, id, h);
        }
        assert!(state.could_exert(&data, &hex("C3018")));
        assert!(state.could_exert(&data, &hex("C3020")));
        let r = evaluate(&data, &state);
        assert_eq!(r.axis, None);
        let fm = condition(&r, Side::Axis, VictoryLevel::Tactical, "fort-maddalena");
        assert!(!fm.met);
        assert_eq!(fm.unsupplied, [pid("unit:it:fort-maddalenna-garrison-bn")]);
        // A friendly unit in C2919 lifts the ZOC there (R-110, 32.17):
        // the route goes out through it.
        put(&mut state, "unit:it:sceferzen-oasis-garrison-bn", "C2919");
        let r = evaluate(&data, &state);
        assert_eq!(r.axis, Some(VictoryLevel::Tactical));
        let fm = condition(&r, Side::Axis, VictoryLevel::Tactical, "fort-maddalena");
        assert!(fm.met);
        assert!(fm.unsupplied.is_empty());
        // Enemy units on the hex itself, and it is not held at all.
        put(&mut state, "unit:cw:1-rtr", "C3019");
        let fm_r = evaluate(&data, &state);
        let fm = condition(&fm_r, Side::Axis, VictoryLevel::Tactical, "fort-maddalena");
        assert!(!fm.met && fm.held.is_empty() && fm.unsupplied.is_empty());
    }

    #[test]
    fn supply_units_real_or_dummy_are_not_units_in_the_trace_or_the_hold() {
        // R-119: only combat units block a route, deny a hold or lift an
        // enemy ZOC; a supply unit or a dummy does none of these.
        let data = graziani();
        let mut state = without(&data, Side::Cw);
        put(&mut state, "unit:it:xvii-libyan-bn", "C4131");
        put(&mut state, "unit:it:xix-libyan-bn", "C4021");
        assert_eq!(evaluate(&data, &state).axis, Some(VictoryLevel::Tactical));
        let place = |state: &mut GameState, id: &PieceId, h: &str| {
            state.pieces.get_mut(id).unwrap().location = Location::Hex(hex(h));
        };
        let cw_supply = PieceId::supply(Side::Cw, 1);
        let cw_dummy = PieceId::dummy_supply(Side::Cw, 1);
        let axis_supply = PieceId::supply(Side::Axis, 1);
        let axis_dummy = PieceId::dummy_supply(Side::Axis, 1);
        for id in [&cw_supply, &cw_dummy, &axis_supply, &axis_dummy] {
            assert!(state.pieces.contains_key(id), "{id}");
        }
        // A Commonwealth supply unit or dummy in Tobruk, the source, cuts
        // nothing; a combat unit there does.
        for id in [&cw_supply, &cw_dummy] {
            let mut s = state.clone();
            place(&mut s, id, "C4807");
            assert_eq!(
                evaluate(&data, &s).axis,
                Some(VictoryLevel::Tactical),
                "{id}"
            );
        }
        // One in Fort Maddalena beside the garrison denies no hold
        // (contrived: the capture of 10.29 would have taken it).
        for id in [&cw_supply, &cw_dummy] {
            let mut s = state.clone();
            place(&mut s, id, "C3019");
            let r = evaluate(&data, &s);
            assert_eq!(r.axis, Some(VictoryLevel::Tactical), "{id}");
            let fm = condition(&r, Side::Axis, VictoryLevel::Tactical, "fort-maddalena");
            assert_eq!(fm.held, [hex("C3019")]);
        }
        // Two Commonwealth stacks blanket Fort Maddalena with ZOC; an
        // Axis supply unit or dummy in C2919 lifts none of it, an Axis
        // combat unit does.
        for (id, h) in [
            ("unit:cw:1-krrc", "C3018"),
            ("unit:cw:3-cldgds", "C3018"),
            ("unit:cw:1-buffs", "C3020"),
            ("unit:cw:1-essex", "C3020"),
        ] {
            put(&mut state, id, h);
        }
        assert_eq!(evaluate(&data, &state).axis, None);
        for id in [&axis_supply, &axis_dummy] {
            let mut s = state.clone();
            place(&mut s, id, "C2919");
            assert_eq!(evaluate(&data, &s).axis, None, "{id}");
        }
        put(&mut state, "unit:it:sceferzen-oasis-garrison-bn", "C2919");
        assert_eq!(evaluate(&data, &state).axis, Some(VictoryLevel::Tactical));
    }

    #[test]
    fn a_hold_the_grammar_cannot_check_is_reported_unchecked() {
        let mut data = graziani();
        data.scenario = cna_data::scenario::Scenario::load("rommels-arrival").unwrap();
        let state = set_up(&graziani(), 5);
        let r = evaluate(&data, &state);
        assert_eq!(r.unchecked.len(), 1);
        assert!(r.unchecked[0].starts_with("axis strategic: "));
        assert!(r.unchecked[0].contains("Egypt"));
        // The Axis holds Tobruk and Bardia at set-up: decisive there; the
        // strategic rung needs Benghazi and the unchecked Egyptian towns.
        assert_eq!(r.axis, Some(VictoryLevel::Decisive));
        assert_eq!(r.cw, None);
    }
}
