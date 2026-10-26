//! Phase K, field repair (SPI §22.2x) with the `breakdown` module on:
//! the phasing player rolls once per broken-down vehicle type in a hex
//! shared with a friendly unit, paying a fuel per point attempted
//! first; what is repaired waits as replacement points (22.5).
//!
//! Towing (21.6x) and facility repair (22.3x) move only toward or at a
//! repair facility, so they come with the facilities in the
//! construction slice.

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use crate::zoc::Zoc;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::movement::Weather;

impl GameState {
    /// Why `side` may not try field repair in `hex` this phase, if it
    /// may not (22.1x, 22.2x): no friendly unit with the vehicles, an
    /// enemy ZOC over them (an undeclared one counts), or a storm.
    fn field_repair_bar(&self, data: &Data, side: Side, hex: &HexId) -> Option<Illegal> {
        if !self
            .pieces
            .values()
            .any(|p| p.side == side && p.hex() == Some(hex) && p.kind != PieceKind::DummySupplyUnit)
        {
            return Some(Illegal::new(
                "22.21",
                format!("no friendly unit shares {hex} with the vehicles"),
            ));
        }
        if !matches!(self.zoc_at(data, hex, side), Zoc::Clear) {
            return Some(Illegal::new("22.13", format!("{hex} is in an enemy ZOC")));
        }
        if self.weather_at(hex) != Weather::Normal {
            return Some(Illegal::new(
                "22.13",
                format!("no field repair under a storm at {hex}"),
            ));
        }
        None
    }

    /// The broken-down vehicle types the phasing player may still try in
    /// the field this phase: (hex, weapon system, points).
    pub fn field_repairs(&self, data: &Data) -> Vec<(HexId, String, u32)> {
        let Some(side) = self.phasing() else {
            return Vec::new();
        };
        if !self.modules.breakdown {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (hex, wrecks) in &self.broken_down {
            if self.field_repair_bar(data, side, hex).is_some() {
                continue;
            }
            for w in wrecks.iter().filter(|w| w.side == side) {
                let key = (side, hex.clone(), w.weapon.clone());
                if self.field_rolled.contains(&key)
                    || out.iter().any(|(h, n, _)| h == hex && *n == w.weapon)
                {
                    continue;
                }
                let points = wrecks
                    .iter()
                    .filter(|x| x.side == side && x.weapon == w.weapon)
                    .map(|x| x.points)
                    .sum();
                if self.modules.abstract_supply && self.repair_fuel(data, side, hex).is_empty() {
                    continue;
                }
                out.push((hex.clone(), w.weapon.clone(), points));
            }
        }
        out
    }

    /// Phase K waits on the phasing player only when it has something to
    /// try; otherwise it is skipped.
    pub(crate) fn repairs_to_try(&self, data: &Data) -> bool {
        !self.field_repairs(data).is_empty()
    }

    /// The real supply units with fuel that may pay for repairs in
    /// `hex`: in the hex, or in range of a friendly unit there (32.17),
    /// sorted by id.
    pub fn repair_fuel(&self, data: &Data, side: Side, hex: &HexId) -> Vec<PieceId> {
        let mut out: Vec<PieceId> = Vec::new();
        for p in self
            .pieces
            .values()
            .filter(|p| p.side == side && p.hex() == Some(hex))
        {
            if p.kind == PieceKind::SupplyUnit {
                out.push(p.id.clone());
            }
            out.extend(self.supply_in_range(data, &p.id));
        }
        out.sort();
        out.dedup();
        out.retain(|q| {
            let p = &self.pieces[q];
            p.kind == PieceKind::SupplyUnit && p.load.is_some_and(|l| l.fuel > 0)
        });
        out
    }

    /// `Order::Repair`: one field roll for the phasing player's `weapon`
    /// in `hex` (22.2x). With abstract supply each point attempted costs
    /// a fuel from `fuel_from` (else the first in range), paid before the
    /// roll; only paid points are attempted.
    pub(crate) fn field_repair(
        &mut self,
        data: &Data,
        player: Side,
        hex: &HexId,
        weapon: &str,
        fuel_from: Option<&PieceId>,
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
        if let Some(e) = self.field_repair_bar(data, player, hex) {
            return Err(e);
        }
        let key = (player, hex.clone(), weapon.to_string());
        if self.field_rolled.contains(&key) {
            return Err(Illegal::new(
                "22.23",
                format!("{weapon} at {hex} has had its roll this phase"),
            ));
        }
        let points: u32 = self
            .broken_down
            .get(hex)
            .into_iter()
            .flatten()
            .filter(|w| w.side == player && w.weapon == weapon)
            .map(|w| w.points)
            .sum();
        if points == 0 {
            return Err(Illegal::new(
                "22.21",
                format!("no broken-down {weapon} of yours at {hex}"),
            ));
        }
        let (attempted, paid_by) = if self.modules.abstract_supply {
            let mut sources = self.repair_fuel(data, player, hex);
            if let Some(f) = fuel_from {
                let Some(i) = sources.iter().position(|q| q == f) else {
                    return Err(Illegal::new(
                        "22.24",
                        format!("{f} cannot pay fuel for repairs at {hex}"),
                    ));
                };
                let first = sources.remove(i);
                sources.insert(0, first);
            }
            let Some(first) = sources.first().cloned() else {
                return Err(Illegal::new(
                    "22.24",
                    format!("no fuel in range of {hex} for repairs"),
                ));
            };
            let mut paid = 0;
            for q in &sources {
                let load = self
                    .pieces
                    .get_mut(q)
                    .expect("listed")
                    .load
                    .as_mut()
                    .expect("has fuel");
                let n = (points - paid).min(load.fuel);
                load.fuel -= n;
                paid += n;
                if paid == points {
                    break;
                }
            }
            (paid, Some(first))
        } else {
            (points, None)
        };
        self.field_rolled.insert(key);
        let die = self.rng.d6();
        let row = data
            .vehicle_repair
            .row(die)
            .expect("vehicle-repair.json has a row for every d6 roll");
        // Recce units' wrecks go by their class (R-125): armoured cars and
        // armoured recce, and trucks, as points; anything else is a tank
        // or gun model, as a percentage (22.8).
        let field =
            |n: Option<u32>| n.expect("vehicle-repair.json has a field column for every d6 roll");
        let repaired = match weapon {
            "armoured-car" | "armoured-recce" => field(row.field_armoured_car).min(attempted),
            "truck" => field(row.field_truck).min(attempted),
            _ => cna_rules::breakdown::losses(attempted, field(row.field_tank_percent)),
        };
        self.take_wrecks(hex, player, weapon, repaired);
        if repaired > 0 {
            *self
                .repaired
                .entry(player)
                .or_default()
                .entry(hex.clone())
                .or_default()
                .entry(weapon.to_string())
                .or_default() += repaired;
        }
        Ok(vec![Event::FieldRepaired {
            hex: hex.clone(),
            weapon: weapon.to_string(),
            attempted,
            die,
            repaired,
            fuel_from: paid_by,
        }])
    }

    /// Removes `n` of `side`'s broken-down `weapon` points from `hex`,
    /// dropping emptied entries and an emptied hex.
    fn take_wrecks(&mut self, hex: &HexId, side: Side, weapon: &str, mut n: u32) {
        let Some(wrecks) = self.broken_down.get_mut(hex) else {
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
            self.broken_down.remove(hex);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::breakdown::Wreck;
    use crate::clock::Half;
    use crate::order::Order;
    use crate::state::Modules;

    /// A module-on game in the Axis's phase K (the Axis Player A), with
    /// four broken-down M13/40 points beside an Axis combat unit that has
    /// fuel in range. Returns the hex.
    fn at_repair(data: &Data, modules: Modules) -> (GameState, HexId) {
        let mut state = GameState::with_modules(data, 5, modules).unwrap();
        crate::state::test_support::auto_place(data, &mut state);
        state.player_a = Some(Side::Axis);
        state.clock.phase = Phase::Repair;
        state.clock.half = Some(Half::A);
        let hex = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .filter_map(|p| p.hex().cloned())
            .find(|h| {
                !modules.abstract_supply || !state.repair_fuel(data, Side::Axis, h).is_empty()
            })
            .expect("an Axis unit with fuel in range");
        state.broken_down.insert(
            hex.clone(),
            vec![Wreck {
                side: Side::Axis,
                nation: None,
                weapon: "M13/40".into(),
                points: 4,
            }],
        );
        (state, hex)
    }

    fn repair(hex: &HexId) -> Order {
        Order::Repair {
            hex: hex.clone(),
            weapon: "M13/40".into(),
            fuel_from: None,
        }
    }

    #[test]
    fn a_save_with_pooled_repaired_points_loads_without_them() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (state, _) = at_repair(&data, Modules::ALL);
        let mut v = serde_json::to_value(&state).unwrap();
        v["repaired"] = serde_json::json!({ "axis": { "M13/40": 2 } });
        let loaded: GameState = serde_json::from_value(v).unwrap();
        assert!(loaded.repaired.is_empty());
        let mut v = serde_json::to_value(&state).unwrap();
        v["repaired"] = serde_json::json!({ "axis": { "C4021": {} } });
        let loaded: GameState = serde_json::from_value(v).unwrap();
        assert!(loaded.repaired.is_empty());
    }

    #[test]
    fn a_field_roll_returns_its_share_and_pays_a_fuel_a_point_22_2x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_repair(&data, Modules::ALL);
        assert_eq!(
            state.field_repairs(&data),
            vec![(hex.clone(), "M13/40".to_string(), 4)]
        );
        let source = state.repair_fuel(&data, Side::Axis, &hex)[0].clone();
        let fuel = state.pieces[&source].load.unwrap().fuel;
        let events = state.apply(&data, Side::Axis, repair(&hex)).unwrap();
        let Event::FieldRepaired {
            die,
            repaired,
            attempted,
            ..
        } = events[0]
        else {
            panic!("{events:?}");
        };
        assert_eq!(attempted, 4.min(fuel));
        let pct = data
            .vehicle_repair
            .row(die)
            .unwrap()
            .field_tank_percent
            .unwrap();
        assert_eq!(repaired, cna_rules::breakdown::losses(attempted, pct));
        assert_eq!(state.pieces[&source].load.unwrap().fuel, fuel - attempted);
        let left: u32 = state
            .broken_down
            .get(&hex)
            .into_iter()
            .flatten()
            .map(|w| w.points)
            .sum();
        assert_eq!(left + repaired, 4);
        // The repaired points stand where they were repaired (20.41).
        let waiting = state
            .repaired
            .get(&Side::Axis)
            .and_then(|m| m.get(&hex))
            .and_then(|m| m.get("M13/40"))
            .copied();
        assert_eq!(waiting.unwrap_or(0), repaired);
    }

    #[test]
    fn armoured_cars_and_recce_trucks_repair_by_points_not_percent_22_8_r_125() {
        let data = Data::load("grazianis-offensive").unwrap();
        type Column = fn(&cna_data::repair::RepairRow) -> Option<u32>;
        let cars: Column = |r| r.field_armoured_car;
        let trucks: Column = |r| r.field_truck;
        for (class, column) in [
            ("armoured-car", cars),
            ("armoured-recce", cars),
            ("truck", trucks),
        ] {
            let (mut state, hex) = at_repair(&data, Modules::ALL);
            state.broken_down.get_mut(&hex).unwrap()[0].weapon = class.into();
            let order = Order::Repair {
                hex: hex.clone(),
                weapon: class.into(),
                fuel_from: None,
            };
            let events = state.apply(&data, Side::Axis, order).unwrap();
            let Event::FieldRepaired {
                die,
                repaired,
                attempted,
                ..
            } = events[0]
            else {
                panic!("{events:?}");
            };
            let row = data.vehicle_repair.row(die).unwrap();
            assert_eq!(repaired, column(row).unwrap().min(attempted), "{class}");
        }
    }

    #[test]
    fn fuel_is_taken_across_the_sources_in_range_22_24() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_repair(&data, Modules::ALL);
        let sources = state.repair_fuel(&data, Side::Axis, &hex);
        assert!(sources.len() >= 2, "{sources:?}");
        // One fuel each in the first two sources, none in the rest.
        for (i, q) in sources.iter().enumerate() {
            state.pieces.get_mut(q).unwrap().load.as_mut().unwrap().fuel = u32::from(i < 2);
        }
        let events = state.apply(&data, Side::Axis, repair(&hex)).unwrap();
        let Event::FieldRepaired { attempted, .. } = events[0] else {
            panic!("{events:?}");
        };
        assert_eq!(attempted, 2);
    }

    #[test]
    fn a_dummy_alone_with_the_vehicles_opens_nothing_22_21() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_repair(
            &data,
            Modules {
                abstract_supply: false,
                breakdown: true,
            },
        );
        let away = HexId::parse("A0101").unwrap();
        let here: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex))
            .map(|p| p.id.clone())
            .collect();
        for id in here {
            state.put(&id, crate::piece::Location::Hex(away.clone()));
        }
        let dummy = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::DummySupplyUnit)
            .map(|p| p.id.clone())
            .expect("an Axis dummy");
        state.put(&dummy, crate::piece::Location::Hex(hex.clone()));
        assert!(state.field_repairs(&data).is_empty());
    }

    #[test]
    fn one_roll_per_type_per_hex_per_phase_22_23() {
        let data = Data::load("grazianis-offensive").unwrap();
        // Many points so that some are still there after the first roll.
        let (mut state, hex) = at_repair(
            &data,
            Modules {
                abstract_supply: false,
                breakdown: true,
            },
        );
        state.broken_down.get_mut(&hex).unwrap()[0].points = 40;
        state.apply(&data, Side::Axis, repair(&hex)).unwrap();
        assert!(state.field_repairs(&data).is_empty());
        let err = state.apply(&data, Side::Axis, repair(&hex)).unwrap_err();
        assert_eq!(err.case, "22.23");
    }

    #[test]
    fn no_field_repair_by_the_other_side_or_without_a_friendly_unit_22_21() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, hex) = at_repair(&data, Modules::ALL);
        // The Commonwealth is not to act (5.2), so cannot repair.
        assert!(state.apply(&data, Side::Cw, repair(&hex)).is_err());
        let ids: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex))
            .map(|p| p.id.clone())
            .collect();
        let elsewhere = HexId::parse("A0101").unwrap();
        for id in ids {
            state.pieces.get_mut(&id).unwrap().location =
                crate::piece::Location::Hex(elsewhere.clone());
        }
        assert!(state.field_repairs(&data).is_empty());
        let err = state.apply(&data, Side::Axis, repair(&hex)).unwrap_err();
        assert_eq!(err.case, "22.21");
    }

    #[test]
    fn phase_k_is_skipped_with_nothing_to_repair() {
        let data = Data::load("grazianis-offensive").unwrap();
        let (mut state, _) = at_repair(&data, Modules::ALL);
        assert!(state.repairs_to_try(&data));
        state.broken_down.clear();
        assert!(!state.repairs_to_try(&data));
    }
}
