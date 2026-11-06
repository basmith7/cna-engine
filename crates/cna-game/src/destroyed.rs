//! Destroyed tanks (SPI 14.5x, 22.4x) with the `destroyed_tanks` module
//! on. A tank point destroyed in combat (anti-armour fire, close assault
//! and the losses that go with them) stays where it died as a
//! destroyed-tank marker, recording its side, nation and weapon system.
//! A battalion or bigger entering a hex of enemy markers with no enemy
//! combat unit takes them (14.52, R-014); in phase K the owner may
//! repair his own at one of his repair facilities on the Destroyed Tank
//! Repair Table, pre-paying two fuel a point attempted under abstract
//! supply (22.4x). Repaired points wait as replacement points (22.5).
//!
//! Not simulated: the captor's choice to eliminate rather than keep
//! (he always keeps: the markers are no use to him dead and a kept
//! marker can still be left), the three-hex shift on capture, towing
//! markers, the field column (tank delivery and recovery squadrons), and
//! barrage losses, which leave no marker.

use crate::breakdown::Wreck;
use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::oa::Nation;
use cna_data::repair::TankOutcome;
use cna_data::scenario::{FacilityKind, Side};

/// One destroyed-tank repair the phasing player may attempt now.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DestroyedTankOffer {
    pub hex: HexId,
    pub weapon: String,
    /// The most points that may be attempted: the marker's, or what the
    /// fuel in range pre-pays.
    pub points: u32,
    pub kind: FacilityKind,
}

/// Fuel pre-paid per destroyed-tank point attempted (22.4x).
const FUEL_A_POINT: u32 = 2;

impl GameState {
    /// One tank point of `id` (listed under `weapon`, else its ID code's)
    /// destroyed in combat: it joins its side's marker in the piece's hex
    /// (14.5x), and the `TanksDestroyed` event for the piece in `out`.
    pub(crate) fn mark_destroyed_tank(
        &mut self,
        id: &PieceId,
        weapon: Option<&str>,
        out: &mut Vec<Event>,
    ) {
        if !self.modules.destroyed_tanks {
            return;
        }
        let p = &self.pieces[id];
        let (Some(hex), Some(weapon)) = (p.hex().cloned(), weapon.or(p.id_code.as_deref())) else {
            return;
        };
        let weapon = weapon.to_string();
        let (side, nation) = (p.side, p.nation);
        let wrecks = self.destroyed_tanks.entry(hex.clone()).or_default();
        match wrecks
            .iter_mut()
            .find(|w| w.side == side && w.nation == nation && w.weapon == weapon)
        {
            Some(w) => w.points += 1,
            None => wrecks.push(Wreck {
                side,
                nation,
                weapon: weapon.clone(),
                points: 1,
                this_stage: 0,
                towed: 0,
            }),
        }
        for e in out.iter_mut() {
            if let Event::TanksDestroyed {
                piece,
                weapon: w,
                points,
                ..
            } = e
                && piece == id
                && *w == weapon
            {
                *points += 1;
                return;
            }
        }
        out.push(Event::TanksDestroyed {
            piece: id.clone(),
            hex,
            weapon,
            points: 1,
        });
    }

    /// `side`'s tank points the captor took in 15.89's captured share
    /// (R-152) are not also left as markers: `points` come off the
    /// markers this assault's `TanksDestroyed` events in `events` made
    /// for `side`, in event order, and off those events.
    pub(crate) fn unmark_captured_tanks(
        &mut self,
        side: Side,
        mut points: u32,
        events: &mut Vec<Event>,
    ) {
        if !self.modules.destroyed_tanks {
            return;
        }
        for e in events.iter_mut() {
            if points == 0 {
                break;
            }
            let Event::TanksDestroyed {
                piece,
                hex,
                weapon,
                points: n,
            } = e
            else {
                continue;
            };
            let p = &self.pieces[piece];
            if p.side != side {
                continue;
            }
            let nation = p.nation;
            let take = points.min(*n);
            if let Some(wrecks) = self.destroyed_tanks.get_mut(hex) {
                if let Some(w) = wrecks
                    .iter_mut()
                    .find(|w| w.side == side && w.nation == nation && w.weapon == *weapon)
                {
                    w.points = w.points.saturating_sub(take);
                }
                wrecks.retain(|w| w.points > 0);
                if wrecks.is_empty() {
                    self.destroyed_tanks.remove(hex);
                }
            }
            *n -= take;
            points -= take;
        }
        events.retain(|e| !matches!(e, Event::TanksDestroyed { points: 0, .. }));
    }

    /// A battalion or bigger of `pieces` entering `at` takes the enemy's
    /// destroyed tanks there when no enemy combat unit stands with them
    /// (14.52, R-014); they are the captor's from now on.
    pub(crate) fn capture_destroyed_tanks(
        &mut self,
        pieces: &[PieceId],
        at: &HexId,
        events: &mut Vec<Event>,
    ) {
        if !self.modules.destroyed_tanks {
            return;
        }
        let Some((side, by)) = self.captor(pieces) else {
            return;
        };
        let Some(wrecks) = self.destroyed_tanks.get(at) else {
            return;
        };
        if !wrecks.iter().any(|w| w.side != side) {
            return;
        }
        let guarded = self
            .pieces
            .values()
            .any(|p| p.side != side && p.hex() == Some(at) && p.kind == PieceKind::Combat);
        if guarded {
            return;
        }
        let wrecks = self.destroyed_tanks.get_mut(at).expect("listed");
        let mut taken = 0;
        for w in wrecks.iter_mut().filter(|w| w.side != side) {
            w.side = side;
            taken += w.points;
        }
        // A captured marker may meet one of the captor's own of the
        // same type: they are one marker now.
        let mut merged: Vec<Wreck> = Vec::new();
        for w in wrecks.drain(..) {
            match merged
                .iter_mut()
                .find(|m| m.side == w.side && m.nation == w.nation && m.weapon == w.weapon)
            {
                Some(m) => m.points += w.points,
                None => merged.push(w),
            }
        }
        *wrecks = merged;
        events.push(Event::DestroyedTanksCaptured {
            by,
            hex: at.clone(),
            points: taken,
        });
    }

    /// `side`'s destroyed `weapon` points at `hex`.
    fn destroyed_points(&self, side: Side, hex: &HexId, weapon: &str) -> u32 {
        self.destroyed_tanks
            .get(hex)
            .into_iter()
            .flatten()
            .filter(|w| w.side == side && w.weapon == weapon)
            .map(|w| w.points)
            .sum()
    }

    /// The fuel `side` may pre-pay repairs at `hex` with.
    fn fuel_in_range(&self, data: &Data, side: Side, hex: &HexId) -> u32 {
        self.repair_fuel(data, side, hex)
            .iter()
            .filter_map(|q| self.pieces[q].load.as_ref())
            .map(|l| l.fuel)
            .sum()
    }

    /// The destroyed-tank repairs the phasing player may still attempt
    /// this phase, in hex and then marker order.
    pub fn destroyed_tank_repairs(&self, data: &Data) -> Vec<DestroyedTankOffer> {
        let Some(side) = self.phasing() else {
            return Vec::new();
        };
        if !self.modules.destroyed_tanks || self.clock.phase != Phase::Repair {
            return Vec::new();
        }
        let mut out: Vec<DestroyedTankOffer> = Vec::new();
        for (hex, wrecks) in &self.destroyed_tanks {
            if !wrecks.iter().any(|w| w.side == side) {
                continue;
            }
            let Ok(kind) = self.facility_bar(data, side, hex) else {
                continue;
            };
            let afford = if self.modules.abstract_supply {
                self.fuel_in_range(data, side, hex) / FUEL_A_POINT
            } else {
                u32::MAX
            };
            for w in wrecks.iter().filter(|w| w.side == side) {
                if afford == 0
                    || self
                        .destroyed_rolled
                        .contains(&(side, hex.clone(), w.weapon.clone()))
                    || out.iter().any(|o| &o.hex == hex && o.weapon == w.weapon)
                {
                    continue;
                }
                out.push(DestroyedTankOffer {
                    hex: hex.clone(),
                    weapon: w.weapon.clone(),
                    points: self.destroyed_points(side, hex, &w.weapon).min(afford),
                    kind,
                });
            }
        }
        out
    }

    /// `Order::RepairDestroyedTanks`: `points` of the phasing player's
    /// destroyed `weapon` tanks at `hex`, one of its repair facilities,
    /// roll a die each on the Destroyed Tank Repair Table, one added at a
    /// temporary (minor) facility (22.4x); under abstract supply two fuel
    /// a point are paid first, whatever the result.
    pub(crate) fn repair_destroyed_tanks(
        &mut self,
        data: &Data,
        player: Side,
        hex: &HexId,
        weapon: &str,
        points: u32,
        fuel_from: Option<&PieceId>,
    ) -> Result<Vec<Event>, Illegal> {
        if !self.modules.destroyed_tanks || self.clock.phase != Phase::Repair {
            return Err(Illegal::new(
                "22.4",
                format!("destroyed tanks are repaired in phase K, not at {}", self.clock),
            ));
        }
        if self.phasing() != Some(player) {
            return Err(Illegal::new("22.11", "only the phasing player repairs"));
        }
        let kind = self.facility_bar(data, player, hex)?;
        let key = (player, hex.clone(), weapon.to_string());
        if self.destroyed_rolled.contains(&key) {
            return Err(Illegal::new(
                "22.4",
                format!("destroyed {weapon} at {hex} have been tried this phase"),
            ));
        }
        let have = self.destroyed_points(player, hex, weapon);
        if points == 0 || points > have {
            return Err(Illegal::new(
                "22.4",
                format!("{have} destroyed {weapon} of yours at {hex} may be tried, not {points}"),
            ));
        }
        if self.modules.abstract_supply
            && self.fuel_in_range(data, player, hex) < points * FUEL_A_POINT
        {
            return Err(Illegal::new(
                "22.4",
                format!(
                    "{} fuel is pre-paid for {points} destroyed tanks; {hex} has {} in range",
                    points * FUEL_A_POINT,
                    self.fuel_in_range(data, player, hex)
                ),
            ));
        }
        let (_, paid_by) =
            self.pay_repair_fuel(data, player, hex, points * FUEL_A_POINT, fuel_from)?;
        self.destroyed_rolled.insert(key);
        let nation = self
            .destroyed_tanks
            .get(hex)
            .into_iter()
            .flatten()
            .find(|w| w.side == player && w.weapon == weapon)
            .and_then(|w| w.nation);
        let plus = u8::from(kind == FacilityKind::Temporary);
        let mut dice = Vec::new();
        let (mut repaired, mut junked) = (0, 0);
        for _ in 0..points {
            let die = self.rng.d6();
            dice.push(die);
            let row = data
                .destroyed_tank_repair
                .row(die + plus)
                .expect("destroyed-tank-repair.json has a row for 1 to 7");
            let outcome = match (player, nation) {
                (Side::Cw, _) => row.cw_facility,
                (Side::Axis, Some(Nation::De)) => row.axis_facility_german,
                (Side::Axis, Some(Nation::It)) => row.axis_facility_italian,
                (Side::Axis, _) => row.axis_facility_commonwealth,
            };
            match outcome {
                Some(TankOutcome::Repaired) => repaired += 1,
                Some(TankOutcome::Junked) => junked += 1,
                None => {}
            }
        }
        self.take_destroyed(hex, player, weapon, repaired + junked);
        // A captured enemy tank repaired is a captured tank point (R-152),
        // which the side's own tank units absorb; under its own model's
        // name no unit of the side could.
        let captured = match player {
            Side::Cw => matches!(nation, Some(Nation::De | Nation::It)),
            Side::Axis => nation == Some(Nation::Cw),
        };
        let key = if captured && self.modules.captured_equipment {
            crate::captured::Spoil::Tank.key()
        } else {
            weapon
        };
        if repaired > 0 {
            *self
                .repaired
                .entry(player)
                .or_default()
                .entry(hex.clone())
                .or_default()
                .entry(key.to_string())
                .or_default() += repaired;
        }
        Ok(vec![Event::DestroyedTanksRepaired {
            hex: hex.clone(),
            weapon: weapon.to_string(),
            kind,
            attempted: points,
            dice,
            repaired,
            junked,
            fuel_from: paid_by,
        }])
    }

    /// Removes `n` of `side`'s destroyed `weapon` points from `hex`,
    /// dropping emptied markers and an emptied hex.
    fn take_destroyed(&mut self, hex: &HexId, side: Side, weapon: &str, mut n: u32) {
        let Some(wrecks) = self.destroyed_tanks.get_mut(hex) else {
            return;
        };
        for w in wrecks
            .iter_mut()
            .filter(|w| w.side == side && w.weapon == weapon)
        {
            let k = n.min(w.points);
            w.points -= k;
            n -= k;
        }
        wrecks.retain(|w| w.points > 0);
        if wrecks.is_empty() {
            self.destroyed_tanks.remove(hex);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Half;
    use crate::order::Order;
    use crate::piece::Location;
    use crate::state::Modules;
    use cna_data::characteristics::Equivalent;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn marker(side: Side, nation: Nation, weapon: &str, points: u32) -> Wreck {
        Wreck {
            side,
            nation: Some(nation),
            weapon: weapon.into(),
            points,
            this_stage: 0,
            towed: 0,
        }
    }

    fn placed(data: &Data, modules: Modules) -> GameState {
        let mut state = GameState::with_modules(data, 5, modules).unwrap();
        crate::state::test_support::auto_place(data, &mut state);
        state
    }

    fn first(state: &GameState, side: Side, size: Option<Equivalent>) -> PieceId {
        state
            .pieces
            .values()
            .find(|p| p.side == side && p.kind == PieceKind::Combat && p.equivalent == size)
            .map(|p| p.id.clone())
            .expect("a unit of that size")
    }

    /// A clear hex holding no piece, for markers.
    fn empty_hex(data: &Data, state: &GameState) -> HexId {
        let h = HexId::parse("D3020").unwrap();
        assert!(state.pieces.values().all(|p| p.hex() != Some(&h)));
        let _ = data;
        h
    }

    #[test]
    fn a_battalion_entering_takes_unguarded_destroyed_tanks_14_52_r014() {
        let data = graziani();
        let mut state = placed(&data, Modules::ALL);
        let at = empty_hex(&data, &state);
        state
            .destroyed_tanks
            .insert(at.clone(), vec![marker(Side::Cw, Nation::Cw, "Matilda", 3)]);
        let bn = first(&state, Side::Axis, Some(Equivalent::Battalion));
        state.put(&bn, Location::Hex(at.clone()));
        let mut events = Vec::new();
        state.capture_destroyed_tanks(std::slice::from_ref(&bn), &at, &mut events);
        assert_eq!(
            events,
            vec![Event::DestroyedTanksCaptured {
                by: bn,
                hex: at.clone(),
                points: 3
            }]
        );
        let m = &state.destroyed_tanks[&at][0];
        assert_eq!((m.side, m.nation, m.points), (Side::Axis, Some(Nation::Cw), 3));
    }

    #[test]
    fn a_company_cannot_capture_destroyed_tanks_r014() {
        let data = graziani();
        let mut state = placed(&data, Modules::ALL);
        let at = empty_hex(&data, &state);
        state
            .destroyed_tanks
            .insert(at.clone(), vec![marker(Side::Cw, Nation::Cw, "Matilda", 3)]);
        let coy = first(&state, Side::Axis, Some(Equivalent::Company));
        state.put(&coy, Location::Hex(at.clone()));
        let mut events = Vec::new();
        state.capture_destroyed_tanks(&[coy], &at, &mut events);
        assert!(events.is_empty());
        assert_eq!(state.destroyed_tanks[&at][0].side, Side::Cw);
    }

    #[test]
    fn a_friendly_combat_unit_guards_its_destroyed_tanks_14_52() {
        let data = graziani();
        let mut state = placed(&data, Modules::ALL);
        let at = empty_hex(&data, &state);
        state
            .destroyed_tanks
            .insert(at.clone(), vec![marker(Side::Cw, Nation::Cw, "Matilda", 3)]);
        let guard = first(&state, Side::Cw, Some(Equivalent::Company));
        state.put(&guard, Location::Hex(at.clone()));
        let bn = first(&state, Side::Axis, Some(Equivalent::Battalion));
        state.put(&bn, Location::Hex(at.clone()));
        let mut events = Vec::new();
        state.capture_destroyed_tanks(&[bn], &at, &mut events);
        assert!(events.is_empty());
        assert_eq!(state.destroyed_tanks[&at][0].side, Side::Cw);
    }

    #[test]
    fn tanks_taken_in_the_captured_share_leave_no_marker_15_89_r152() {
        let data = graziani();
        let mut state = placed(&data, Modules::ALL);
        let id = first(&state, Side::Cw, Some(Equivalent::Battalion));
        let at = state.pieces[&id].hex().cloned().unwrap();
        state.destroyed_tanks.remove(&at);
        let mut events = Vec::new();
        for _ in 0..3 {
            state.mark_destroyed_tank(&id, Some("Matilda"), &mut events);
        }
        let left = |s: &GameState| s.destroyed_tanks.get(&at).map(|w| w[0].points);
        assert_eq!(left(&state), Some(3));
        // The other side's share touches none of them.
        state.unmark_captured_tanks(Side::Axis, 5, &mut events);
        assert_eq!(left(&state), Some(3));
        state.unmark_captured_tanks(Side::Cw, 2, &mut events);
        assert_eq!(left(&state), Some(1));
        assert!(matches!(events[..], [Event::TanksDestroyed { points: 1, .. }]));
        // More taken than marked: the marker and its event are gone.
        state.unmark_captured_tanks(Side::Cw, 4, &mut events);
        assert_eq!(left(&state), None);
        assert!(events.is_empty());
    }

    /// Graziani in the Commonwealth's phase K with `points` destroyed
    /// Matildas of its own in `hex`; no abstract supply.
    fn at_facility(data: &Data, hex: &str, points: u32) -> (GameState, HexId) {
        let modules = Modules {
            destroyed_tanks: true,
            ..Modules::default()
        };
        at_facility_with(data, modules, hex, points)
    }

    fn at_facility_with(
        data: &Data,
        modules: Modules,
        hex: &str,
        points: u32,
    ) -> (GameState, HexId) {
        let mut state = placed(data, modules);
        state.player_a = Some(Side::Cw);
        state.clock.phase = Phase::Repair;
        state.clock.half = Some(Half::A);
        let hex = HexId::parse(hex).unwrap();
        state.destroyed_tanks.insert(
            hex.clone(),
            vec![marker(Side::Cw, Nation::Cw, "Matilda", points)],
        );
        (state, hex)
    }

    fn order(hex: &HexId, points: u32) -> Order {
        Order::RepairDestroyedTanks {
            hex: hex.clone(),
            weapon: "Matilda".into(),
            points,
            fuel_from: None,
        }
    }

    /// The outcomes `dice` give on the Commonwealth facility column with
    /// `plus` added: (repaired, junked).
    fn read(data: &Data, dice: &[u8], plus: u8) -> (u32, u32) {
        let mut out = (0, 0);
        for d in dice {
            match data.destroyed_tank_repair.row(d + plus).unwrap().cw_facility {
                Some(TankOutcome::Repaired) => out.0 += 1,
                Some(TankOutcome::Junked) => out.1 += 1,
                None => {}
            }
        }
        out
    }

    #[test]
    fn a_major_facility_rolls_a_die_a_point_on_the_destroyed_tank_table_22_4() {
        let data = graziani();
        let (mut state, hex) = at_facility(&data, "E3613", 6);
        assert_eq!(
            state.destroyed_tank_repairs(&data),
            vec![DestroyedTankOffer {
                hex: hex.clone(),
                weapon: "Matilda".into(),
                points: 6,
                kind: FacilityKind::Major,
            }]
        );
        let events = state.apply(&data, Side::Cw, order(&hex, 5)).unwrap();
        let Event::DestroyedTanksRepaired {
            ref dice,
            attempted,
            repaired,
            junked,
            kind,
            ..
        } = events[0]
        else {
            panic!("{events:?}");
        };
        assert_eq!((attempted, kind, dice.len()), (5, FacilityKind::Major, 5));
        assert_eq!((repaired, junked), read(&data, dice, 0));
        let left = state
            .destroyed_tanks
            .get(&hex)
            .map_or(0, |m| m.iter().map(|w| w.points).sum::<u32>());
        assert_eq!(left, 6 - repaired - junked);
        let waiting = state
            .repaired
            .get(&Side::Cw)
            .and_then(|h| h.get(&hex))
            .and_then(|w| w.get("Matilda"))
            .copied()
            .unwrap_or(0);
        assert_eq!(waiting, repaired);
        // One attempt per type per phase K.
        assert!(state.destroyed_tank_repairs(&data).is_empty());
        if left > 0 {
            let e = state.apply(&data, Side::Cw, order(&hex, 1)).unwrap_err();
            assert_eq!(e.case, "22.4");
        }
    }

    #[test]
    fn a_temporary_facility_adds_one_to_the_die_22_44() {
        let data = graziani();
        let (mut state, hex) = at_facility(&data, "D3714", 6);
        let events = state.apply(&data, Side::Cw, order(&hex, 6)).unwrap();
        let Event::DestroyedTanksRepaired {
            ref dice,
            repaired,
            junked,
            kind,
            ..
        } = events[0]
        else {
            panic!("{events:?}");
        };
        assert_eq!(kind, FacilityKind::Temporary);
        assert_eq!((repaired, junked), read(&data, dice, 1));
    }

    #[test]
    fn destroyed_tanks_outside_a_facility_are_not_repaired_22_4() {
        let data = graziani();
        let (mut state, _) = at_facility(&data, "E3613", 2);
        let at = empty_hex(&data, &state);
        let m = state.destroyed_tanks.remove(&HexId::parse("E3613").unwrap());
        state.destroyed_tanks.insert(at.clone(), m.unwrap());
        assert!(state.destroyed_tank_repairs(&data).is_empty());
        let e = state.apply(&data, Side::Cw, order(&at, 1)).unwrap_err();
        assert_eq!(e.case, "22.31");
    }

    #[test]
    fn a_captured_matilda_repaired_by_the_axis_is_a_captured_tank_point_r152() {
        let data = graziani();
        let modules = Modules {
            destroyed_tanks: true,
            captured_equipment: true,
            ..Modules::default()
        };
        let (mut state, _) = at_facility_with(&data, modules, "E3613", 1);
        state.destroyed_tanks.clear();
        state.player_a = Some(Side::Axis);
        let hex = HexId::parse("C4807").unwrap();
        state
            .destroyed_tanks
            .insert(hex.clone(), vec![marker(Side::Axis, Nation::Cw, "Matilda", 6)]);
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::RepairDestroyedTanks {
                    hex: hex.clone(),
                    weapon: "Matilda".into(),
                    points: 6,
                    fuel_from: None,
                },
            )
            .unwrap();
        let Event::DestroyedTanksRepaired { repaired, .. } = events[0] else {
            panic!("{events:?}");
        };
        let waiting = state
            .repaired
            .get(&Side::Axis)
            .and_then(|h| h.get(&hex))
            .cloned()
            .unwrap_or_default();
        assert!(repaired > 0, "seed 5 repairs some of six");
        assert_eq!(
            waiting,
            std::collections::BTreeMap::from([("captured tank".to_string(), repaired)])
        );
    }

    #[test]
    fn an_axis_facility_reads_the_italian_column_for_italian_tanks_22_44() {
        let data = graziani();
        let (mut state, _) = at_facility(&data, "E3613", 1);
        state.destroyed_tanks.clear();
        state.player_a = Some(Side::Axis);
        let hex = HexId::parse("C4807").unwrap();
        state
            .destroyed_tanks
            .insert(hex.clone(), vec![marker(Side::Axis, Nation::It, "M11/39", 6)]);
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::RepairDestroyedTanks {
                    hex: hex.clone(),
                    weapon: "M11/39".into(),
                    points: 6,
                    fuel_from: None,
                },
            )
            .unwrap();
        let Event::DestroyedTanksRepaired {
            ref dice,
            repaired,
            junked,
            ..
        } = events[0]
        else {
            panic!("{events:?}");
        };
        let mut want = (0, 0);
        for d in dice {
            match data.destroyed_tank_repair.row(d + 1).unwrap().axis_facility_italian {
                Some(TankOutcome::Repaired) => want.0 += 1,
                Some(TankOutcome::Junked) => want.1 += 1,
                None => {}
            }
        }
        assert_eq!((repaired, junked), want);
    }

    #[test]
    fn with_abstract_supply_two_fuel_a_point_are_pre_paid_22_4() {
        let data = graziani();
        let (mut state, hex) = at_facility_with(&data, Modules::ALL, "E3613", 4);
        let source = state
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::SupplyUnit)
            .map(|p| p.id.clone())
            .expect("a Commonwealth supply unit");
        state.put(&source, Location::Hex(hex.clone()));
        for q in state.repair_fuel(&data, Side::Cw, &hex) {
            state.pieces.get_mut(&q).unwrap().load.as_mut().unwrap().fuel = 0;
        }
        state.pieces.get_mut(&source).unwrap().load.as_mut().unwrap().fuel = 5;
        // Five fuel pre-pays two points, not three.
        assert_eq!(state.destroyed_tank_repairs(&data)[0].points, 2);
        let e = state.apply(&data, Side::Cw, order(&hex, 3)).unwrap_err();
        assert_eq!(e.case, "22.4");
        let events = state.apply(&data, Side::Cw, order(&hex, 2)).unwrap();
        assert!(matches!(
            events[0],
            Event::DestroyedTanksRepaired { attempted: 2, .. }
        ));
        assert_eq!(state.pieces[&source].load.unwrap().fuel, 1);
    }
}
