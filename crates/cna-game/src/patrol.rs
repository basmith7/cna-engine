//! Phase L, patrols (SPI 16.x; cna `rules/60-combat.md`, *Patrols and
//! reconnaissance*): the phasing player sends patrol points from eligible
//! units against an enemy hex. Nothing moves; three dice decide what the
//! patrol loses, what it learns and what it costs the hex patrolled.
//! The defender's choices (which units are described, which points go)
//! are made by the engine (`NOT_SIMULATED.md`).

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::characteristics::Equivalent;
use cna_data::map::HexId;
use cna_data::patrol::Revealed;
use cna_data::scenario::Side;
use cna_rules::movement::Weather as StepWeather;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Points a hex may send in one patrol (16.13).
const PER_HEX: u32 = 2;
/// Points one patrol may send against its target (16.13).
const PER_TARGET: u32 = 3;
/// How far the target may be, in hexes (16.22).
const RANGE: u32 = 5;
/// A unit at this cohesion or worse supplies no points (16.14).
const COHESION_BAR: i32 = -8;
/// The light tanks whose points patrol (16.11, E-010), by weapon name.
const LIGHT_TANKS: &[&str] = &[
    "CV 33(L3/35)",
    "L6/40",
    "Mark VI Light",
    "Stuart M3",
    "PZ I",
];

/// What this OpStage has recorded for phase L (16.21, 16.22); cleared as
/// a stage begins.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageRecord {
    /// Sides that fired anti-armour or close-assaulted this stage.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub fought: BTreeSet<Side>,
    /// Hexes barraged this stage: none may be patrolled.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub barraged: BTreeSet<HexId>,
    /// Patrol points sent this phase L, by the hex they left and by the
    /// hex they went against: 16.13's limits hold across every patrol of
    /// the phase, or two patrols would send what one may not.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub patrolled_from: BTreeMap<HexId, u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub patrolled_at: BTreeMap<HexId, u32>,
}

impl StageRecord {
    pub(crate) fn is_empty(&self) -> bool {
        self.fought.is_empty()
            && self.barraged.is_empty()
            && self.patrolled_from.is_empty()
            && self.patrolled_at.is_empty()
    }
}

/// A unit the defender describes to a patrol (16.5x): its counter, and
/// its TOE strength stated exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Described {
    pub piece: PieceId,
    pub toe: u32,
}

/// One patrol's three rolls and what came of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatrolReport {
    pub side: Side,
    pub target: HexId,
    pub sources: Vec<(PieceId, u32)>,
    /// The Patrol Survival die as rolled; none when the target holds no
    /// combat units (16.31).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survival_die: Option<u8>,
    /// Patrol points killed and captured.
    pub killed: u32,
    pub captured: u32,
    /// The Reconnaissance die; none for a wiped-out patrol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recon_die: Option<u8>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub described: Vec<Described>,
    /// The Objective Loss die and the target's TOE points lost.
    pub objective_die: u8,
    pub objective_killed: u32,
    pub objective_captured: u32,
}

/// Which kind of unit a piece is, for patrols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    Combat,
    AntiAir,
    Engineer,
    Hq,
}

fn bad(case: &'static str, reason: impl Into<String>) -> Illegal {
    Illegal::new(case, reason)
}

impl GameState {
    /// As a combat segment closes: whether the phasing player fired
    /// anti-armour or close-assaulted in it, and which hexes were
    /// barraged (16.21, 16.22).
    /// Only with the module on, so a game without it saves as before.
    pub(crate) fn record_for_patrols(&mut self) {
        if !self.modules.patrols {
            return;
        }
        if let Some(side) = self.phasing() {
            let fought = self.combat.assignments.get(&side).is_some_and(|all| {
                all.iter()
                    .flat_map(|a| a.anti_armour.iter().chain(&a.close_assault))
                    .any(|a| a.target.is_some() && a.points > 0)
            });
            if fought {
                self.stage_record.fought.insert(side);
            }
        }
        let barraged = self.combat.barraged.keys().cloned();
        self.stage_record.barraged.extend(barraged);
    }

    fn unit_type<'a>(&self, data: &'a Data, p: &Piece) -> Option<&'a str> {
        p.nation
            .zip(p.id_code.as_deref())
            .and_then(|(n, code)| data.chars.row(n, code))
            .map(|r| r.unit_type.as_str())
    }

    fn is_recce(&self, data: &Data, p: &Piece) -> bool {
        p.id.unit()
            .and_then(|u| data.oa.unit(&u))
            .is_some_and(|u| u.vehicle.is_some())
    }

    /// TOE points `p` could send on patrol (16.11): all of a recce or
    /// motorised (or mechanised) infantry unit's, else its light tanks'.
    pub(crate) fn patrol_points(&self, data: &Data, p: &Piece) -> u32 {
        if p.kind != PieceKind::Combat || p.hex().is_none() || p.cohesion.level <= COHESION_BAR {
            return 0;
        }
        let infantry = self
            .unit_type(data, p)
            .is_some_and(|t| t.starts_with("Infantry"));
        if self.is_recce(data, p) || (infantry && self.mobility(data, p).motorised()) {
            return p.toe;
        }
        p.weapons
            .iter()
            .filter(|w| LIGHT_TANKS.contains(&w.weapon.as_str()))
            .map(|w| w.points)
            .sum()
    }

    /// Patrol losses come off the points sent (16.31): a unit that sends
    /// only its light tanks loses light tanks.
    fn take_patrol_loss(
        &mut self,
        data: &Data,
        id: &PieceId,
        points: u32,
        events: &mut Vec<Event>,
    ) -> u32 {
        let p = &self.pieces[id];
        if self.patrol_points(data, p) == p.toe {
            return self.take_loss(id, points, events);
        }
        let mut taken = 0;
        while taken < points {
            let light = self.pieces[id]
                .weapons
                .iter()
                .find(|w| w.points > 0 && LIGHT_TANKS.contains(&w.weapon.as_str()))
                .map(|w| w.weapon.clone());
            if light.is_none() || !self.take_loss_entry(id, light.as_deref()) {
                break;
            }
            taken += 1;
        }
        self.note_losses(id, taken, events);
        taken
    }

    fn role(&self, data: &Data, p: &Piece) -> Role {
        if p.kind == PieceKind::Hq {
            return Role::Hq;
        }
        match self.unit_type(data, p) {
            Some(t) if t.contains("Anti-air") => Role::AntiAir,
            Some(t) if t.starts_with("Engineer") || t.starts_with("Construction") => Role::Engineer,
            Some(t) if t.starts_with("Headquarters") => Role::Hq,
            _ => Role::Combat,
        }
    }

    /// The enemy's units in `hex` a patrol meets: combat units and HQs.
    /// Supply units and Rommel are neither described nor lose TOE here
    /// (trucks among the losses are not simulated).
    fn patrolled(&self, hex: &HexId, side: Side) -> Vec<PieceId> {
        self.pieces_at(hex)
            .into_iter()
            .filter(|p| {
                p.side == side.other() && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
            })
            .map(|p| p.id.clone())
            .collect()
    }

    /// Whether `side` may patrol in this phase L at all (16.21): it fired
    /// no anti-armour and made no close assault this stage.
    fn may_patrol(&self, side: Side) -> bool {
        self.modules.patrols && !self.stage_record.fought.contains(&side)
    }

    /// Whether phase L opens: patrols are allowed, and some eligible unit
    /// stands within range of an enemy-held hex not barraged this stage.
    pub(crate) fn patrols_to_send(&self, data: &Data) -> bool {
        let Some(side) = self.phasing() else {
            return false;
        };
        if !self.may_patrol(side) {
            return false;
        }
        let targets: BTreeSet<HexId> = self
            .pieces
            .values()
            .filter(|p| p.side == side.other() && p.kind.holds_ground())
            .filter_map(|p| p.hex().cloned())
            .filter(|h| !self.stage_record.barraged.contains(h))
            .filter(|h| self.weather_at(h) == StepWeather::Normal)
            .collect();
        if targets.is_empty() {
            return false;
        }
        let sources: BTreeSet<HexId> = self
            .pieces
            .values()
            .filter(|p| p.side == side && self.patrol_points(data, p) > 0)
            .filter_map(|p| p.hex().cloned())
            .filter(|h| self.weather_at(h) == StepWeather::Normal)
            .collect();
        sources.iter().any(|s| {
            data.map
                .within(s, RANGE)
                .iter()
                .any(|h| targets.contains(h) && self.patrol_path(data, side, s, h))
        })
    }

    /// A path of at most five hexes from `from` to `target` that avoids
    /// enemy-held hexes and enemy-controlled ones, bar the target's own
    /// ZOC (16.22).
    // Every hex next to an enemy-held one counts as enemy-controlled: the
    // patroller sees who holds a hex, not whether it could exert a ZOC
    // (its strength is hidden, 3.61), so a refusal must not tell it.
    fn patrol_path(&self, data: &Data, side: Side, from: &HexId, target: &HexId) -> bool {
        let around_target: BTreeSet<HexId> = data.map.neighbours(target).into_iter().collect();
        let open = |h: &HexId| -> bool {
            if h == target {
                return true;
            }
            if self.holder(h) == Some(side.other()) {
                return false;
            }
            around_target.contains(h)
                || !data
                    .map
                    .neighbours(h)
                    .iter()
                    .any(|n| self.holder(n) == Some(side.other()))
        };
        let mut seen: BTreeSet<HexId> = BTreeSet::from([from.clone()]);
        let mut frontier = vec![from.clone()];
        for _ in 0..RANGE {
            let mut next = Vec::new();
            for h in &frontier {
                for n in data.map.neighbours(h) {
                    if n == *target {
                        return true;
                    }
                    if seen.contains(&n) || !open(&n) {
                        continue;
                    }
                    seen.insert(n.clone());
                    next.push(n);
                }
            }
            frontier = next;
        }
        false
    }

    /// The checks of 16.1x and 16.2x, in the rules' order.
    fn check_patrol(
        &self,
        data: &Data,
        player: Side,
        sources: &[(PieceId, u32)],
        target: &HexId,
    ) -> Result<(), Illegal> {
        if !self.modules.patrols || self.clock.phase != Phase::Patrol {
            return Err(bad(
                "16.21",
                format!("patrols go out in phase L, not at {}", self.clock),
            ));
        }
        if self.phasing() != Some(player) {
            return Err(bad("16.21", "only the phasing player patrols"));
        }
        if !self.may_patrol(player) {
            return Err(bad(
                "16.21",
                "no patrol in a stage you fired anti-armour or close-assaulted",
            ));
        }
        if sources.is_empty() || sources.iter().any(|(_, n)| *n == 0) {
            return Err(bad("16.1", "a patrol sends at least one point a unit"));
        }
        let mut per_hex: BTreeMap<HexId, u32> = BTreeMap::new();
        let mut per_piece: BTreeMap<&PieceId, u32> = BTreeMap::new();
        for (id, n) in sources {
            let p = self
                .pieces
                .get(id)
                .filter(|p| p.side == player && p.kind == PieceKind::Combat)
                .ok_or_else(|| bad("16.11", format!("{id} is not one of your combat units")))?;
            let hex = p
                .hex()
                .ok_or_else(|| bad("16.11", format!("{id} is not on the map")))?;
            if p.cohesion.level <= COHESION_BAR {
                return Err(bad(
                    "16.14",
                    format!("{id} is at cohesion {} and cannot patrol", p.cohesion.level),
                ));
            }
            let can = self.patrol_points(data, p);
            if can == 0 {
                return Err(bad(
                    "16.11",
                    format!("{id} has no recce, light tank or motorised infantry points"),
                ));
            }
            let asked = per_piece.entry(id).or_insert(0);
            *asked += n;
            if *asked > can {
                return Err(bad(
                    "16.11",
                    format!("{id} has {can} patrol points, not {asked}"),
                ));
            }
            let from = per_hex.entry(hex.clone()).or_insert(0);
            *from += n;
            let sent = self
                .stage_record
                .patrolled_from
                .get(hex)
                .copied()
                .unwrap_or(0);
            if sent + *from > PER_HEX {
                return Err(bad(
                    "16.13",
                    format!("at most {PER_HEX} patrol points leave {hex}"),
                ));
            }
            if self.weather_at(hex) != StepWeather::Normal {
                return Err(bad(
                    "16.17",
                    format!("no patrol starts in a storm at {hex}"),
                ));
            }
        }
        let total: u32 = sources.iter().map(|(_, n)| n).sum::<u32>()
            + self
                .stage_record
                .patrolled_at
                .get(target)
                .copied()
                .unwrap_or(0);
        if total > PER_TARGET {
            return Err(bad(
                "16.13",
                format!("at most {PER_TARGET} patrol points go against one hex, not {total}"),
            ));
        }
        if self.holder(target) != Some(player.other()) {
            return Err(bad("16.22", format!("{target} holds no enemy unit")));
        }
        if self.weather_at(target) != StepWeather::Normal {
            return Err(bad(
                "16.17",
                format!("no patrol enters a storm at {target}"),
            ));
        }
        if self.stage_record.barraged.contains(target) {
            return Err(bad(
                "16.22",
                format!("{target} was barraged this stage and cannot be patrolled"),
            ));
        }
        for hex in per_hex.keys() {
            if !self.patrol_path(data, player, hex, target) {
                return Err(bad(
                    "16.22",
                    format!(
                        "no path of {RANGE} hexes or fewer from {hex} to {target} clear of the enemy"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// `Order::Patrol` (16.x): the checks, then the three dice.
    pub(crate) fn patrol(
        &mut self,
        data: &Data,
        player: Side,
        sources: &[(PieceId, u32)],
        target: &HexId,
        exclude: &[PieceId],
    ) -> Result<Vec<Event>, Illegal> {
        self.check_patrol(data, player, sources, target)?;
        let mut events = Vec::new();
        let enemy = player.other();
        let sent: u32 = sources.iter().map(|(_, n)| n).sum();
        for (id, n) in sources {
            let hex = self.pieces[id].hex().cloned().expect("checked on the map");
            *self.stage_record.patrolled_from.entry(hex).or_insert(0) += n;
        }
        *self
            .stage_record
            .patrolled_at
            .entry(target.clone())
            .or_insert(0) += sent;
        let defenders = self.patrolled(target, player);
        let combat_present = defenders
            .iter()
            .any(|id| self.role(data, &self.pieces[id]) == Role::Combat);

        // 1. Patrol Survival (16.31, 16.6).
        let mut survival_die = None;
        let (mut killed, mut captured) = (0, 0);
        if combat_present {
            let die = self.rng.d6();
            survival_die = Some(die);
            let all_recce = sources
                .iter()
                .all(|(id, _)| self.is_recce(data, &self.pieces[id]));
            let modifier = if all_recce {
                data.patrol.survival.recce_only_modifier
            } else {
                0
            };
            let row = data
                .patrol
                .survival
                .row(i32::from(die) + modifier)
                .expect("patrol-survival.json has a row for every modified die");
            killed = row.killed.min(sent);
            captured = row.captured.min(sent - killed);
            let mut owed = killed + captured;
            for (id, n) in sources {
                if owed == 0 {
                    break;
                }
                owed -= self.take_patrol_loss(data, id, owed.min(*n), &mut events);
            }
            if captured > 0 {
                self.capture_prisoners(data, enemy, target, captured, &mut events);
            }
        }
        let surviving = sent - killed - captured;
        let wiped = surviving == 0;

        // 2. Reconnaissance (16.5x, 16.7).
        let mut recon_die = None;
        let mut described = Vec::new();
        if !wiped {
            let die = self.rng.d6();
            recon_die = Some(die);
            let count = data.patrol.reconnaissance.revealed(die, surviving);
            let mut candidates: Vec<&Piece> = defenders
                .iter()
                .map(|id| &self.pieces[id])
                .filter(|p| !exclude.contains(&p.id))
                .filter(|p| {
                    !combat_present || !matches!(self.role(data, p), Role::Hq | Role::AntiAir)
                })
                .collect();
            // Battalions first, then by id (16.53).
            candidates.sort_by_key(|p| (p.equivalent != Some(Equivalent::Battalion), p.id.clone()));
            described = candidates
                .into_iter()
                .take(match count {
                    Revealed::Count(n) => n as usize,
                    Revealed::All(_) => usize::MAX,
                })
                .map(|p| Described {
                    piece: p.id.clone(),
                    toe: p.toe,
                })
                .collect();
        }

        // 3. Objective Loss (16.32, 16.8).
        let die = self.rng.d6();
        let row = data
            .patrol
            .objective_loss
            .row(i32::from(die))
            .expect("objective-loss.json has a row for every die");
        let (mut objective_killed, mut objective_captured) = (row.killed, row.captured);
        if wiped {
            objective_killed += objective_captured;
            objective_captured = 0;
        }
        let mut victims: Vec<PieceId> = defenders.clone();
        victims.sort_by_key(|id| (self.role(data, &self.pieces[id]), id.clone()));
        let mut owed = objective_killed + objective_captured;
        let mut taken = 0;
        for id in &victims {
            if owed == 0 {
                break;
            }
            let t = self.take_loss(id, owed, &mut events);
            owed -= t;
            taken += t;
        }
        let objective_captured = objective_captured.min(taken);
        let objective_killed = taken - objective_captured;
        if objective_captured > 0 {
            self.capture_prisoners(data, player, target, objective_captured, &mut events);
        }

        events.insert(
            0,
            Event::Patrolled(Box::new(PatrolReport {
                side: player,
                target: target.clone(),
                sources: sources.to_vec(),
                survival_die,
                killed,
                captured,
                recon_die,
                described,
                objective_die: die,
                objective_killed,
                objective_captured,
            })),
        );
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, moving, pid};
    use crate::order::{Allocation, Assignment, Order};
    use crate::piece::Location;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    const KRRC: &str = "unit:cw:1-krrc";

    /// The Commonwealth's phase K, patrols on, 1 KRRC (on foot)
    /// at C4021 beside the Axis in C4020.
    fn at_repair(data: &Data) -> GameState {
        let mut state = moving(data, 3, Side::Cw);
        state.modules.patrols = true;
        state.pieces.get_mut(&pid(KRRC)).unwrap().location = Location::Hex(hex("C4021"));
        state.clock.phase = Phase::Repair;
        state.clock.segment = None;
        state
    }

    fn at_patrol(data: &Data) -> GameState {
        let mut state = at_repair(data);
        state.apply(data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.phase, Phase::Patrol);
        state
    }

    fn patrol(sources: &[(&str, u32)], target: &str) -> Order {
        Order::Patrol {
            sources: sources.iter().map(|(p, n)| (pid(p), *n)).collect(),
            target: hex(target),
            exclude: vec![],
        }
    }

    const HUS: &str = "unit:cw:11-hus";
    const AUSCAV: &str = "unit:cw:6-auscav";

    fn put(state: &mut GameState, id: &str, at: &str) {
        state.pieces.get_mut(&pid(id)).unwrap().location = Location::Hex(hex(at));
    }

    /// 11 Hussars (recce, 6 points) beside the Axis in C4020.
    fn hussars(data: &Data) -> GameState {
        let mut state = at_patrol(data);
        put(&mut state, HUS, "C4021");
        state
    }

    fn refusal(state: &mut GameState, data: &Data, order: Order) -> Illegal {
        let before = serde_json::to_string(&*state).unwrap();
        let e = state.apply(data, Side::Cw, order).unwrap_err();
        assert_eq!(serde_json::to_string(&*state).unwrap(), before);
        e
    }

    #[test]
    fn phase_l_opens_after_a_stage_with_no_assault_and_stays_skipped_after_one_16_21() {
        let data = graziani();
        let state = at_patrol(&data);
        assert_eq!(state.clock.phase, Phase::Patrol);
        let mut fought = at_repair(&data);
        fought.stage_record.fought.insert(Side::Cw);
        fought.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_ne!(fought.clock.phase, Phase::Patrol);
        // A storm over every source keeps it shut (16.17).
        let mut storm = at_repair(&data);
        storm.weather = crate::state::Weather::Sandstorm { sheets: vec!['C'] };
        storm.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_ne!(storm.clock.phase, Phase::Patrol);
        // Off without the module, as old saves play.
        let mut old = at_repair(&data);
        old.modules.patrols = false;
        old.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_ne!(old.clock.phase, Phase::Patrol);
        // Pass ends phase L.
        let mut state = state;
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_ne!(state.clock.phase, Phase::Patrol);
    }

    #[test]
    fn a_combat_segment_with_anti_armour_or_assault_records_the_stage_fought() {
        let data = graziani();
        let mut state = at_repair(&data);
        state.clock.phase = Phase::MovementCombat;
        state.combat.assignments.insert(
            Side::Cw,
            vec![Assignment {
                piece: pid(KRRC),
                anti_armour: vec![],
                close_assault: vec![Allocation {
                    target: None,
                    points: 3,
                }],
            }],
        );
        state.combat.barraged.insert(hex("C4120"), 4);
        state.record_for_patrols();
        assert!(state.stage_record.fought.is_empty(), "nothing aimed");
        assert!(state.stage_record.barraged.contains(&hex("C4120")));
        state.combat.assignments.get_mut(&Side::Cw).unwrap()[0].close_assault[0].target =
            Some(hex("C4020"));
        state.record_for_patrols();
        assert!(state.stage_record.fought.contains(&Side::Cw));
        state.start_stage();
        assert!(state.stage_record.is_empty());
    }

    #[test]
    fn each_eligibility_limit_refuses_16_1x_16_2x() {
        let data = graziani();
        let mut state = hussars(&data);
        // Not phase L.
        let mut early = at_repair(&data);
        put(&mut early, HUS, "C4021");
        refusal(&mut early, &data, patrol(&[(HUS, 1)], "C4020"));
        // Not the phasing player.
        assert!(
            state
                .apply(&data, Side::Axis, patrol(&[(HUS, 1)], "C4020"))
                .is_err()
        );
        // Infantry on foot sends no points.
        let e = refusal(&mut state, &data, patrol(&[(KRRC, 1)], "C4020"));
        assert_eq!(e.case, "16.11");
        // More points than the unit has.
        let mut weak = hussars(&data);
        weak.pieces.get_mut(&pid(HUS)).unwrap().toe = 1;
        assert_eq!(
            refusal(&mut weak, &data, patrol(&[(HUS, 2)], "C4020")).case,
            "16.11"
        );
        // Two a hex.
        let e = refusal(&mut state, &data, patrol(&[(HUS, 3)], "C4020"));
        assert_eq!(e.case, "16.13");
        // Three a target.
        let mut pooled = hussars(&data);
        put(&mut pooled, AUSCAV, "C4022");
        let e = refusal(
            &mut pooled,
            &data,
            patrol(&[(HUS, 2), (AUSCAV, 2)], "C4020"),
        );
        assert_eq!(e.case, "16.13");
        assert!(e.reason.contains("at most 3"));
        // Cohesion -8.
        let mut shaken = hussars(&data);
        shaken.pieces.get_mut(&pid(HUS)).unwrap().cohesion.level = -8;
        assert_eq!(
            refusal(&mut shaken, &data, patrol(&[(HUS, 1)], "C4020")).case,
            "16.14"
        );
        // A storm on the sheet.
        let mut storm = hussars(&data);
        storm.weather = crate::state::Weather::Sandstorm { sheets: vec!['C'] };
        assert_eq!(
            refusal(&mut storm, &data, patrol(&[(HUS, 1)], "C4020")).case,
            "16.17"
        );
        // No enemy in the target.
        assert_eq!(
            refusal(&mut state, &data, patrol(&[(HUS, 1)], "C3922")).case,
            "16.22"
        );
        // Barraged this stage.
        let mut barraged = hussars(&data);
        barraged.stage_record.barraged.insert(hex("C4020"));
        assert_eq!(
            refusal(&mut barraged, &data, patrol(&[(HUS, 1)], "C4020")).case,
            "16.22"
        );
        // Too far.
        let mut far = hussars(&data);
        put(&mut far, HUS, "E1730");
        let e = refusal(&mut far, &data, patrol(&[(HUS, 1)], "C4020"));
        assert!(e.reason.contains("no path"), "{e:?}");
        // And the patrol the limits allow goes out.
        state
            .apply(&data, Side::Cw, patrol(&[(HUS, 2)], "C4020"))
            .unwrap();
    }

    #[test]
    fn the_hex_and_target_limits_hold_across_the_phases_patrols_16_13() {
        let data = graziani();
        // Two a hex: one point now, two more refused, one more allowed.
        let mut state = hussars(&data);
        state
            .apply(&data, Side::Cw, patrol(&[(HUS, 1)], "C4020"))
            .unwrap();
        assert_eq!(
            refusal(&mut state, &data, patrol(&[(HUS, 2)], "C4020")).case,
            "16.13"
        );
        state
            .apply(&data, Side::Cw, patrol(&[(HUS, 1)], "C4020"))
            .unwrap();
        // Three a target, pooled from two hexes over two patrols.
        let mut pooled = hussars(&data);
        put(&mut pooled, AUSCAV, "C4022");
        pooled
            .apply(&data, Side::Cw, patrol(&[(HUS, 2)], "C4020"))
            .unwrap();
        let e = refusal(&mut pooled, &data, patrol(&[(AUSCAV, 2)], "C4020"));
        assert!(e.reason.contains("at most 3"), "{e:?}");
        pooled
            .apply(&data, Side::Cw, patrol(&[(AUSCAV, 1)], "C4020"))
            .unwrap();
        // The next stage starts afresh.
        pooled.start_stage();
        assert!(pooled.stage_record.is_empty());
    }

    fn report(events: &[Event]) -> PatrolReport {
        events
            .iter()
            .find_map(|e| match e {
                Event::Patrolled(r) => Some((**r).clone()),
                _ => None,
            })
            .expect("a patrol report")
    }

    #[test]
    fn the_three_rolls_apply_each_table_row_16_6_to_16_8() {
        let data = graziani();
        let t = &data.patrol;
        let mut survival_rows = BTreeSet::new();
        let mut objective_rows = BTreeSet::new();
        let mut wiped_seen = false;
        let base = hussars(&data);
        for seed in 0..80 {
            let mut state = base.clone();
            state.rng = crate::rng::GameRng::new(seed);
            let toe_before = state.pieces[&pid(HUS)].toe;
            let defenders = state.patrolled(&hex("C4020"), Side::Cw);
            let enemy_toe: u32 = defenders.iter().map(|id| state.pieces[id].toe).sum();
            let events = state
                .apply(&data, Side::Cw, patrol(&[(HUS, 1)], "C4020"))
                .unwrap();
            let r = report(&events);
            // All recce: one off the die.
            let die = r.survival_die.expect("combat units in C4020");
            let row = t.survival.row(i32::from(die) - 1).unwrap();
            survival_rows.insert((row.killed, row.captured));
            assert_eq!(r.killed, row.killed.min(1));
            assert_eq!(r.captured, row.captured.min(1 - r.killed));
            assert_eq!(
                state.pieces[&pid(HUS)].toe,
                toe_before - r.killed - r.captured
            );
            let wiped = r.killed + r.captured == 1;
            let o = t.objective_loss.row(i32::from(r.objective_die)).unwrap();
            objective_rows.insert((o.killed, o.captured));
            if wiped {
                wiped_seen = true;
                assert_eq!(r.recon_die, None, "a wiped-out patrol learns nothing");
                assert!(r.described.is_empty());
                assert_eq!(r.objective_captured, 0, "its captures read as kills");
                assert_eq!(r.objective_killed, o.killed + o.captured);
            } else {
                let Revealed::Count(n) = t.reconnaissance.revealed(r.recon_die.unwrap(), 1) else {
                    panic!("one point never reveals all")
                };
                assert_eq!(r.described.len(), n as usize);
                assert!(
                    r.described.iter().all(|d| {
                        data.chars
                            .row(
                                state.pieces[&d.piece].nation.unwrap(),
                                state.pieces[&d.piece].id_code.as_deref().unwrap(),
                            )
                            .is_some_and(|row| row.unit_type != "Headquarters")
                    }),
                    "no HQ while combat units are present"
                );
                assert_eq!(
                    (r.objective_killed, r.objective_captured),
                    (o.killed, o.captured)
                );
            }
            let after: u32 = defenders.iter().map(|id| state.pieces[id].toe).sum();
            assert_eq!(enemy_toe - after, r.objective_killed + r.objective_captured);
        }
        // All recce, the die never reads 6: the both-lost row is out of reach.
        assert_eq!(survival_rows.len(), 3, "{survival_rows:?}");
        assert!(!survival_rows.contains(&(1, 1)));
        assert_eq!(objective_rows.len(), 3, "{objective_rows:?}");
        assert!(wiped_seen);
    }

    #[test]
    fn described_units_are_battalions_first_and_never_an_excluded_one_16_53() {
        let data = graziani();
        let base = hussars(&data);
        for seed in 0..40 {
            let mut state = base.clone();
            state.rng = crate::rng::GameRng::new(seed);
            let exclude = vec![pid("unit:it:ix-libyan-bn")];
            let before = state.pieces.clone();
            let events = state
                .apply(
                    &data,
                    Side::Cw,
                    Order::Patrol {
                        sources: vec![(pid(HUS), 2)],
                        target: hex("C4020"),
                        exclude: exclude.clone(),
                    },
                )
                .unwrap();
            let r = report(&events);
            assert!(r.described.iter().all(|d| !exclude.contains(&d.piece)));
            assert!(
                r.described
                    .iter()
                    .all(|d| { state.pieces[&d.piece].equivalent == Some(Equivalent::Battalion) }),
                "six battalions outnumber any roll on two points"
            );
            for d in &r.described {
                assert_eq!(d.toe, before[&d.piece].toe, "stated exactly");
            }
        }
    }

    #[test]
    fn the_enemy_sees_the_descriptions_and_nothing_more_3_61_16_5x() {
        let data = graziani();
        let base = hussars(&data);
        let aliases = crate::view::Aliases::new([3; 32]);
        let mut described = false;
        for seed in 0..30 {
            let mut state = base.clone();
            state.rng = crate::rng::GameRng::new(seed);
            let events = state
                .apply(&data, Side::Cw, patrol(&[(HUS, 2)], "C4020"))
                .unwrap();
            let r = report(&events);
            described |= !r.described.is_empty();
            for viewer in [Side::Cw, Side::Axis] {
                let seen = crate::view::events_for(&state, &aliases, &events, Some(viewer), false);
                let patrolled: Vec<_> = seen
                    .iter()
                    .filter(|v| v.get("patrolled").is_some())
                    .collect();
                assert_eq!(patrolled.len(), 1, "public to both");
                for v in &seen {
                    assert_eq!(
                        crate::view::leaks(&state, v, viewer.other()),
                        Vec::<String>::new(),
                        "{v}"
                    );
                }
            }
            // The described TOE is the one internal let through.
            let cw = crate::view::events_for(&state, &aliases, &events, Some(Side::Cw), false);
            let p = cw.iter().find(|v| v.get("patrolled").is_some()).unwrap();
            assert_eq!(
                p["patrolled"]["described"].as_array().map_or(0, Vec::len),
                r.described.len()
            );
            // The Axis's own losses stay its own.
            assert!(
                cw.iter().all(|v| v.get("losses").is_none_or(|l| {
                    state.pieces[&PieceId::from(l["piece"].as_str().unwrap().to_owned())].side
                        == Side::Cw
                })),
                "{cw:?}"
            );
        }
        assert!(described);
    }

    #[test]
    fn a_supply_unit_in_the_target_is_never_described_nor_loses_toe() {
        let data = graziani();
        let mut base = hussars(&data);
        let supply = base
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind.is_supply())
            .map(|p| p.id.clone())
            .expect("an Axis supply unit");
        base.pieces.get_mut(&supply).unwrap().location = Location::Hex(hex("C4020"));
        for seed in 0..30 {
            let mut state = base.clone();
            state.rng = crate::rng::GameRng::new(seed);
            let events = state
                .apply(&data, Side::Cw, patrol(&[(HUS, 2)], "C4020"))
                .unwrap();
            assert!(report(&events).described.iter().all(|d| d.piece != supply));
            assert_eq!(state.pieces[&supply].toe, base.pieces[&supply].toe);
        }
    }

    #[test]
    fn a_unit_that_sends_its_light_tanks_loses_light_tanks_16_31() {
        let data = graziani();
        let mut s = moving(&data, 3, Side::Cw);
        let id = pid("unit:cw:1-6-rajrf");
        let p = s.pieces.get_mut(&id).unwrap();
        p.toe = 6;
        p.weapons = vec![
            cna_data::oa::ToeEntry {
                weapon: "Mark VI Light".to_owned(),
                points: 2,
            },
            cna_data::oa::ToeEntry {
                weapon: "A13 Cruiser".to_owned(),
                points: 4,
            },
        ];
        assert_eq!(s.patrol_points(&data, &s.pieces[&id]), 2);
        let mut events = Vec::new();
        assert_eq!(s.take_patrol_loss(&data, &id, 2, &mut events), 2);
        let left: Vec<(String, u32)> = s.pieces[&id]
            .weapons
            .iter()
            .map(|w| (w.weapon.clone(), w.points))
            .collect();
        assert!(left.contains(&("A13 Cruiser".to_owned(), 4)), "{left:?}");
        assert_eq!(s.patrol_points(&data, &s.pieces[&id]), 0);
    }
}
