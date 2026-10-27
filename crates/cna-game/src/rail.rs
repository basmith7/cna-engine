//! Phase J under abstract supply (SPI 8.7x, 32.3x, R-005): the
//! Commonwealth runs one stack each way along the Alexandria–Mersa
//! Matruh railway, each up to 2 SP of units with what is attached to
//! them, and up to two supply units a stage, all one way.

use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::GameState;
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::{HexId, HexsideFeature};
use cna_data::scenario::Side;

/// Supply units the Commonwealth may run by rail in one stage (32.3x).
const RAIL_SUPPLY: usize = 2;

/// Stacking points of units in one rail run (8.7x).
const RAIL_SP: u32 = 2;

/// Which way a rail run goes along the line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RailWay {
    East,
    West,
}

impl std::fmt::Display for RailWay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RailWay::East => "east",
            RailWay::West => "west",
        })
    }
}

/// A run that may start: where on the line, and everything it carries.
struct RailStack {
    at: usize,
    pieces: Vec<PieceId>,
    supply: bool,
}

/// The railway, west to east: the shortest run of finished railroad
/// hexsides from Mersa Matruh, where it ends at the start (60.7), to
/// Alexandria's west hex. The Delta's lines beyond are not part of it
/// (8.7x: only this railway matters); extension by construction comes
/// with engineering.
pub(crate) fn railway(data: &Data) -> Vec<HexId> {
    let (Some(west), Some(east)) = (
        data.map.place("mersa-matruh"),
        data.map.place("alexandria-west"),
    ) else {
        return Vec::new();
    };
    let mut came: BTreeMap<HexId, HexId> = BTreeMap::new();
    let mut queue = VecDeque::from([west.hex.clone()]);
    while let Some(h) = queue.pop_front() {
        if h == east.hex {
            let mut line = vec![h];
            while let Some(prev) = came.get(line.last().expect("non-empty")) {
                line.push(prev.clone());
            }
            line.reverse();
            return line;
        }
        for n in data.map.neighbours(&h) {
            let rail = data
                .map
                .hexside(&h, &n)
                .is_some_and(|hs| hs.has(HexsideFeature::Railroad));
            if rail && n != west.hex && !came.contains_key(&n) {
                came.insert(n.clone(), h.clone());
                queue.push_back(n);
            }
        }
    }
    Vec::new()
}

impl GameState {
    /// Where on the railway the Commonwealth may still run: the hexes
    /// east of the easternmost Axis combat unit on the line (8.7x).
    fn open_line(&self, line: &[HexId]) -> usize {
        line.iter()
            .rposition(|h| {
                self.pieces.values().any(|p| {
                    p.side == Side::Axis && p.kind == PieceKind::Combat && p.hex() == Some(h)
                })
            })
            .map_or(0, |i| i + 1)
    }

    /// Supply units that have ridden the railway this stage.
    fn railed_supply(&self) -> usize {
        self.railed
            .iter()
            .filter(|id| self.pieces[*id].kind.is_supply())
            .count()
    }

    /// A Commonwealth combat unit, HQ or supply unit that may still ride
    /// the railway this stage: on the open line, no CP spent (8.7x), not
    /// already railed.
    fn may_rail(&self, data: &Data, line: &[HexId], id: &PieceId) -> Result<usize, Illegal> {
        let p = self
            .pieces
            .get(id)
            .ok_or_else(|| Illegal::new("32.3x", format!("no piece {id}")))?;
        let rides = p.kind.is_supply() || matches!(p.kind, PieceKind::Combat | PieceKind::Hq);
        if p.side != Side::Cw || !rides {
            return Err(Illegal::new(
                "8.7x",
                format!("{id}: only Commonwealth units and supply ride the railway"),
            ));
        }
        let at = p
            .hex()
            .and_then(|h| line.iter().position(|l| l == h))
            .filter(|i| *i >= self.open_line(line))
            .ok_or_else(|| Illegal::new("8.7x", format!("{id} is not on the open railway")))?;
        if p.cp_spent != Cp::ZERO {
            return Err(Illegal::new(
                "8.7x",
                format!("{id} has spent CP this stage: it may not ride the railway"),
            ));
        }
        if p.flown {
            return Err(Illegal::new(
                "32.37",
                format!("{id} was flown this stage: it stays put"),
            ));
        }
        if self.railed.contains(id) {
            return Err(Illegal::new(
                "32.3x",
                format!("{id} has ridden the railway this stage"),
            ));
        }
        if self.enemy_zoc(data, &line[at]) {
            return Err(Illegal::new("8.7x", format!("{id} starts in an enemy ZOC")));
        }
        Ok(at)
    }

    /// An Axis ZOC over `hex` for the railway (8.7x). Outside G there is
    /// no declaration to ask for, so any Axis hex that could exert one
    /// counts.
    fn enemy_zoc(&self, data: &Data, hex: &HexId) -> bool {
        !matches!(self.zoc_at(data, hex, Side::Cw), Zoc::Clear)
    }

    /// Phase J waits on the Commonwealth only when the module is on and
    /// it has a supply unit that may still ride the railway somewhere;
    /// otherwise it is skipped.
    pub(crate) fn rail_to_move(&self, data: &Data) -> bool {
        if !self.modules.abstract_supply
            || self.phasing() != Some(Side::Cw)
            || self.railed_supply() >= RAIL_SUPPLY
        {
            return false;
        }
        // Units ride once the phase is open; only supply opens it (for
        // now: opening it for units adds stages to the goldens).
        self.pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind.is_supply())
            .any(|p| !self.rail_destinations(data, std::slice::from_ref(&p.id)).is_empty())
    }

    /// Why a stack may not run `way` this stage: that way's stack has
    /// gone (8.7x), or supply has gone the other way (32.3x).
    fn way_refusal(&self, way: RailWay, supply: bool) -> Option<Illegal> {
        if self.rail_runs.contains(&way) {
            return Some(Illegal::new(
                "8.7x",
                format!("one stack a stage runs {way}, and it has gone"),
            ));
        }
        match self.rail_supply_way {
            Some(w) if supply && w != way => Some(Illegal::new(
                "32.3x",
                format!("supply has gone {w} by rail this stage: all of it goes one way"),
            )),
            _ => None,
        }
    }

    /// The railway hexes `pieces` may run to from where they stand, in
    /// order out from them: every hex east, then every hex west, short
    /// of the first enemy hex, enemy ZOC or the line's closed part. Empty
    /// when they may not ride at all.
    pub fn rail_destinations(&self, data: &Data, pieces: &[PieceId]) -> Vec<HexId> {
        let line = railway(data);
        let Ok(RailStack { at, supply, .. }) = self.rail_start(data, &line, pieces) else {
            return Vec::new();
        };
        let open = self.open_line(&line);
        let clear = |h: &HexId| self.holder(h) != Some(Side::Axis) && !self.enemy_zoc(data, h);
        let mut out = Vec::new();
        if self.way_refusal(RailWay::East, supply).is_none() {
            out.extend(line[at + 1..].iter().take_while(|h| clear(h)).cloned());
        }
        if self.way_refusal(RailWay::West, supply).is_none() {
            out.extend(
                line[open..at]
                    .iter()
                    .rev()
                    .take_while(|h| clear(h))
                    .cloned(),
            );
        }
        out
    }

    /// `reachable` for `Purpose::Rail`: each hex the run may end in,
    /// with its path along the line; rail costs no CP and no fuel.
    pub(crate) fn rail_reaches(
        &self,
        data: &Data,
        pieces: &[PieceId],
    ) -> Result<Vec<crate::reach::Reach>, Illegal> {
        let line = railway(data);
        let at = self.rail_start(data, &line, pieces)?.at;
        Ok(self
            .rail_destinations(data, pieces)
            .into_iter()
            .map(|hex| {
                let to = line.iter().position(|h| *h == hex).expect("on the line");
                let path = if to > at {
                    line[at + 1..=to].to_vec()
                } else {
                    line[to..at].iter().rev().cloned().collect()
                };
                crate::reach::Reach {
                    hex,
                    cp: Cp::ZERO,
                    path,
                    dp: 0,
                    ends_in_zoc: false,
                    may_be_interrupted: false,
                    short: 0,
                    fuel: 0,
                }
            })
            .collect())
    }

    /// The run of `pieces` and what is attached to them (19.4x): all in
    /// one hex, each free to ride, the units within 2 SP and the supply
    /// within the stage's two.
    fn rail_start(
        &self,
        data: &Data,
        line: &[HexId],
        pieces: &[PieceId],
    ) -> Result<RailStack, Illegal> {
        if !self.modules.abstract_supply
            || self.clock.phase != Phase::Rail
            || self.phasing() != Some(Side::Cw)
        {
            return Err(Illegal::new(
                "8.7x",
                format!(
                    "the railway runs in the Commonwealth's phase J, not at {}",
                    self.clock
                ),
            ));
        }
        if pieces.is_empty() {
            return Err(Illegal::new("8.7x", "a rail run carries something"));
        }
        if let Some(id) = pieces
            .iter()
            .enumerate()
            .find_map(|(i, id)| pieces[..i].contains(id).then_some(id))
        {
            return Err(Illegal::new("8.7x", format!("{id} is listed twice")));
        }
        let pieces = &self.carried(pieces)?;
        let supply = pieces
            .iter()
            .filter(|id| self.pieces.get(*id).is_some_and(|p| p.kind.is_supply()))
            .count();
        if supply > 0 && self.railed_supply() + supply > RAIL_SUPPLY {
            return Err(Illegal::new(
                "32.3x",
                format!(
                    "two supply units a stage ride the railway; {} already have",
                    self.railed_supply()
                ),
            ));
        }
        let sp: u32 = pieces
            .iter()
            .filter_map(|id| self.pieces.get(id))
            .filter(|p| !p.kind.is_supply())
            .map(|p| cna_rules::stacking::points(&data.stacking, p.equivalent))
            .sum();
        if sp > RAIL_SP {
            return Err(Illegal::new(
                "8.7x",
                format!("a rail run carries at most {RAIL_SP} SP of units, not {sp}"),
            ));
        }
        let mut start = None;
        for id in pieces {
            let at = self.may_rail(data, line, id)?;
            if start.is_some_and(|s| s != at) {
                return Err(Illegal::new("8.7x", "a rail run starts as one stack"));
            }
            start = Some(at);
        }
        Ok(RailStack {
            at: start.expect("non-empty"),
            pieces: pieces.clone(),
            supply: supply > 0,
        })
    }

    /// `Order::Rail`: the stack, with what is attached to it, runs along
    /// the railway to the end of `path`, one hex at a time, never into an
    /// Axis hex or ZOC; one stack a stage each way (8.7x), supply all one
    /// way (32.3x). Rail pays no fuel (32.2x).
    pub(crate) fn rail(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        path: &[HexId],
    ) -> Result<Vec<Event>, Illegal> {
        let Some(end) = path.last() else {
            return Err(Illegal::new("8.7x", "a rail run goes somewhere"));
        };
        let line = railway(data);
        let stack = self.rail_start(data, &line, pieces)?;
        let way = match line.iter().position(|h| h == end) {
            Some(to) if to > stack.at => RailWay::East,
            _ => RailWay::West,
        };
        if let Some(e) = self.way_refusal(way, stack.supply) {
            return Err(e);
        }
        let Some(reach) = self
            .rail_reaches(data, pieces)?
            .into_iter()
            .find(|r| r.hex == *end)
        else {
            return Err(Illegal::new(
                "8.7x",
                format!("{end} is not on the railway open to this run"),
            ));
        };
        if path != reach.path {
            return Err(Illegal::new(
                "8.7x",
                "a rail run follows the railway hex by hex",
            ));
        }
        let from = line[stack.at].clone();
        let supply = stack
            .pieces
            .iter()
            .filter(|id| self.pieces[*id].kind.is_supply())
            .count();
        if supply > 0
            && let Some(e) = self.supply_stack_refusal(data, PieceKind::SupplyUnit, end, supply)
        {
            return Err(e);
        }
        for id in &stack.pieces {
            self.pieces.get_mut(id).expect("checked").location = Location::Hex(end.clone());
            self.railed.insert(id.clone());
        }
        self.rail_runs.insert(way);
        if stack.supply {
            self.rail_supply_way = Some(way);
        }
        Ok(vec![Event::Railed {
            pieces: stack.pieces,
            from,
            to: end.clone(),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Half;
    use crate::order::Order;
    use crate::state::Modules;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// A module-on game, the Axis Player A, played on with the default
    /// answers to the Commonwealth's phase J, two Commonwealth supply
    /// units put at `hex` first.
    fn at_rail(data: &Data, hex: &str) -> (GameState, Vec<PieceId>) {
        use crate::state::Pending;
        use crate::state::test_support::through_combat;
        let mut state = GameState::with_modules(data, 5, Modules::ALL).unwrap();
        crate::state::test_support::auto_place(data, &mut state);
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        let hex = HexId::parse(hex).unwrap();
        let ids: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::SupplyUnit && p.hex().is_some())
            .take(2)
            .map(|p| p.id.clone())
            .collect();
        assert_eq!(ids.len(), 2);
        for id in &ids {
            state.put(id, Location::Hex(hex.clone()));
        }
        while !(state.clock.phase == Phase::Rail && state.clock.half == Some(Half::B)) {
            let player = state.to_act().unwrap();
            if state.clock.phase == Phase::MovementCombat
                && state.clock.segment == Some(crate::clock::Segment::Movement)
                && state.pending.is_none()
                && state.sides_adjacent(data)
            {
                through_combat(data, &mut state);
                continue;
            }
            let order = match &state.pending {
                Some(Pending::DeclareZoc { .. }) => Order::DeclareZoc { exerting: vec![] },
                _ => Order::Pass,
            };
            state.apply(data, player, order).unwrap();
        }
        (state, ids)
    }

    fn run(pieces: &[PieceId], path: &[HexId]) -> Order {
        Order::Rail {
            pieces: pieces.to_vec(),
            path: path.to_vec(),
        }
    }

    #[test]
    fn the_railway_runs_from_mersa_matruh_to_alexandria() {
        let data = graziani();
        let line = railway(&data);
        assert_eq!(line.first().unwrap().to_string(), "D3714");
        assert_eq!(line.last().unwrap().to_string(), "E3613");
        for w in line.windows(2) {
            assert!(data.map.adjacent(&w[0], &w[1]));
        }
    }

    #[test]
    fn phase_j_waits_on_the_commonwealth_with_supply_on_the_line_32_3x() {
        let data = graziani();
        let (state, _) = at_rail(&data, "E3613");
        assert_eq!(state.clock.phase, Phase::Rail);
        assert_eq!(state.to_act(), Some(Side::Cw));
    }

    #[test]
    fn two_supply_units_run_west_by_rail_and_pay_nothing_8_7x() {
        let data = graziani();
        let line = railway(&data);
        let (mut state, ids) = at_rail(&data, "E3613");
        // Mersa Matruh holds the Commonwealth's other supply at the start.
        let path: Vec<HexId> = line[1..line.len() - 1].iter().rev().cloned().collect();
        let loads: Vec<_> = ids.iter().map(|id| state.pieces[id].load).collect();
        let twice = [ids[0].clone(), ids[0].clone()];
        let err = state
            .apply(&data, Side::Cw, run(&twice, &path))
            .unwrap_err();
        assert_eq!(err.case, "8.7x");
        state.apply(&data, Side::Cw, run(&ids, &path)).unwrap();
        for (id, load) in ids.iter().zip(loads) {
            assert_eq!(state.pieces[id].hex(), Some(&line[1]));
            assert_eq!(state.pieces[id].load, load);
        }
        // Two a stage: nothing more may ride.
        assert!(state.rail_destinations(&data, &ids[..1]).is_empty());
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_ne!(state.clock.phase, Phase::Rail);
    }

    #[test]
    fn a_rail_run_follows_the_line_one_way_32_3x() {
        let data = graziani();
        let line = railway(&data);
        let mid = &line[line.len() / 2];
        let (mut state, ids) = at_rail(&data, &mid.to_string());
        let i = line.len() / 2;
        // Off the line, or skipping a hex: refused.
        let err = state
            .apply(&data, Side::Cw, run(&ids[..1], &[line[i + 2].clone()]))
            .unwrap_err();
        assert_eq!(err.case, "8.7x");
        state
            .apply(&data, Side::Cw, run(&ids[..1], &[line[i + 1].clone()]))
            .unwrap();
        // The second unit may not go the other way (32.3x), nor east,
        // where the stage's one stack has gone (8.7x).
        assert_eq!(state.clock.phase, Phase::Rail);
        assert!(state.rail_destinations(&data, &ids[1..]).is_empty());
        let err = state
            .apply(&data, Side::Cw, run(&ids[1..], &[line[i - 1].clone()]))
            .unwrap_err();
        assert_eq!(err.case, "32.3x");
        let err = state
            .apply(&data, Side::Cw, run(&ids[1..], &[line[i + 2].clone()]))
            .unwrap_err();
        assert_eq!(err.case, "8.7x");
        // Nor ride twice.
        let err = state
            .apply(&data, Side::Cw, run(&ids[..1], &[line[i + 2].clone()]))
            .unwrap_err();
        assert_eq!(err.case, "32.3x");
    }

    #[test]
    fn the_line_west_of_an_axis_combat_unit_is_closed_8_7x() {
        let data = graziani();
        let line = railway(&data);
        let (mut state, ids) = at_rail(&data, &line[20].to_string());
        // An Axis combat unit big enough to exert a ZOC (10.1x) sits on
        // the line.
        let axis: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::Combat && p.hex().is_some())
            .map(|p| p.id.clone())
            .collect();
        let mut placed = false;
        for id in axis {
            let mut s = state.clone();
            s.put(&id, Location::Hex(line[5].clone()));
            if s.enemy_zoc(&data, &line[6]) {
                state = s;
                placed = true;
                break;
            }
        }
        assert!(placed);
        let reach = state.rail_destinations(&data, &ids);
        assert!(reach.contains(&line[21]));
        // West: stops short of the Axis unit's ZOC, never past it.
        for h in &line[..=6] {
            assert!(!reach.contains(h), "{h}");
        }
        assert!(reach.contains(&line[8]));
    }

    #[test]
    fn a_unit_that_spent_cp_or_the_axis_may_not_ride_8_7x() {
        let data = graziani();
        let line = railway(&data);
        let (mut state, ids) = at_rail(&data, &line[10].to_string());
        state.pieces.get_mut(&ids[0]).unwrap().cp_spent = Cp::whole(1);
        let err = state
            .apply(&data, Side::Cw, run(&ids[..1], &[line[11].clone()]))
            .unwrap_err();
        assert_eq!(err.case, "8.7x");
        assert!(state.rail_destinations(&data, &ids[..1]).is_empty());
        let err = state
            .apply(&data, Side::Axis, run(&ids[1..], &[line[11].clone()]))
            .unwrap_err();
        assert_eq!(err.case, "5.2");
    }

    #[test]
    fn reachable_by_rail_is_exactly_what_apply_accepts() {
        use crate::reach::Purpose;
        let data = graziani();
        let line = railway(&data);
        let (mut state, ids) = at_rail(&data, &line[20].to_string());
        // An Axis unit on the line closes part of it.
        let axis = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat && p.hex().is_some())
            .unwrap()
            .id
            .clone();
        state.put(&axis, Location::Hex(line[8].clone()));
        let reaches = state
            .reachable(&data, Side::Cw, &ids, Purpose::Rail)
            .unwrap();
        assert!(!reaches.is_empty());
        for (i, h) in line.iter().enumerate() {
            let path: Vec<HexId> = if i > 20 {
                line[21..=i].to_vec()
            } else {
                line[i..20].iter().rev().cloned().collect()
            };
            let mut probe = state.clone();
            let got = probe.apply(&data, Side::Cw, run(&ids, &path));
            match reaches.iter().find(|r| r.hex == *h) {
                Some(r) => {
                    assert_eq!(r.path, path);
                    assert!(got.is_ok(), "{h}: {got:?}");
                    assert_eq!(probe.pieces[&ids[0]].hex(), Some(h));
                }
                None => assert!(got.is_err(), "{h} was refused by reachable"),
            }
        }
    }

    #[test]
    fn module_off_phase_j_is_skipped() {
        let data = graziani();
        let mut state = GameState::new(&data, 5).unwrap();
        assert!(!state.rail_to_move(&data));
        state.clock.phase = Phase::Rail;
        assert!(state.rail_destinations(&data, &[]).is_empty());
    }

    fn pid(s: &str) -> PieceId {
        PieceId::from(cna_data::oa::UnitId::parse(s).unwrap())
    }

    /// `at_rail` at Alexandria's west hex, with `units` put there too,
    /// alone and with no CP spent this stage.
    fn units_at_alexandria(data: &Data, units: &[&str]) -> (GameState, Vec<PieceId>) {
        let (mut state, _) = at_rail(data, "E3613");
        let at = HexId::parse("E3613").unwrap();
        let ids: Vec<PieceId> = units.iter().map(|u| pid(u)).collect();
        for id in &ids {
            state.put(id, Location::Hex(at.clone()));
            state.pieces.get_mut(id).unwrap().cp_spent = Cp::ZERO;
        }
        (state, ids)
    }

    #[test]
    fn a_combat_unit_rides_the_railway_free_8_7x() {
        let data = graziani();
        let line = railway(&data);
        let (mut state, ids) = units_at_alexandria(&data, &["unit:cw:1-krrc"]);
        let to = line[line.len() - 3].clone();
        let path = vec![line[line.len() - 2].clone(), to.clone()];
        let events = state.apply(&data, Side::Cw, run(&ids, &path)).unwrap();
        assert!(matches!(&events[..], [Event::Railed { .. }]), "{events:?}");
        let p = &state.pieces[&ids[0]];
        assert_eq!(p.hex(), Some(&to));
        assert_eq!(p.cp_spent, Cp::ZERO);
        // It has ridden: not again this stage.
        assert!(state.rail_destinations(&data, &ids).is_empty());
    }

    #[test]
    fn a_unit_that_spent_cp_or_a_stack_over_two_sp_does_not_ride_8_7x() {
        let data = graziani();
        let line = railway(&data);
        let path = vec![line[line.len() - 2].clone()];
        let (mut state, ids) = units_at_alexandria(&data, &["unit:cw:1-krrc"]);
        state.pieces.get_mut(&ids[0]).unwrap().cp_spent = Cp::whole(1);
        let err = state.apply(&data, Side::Cw, run(&ids, &path)).unwrap_err();
        assert_eq!(err.case, "8.7x", "{}", err.reason);
        let (mut state, ids) = units_at_alexandria(
            &data,
            &["unit:cw:1-krrc", "unit:cw:3-cldgds", "unit:cw:1-rnf"],
        );
        let before = state.clone();
        let err = state.apply(&data, Side::Cw, run(&ids, &path)).unwrap_err();
        assert_eq!(err.case, "8.7x", "{}", err.reason);
        assert!(err.reason.contains("SP"), "{}", err.reason);
        assert_eq!(state, before);
        // Two battalions are 2 SP: they go.
        state.apply(&data, Side::Cw, run(&ids[..2], &path)).unwrap();
    }

    #[test]
    fn attached_units_ride_with_their_parent_19_4x() {
        let data = graziani();
        let line = railway(&data);
        let path = vec![line[line.len() - 2].clone()];
        let (mut state, ids) =
            units_at_alexandria(&data, &["unit:cw:1-krrc", "unit:cw:3-cldgds"]);
        state.pieces.get_mut(&ids[1]).unwrap().attached_to = ids[0].unit();
        let err = state
            .apply(&data, Side::Cw, run(&ids[1..], &path))
            .unwrap_err();
        assert_eq!(err.case, "19.4x", "{}", err.reason);
        let events = state.apply(&data, Side::Cw, run(&ids[..1], &path)).unwrap();
        let Event::Railed { pieces, .. } = &events[0] else {
            panic!("{events:?}");
        };
        assert_eq!(pieces, &ids);
        for id in &ids {
            assert_eq!(state.pieces[id].hex(), Some(&path[0]));
        }
    }

    #[test]
    fn one_stack_runs_each_way_a_stage_r005() {
        let data = graziani();
        let line = railway(&data);
        let (mut state, ids) = units_at_alexandria(
            &data,
            &["unit:cw:1-krrc", "unit:cw:3-cldgds", "unit:cw:1-rnf"],
        );
        let west = vec![line[line.len() - 2].clone()];
        state.apply(&data, Side::Cw, run(&ids[..1], &west)).unwrap();
        // The westward stack has gone: no second one.
        assert!(state.rail_destinations(&data, &ids[1..2]).is_empty());
        let err = state
            .apply(&data, Side::Cw, run(&ids[1..2], &west))
            .unwrap_err();
        assert_eq!(err.case, "8.7x", "{}", err.reason);
        // Eastward is still open, from further up the line.
        let mid = line.len() / 2;
        state.put(&ids[2], Location::Hex(line[mid].clone()));
        let east = state.rail_destinations(&data, &ids[2..]);
        assert!(!east.is_empty());
        let i = |h: &HexId| line.iter().position(|l| l == h).unwrap();
        assert!(east.iter().all(|h| i(h) > mid), "{east:?}");
        state
            .apply(&data, Side::Cw, run(&ids[2..], &[line[mid + 1].clone()]))
            .unwrap();
        // Both ways run: phase J has nothing left to offer.
        assert!(!state.rail_to_move(&data));
    }

    #[test]
    fn supply_rides_in_a_stack_one_way_a_stage_32_3x() {
        let data = graziani();
        let line = railway(&data);
        let mid = line.len() / 2;
        let (mut state, supply) = at_rail(&data, &line[mid].to_string());
        let (_, units) = units_at_alexandria(&data, &["unit:cw:1-krrc"]);
        for id in &units {
            state.put(id, Location::Hex(line[mid].clone()));
            state.pieces.get_mut(id).unwrap().cp_spent = Cp::ZERO;
        }
        // A battalion and a supply unit go east together (R-005).
        let both = [units[0].clone(), supply[0].clone()];
        state
            .apply(&data, Side::Cw, run(&both, &[line[mid + 1].clone()]))
            .unwrap();
        // The other supply unit may not go west: supply goes one way.
        assert!(state.rail_destinations(&data, &supply[1..]).is_empty());
        let err = state
            .apply(&data, Side::Cw, run(&supply[1..], &[line[mid - 1].clone()]))
            .unwrap_err();
        assert_eq!(err.case, "32.3x", "{}", err.reason);
    }
}

