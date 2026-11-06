//! Phase K, facility repair (SPI §22.3x) with the `breakdown` module on:
//! the phasing player rolls once per broken-down vehicle type in one of
//! its repair facilities, for the points that began the OpStage there,
//! on the Vehicle Repair Table's temporary or major column; with
//! abstract supply each point attempted costs a fuel, paid first. What
//! is repaired waits as replacement points (22.5), as field repair's
//! does.
//!
//! The facilities are the scenario's (R-099); a side whose set-up lists
//! none has 22.31's: Alexandria and Cairo for the Commonwealth, the
//! Tripoli box for the Axis, Tobruk for whoever holds it. Wrecks stand
//! only in map hexes, so the Tripoli box repairs nothing yet.

use crate::breakdown::Wreck;
use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::{HexId, PlaceKind};
use cna_data::oa::UnitId;
use cna_data::scenario::{FacilityKind, Locator, Side};
use cna_rules::movement::Weather;
use cna_rules::movement::{Mobility, Step, step_cost};
use std::collections::BTreeMap;

/// What a tow may spend in a repair phase: a medium truck's 10 CP (21.6x).
const TOW_CP: Cp = Cp::whole(10);

/// One facility-repair roll the phasing player may make now.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FacilityOffer {
    pub hex: HexId,
    pub weapon: String,
    /// Points that began the OpStage in the hex: what the roll attempts
    /// before fuel runs short.
    pub points: u32,
    pub kind: FacilityKind,
}

impl GameState {
    /// `side`'s repair facilities in map hexes (22.31, R-099): its
    /// scenario's list, else 22.31's. A hex listed twice keeps the major.
    /// Placements the player chooses (a village in a region) are not
    /// resolved.
    pub fn repair_facilities(&self, data: &Data, side: Side) -> BTreeMap<HexId, FacilityKind> {
        let mut out: BTreeMap<HexId, FacilityKind> = BTreeMap::new();
        let mut add = |hex: &HexId, kind: FacilityKind| {
            let k = out.entry(hex.clone()).or_insert(kind);
            *k = (*k).max(kind);
        };
        let place_hexes = |id: &str| -> Vec<HexId> {
            data.map
                .place(id)
                .map(|p| p.hexes.clone())
                .unwrap_or_default()
        };
        let listed = &data.scenario.side(side).repair;
        if listed.is_empty() {
            if side == Side::Cw {
                for id in ["alexandria-east", "alexandria-west", "cairo"] {
                    for h in place_hexes(id) {
                        add(&h, FacilityKind::Major);
                    }
                }
            }
            // Tobruk serves its controller: here, a side with a unit in
            // it and no enemy unit there.
            for h in place_hexes("tobruk") {
                let held_by = |s: Side| {
                    self.pieces.values().any(|p| {
                        p.side == s && p.hex() == Some(&h) && p.kind != PieceKind::DummySupplyUnit
                    })
                };
                if held_by(side) && !held_by(side.other()) {
                    add(&h, FacilityKind::Major);
                }
            }
            return out;
        }
        for block in listed {
            match block.placement.sole() {
                Some(Locator::Hexes(hexes)) => {
                    for h in hexes {
                        add(h, block.kind);
                    }
                }
                Some(Locator::Place(ids)) => {
                    for id in ids {
                        for h in place_hexes(id) {
                            add(&h, block.kind);
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Why `side` may not repair at a facility in `hex`, if it may not:
    /// no facility of its own there (22.31), or a temporary one under an
    /// enemy ZOC (outside a major city) or a storm (22.13).
    pub(crate) fn facility_bar(
        &self,
        data: &Data,
        side: Side,
        hex: &HexId,
    ) -> Result<FacilityKind, Illegal> {
        if let Some(bar) = self.repair_hold_bar(hex) {
            return Err(bar);
        }
        let Some(kind) = self.repair_facilities(data, side).get(hex).copied() else {
            return Err(Illegal::new(
                "22.31",
                format!("no repair facility of yours at {hex}"),
            ));
        };
        if kind == FacilityKind::Major {
            return Ok(kind);
        }
        let major_city = data
            .map
            .places()
            .any(|p| p.kind == PlaceKind::MajorCity && p.hexes.contains(hex));
        if !major_city && !matches!(self.zoc_at(data, hex, side), Zoc::Clear) {
            return Err(Illegal::new(
                "22.13",
                format!("{hex} is in an enemy ZOC: its temporary facility stops"),
            ));
        }
        if self.weather_at(hex) != Weather::Normal {
            return Err(Illegal::new(
                "22.13",
                format!("a storm at {hex} stops its temporary facility"),
            ));
        }
        Ok(kind)
    }

    /// `side`'s broken-down `weapon` points in `hex` that began the
    /// OpStage there.
    fn settled_points(&self, side: Side, hex: &HexId, weapon: &str) -> u32 {
        self.broken_down
            .get(hex)
            .into_iter()
            .flatten()
            .filter(|w| w.side == side && w.weapon == weapon)
            .map(|w| w.points - w.this_stage)
            .sum()
    }

    /// The facility rolls the phasing player may still make this phase,
    /// in hex and then wreck order.
    pub fn facility_repairs(&self, data: &Data) -> Vec<FacilityOffer> {
        let Some(side) = self.phasing() else {
            return Vec::new();
        };
        if !self.modules.breakdown || self.clock.phase != Phase::Repair {
            return Vec::new();
        }
        let mut out: Vec<FacilityOffer> = Vec::new();
        for (hex, wrecks) in &self.broken_down {
            let Ok(kind) = self.facility_bar(data, side, hex) else {
                continue;
            };
            if self.modules.abstract_supply && self.repair_fuel(data, side, hex).is_empty() {
                continue;
            }
            for w in wrecks.iter().filter(|w| w.side == side) {
                let key = (side, hex.clone(), w.weapon.clone());
                if self.field_rolled.contains(&key)
                    || out.iter().any(|o| &o.hex == hex && o.weapon == w.weapon)
                {
                    continue;
                }
                let points = self.settled_points(side, hex, &w.weapon);
                if points > 0 {
                    out.push(FacilityOffer {
                        hex: hex.clone(),
                        weapon: w.weapon.clone(),
                        points,
                        kind,
                    });
                }
            }
        }
        out
    }

    /// `Order::FacilityRepair`: one facility roll for the phasing
    /// player's `weapon` in `hex` (22.3x), on the facility's column, for
    /// the points that began the OpStage there; fuel as for field repair.
    pub(crate) fn facility_repair(
        &mut self,
        data: &Data,
        player: Side,
        hex: &HexId,
        weapon: &str,
        fuel_from: Option<&PieceId>,
        unit: Option<&UnitId>,
    ) -> Result<Vec<Event>, Illegal> {
        if !self.modules.breakdown || self.clock.phase != Phase::Repair {
            return Err(Illegal::new(
                "22.11",
                format!("vehicles are repaired in phase K, not at {}", self.clock),
            ));
        }
        if self.phasing() != Some(player) {
            return Err(Illegal::new("22.11", "only the phasing player repairs"));
        }
        let kind = self.facility_bar(data, player, hex)?;
        let key = (player, hex.clone(), weapon.to_string());
        if self.field_rolled.contains(&key) {
            return Err(Illegal::new(
                "22.23",
                format!("{weapon} at {hex} has had its roll this phase"),
            ));
        }
        let points = self.settled_points(player, hex, weapon);
        if points == 0 {
            return Err(Illegal::new(
                "22.31",
                format!("no broken-down {weapon} of yours began this OpStage at {hex}"),
            ));
        }
        if let Some(u) = unit {
            self.wreck_owner_bar(hex, player, weapon, u, true)?;
        }
        let (attempted, paid_by) = self.pay_repair_fuel(data, player, hex, points, fuel_from)?;
        self.field_rolled.insert(key);
        let die = self.rng.d6();
        let row = data
            .vehicle_repair
            .row(die)
            .expect("vehicle-repair.json has a row for every d6 roll");
        let (column, percent) = match kind {
            FacilityKind::Temporary => {
                ("temporary_facility_percent", row.temporary_facility_percent)
            }
            FacilityKind::Major => ("major_facility_percent", row.major_facility_percent),
        };
        let percent = percent.expect("vehicle-repair.json has facility columns for every roll");
        // A starred 10 % is ignored for a single point (22.8).
        let starred = row
            .single_point_ignores_ten_percent
            .iter()
            .any(|c| c == column);
        let repaired = if attempted == 1 && starred {
            0
        } else {
            (attempted * percent).div_ceil(100)
        };
        let taken = self.take_wrecks(hex, player, weapon, repaired, true, unit);
        self.credit_repaired(player, hex, weapon, taken);
        Ok(vec![Event::FacilityRepaired {
            hex: hex.clone(),
            weapon: weapon.to_string(),
            kind,
            attempted,
            die,
            repaired,
            fuel_from: paid_by,
        }])
    }

    /// Whether the phasing player has broken-down points that might be
    /// towed: not yet towed, away from a repair facility and in no
    /// enemy ZOC a friendly combat unit fails to cancel (21.6x). Whether
    /// a path exists is left to `Order::Tow`.
    pub(crate) fn tows_to_try(&self, data: &Data) -> bool {
        let Some(player) = self.phasing() else {
            return false;
        };
        let facilities = self.repair_facilities(data, player);
        !facilities.is_empty()
            && self.broken_down.iter().any(|(hex, wrecks)| {
                !facilities.contains_key(hex)
                    && wrecks
                        .iter()
                        .any(|w| w.side == player && w.points > w.towed)
                    && (self.zoc_at(data, hex, player) == Zoc::Clear
                        || self.friendly_combat_in(hex, player, &[]))
            })
    }

    /// `Order::Tow`: the phasing player's broken-down `weapon` points at
    /// `hex` move along `path` as a medium truck on 10 CP, ending nearer
    /// the repair facility nearest `hex`; not out of an enemy ZOC no
    /// friendly unit cancels, nor through one, nor after a repair roll
    /// on them this phase. They arrive as not having begun the OpStage
    /// there and are not repaired this phase (21.6x, 22.13).
    pub(crate) fn tow(
        &mut self,
        data: &Data,
        player: Side,
        hex: &HexId,
        weapon: &str,
        points: u32,
        path: &[HexId],
    ) -> Result<Vec<Event>, Illegal> {
        if !self.modules.breakdown || self.clock.phase != Phase::Repair {
            return Err(Illegal::new(
                "21.6x",
                format!("vehicles are towed in phase K, not at {}", self.clock),
            ));
        }
        if self.phasing() != Some(player) {
            return Err(Illegal::new("21.6x", "only the phasing player tows"));
        }
        if self
            .field_rolled
            .contains(&(player, hex.clone(), weapon.to_string()))
        {
            return Err(Illegal::new(
                "21.6x",
                format!("{weapon} at {hex} had a repair roll this phase"),
            ));
        }
        let free: u32 = self
            .broken_down
            .get(hex)
            .into_iter()
            .flatten()
            .filter(|w| w.side == player && w.weapon == weapon)
            .map(|w| w.points - w.towed)
            .sum();
        if points == 0 || points > free {
            return Err(Illegal::new(
                "21.6x",
                format!("{free} broken-down {weapon} of yours at {hex} may be towed, not {points}"),
            ));
        }
        let Some(to) = path.last() else {
            return Err(Illegal::new("21.6x", "an empty path"));
        };
        let in_zoc = |state: &GameState, h: &HexId| {
            state.zoc_at(data, h, player) != Zoc::Clear && !state.friendly_combat_in(h, player, &[])
        };
        if in_zoc(self, hex) {
            return Err(Illegal::new(
                "21.6x",
                format!("{hex} is enemy-controlled: nothing is towed out of it"),
            ));
        }
        let mut cost = Cp::ZERO;
        let mut at = hex;
        for (i, next) in path.iter().enumerate() {
            if !data.map.adjacent(at, next) {
                return Err(Illegal::new("21.6x", format!("{next} is not next to {at}")));
            }
            let built = self.hexside(data, at, next);
            let hexside = built.as_deref();
            let step = Step {
                from: data.map.hex(at).expect("on the map"),
                to: data.map.hex(next).expect("on the map"),
                hexside,
                climbs: hexside.is_some_and(|hs| hs.climbs(at, next)),
                weather: self.weather_at(next),
            };
            cost += step_cost(&data.terrain, Mobility::Vehicle, &step).map_err(|p| {
                Illegal::new(
                    p.case,
                    format!("a tow may not enter {next} from {at}: {}", p.reason),
                )
            })?;
            if i + 1 < path.len() && in_zoc(self, next) {
                return Err(Illegal::new(
                    "21.6x",
                    format!("{next} is in an enemy ZOC: the tow ends there"),
                ));
            }
            at = next;
        }
        if cost > TOW_CP {
            return Err(Illegal::new(
                "21.6x",
                format!("the tow costs {cost} CP, more than {TOW_CP}"),
            ));
        }
        // Toward the nearest facility: closer to one of those nearest
        // `hex` than `hex` is.
        let facilities: Vec<HexId> = self.repair_facilities(data, player).into_keys().collect();
        let limit = 200;
        let near = facilities
            .iter()
            .map(|f| GameState::distance(data, hex, f, limit))
            .min();
        let toward = near.is_some_and(|d| {
            facilities.iter().any(|f| {
                GameState::distance(data, hex, f, limit) == d
                    && GameState::distance(data, to, f, limit) < d
            })
        });
        if !toward {
            return Err(Illegal::new(
                "21.6x",
                format!("{to} is no nearer the repair facility nearest {hex}"),
            ));
        }
        // Move the points, keeping each wreck's nation.
        let mut moved: Vec<(Option<cna_data::oa::Nation>, Option<cna_data::oa::UnitId>, u32)> =
            Vec::new();
        let mut left = points;
        let wrecks = self.broken_down.get_mut(hex).expect("checked above");
        for w in wrecks
            .iter_mut()
            .filter(|w| w.side == player && w.weapon == weapon)
        {
            let k = left.min(w.points - w.towed);
            if k == 0 {
                continue;
            }
            w.points -= k;
            w.this_stage = w.this_stage.min(w.points);
            left -= k;
            moved.push((w.nation, w.unit.clone(), k));
        }
        wrecks.retain(|w| w.points > 0);
        if wrecks.is_empty() {
            self.broken_down.remove(hex);
        }
        let dest = self.broken_down.entry(to.clone()).or_default();
        for (nation, unit, k) in moved {
            match dest.iter_mut().find(|w| {
                w.side == player && w.weapon == weapon && w.nation == nation && w.unit == unit
            })
            {
                Some(w) => {
                    w.points += k;
                    w.this_stage += k;
                    w.towed += k;
                }
                None => dest.push(Wreck {
                    side: player,
                    nation,
                    weapon: weapon.to_string(),
                    points: k,
                    this_stage: k,
                    towed: k,
                    unit,
                }),
            }
        }
        Ok(vec![Event::Towed {
            side: player,
            from: hex.clone(),
            to: to.clone(),
            weapon: weapon.to_string(),
            points,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Half;
    use crate::order::Order;
    use crate::piece::Location;
    use crate::state::Modules;

    /// Graziani in the Commonwealth's phase K (the Commonwealth Player
    /// A), with four broken-down Matilda points that began the OpStage in
    /// `hex`.
    fn at_facility(data: &Data, modules: Modules, hex: &str) -> (GameState, HexId) {
        let mut state = GameState::with_modules(data, 5, modules).unwrap();
        crate::state::test_support::auto_place(data, &mut state);
        state.player_a = Some(Side::Cw);
        state.clock.phase = Phase::Repair;
        state.clock.half = Some(Half::A);
        let hex = HexId::parse(hex).unwrap();
        state.broken_down.insert(
            hex.clone(),
            vec![Wreck {
                side: Side::Cw,
                nation: None,
                weapon: "Matilda".into(),
                points: 4,
                this_stage: 0,
                towed: 0,
                unit: None,
            }],
        );
        (state, hex)
    }

    fn breakdown_only() -> Modules {
        Modules {
            breakdown: true,
            ..Modules::default()
        }
    }

    fn order(hex: &HexId) -> Order {
        Order::FacilityRepair {
            hex: hex.clone(),
            weapon: "Matilda".into(),
            fuel_from: None,
            unit: None,
        }
    }

    /// An Axis combat unit next to `hex`, so its ZOC covers it.
    fn enemy_beside(data: &Data, state: &mut GameState, hex: &HexId) {
        let axis: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .map(|p| p.id.clone())
            .collect();
        for next in data.map.neighbours(hex) {
            for id in &axis {
                let was = state.pieces[id].location.clone();
                state.put(id, Location::Hex(next.clone()));
                if state.zoc_at(data, hex, Side::Cw) != Zoc::Clear {
                    return;
                }
                state.put(id, was);
            }
        }
        panic!("no Axis unit casts a ZOC over {hex}");
    }

    #[test]
    fn graziani_s_facilities_are_its_own_list_r_099() {
        let data = Data::load("grazianis-offensive").unwrap();
        let state = GameState::with_modules(&data, 5, Modules::ALL).unwrap();
        let cw = state.repair_facilities(&data, Side::Cw);
        let h = |s: &str| HexId::parse(s).unwrap();
        assert_eq!(cw.get(&h("E3613")), Some(&FacilityKind::Major));
        assert_eq!(cw.get(&h("D3714")), Some(&FacilityKind::Temporary));
        // Graziani lists no Cairo facility: the scenario's list governs.
        assert_eq!(cw.get(&h("E1730")), None);
        let axis = state.repair_facilities(&data, Side::Axis);
        assert_eq!(axis.get(&h("C4807")), Some(&FacilityKind::Temporary));
        assert_eq!(axis.get(&h("A4827")), Some(&FacilityKind::Temporary));
    }

    #[test]
    fn a_side_listing_none_has_22_31_s_facilities_r_099() {
        let data = Data::load("rommels-arrival").unwrap();
        let state = GameState::new(&data, 5).unwrap();
        // Tobruk serves the Axis only while it holds Tobruk.
        let axis = state.repair_facilities(&data, Side::Axis);
        assert!(axis.keys().all(|h| h.to_string() == "C4807"), "{axis:?}");
    }

    #[test]
    fn alexandria_s_major_facility_repairs_a_matilda_in_a_sandstorm_22_3x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_facility(&data, breakdown_only(), "E3613");
        state.weather = crate::state::Weather::Sandstorm { sheets: vec!['E'] };
        enemy_beside(&data, &mut state, &hex);
        let offer = FacilityOffer {
            hex: hex.clone(),
            weapon: "Matilda".into(),
            points: 4,
            kind: FacilityKind::Major,
        };
        assert_eq!(state.facility_repairs(&data), vec![offer]);
        // No field repair here: no unit, and a storm.
        assert!(state.field_repairs(&data).is_empty());
        let events = state.apply(&data, Side::Cw, order(&hex)).unwrap();
        let Event::FacilityRepaired {
            die,
            repaired,
            attempted,
            kind,
            ..
        } = events[0]
        else {
            panic!("{events:?}");
        };
        assert_eq!((attempted, kind), (4, FacilityKind::Major));
        let pct = data
            .vehicle_repair
            .row(die)
            .unwrap()
            .major_facility_percent
            .unwrap();
        assert_eq!(repaired, (4 * pct).div_ceil(100));
        assert!(
            repaired >= 1,
            "the major column never returns nothing on 4 points"
        );
        let waiting = state.repaired[&Side::Cw][&hex]["Matilda"];
        assert_eq!(waiting, repaired);
        let left: u32 = state
            .broken_down
            .get(&hex)
            .into_iter()
            .flatten()
            .map(|w| w.points)
            .sum();
        assert_eq!(left, 4 - repaired);
        // One roll per type per phase.
        assert_eq!(
            state.apply(&data, Side::Cw, order(&hex)).unwrap_err().case,
            "22.23"
        );
    }

    #[test]
    fn mersa_matruh_s_temporary_facility_stops_under_an_enemy_zoc_or_a_storm_22_13() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_facility(&data, breakdown_only(), "D3714");
        assert_eq!(
            state.facility_repairs(&data)[0].kind,
            FacilityKind::Temporary
        );
        let mut zoc = state.clone();
        enemy_beside(&data, &mut zoc, &hex);
        assert!(zoc.facility_repairs(&data).is_empty());
        assert_eq!(
            zoc.apply(&data, Side::Cw, order(&hex)).unwrap_err().case,
            "22.13"
        );
        state.weather = crate::state::Weather::Rainstorm { sheets: vec!['D'] };
        assert!(state.facility_repairs(&data).is_empty());
        assert_eq!(
            state.apply(&data, Side::Cw, order(&hex)).unwrap_err().case,
            "22.13"
        );
    }

    #[test]
    fn a_vehicle_that_broke_down_this_opstage_waits_22_31() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_facility(&data, breakdown_only(), "E3613");
        state.broken_down.get_mut(&hex).unwrap()[0].this_stage = 4;
        assert!(state.facility_repairs(&data).is_empty());
        assert_eq!(
            state.apply(&data, Side::Cw, order(&hex)).unwrap_err().case,
            "22.31"
        );
        // Three of the four arrived this OpStage: only one is attempted.
        state.broken_down.get_mut(&hex).unwrap()[0].this_stage = 3;
        let events = state.apply(&data, Side::Cw, order(&hex)).unwrap();
        let Event::FacilityRepaired {
            attempted,
            repaired,
            ..
        } = events[0]
        else {
            panic!("{events:?}");
        };
        assert_eq!(attempted, 1);
        let w = &state.broken_down[&hex][0];
        assert_eq!((w.points, w.this_stage), (4 - repaired, 3));
        // The next OpStage they have all begun there.
        state.start_stage();
        assert_eq!(state.broken_down[&hex][0].this_stage, 0);
    }

    #[test]
    fn no_facility_repair_away_from_one_or_by_the_other_side_22_31() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_facility(&data, breakdown_only(), "E1730");
        assert!(state.facility_repairs(&data).is_empty());
        assert_eq!(
            state.apply(&data, Side::Cw, order(&hex)).unwrap_err().case,
            "22.31"
        );
        let (mut state, hex) = at_facility(&data, breakdown_only(), "E3613");
        assert!(state.apply(&data, Side::Axis, order(&hex)).is_err());
    }

    #[test]
    fn with_abstract_supply_each_point_costs_a_fuel_paid_first_22_3x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_facility(&data, Modules::ALL, "E3613");
        let source = state
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::SupplyUnit)
            .map(|p| p.id.clone())
            .expect("a Commonwealth supply unit");
        // No fuel in reach: nothing offered.
        if state.repair_fuel(&data, Side::Cw, &hex).is_empty() {
            assert!(state.facility_repairs(&data).is_empty());
        }
        state.put(&source, Location::Hex(hex.clone()));
        state
            .pieces
            .get_mut(&source)
            .unwrap()
            .load
            .as_mut()
            .unwrap()
            .fuel = 3;
        let fuel_before: u32 = state
            .repair_fuel(&data, Side::Cw, &hex)
            .iter()
            .map(|q| state.pieces[q].load.unwrap().fuel)
            .sum();
        let events = state.apply(&data, Side::Cw, order(&hex)).unwrap();
        let Event::FacilityRepaired {
            attempted,
            ref fuel_from,
            ..
        } = events[0]
        else {
            panic!("{events:?}");
        };
        assert_eq!(attempted, 4.min(fuel_before));
        assert!(fuel_from.is_some());
        let fuel_after: u32 = state
            .repair_fuel(&data, Side::Cw, &hex)
            .iter()
            .chain(std::iter::once(&source))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|q| state.pieces[q].load.unwrap().fuel)
            .sum();
        assert_eq!(fuel_before - fuel_after, attempted);
    }

    /// Four Matilda points two hexes from Mersa Matruh's temporary
    /// facility, and a clear hex between them to tow to.
    fn two_out(data: &Data) -> (GameState, HexId, HexId, HexId) {
        let facility = HexId::parse("D3714").unwrap();
        let (mut state, _) = at_facility(data, breakdown_only(), "D3714");
        let wrecks = state.broken_down.remove(&facility).unwrap();
        for far in data.map.within(&facility, 2) {
            if GameState::distance(data, &far, &facility, 2) != 2 {
                continue;
            }
            for near in data.map.neighbours(&far) {
                if !data.map.adjacent(&near, &facility) {
                    continue;
                }
                let mut s = state.clone();
                s.broken_down.insert(far.clone(), wrecks.clone());
                if s.clone()
                    .apply(data, Side::Cw, tow(&far, 1, &[&near]))
                    .is_ok()
                {
                    return (s, facility, far, near);
                }
            }
        }
        panic!("no hex two from {facility} tows one step in");
    }

    fn tow(hex: &HexId, points: u32, path: &[&HexId]) -> Order {
        Order::Tow {
            hex: hex.clone(),
            weapon: "Matilda".into(),
            points,
            path: path.iter().map(|h| (*h).clone()).collect(),
        }
    }

    #[test]
    fn phase_k_opens_for_a_tow_only_with_towing_on_21_6x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, facility, far, _) = two_out(&data);
        state.broken_down.retain(|h, _| *h == far);
        assert!(state.field_repairs(&data).is_empty());
        assert!(state.facility_repairs(&data).is_empty());
        // Old games: no roll to make, so phase K stays skipped.
        assert!(!state.repairs_to_try(&data));
        state.modules.towing = true;
        assert!(state.repairs_to_try(&data));
        // At the facility already, there is nothing to tow.
        let wrecks = state.broken_down.remove(&far).unwrap();
        state.broken_down.insert(facility, wrecks.clone());
        assert!(!state.tows_to_try(&data));
        // Nor out of an enemy ZOC no friendly unit cancels.
        state.broken_down.clear();
        state.broken_down.insert(far.clone(), wrecks);
        assert!(state.tows_to_try(&data));
        enemy_beside(&data, &mut state, &far);
        assert!(!state.friendly_combat_in(&far, Side::Cw, &[]));
        assert!(!state.tows_to_try(&data));
    }

    #[test]
    fn part_of_a_hex_s_wrecks_are_towed_in_and_wait_out_the_phase_21_6x_22_13() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, facility, far, near) = two_out(&data);
        let events = state
            .apply(&data, Side::Cw, tow(&far, 3, &[&near, &facility]))
            .unwrap();
        assert!(matches!(
            &events[..],
            [Event::Towed { from, to, points: 3, .. }] if *from == far && *to == facility
        ));
        // One point stays behind, a second marker (§32).
        assert_eq!(state.broken_down[&far][0].points, 1);
        let w = &state.broken_down[&facility][0];
        assert_eq!((w.points, w.this_stage, w.towed), (3, 3, 3));
        // Towed points are not repaired this phase, in the field or at
        // the facility (22.13), and are not towed again.
        assert!(state.facility_repairs(&data).is_empty());
        assert!(
            state
                .field_repairs(&data)
                .iter()
                .all(|(h, _, _)| *h != facility)
        );
        assert_eq!(
            state
                .clone()
                .apply(&data, Side::Cw, order(&facility))
                .unwrap_err()
                .case,
            "22.31"
        );
        assert!(
            state
                .clone()
                .apply(&data, Side::Cw, tow(&facility, 1, &[&near]))
                .is_err()
        );
        // Next OpStage they began it there and the facility may roll.
        state.start_stage();
        assert_eq!(state.facility_repairs(&data)[0].points, 3);
    }

    #[test]
    fn a_tow_goes_toward_the_nearest_facility_on_10_cp_21_6x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (state, facility, far, near) = two_out(&data);
        let away = data
            .map
            .neighbours(&far)
            .into_iter()
            .find(|h| GameState::distance(&data, h, &facility, 3) == 3)
            .unwrap();
        assert_eq!(
            state
                .clone()
                .apply(&data, Side::Cw, tow(&far, 1, &[&away]))
                .unwrap_err()
                .case,
            "21.6x"
        );
        // Back and forth past 10 CP: in, out and in again three times.
        let long: Vec<&HexId> = [&near, &far].repeat(6).into_iter().chain([&near]).collect();
        assert_eq!(
            state
                .clone()
                .apply(&data, Side::Cw, tow(&far, 1, &long))
                .unwrap_err()
                .case,
            "21.6x"
        );
        assert_eq!(
            state
                .clone()
                .apply(&data, Side::Cw, tow(&far, 5, &[&near]))
                .unwrap_err()
                .case,
            "21.6x"
        );
    }

    #[test]
    fn no_tow_after_a_repair_roll_or_out_of_an_enemy_zoc_21_6x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (state, _, far, near) = two_out(&data);
        // A field roll on them this phase, whatever it fixed (21.6x).
        let mut rolled = state.clone();
        rolled
            .field_rolled
            .insert((Side::Cw, far.clone(), "Matilda".into()));
        assert_eq!(
            rolled
                .apply(&data, Side::Cw, tow(&far, 1, &[&near]))
                .unwrap_err()
                .case,
            "21.6x"
        );
        let mut zoc = state.clone();
        enemy_beside(&data, &mut zoc, &far);
        assert_eq!(
            zoc.apply(&data, Side::Cw, tow(&far, 1, &[&near]))
                .unwrap_err()
                .case,
            "21.6x"
        );
    }
}
