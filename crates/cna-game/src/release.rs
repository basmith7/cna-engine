//! A scenario's release schedule (61.38, R-159): the units in its release
//! cities when play starts leave them only as the schedule allows. Nothing
//! held moves before `first`; from then on `per_opstage` counters a stage
//! leave, every HQ freely, a `counts_as_one` unit with whatever is attached
//! to it as one, and a formation's own allowance comes on top. Units that
//! arrive later are never held.

use crate::data::Data;
use crate::event::Illegal;
use crate::order::Order;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::oa::UnitId;
use cna_data::scenario::{Locator, Side};
use cna_data::time::GameTime;
use cna_data::weapons::WeaponClass;
use cna_rules::supply::FuelClass;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The held units and what this stage has released.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// Units in a release city when play started and not yet released.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub held: BTreeSet<PieceId>,
    /// The OpStage the counts below belong to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<GameTime>,
    /// Counters released this stage under the general allowance, and
    /// under the formation's tank and other allowances.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub general: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tanks: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub other: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

impl Release {
    pub fn is_empty(&self) -> bool {
        *self == Release::default()
    }
}

/// What an accepted order releases: the pieces leaving, and the count
/// it takes from each allowance this stage.
#[derive(Debug, Default)]
pub(crate) struct Released {
    pieces: Vec<PieceId>,
    stage: Option<GameTime>,
    general: u32,
    tanks: u32,
    other: u32,
}

/// What `moved_by` reads off an order: the moving pieces, the path,
/// and each rail stop's hex with the pieces set down there.
type Moved<'a> = (Vec<PieceId>, &'a [HexId], Vec<(&'a HexId, &'a [PieceId])>);

impl GameState {
    /// The release zone: the `from` hexes, and around them the hexes of
    /// any set-up block the scenario places "within" so many hexes of one
    /// (Rommel's Arrival's Alexandria group, set up within one hex of
    /// Alexandria, is the Alexandria of 61.38).
    fn release_hexes(data: &Data, side: Side) -> Option<BTreeSet<HexId>> {
        let setup = data.scenario.side(side);
        let r = setup.release.as_ref()?;
        let mut hexes = BTreeSet::new();
        for l in &r.from.locators {
            match l {
                Locator::Place(ids) => {
                    for id in ids {
                        if let Some(p) = data.map.place(id) {
                            hexes.extend(p.hexes.iter().cloned());
                        }
                    }
                }
                Locator::Hexes(h) => hexes.extend(h.iter().cloned()),
                _ => {}
            }
        }
        let mut around = BTreeSet::new();
        for d in &setup.deployments {
            for l in &d.placement.locators {
                if let Locator::Within { of, hexes: n } = l
                    && hexes.contains(of)
                {
                    around.extend(data.map.within(of, *n));
                }
            }
        }
        hexes.extend(around);
        Some(hexes)
    }

    /// When play starts: every combat unit then standing in a release
    /// city is held (61.38). HQs are never held (R-159).
    pub(crate) fn hold_for_release(&mut self, data: &Data) {
        if !self.modules.release_schedule {
            return;
        }
        for side in [Side::Axis, Side::Cw] {
            let Some(hexes) = Self::release_hexes(data, side) else {
                continue;
            };
            let held: Vec<PieceId> = self
                .pieces
                .values()
                .filter(|p| {
                    p.side == side
                        && p.kind == PieceKind::Combat
                        && p.hex().is_some_and(|h| hexes.contains(h))
                })
                .map(|p| p.id.clone())
                .collect();
            self.release.held.extend(held);
        }
    }

    /// Whether `piece` belongs to the formation headed by `hq`: its own
    /// parent or an OA ancestor of it is `hq`.
    pub(crate) fn in_formation(&self, data: &Data, piece: &PieceId, hq: &UnitId) -> bool {
        let mut at = self.pieces.get(piece).and_then(|p| p.parent.clone());
        let mut steps = 0;
        while let Some(u) = at {
            if &u == hq {
                return true;
            }
            steps += 1;
            if steps > 16 {
                break;
            }
            at = data.oa.unit(&u).and_then(|r| r.parent.clone());
        }
        false
    }

    /// Tank (or other) points broken down and held for `piece`'s unit,
    /// to go back into it when repaired (61.38's `all_tanks`).
    pub(crate) fn held_for(&self, piece: &PieceId) -> u32 {
        let Some(unit) = piece.unit() else {
            return 0;
        };
        self.broken_down
            .values()
            .flatten()
            .filter(|w| w.unit.as_ref() == Some(&unit))
            .map(|w| w.points)
            .sum()
    }

    /// The share, in hundredths, of a battalion's tank TOE points that
    /// are repaired: those it holds against those plus the ones still
    /// broken down for it (61.38's `all_tanks`, `tanks_with_units`: "a
    /// tank battalion is released only once at least three quarters
    /// repaired"). A battalion with no tank points owed counts as whole.
    fn tanks_repaired_pct(&self, data: &Data, piece: &PieceId) -> u32 {
        let Some(p) = self.pieces.get(piece) else {
            return 100;
        };
        let have: u32 = p
            .weapons
            .iter()
            .filter(|w| {
                p.nation
                    .and_then(|n| data.weapons.resolve(n, &w.weapon))
                    .is_some_and(|r| r.class == WeaponClass::Tank)
            })
            .map(|w| w.points)
            .sum();
        let held = self.held_for(piece);
        (have * 100).checked_div(have + held).unwrap_or(100)
    }

    /// The pieces an order names as moving along a path (a rail run's
    /// boarders included), that path, and the pieces a rail run sets
    /// down at each stop.
    fn moved_by(order: &Order) -> Option<Moved<'_>> {
        match order {
            Order::Move { pieces, path, .. }
            | Order::React { pieces, path }
            | Order::RetreatBeforeAssault { pieces, path } => {
                Some((pieces.clone(), path, Vec::new()))
            }
            Order::Rail {
                pieces,
                path,
                stops,
            } => {
                let mut all = pieces.clone();
                for st in stops {
                    all.extend(st.on.iter().cloned());
                }
                let off = stops.iter().map(|st| (&st.at, st.off.as_slice())).collect();
                Some((all, path, off))
            }
            _ => None,
        }
    }

    /// Refuses an order that would take more held units out of a release
    /// city than the schedule allows this stage (61.38, R-159); otherwise
    /// what it releases, for `release_commit` once the order is applied.
    pub(crate) fn release_check(
        &self,
        data: &Data,
        player: Side,
        order: &Order,
    ) -> Result<Released, Illegal> {
        let none = Released::default();
        if !self.modules.release_schedule || self.release.held.is_empty() {
            return Ok(none);
        }
        let (Some((listed, path, set_down)), Some(r)) = (
            Self::moved_by(order),
            data.scenario.side(player).release.as_ref(),
        ) else {
            return Ok(none);
        };
        let hexes = Self::release_hexes(data, player).unwrap_or_default();
        // Everything attached to a moving piece goes with it, a free HQ's
        // held units included; a rail passenger set down inside the
        // cities does not leave.
        let mut moving: BTreeSet<PieceId> = match self.carried(&listed) {
            Ok(c) => c.into_iter().collect(),
            Err(_) => listed.iter().cloned().collect(),
        };
        for (at, off) in set_down {
            if hexes.contains(at) {
                for id in off {
                    moving.remove(id);
                }
            }
        }
        let held: Vec<PieceId> = moving
            .into_iter()
            .filter(|id| {
                self.release.held.contains(id)
                    && self.pieces[id].hex().is_some_and(|h| hexes.contains(h))
            })
            .collect();
        if held.is_empty() {
            return Ok(none);
        }
        if path.iter().all(|h| hexes.contains(h)) {
            // Staying within the cities releases nothing.
            return Ok(none);
        }
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        if now < r.first {
            return Err(Illegal::new(
                "61.38",
                format!(
                    "{} is held in Cairo or Alexandria: nothing there moves before {}",
                    held[0],
                    stage_name(r.first)
                ),
            ));
        }
        // Group the leavers: a counts_as_one unit and everything attached
        // to it (or under it) are one.
        let mut groups: Vec<Vec<PieceId>> = Vec::new();
        let mut grouped: BTreeSet<PieceId> = BTreeSet::new();
        for one in &r.counts_as_one {
            let g: Vec<PieceId> = held
                .iter()
                .filter(|id| {
                    let p = &self.pieces[*id];
                    id.unit().as_ref() == Some(one)
                        || p.attached_to.as_ref() == Some(one)
                        || p.parent.as_ref() == Some(one)
                })
                .cloned()
                .collect();
            if !g.is_empty() {
                grouped.extend(g.iter().cloned());
                groups.push(g);
            }
        }
        for id in &held {
            if !grouped.contains(id) {
                groups.push(vec![id.clone()]);
            }
        }
        let fresh = self.release.stage != Some(now);
        let (mut general, mut tanks, mut other) = if fresh {
            (0, 0, 0)
        } else {
            (self.release.general, self.release.tanks, self.release.other)
        };
        let formation = r.formations.iter().find(|f| now >= f.from);
        for g in &groups {
            let ours =
                formation.filter(|f| g.iter().any(|id| self.in_formation(data, id, &f.unit)));
            if let Some(f) = ours {
                let tank = g
                    .iter()
                    .any(|id| self.fuel_class(data, &self.pieces[id]) == FuelClass::Tank);
                if tank {
                    let min = f.tank_battalions.min_repaired_pct.unwrap_or(0);
                    if let Some(short) =
                        g.iter().find(|id| self.tanks_repaired_pct(data, id) < min)
                    {
                        return Err(Illegal::new(
                            "61.38",
                            format!(
                                "{short} may not leave: under {min} % of its tanks are repaired"
                            ),
                        ));
                    }
                    if tanks < f.tank_battalions.per_opstage {
                        tanks += 1;
                        continue;
                    }
                } else if other < f.other_battalions.per_opstage {
                    other += 1;
                    continue;
                }
            }
            if general < r.per_opstage {
                general += 1;
                continue;
            }
            return Err(Illegal::new(
                "61.38",
                format!(
                    "{} may not leave Cairo or Alexandria: this stage's release is used up",
                    g[0]
                ),
            ));
        }
        Ok(Released {
            pieces: held,
            stage: Some(now),
            general,
            tanks,
            other,
        })
    }

    /// Records what an applied order released.
    pub(crate) fn release_commit(&mut self, r: Released) {
        let Some(stage) = r.stage else {
            return;
        };
        for id in &r.pieces {
            self.release.held.remove(id);
        }
        self.release.stage = Some(stage);
        self.release.general = r.general;
        self.release.tanks = r.tanks;
        self.release.other = r.other;
    }
}

fn stage_name(t: GameTime) -> String {
    format!("Game-Turn {} OpStage {}", t.game_turn, t.opstage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{Phase, Segment};
    use crate::state::Modules;
    use crate::state::test_support::{auto_place, pass_stage};

    fn data() -> Data {
        Data::load("rommels-arrival").unwrap()
    }

    fn game(data: &Data, modules: Modules) -> GameState {
        let mut s = GameState::with_modules(data, 3, modules).unwrap();
        auto_place(data, &mut s);
        s
    }

    fn pid(s: &str) -> PieceId {
        PieceId::from(UnitId::parse(s).unwrap())
    }

    /// Passes until the Commonwealth moves in this OpStage's movement
    /// segment, as Player A.
    fn cw_moving(data: &Data, s: &mut GameState) {
        for _ in 0..200 {
            if s.clock.phase == Phase::MovementCombat
                && s.clock.segment == Some(Segment::Movement)
                && s.pending.is_none()
                && s.to_act() == Some(Side::Cw)
            {
                return;
            }
            let player = s.to_act().unwrap();
            let order = if s.clock.phase == Phase::InitiativeDeclaration {
                Order::DeclareInitiative { player_a: Side::Cw }
            } else {
                Order::Pass
            };
            s.apply(data, player, order).unwrap();
        }
        panic!("never reached the Commonwealth's movement at {}", s.clock);
    }

    /// The game at the Commonwealth's movement in GT `gt` OpStage `stage`.
    fn at(data: &Data, modules: Modules, gt: u8, stage: u8) -> GameState {
        let mut s = game(data, modules);
        while (s.clock.game_turn, s.clock.stage) < (gt, stage) {
            pass_stage(data, &mut s);
        }
        cw_moving(data, &mut s);
        s
    }

    /// A move taking `ids` (stacked) out of the release zone that every
    /// other rule allows: one or two hexes.
    fn leave(data: &Data, s: &GameState, ids: &[&str]) -> Order {
        let zone = GameState::release_hexes(data, Side::Cw).unwrap();
        let pieces: Vec<PieceId> = ids.iter().map(|i| pid(i)).collect();
        let from = s.pieces[&pieces[0]].hex().unwrap().clone();
        let mut paths: Vec<Vec<HexId>> = Vec::new();
        for n in data.map.neighbours(&from) {
            paths.push(vec![n.clone()]);
            for m in data.map.neighbours(&n) {
                paths.push(vec![n.clone(), m]);
            }
        }
        for path in paths {
            if path.iter().all(|h| zone.contains(h)) {
                continue;
            }
            let order = Order::Move {
                pieces: pieces.clone(),
                path,
                assault: false,
                fuel_from: None,
            };
            let mut probe = s.clone();
            probe.modules.release_schedule = false;
            if probe.apply(data, Side::Cw, order.clone()).is_ok() {
                return order;
            }
        }
        panic!("{ids:?} cannot leave at all");
    }

    fn refused(data: &Data, s: &mut GameState, ids: &[&str]) {
        let order = leave(data, s, ids);
        let err = s.apply(data, Side::Cw, order).unwrap_err();
        assert_eq!(err.case, "61.38", "{ids:?}: {}", err.reason);
    }

    fn allowed(data: &Data, s: &mut GameState, ids: &[&str]) {
        let order = leave(data, s, ids);
        s.apply(data, Side::Cw, order).unwrap();
    }

    #[test]
    fn units_in_cairo_and_alexandria_are_held_at_the_start_61_38() {
        let data = data();
        let s = game(&data, Modules::ALL);
        for id in ["unit:cw:11-czech", "unit:cw:1-rtr", "unit:cw:2-sctgds"] {
            assert!(s.release.held.contains(&pid(id)), "{id}");
        }
        assert!(
            s.release
                .held
                .iter()
                .all(|id| s.pieces[id].kind == PieceKind::Combat),
            "HQs are never held (R-159)"
        );
        let off = game(
            &data,
            Modules {
                release_schedule: false,
                ..Modules::ALL
            },
        );
        assert!(off.release.is_empty());
    }

    #[test]
    fn nothing_leaves_in_the_opening_stage_but_hqs_do_61_38() {
        let data = data();
        let mut s = at(&data, Modules::ALL, 26, 3);
        refused(&data, &mut s, &["unit:cw:11-czech"]);
        refused(&data, &mut s, &["unit:cw:7-hus"]);
        allowed(&data, &mut s, &["unit:cw:70th-infantry-div-hq"]);
        let mut off = at(
            &data,
            Modules {
                release_schedule: false,
                ..Modules::ALL
            },
            26,
            3,
        );
        allowed(&data, &mut off, &["unit:cw:11-czech"]);
    }

    #[test]
    fn a_free_hq_does_not_carry_its_held_units_out_61_38() {
        let data = data();
        let mut s = at(&data, Modules::ALL, 26, 3);
        let attached = s
            .pieces
            .values()
            .filter(|p| {
                p.attached_to.as_ref().map(|u| u.to_string()).as_deref()
                    == Some("unit:cw:polish-hq")
            })
            .count();
        assert!(attached > 0, "the Polish brigade starts attached to its HQ");
        refused(&data, &mut s, &["unit:cw:polish-hq"]);
    }

    #[test]
    fn a_held_unit_may_shift_within_the_cities_in_the_opening_stage_61_38() {
        let data = data();
        let mut s = at(&data, Modules::ALL, 26, 3);
        let zone = GameState::release_hexes(&data, Side::Cw).unwrap();
        let id = s
            .release
            .held
            .iter()
            .find(|id| s.pieces[*id].attached_to.is_none())
            .cloned()
            .unwrap();
        let from = s.pieces[&id].hex().unwrap().clone();
        let inside = data
            .map
            .neighbours(&from)
            .into_iter()
            .find(|n| zone.contains(n))
            .expect("a city hex next door");
        s.apply(
            &data,
            Side::Cw,
            Order::Move {
                pieces: vec![id.clone()],
                path: vec![inside],
                assault: false,
                fuel_from: None,
            },
        )
        .unwrap_or_else(|e| panic!("{id}: {}", e.reason));
    }

    #[test]
    fn one_battalion_a_stage_and_the_7th_armoured_s_own_on_top_r_159() {
        let data = data();
        let mut s = at(&data, Modules::ALL, 27, 1);
        allowed(&data, &mut s, &["unit:cw:11-czech"]);
        refused(&data, &mut s, &["unit:cw:2-kor"]);
        // The 7th Armoured's tank battalion and other battalion.
        while repair_one(&mut s) {}
        allowed(&data, &mut s, &["unit:cw:7-hus"]);
        allowed(&data, &mut s, &["unit:cw:2-rflbde"]);
        refused(&data, &mut s, &["unit:cw:3-rha"]);
        assert!(!s.release.held.contains(&pid("unit:cw:11-czech")));
        // The next stage lets another go.
        pass_stage(&data, &mut s);
        cw_moving(&data, &mut s);
        allowed(&data, &mut s, &["unit:cw:2-kor"]);
        refused(&data, &mut s, &["unit:cw:1-ches"]);
    }

    /// Repairs one of the 7th Armoured's broken-down tank points, back
    /// into its unit; false once none is left.
    fn repair_one(s: &mut GameState) -> bool {
        let Some((hex, weapon)) = s
            .broken_down
            .iter()
            .flat_map(|(h, ws)| ws.iter().map(move |w| (h, w)))
            .find(|(_, w)| w.unit.is_some())
            .map(|(h, w)| (h.clone(), w.weapon.clone()))
        else {
            return false;
        };
        let taken = s.take_wrecks(&hex, Side::Cw, &weapon, 1, false, None);
        s.credit_repaired(Side::Cw, &hex, &weapon, taken);
        true
    }

    /// Repairs `n` of the points held for `unit`, back into it.
    fn repair_for(s: &mut GameState, unit: &str, n: u32) {
        let unit = UnitId::parse(unit).unwrap();
        for _ in 0..n {
            let (hex, weapon) = s
                .broken_down
                .iter()
                .flat_map(|(h, ws)| ws.iter().map(move |w| (h, w)))
                .find(|(_, w)| w.unit.as_ref() == Some(&unit))
                .map(|(h, w)| (h.clone(), w.weapon.clone()))
                .expect("a point held for the unit");
            let taken = s.take_wrecks(&hex, Side::Cw, &weapon, 1, false, None);
            assert_eq!(taken, vec![(Some(unit.clone()), 1)], "took the unit's own point");
            s.credit_repaired(Side::Cw, &hex, &weapon, taken);
        }
    }

    #[test]
    fn a_tank_battalion_leaves_once_75_percent_repaired_61_38() {
        let data = data();
        let mut s = at(&data, Modules::ALL, 27, 1);
        let hus7 = pid("unit:cw:7-hus");
        assert_eq!(s.tanks_repaired_pct(&data, &hus7), 0);
        refused(&data, &mut s, &["unit:cw:7-hus"]);
        // The 7th Hussars' 10 Mark VI: 7 is 70 %, 8 is 80 %.
        repair_for(&mut s, "unit:cw:7-hus", 7);
        assert_eq!(s.tanks_repaired_pct(&data, &hus7), 70);
        refused(&data, &mut s, &["unit:cw:7-hus"]);
        repair_for(&mut s, "unit:cw:7-hus", 1);
        assert_eq!(s.tanks_repaired_pct(&data, &hus7), 80);
        allowed(&data, &mut s, &["unit:cw:7-hus"]);
        // Without the module the tanks start whole and the test passes.
        let mut off = Modules::ALL;
        off.tanks_with_units = false;
        let mut plain = at(&data, off, 27, 1);
        assert_eq!(plain.tanks_repaired_pct(&data, &hus7), 100);
        allowed(&data, &mut plain, &["unit:cw:7-hus"]);
    }

    #[test]
    fn the_rest_of_the_division_repaired_does_not_release_a_battalion_61_38() {
        use std::collections::BTreeMap;
        let data = data();
        let mut s = at(&data, Modules::ALL, 27, 1);
        // Everything repaired but the 8th Hussars' 7 A10, then one of
        // those: the division is at 77 %, the battalion at 14 %.
        let mut owed: BTreeMap<String, u32> = BTreeMap::new();
        for w in s.broken_down.values().flatten() {
            if let Some(u) = &w.unit {
                *owed.entry(u.to_string()).or_default() += w.points;
            }
        }
        assert_eq!(owed.remove("unit:cw:8-hus"), Some(7));
        for (unit, n) in owed {
            repair_for(&mut s, &unit, n);
        }
        repair_for(&mut s, "unit:cw:8-hus", 1);
        assert_eq!(s.tanks_repaired_pct(&data, &pid("unit:cw:8-hus")), 14);
        // A whole battalion is not held back by the 8th's.
        assert_eq!(s.tanks_repaired_pct(&data, &pid("unit:cw:7-hus")), 100);
        allowed(&data, &mut s, &["unit:cw:7-hus"]);
    }

    #[test]
    fn the_22nd_guards_brigade_counts_as_one_61_38() {
        let data = data();
        let mut s = at(&data, Modules::ALL, 27, 1);
        allowed(&data, &mut s, &["unit:cw:2-sctgds", "unit:cw:3-cldgds"]);
        refused(&data, &mut s, &["unit:cw:11-czech"]);
    }

    #[test]
    fn units_arriving_later_are_not_held_61_38() {
        let data = data();
        let start = game(&data, Modules::ALL);
        let s = at(&data, Modules::ALL, 27, 2);
        let zone = GameState::release_hexes(&data, Side::Cw).unwrap();
        let arrived: Vec<&PieceId> = s
            .pieces
            .values()
            .filter(|p| {
                p.side == Side::Cw
                    && p.kind == PieceKind::Combat
                    && p.hex().is_some_and(|h| zone.contains(h))
                    && !start.release.held.contains(&p.id)
            })
            .map(|p| &p.id)
            .collect();
        assert!(
            !arrived.is_empty(),
            "the 18th Australian lands at Alexandria"
        );
        assert!(arrived.iter().all(|id| !s.release.held.contains(*id)));
    }

    #[test]
    fn the_schedule_replays_byte_for_byte() {
        let data = data();
        let run = || {
            let mut s = at(&data, Modules::ALL, 27, 1);
            allowed(&data, &mut s, &["unit:cw:11-czech"]);
            serde_json::to_string(&s).unwrap()
        };
        assert_eq!(run(), run());
    }
}
