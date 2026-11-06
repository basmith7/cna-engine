//! Breakdown (SPI §21) with the `breakdown` module on: every motorised
//! unit collects the breakdown points of each step it takes, in any
//! kind of move, until the OpStage ends (21.2x).

use crate::data::Data;
use crate::event::Event;
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use crate::state::Weather;
use cna_data::map::HexId;
use cna_data::oa::{Nation, UnitId};
use cna_data::scenario::Side;
use cna_data::weapons::WeaponClass;
use cna_rules::movement::Step;
use serde::{Deserialize, Serialize};

/// Broken-down TOE points of one weapon system in a hex (21.4x): no CPA,
/// no part in combat, until towed or repaired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wreck {
    pub side: Side,
    pub nation: Option<Nation>,
    pub weapon: String,
    pub points: u32,
    /// How many of `points` broke down this OpStage: they did not begin
    /// it in the hex, so no facility there repairs them yet (22.31).
    #[serde(default, skip_serializing_if = "crate::breakdown::is_zero")]
    pub this_stage: u32,
    /// How many of `this_stage` were towed in this phase: no repair
    /// touches them until the next (21.6x, 22.13).
    #[serde(default, skip_serializing_if = "crate::breakdown::is_zero")]
    pub towed: u32,
    /// The unit these points broke down with at the start, which repair
    /// returns them to (61.38's `all_tanks`); none for loose points,
    /// which repair turns into replacement points (22.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<UnitId>,
}

pub(crate) fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// A breakdown check's points: some of a piece's listed models, or a
/// recce unit's unlisted TOE points under its vehicle class (R-125).
enum Group {
    Models(Vec<String>),
    Unlisted(&'static str),
}

/// A recce vehicle class's column shift (21.14); none for camels and
/// motorcycles, which never break down (21.11).
pub(crate) fn recce_shift(data: &Data, vehicle: cna_data::oa::ReconVehicle) -> Option<i32> {
    if vehicle == cna_data::oa::ReconVehicle::Exempt {
        return None;
    }
    data.breakdown.adjustment(vehicle.as_str()).map(|a| a.shift)
}

impl GameState {
    /// The step from `from` to `to` adds its breakdown points to each
    /// motorised piece in `pieces` (21.2x). Supply units' motorisation
    /// points never break down (32.5x).
    pub(crate) fn collect_breakdown(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        from: &HexId,
        to: &HexId,
    ) {
        if !self.modules.breakdown {
            return;
        }
        let (Some(f), Some(t)) = (data.map.hex(from), data.map.hex(to)) else {
            return;
        };
        let built = self.hexside(data, from, to);
        let hexside = built.as_deref();
        let step = Step {
            from: f,
            to: t,
            hexside,
            climbs: hexside.is_some_and(|hs| hs.climbs(from, to)),
            weather: self.weather_at(to),
        };
        let halves = (cna_rules::breakdown::step_points(&data.terrain, &step) * 2.0).round() as u32;
        for id in pieces {
            let p = &self.pieces[id];
            if matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
                && self.mobility(data, p).motorised()
            {
                self.pieces.get_mut(id).expect("listed").breakdown_halves += halves;
            }
        }
    }
}

/// Whether at least half of a move's CP went on sandstorm hexes (21.37),
/// from its steps' (hex entered, cost).
pub(crate) fn mostly_in_sandstorm<'a>(
    state: &GameState,
    steps: impl IntoIterator<Item = (&'a HexId, cna_data::cp::Cp)>,
) -> bool {
    let (mut all, mut storm) = (cna_data::cp::Cp::ZERO, cna_data::cp::Cp::ZERO);
    for (to, cost) in steps {
        all += cost;
        if state.weather_at(to) == cna_rules::movement::Weather::Sandstorm {
            storm += cost;
        }
    }
    all > cna_data::cp::Cp::ZERO && storm + storm >= all
}

impl GameState {
    /// Whether a move from `start` began within two hexes of an enemy
    /// combat unit bigger than a battalion with nothing between (21.41):
    /// adjacent, or two away through a hex that holds no friendly unit,
    /// lies in no friendly ZOC, and could be entered (not sea).
    fn began_near_the_enemy(&self, data: &Data, start: &HexId, side: Side) -> bool {
        use cna_data::characteristics::Equivalent;
        let big = |h: &HexId| {
            self.pieces.values().any(|p| {
                p.side != side
                    && p.kind == PieceKind::Combat
                    && p.hex() == Some(h)
                    && p.equivalent == Some(Equivalent::Brigade)
            })
        };
        let open = |h: &HexId| {
            self.holder(h) != Some(side)
                && self.zoc_at(data, h, side.other()) == crate::zoc::Zoc::Clear
                && data
                    .map
                    .hex(h)
                    .is_some_and(|x| x.terrain != cna_data::map::Terrain::Sea)
        };
        let first = data.map.neighbours(start);
        first.iter().any(big)
            || first
                .iter()
                .filter(|h| open(h))
                .any(|h| data.map.neighbours(h).iter().any(big))
    }

    /// The first combat unit of battalion size or larger among `pieces`,
    /// with its side: what may capture broken-down vehicles (21.52) and
    /// destroyed tanks (14.52, R-014).
    pub(crate) fn captor(&self, pieces: &[PieceId]) -> Option<(Side, PieceId)> {
        use cna_data::characteristics::Equivalent;
        pieces
            .iter()
            .map(|id| &self.pieces[id])
            .find(|p| {
                p.kind == PieceKind::Combat
                    && matches!(
                        p.equivalent,
                        Some(Equivalent::Battalion | Equivalent::Brigade)
                    )
            })
            .map(|p| (p.side, p.id.clone()))
    }

    /// A battalion or bigger of `pieces` now at `at` captures the enemy's
    /// broken-down vehicles in `at` and the hexes next to it (21.5x),
    /// unless enemy combat units or HQs stand with them or an enemy ZOC
    /// covers them. They become the captor's where they stand (the
    /// three-hex placement and the passable-terrain test are not
    /// simulated).
    pub(crate) fn capture_wrecks(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        at: &HexId,
        events: &mut Vec<Event>,
    ) {
        self.capture_destroyed_tanks(pieces, at, events);
        let Some(captor) = self.captor(pieces) else {
            return;
        };
        let (side, by) = captor;
        let mut hexes = data.map.neighbours(at);
        hexes.push(at.clone());
        for hex in hexes {
            let Some(wrecks) = self.broken_down.get(&hex) else {
                continue;
            };
            if !wrecks.iter().any(|w| w.side != side) {
                continue;
            }
            let guarded = self.pieces.values().any(|p| {
                p.side != side
                    && p.hex() == Some(&hex)
                    && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
            });
            if guarded || self.zoc_at(data, &hex, side) != crate::zoc::Zoc::Clear {
                continue;
            }
            let wrecks = self.broken_down.get_mut(&hex).expect("listed");
            let mut taken = 0;
            for w in wrecks.iter_mut().filter(|w| w.side != side) {
                w.side = side;
                w.unit = None;
                taken += w.points;
            }
            events.push(Event::WrecksCaptured {
                by: by.clone(),
                hex: hex.clone(),
                points: taken,
            });
        }
    }

    /// The pieces of a move that has stopped check for breakdown (21.2x):
    /// each over 3 points in a higher column than its last check rolls
    /// once per vehicle weapon system, the column shifted by the model's
    /// BAR and the weather (21.3x): hot, or `storm` (at least half the
    /// move's CP spent under a sandstorm); the points lost are removed.
    /// Tanks, SP guns and self-propelled anti-tank guns by their models'
    /// BARs; recce units' points by their vehicle class (R-125).
    pub(crate) fn check_breakdown(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        storm: bool,
        start: &HexId,
        events: &mut Vec<Event>,
    ) {
        use cna_rules::breakdown::{column, losses, must_check, percent, weather_shift};
        if !self.modules.breakdown {
            return;
        }
        let t = &data.breakdown;
        let hot = matches!(self.weather, Weather::Hot);
        for id in pieces {
            let p = &self.pieces[id];
            if p.hex().is_none() {
                continue;
            }
            let points = f64::from(p.breakdown_halves) / 2.0;
            let last = p.breakdown_checked.map(usize::from);
            if !must_check(t, points, last) {
                continue;
            }
            let now = t.band_index(points.ceil() as u32).expect("checked") as u8;
            // One check per BAR (21.2x: per type, and per BAR within it;
            // tanks and SP guns are one type), the losses spread over its
            // models, the most numerous first (21.34); then a recce unit's
            // unlisted TOE points by its vehicle class (R-125).
            let mut bars: std::collections::BTreeMap<i32, Vec<String>> = Default::default();
            for w in p.weapons.iter().filter(|w| w.points > 0) {
                if let Some(bar) = p
                    .nation
                    .and_then(|n| data.weapons.resolve(n, &w.weapon))
                    .and_then(|r| r.bar)
                {
                    bars.entry(bar).or_default().push(w.weapon.clone());
                }
            }
            let mut groups: Vec<(i32, Group)> = bars
                .into_iter()
                .map(|(bar, models)| (bar, Group::Models(models)))
                .collect();
            if let Some(vehicle) =
                p.id.unit()
                    .and_then(|u| data.oa.unit(&u))
                    .and_then(|u| u.vehicle)
                && let Some(shift) = recce_shift(data, vehicle)
            {
                groups.push((shift, Group::Unlisted(vehicle.as_str())));
            }
            let side = p.side;
            self.pieces.get_mut(id).expect("listed").breakdown_checked = Some(now);
            let near = self.began_near_the_enemy(data, start, side);
            let mut lost = 0;
            for (bar, group) in groups {
                let Some(col) = column(t, points, bar + weather_shift(hot, storm)) else {
                    continue;
                };
                let p = &self.pieces[id];
                let count: u32 = match &group {
                    Group::Models(models) => p
                        .weapons
                        .iter()
                        .filter(|w| models.contains(&w.weapon))
                        .map(|w| w.points)
                        .sum(),
                    Group::Unlisted(_) => {
                        p.toe - p.weapons.iter().map(|w| w.points).sum::<u32>().min(p.toe)
                    }
                };
                let reading = self.rng.d6() * 10 + self.rng.d6();
                let pct = percent(t, col, reading);
                let n = losses(count, pct);
                // Near the enemy, at least half stays where the move
                // began (21.41).
                let behind = if near { n.div_ceil(2) } else { 0 };
                events.push(Event::BrokeDown {
                    piece: id.clone(),
                    weapon: match &group {
                        Group::Models(models) => models.join(", "),
                        Group::Unlisted(class) => (*class).to_string(),
                    },
                    reading,
                    percent: pct,
                    points: n,
                });
                for i in 0..n {
                    let weapon = match &group {
                        Group::Models(models) => {
                            let Some(weapon) = self.pieces[id]
                                .weapons
                                .iter()
                                .filter(|w| models.contains(&w.weapon) && w.points > 0)
                                .max_by_key(|w| w.points)
                                .map(|w| w.weapon.clone())
                            else {
                                break;
                            };
                            if !self.take_loss_entry(id, Some(&weapon)) {
                                break;
                            }
                            weapon
                        }
                        Group::Unlisted(class) => {
                            if !self.take_loss_entry(id, None) {
                                break;
                            }
                            (*class).to_string()
                        }
                    };
                    lost += 1;
                    // They stay where the unit stopped, or where it began
                    // (21.41).
                    let p = &self.pieces[id];
                    let (side, nation) = (p.side, p.nation);
                    let Some(stop) = p.hex().cloned() else { break };
                    let at = if i < behind { start.clone() } else { stop };
                    let wrecks = self.broken_down.entry(at).or_default();
                    match wrecks
                        .iter_mut()
                        .find(|w| {
                            w.side == side && w.nation == nation && w.weapon == weapon && w.unit.is_none()
                        })
                    {
                        Some(w) => {
                            w.points += 1;
                            w.this_stage += 1;
                        }
                        None => wrecks.push(Wreck {
                            side,
                            nation,
                            weapon,
                            points: 1,
                            this_stage: 1,
                            towed: 0,
                            unit: None,
                        }),
                    }
                }
            }
            self.note_losses(id, lost, events);
        }
    }
}

impl GameState {
    /// The scenario's tanks that start broken down (its sides'
    /// `broken_down` rows), with the `broken_down_starts` and `breakdown`
    /// modules on: each row's points sit loose in its hex, assigned to no
    /// unit, and no repair touches that hex before the row's
    /// `repair_from` (61.38, 62.32, 63.42). A row offering a choice of
    /// hexes takes the first; a place, its hex. Rows naming units
    /// (`all_tanks`) are not placed yet.
    pub(crate) fn place_broken_down_starts(&mut self, data: &Data) {
        use cna_data::scenario::Locator;
        if !(self.modules.broken_down_starts && self.modules.breakdown) {
            return;
        }
        for (side, setup) in &data.scenario.sides {
            for row in &setup.broken_down {
                let (Some(placement), Some(id), Some(points)) =
                    (&row.placement, &row.weapon, row.points)
                else {
                    continue;
                };
                let hex = match placement.locators.first() {
                    Some(Locator::Hexes(h)) => h.first().cloned(),
                    Some(Locator::Place(ids)) => ids
                        .first()
                        .and_then(|p| data.map.place(p))
                        .map(|p| p.hex.clone()),
                    _ => None,
                };
                let (Some(hex), Some(name), Some(weapon)) =
                    (hex, data.weapons.oa_name(id), data.weapons.row(id))
                else {
                    continue;
                };
                let nation = Some(weapon.nation);
                let wrecks = self.broken_down.entry(hex.clone()).or_default();
                match wrecks
                    .iter_mut()
                    .find(|w| {
                        w.side == *side && w.nation == nation && w.weapon == name && w.unit.is_none()
                    })
                {
                    Some(w) => w.points += points,
                    None => wrecks.push(Wreck {
                        side: *side,
                        nation,
                        weapon: name.to_string(),
                        points,
                        this_stage: 0,
                        towed: 0,
                        unit: None,
                    }),
                }
                if let Some(from) = row.repair_from {
                    let held = self.repair_holds.entry(hex).or_insert(from);
                    *held = (*held).max(from);
                }
            }
        }
    }
}

impl GameState {
    /// Rommel's Arrival's `all_tanks` row, with the `tanks_with_units` and
    /// `breakdown` modules on: every tank TOE point of each unit under the
    /// row's units (as set up, assignments included) starts broken down
    /// in the unit's hex, at the unit's OA maximum, held for that unit
    /// (61.38). Run once set-up is done, since the 7th Armoured is placed
    /// by its owner. A battalion left with no TOE is not eliminated: its
    /// tanks are its own, waiting for repair.
    pub(crate) fn place_tanks_with_units(&mut self, data: &Data) {
        if !(self.modules.tanks_with_units && self.modules.breakdown) {
            return;
        }
        for (side, setup) in &data.scenario.sides {
            for row in setup.broken_down.iter().filter(|r| r.all_tanks) {
                let ids: Vec<PieceId> = self
                    .pieces
                    .values()
                    .filter(|p| {
                        p.side == *side
                            && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
                            && p.hex().is_some()
                            && row.units.iter().any(|hq| {
                                p.id.unit().as_ref() == Some(hq) || self.in_formation(data, &p.id, hq)
                            })
                    })
                    .map(|p| p.id.clone())
                    .collect();
                for id in ids {
                    let p = &self.pieces[&id];
                    let (Some(nation), Some(unit), Some(hex)) =
                        (p.nation, id.unit(), p.hex().cloned())
                    else {
                        continue;
                    };
                    let Some(oa) = data.oa.unit(&unit) else {
                        continue;
                    };
                    let tanks: Vec<(String, u32)> = oa
                        .toe
                        .iter()
                        .filter(|t| {
                            data.weapons
                                .resolve(nation, &t.weapon)
                                .is_some_and(|r| r.class == WeaponClass::Tank)
                        })
                        .map(|t| (t.weapon.clone(), t.points))
                        .filter(|(_, n)| *n > 0)
                        .collect();
                    if tanks.is_empty() {
                        continue;
                    }
                    let p = self.pieces.get_mut(&id).expect("listed");
                    for (weapon, _) in &tanks {
                        if let Some(w) = p.weapons.iter_mut().find(|w| &w.weapon == weapon) {
                            p.toe -= w.points.min(p.toe);
                            w.points = 0;
                        }
                    }
                    let wrecks = self.broken_down.entry(hex.clone()).or_default();
                    for (weapon, max) in tanks {
                        wrecks.push(Wreck {
                            side: *side,
                            nation: Some(nation),
                            weapon,
                            points: max,
                            this_stage: 0,
                            towed: 0,
                            unit: Some(unit.clone()),
                        });
                    }
                    if let Some(from) = row.repair_from {
                        let held = self.repair_holds.entry(hex).or_insert(from);
                        *held = (*held).max(from);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Data;

    #[test]
    fn crusader_starts_with_its_tanks_broken_down_in_alexandria_62_32() {
        use cna_data::map::HexId;
        use cna_data::oa::Nation;
        let data = Data::load("operation-crusader").unwrap();
        let s = GameState::with_modules(&data, 1, crate::state::Modules::ALL).unwrap();
        let alex = HexId::parse("E3613").unwrap();
        let wrecks = &s.broken_down[&alex];
        assert!(
            wrecks
                .iter()
                .all(|w| w.side == Side::Cw && w.this_stage == 0)
        );
        assert_eq!(
            wrecks.iter().map(|w| w.points).sum::<u32>(),
            3 + 15 + 18 + 7 + 8
        );
        for w in wrecks {
            assert!(
                data.weapons.resolve(Nation::Cw, &w.weapon).is_some(),
                "{} is an OA name",
                w.weapon
            );
        }
        let until = s.repair_holds[&alex];
        assert_eq!((until.game_turn, until.opstage), (58, 1));
        let bar = s.repair_hold_bar(&alex).expect("held in the first OpStage");
        assert_eq!(bar.case, "62.32");
        let mut later = s.clone();
        later.clock.game_turn = 58;
        later.clock.stage = 1;
        assert!(later.repair_hold_bar(&alex).is_none());
    }

    #[test]
    fn the_last_chance_starts_with_axis_tanks_broken_down_in_tobruk_63_42() {
        let data = Data::load("the-last-chance").unwrap();
        let s = GameState::with_modules(&data, 1, crate::state::Modules::ALL).unwrap();
        let tobruk = data.map.place("tobruk").unwrap().hex.clone();
        let mut got: Vec<(String, u32)> = s.broken_down[&tobruk]
            .iter()
            .map(|w| {
                assert_eq!(w.side, Side::Axis);
                (
                    data.weapons
                        .resolve(w.nation.unwrap(), &w.weapon)
                        .unwrap()
                        .id
                        .clone(),
                    w.points,
                )
            })
            .collect();
        got.sort();
        assert_eq!(
            got,
            [
                ("weapon:de:pz-iii-h".to_string(), 3),
                ("weapon:de:pz-iv-d".to_string(), 1)
            ]
        );
        assert!(
            s.repair_hold_bar(&tobruk).is_none(),
            "no date printed: repair at once"
        );
    }

    #[test]
    fn without_the_module_nothing_starts_broken_down() {
        let data = Data::load("operation-crusader").unwrap();
        let mut modules = crate::state::Modules::ALL;
        modules.broken_down_starts = false;
        let s = GameState::with_modules(&data, 1, modules).unwrap();
        assert!(s.broken_down.is_empty() && s.repair_holds.is_empty());
        let graziani = Data::load("grazianis-offensive").unwrap();
        let g = GameState::with_modules(&graziani, 1, crate::state::Modules::ALL).unwrap();
        assert!(g.broken_down.is_empty(), "Graziani's prints none");
    }
    /// Rommel's Arrival with every module on, set up, at the start of play.
    fn rommel(modules: Modules) -> (Data, GameState) {
        let data = Data::load("rommels-arrival").unwrap();
        let mut s = GameState::with_modules(&data, 3, modules).unwrap();
        crate::state::test_support::auto_place(&data, &mut s);
        assert!(s.pending.is_none());
        (data, s)
    }

    /// The 7th Armoured's pieces (its HQs included) whose OA unit
    /// lists tanks, with them.
    fn tank_units(data: &Data, s: &GameState) -> Vec<(crate::piece::PieceId, Vec<(String, u32)>)> {
        use cna_data::weapons::WeaponClass;
        let hq = cna_data::oa::UnitId::parse("unit:cw:7th-armored-div-hq").unwrap();
        s.pieces
            .values()
            .filter(|p| {
                matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
                    && (p.id.unit() == Some(hq.clone()) || s.in_formation(data, &p.id, &hq))
            })
            .filter_map(|p| {
                let oa = data.oa.unit(&p.id.unit()?)?;
                let tanks: Vec<(String, u32)> = oa
                    .toe
                    .iter()
                    .filter(|t| {
                        data.weapons
                            .resolve(p.nation.unwrap(), &t.weapon)
                            .is_some_and(|r| r.class == WeaponClass::Tank)
                            && t.points > 0
                    })
                    .map(|t| (t.weapon.clone(), t.points))
                    .collect();
                (!tanks.is_empty()).then(|| (p.id.clone(), tanks))
            })
            .collect()
    }

    #[test]
    fn the_7th_armoured_s_tanks_start_broken_down_with_their_units_61_38() {
        let (data, s) = rommel(Modules::ALL);
        let units = tank_units(&data, &s);
        assert_eq!(units.len(), 6, "three battalions and three HQs: {units:?}");
        let mut emptied = 0;
        for (id, tanks) in &units {
            let p = &s.pieces[id];
            assert!(p.hex().is_some(), "{id} is not eliminated: {:?}", p.location);
            for (weapon, max) in tanks {
                let on: u32 = p
                    .weapons
                    .iter()
                    .filter(|w| &w.weapon == weapon)
                    .map(|w| w.points)
                    .sum();
                assert_eq!(on, 0, "{id}'s {weapon} are all broken down");
                let held: u32 = s.broken_down[p.hex().unwrap()]
                    .iter()
                    .filter(|w| w.unit == id.unit() && &w.weapon == weapon)
                    .map(|w| w.points)
                    .sum();
                assert_eq!(held, *max, "{id}'s {weapon}: its OA maximum, in its hex");
            }
            if p.toe == 0 {
                emptied += 1;
            }
        }
        assert!(emptied > 0, "a tank battalion with no TOE left stands");
        let mut off = Modules::ALL;
        off.tanks_with_units = false;
        let (_, plain) = rommel(off);
        assert!(plain.broken_down.values().flatten().all(|w| w.unit.is_none()));
    }

    #[test]
    fn repair_returns_the_tanks_to_their_unit_not_as_replacements_61_38() {
        use crate::clock::{Half, Phase};
        let mut modules = Modules::ALL;
        modules.abstract_supply = false;
        let (data, mut s) = rommel(modules);
        let (id, tanks) = tank_units(&data, &s).remove(0);
        let (weapon, max) = tanks[0].clone();
        let hex = s.pieces[&id].hex().unwrap().clone();
        let toe = s.pieces[&id].toe;
        s.player_a = Some(Side::Cw);
        s.clock.phase = Phase::Repair;
        s.clock.half = Some(Half::A);
        let mut repaired = 0;
        for _ in 0..50 {
            s.field_rolled.clear();
            let order = Order::Repair {
                hex: hex.clone(),
                weapon: weapon.clone(),
                fuel_from: None,
                unit: None,
            };
            let events = s.apply(&data, Side::Cw, order).unwrap();
            let crate::event::Event::FieldRepaired { repaired: n, .. } = events[0] else {
                panic!("{events:?}");
            };
            repaired += n;
            if repaired > 0 {
                break;
            }
        }
        assert!(repaired > 0, "fifty rolls repaired nothing");
        let p = &s.pieces[&id];
        let on: u32 = p
            .weapons
            .iter()
            .filter(|w| w.weapon == weapon)
            .map(|w| w.points)
            .sum();
        assert_eq!(on, repaired, "back in the battalion");
        assert_eq!(p.toe, toe + repaired);
        assert!(s.repaired.is_empty(), "no replacement points: {:?}", s.repaired);
        let left: u32 = s.broken_down[&hex]
            .iter()
            .filter(|w| w.unit == id.unit() && w.weapon == weapon)
            .map(|w| w.points)
            .sum();
        assert_eq!(left + repaired, max);
    }

    use crate::order::Order;
    use crate::piece::PieceKind;
    use crate::state::{GameState, Modules};
    use cna_data::scenario::Side;

    /// The Axis's first movement segment of a new module-on game, or
    /// with `modules`.
    fn movement(data: &Data, modules: Modules) -> GameState {
        let mut state = GameState::with_modules(data, 5, modules).unwrap();
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
        while state.clock.segment != Some(crate::clock::Segment::Movement) {
            let side = state.to_act().unwrap();
            state.apply(data, side, Order::Pass).unwrap();
        }
        state
    }

    #[test]
    fn near_a_big_enemy_unit_half_the_breakdown_stays_where_the_move_began_21_41() {
        use crate::piece::{Location, PieceId};
        use cna_data::characteristics::Equivalent;
        use cna_data::map::HexId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = movement(&data, Modules::ALL);
        let tanks = PieceId::from(cna_data::oa::UnitId::parse("unit:it:lxl-tank-bn").unwrap());
        state.stand_alone(&tanks);
        state.pieces.get_mut(&tanks).unwrap().breakdown_halves = 200;
        let start = state.pieces[&tanks].hex().unwrap().clone();
        // Graziani's Commonwealth has no brigade-equivalent: make one.
        let brigade = state
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::Combat && p.hex().is_some())
            .unwrap()
            .id
            .clone();
        state.pieces.get_mut(&brigade).unwrap().equivalent = Some(Equivalent::Brigade);
        // The brigade next door, the tanks stopped one hex further on
        // (placed directly: a real move from there would first wait on
        // the brigade's ZOC declaration).
        let mut hexes = data
            .map
            .neighbours(&start)
            .into_iter()
            .filter(|h| state.holder(h).is_none());
        let (next, stop) = (hexes.next().unwrap(), hexes.next().unwrap());
        state.put(&brigade, Location::Hex(next));
        assert!(state.began_near_the_enemy(&data, &start, Side::Axis));
        state.put(&tanks, Location::Hex(stop.clone()));
        let mut events = Vec::new();
        state.check_breakdown(
            &data,
            std::slice::from_ref(&tanks),
            false,
            &start,
            &mut events,
        );
        let lost = state
            .broken_down
            .values()
            .flatten()
            .map(|w| w.points)
            .sum::<u32>();
        assert!(lost > 0);
        let at = |h: &HexId| {
            state
                .broken_down
                .get(h)
                .map_or(0, |v| v.iter().map(|w| w.points).sum::<u32>())
        };
        assert_eq!(at(&start), lost.div_ceil(2));
        assert_eq!(at(&stop), lost - lost.div_ceil(2));
    }

    /// `unit` with `halves` breakdown points checks where it stands.
    fn checked(unit: &str, halves: u32) -> (GameState, Vec<crate::event::Event>) {
        use crate::piece::PieceId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = movement(&data, Modules::ALL);
        let id = PieceId::from(cna_data::oa::UnitId::parse(unit).unwrap());
        state.pieces.get_mut(&id).unwrap().breakdown_halves = halves;
        let at = state.pieces[&id].hex().unwrap().clone();
        let mut events = Vec::new();
        state.check_breakdown(&data, std::slice::from_ref(&id), false, &at, &mut events);
        (state, events)
    }

    #[test]
    fn armoured_cars_and_recce_check_by_their_vehicle_class_r_125() {
        use crate::event::Event;
        // 11th Hussars (armoured cars) and 6th Australian Cavalry
        // (armoured recce), 6 and 8 unlisted TOE points: 100 breakdown
        // points is the chart's last column, where every reading loses.
        for (unit, class, toe) in [
            ("unit:cw:11-hus", "armoured-car", 6),
            ("unit:cw:6-auscav", "armoured-recce", 8),
        ] {
            let (state, events) = checked(unit, 200);
            let lost: u32 = events
                .iter()
                .filter_map(|e| match e {
                    Event::BrokeDown { weapon, points, .. } if weapon == class => Some(*points),
                    _ => None,
                })
                .sum();
            assert!(lost > 0, "{unit}: {events:?}");
            let id = crate::piece::PieceId::from(cna_data::oa::UnitId::parse(unit).unwrap());
            let p = &state.pieces[&id];
            assert_eq!(p.toe + lost, toe, "{unit}");
            let wrecked: u32 = state.broken_down[p.hex().unwrap()]
                .iter()
                .filter(|w| w.weapon == class)
                .map(|w| w.points)
                .sum();
            assert_eq!(wrecked, lost, "{unit}");
        }
    }

    #[test]
    fn camels_and_motorcycles_never_break_down_r_125() {
        let (_, events) = checked("unit:it:1-sahariano-camel-bn", 200);
        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn a_class_shifts_by_its_adjustment_21_14() {
        use cna_data::oa::ReconVehicle;
        let data = Data::load("grazianis-offensive").unwrap();
        let shift = |v| super::recce_shift(&data, v);
        assert_eq!(shift(ReconVehicle::Truck), Some(-2));
        assert_eq!(shift(ReconVehicle::ArmouredCar), Some(-1));
        assert_eq!(shift(ReconVehicle::ArmouredRecce), Some(0));
        assert_eq!(shift(ReconVehicle::Exempt), None);
    }

    #[test]
    fn a_battalion_moving_next_to_enemy_wrecks_takes_them_21_5x() {
        use crate::breakdown::Wreck;
        use crate::event::Event;
        use crate::piece::PieceId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = movement(&data, Modules::ALL);
        let tanks = PieceId::from(cna_data::oa::UnitId::parse("unit:it:lxl-tank-bn").unwrap());
        state.stand_alone(&tanks);
        let at = state.pieces[&tanks].hex().unwrap().clone();
        for n in data.map.neighbours(&at) {
            let mut s = state.clone();
            // Commonwealth wrecks one hex beyond the step, out of any ZOC.
            let Some(beyond) = data.map.neighbours(&n).into_iter().find(|h| {
                *h != at
                    && s.holder(h).is_none()
                    && s.zoc_at(&data, h, Side::Axis) == crate::zoc::Zoc::Clear
            }) else {
                continue;
            };
            s.broken_down.insert(
                beyond.clone(),
                vec![Wreck {
                    side: Side::Cw,
                    nation: None,
                    weapon: "Mark VI Light".into(),
                    points: 2,
                    this_stage: 0,
                    towed: 0,
                    unit: None,
                }],
            );
            let order = Order::Move {
                pieces: vec![tanks.clone()],
                path: vec![n.clone()],
                assault: false,
                fuel_from: None,
            };
            let Ok(events) = s.apply(&data, Side::Axis, order) else {
                continue;
            };
            assert!(events.contains(&Event::WrecksCaptured {
                by: tanks.clone(),
                hex: beyond.clone(),
                points: 2
            }));
            assert_eq!(s.broken_down[&beyond][0].side, Side::Axis);
            return;
        }
        panic!("no step with wrecks beyond it");
    }

    #[test]
    fn a_move_mostly_under_a_sandstorm_shifts_the_check_21_37() {
        use cna_data::cp::Cp;
        use cna_data::map::HexId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = movement(&data, Modules::ALL);
        state.weather = crate::state::Weather::Sandstorm { sheets: vec!['C'] };
        let (c, d) = (
            HexId::parse("C3920").unwrap(),
            HexId::parse("D3714").unwrap(),
        );
        let half = [(&c, Cp::whole(2)), (&d, Cp::whole(2))];
        assert!(super::mostly_in_sandstorm(&state, half));
        let less = [(&c, Cp::whole(1)), (&d, Cp::whole(2))];
        assert!(!super::mostly_in_sandstorm(&state, less));
        assert!(!super::mostly_in_sandstorm(&state, []));
    }

    #[test]
    fn a_tank_battalion_that_stops_past_seventy_points_loses_tanks_21_3x() {
        use crate::event::Event;
        use crate::piece::PieceId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = movement(&data, Modules::ALL);
        // L3s (BAR 2R) at Bardia, already at 100 points this stage: 71+
        // reads past every escape.
        let tanks = PieceId::from(cna_data::oa::UnitId::parse("unit:it:lxl-tank-bn").unwrap());
        state.stand_alone(&tanks);
        state.pieces.get_mut(&tanks).unwrap().breakdown_halves = 200;
        let at = state.pieces[&tanks].hex().unwrap().clone();
        let toe = state.pieces[&tanks].toe;
        let mut moved = None;
        for n in data.map.neighbours(&at) {
            let mut s = state.clone();
            let order = Order::Move {
                pieces: vec![tanks.clone()],
                path: vec![n],
                assault: false,
                fuel_from: None,
            };
            if let Ok(events) = s.apply(&data, Side::Axis, order) {
                moved = Some((s, events));
                break;
            }
        }
        let (s, events) = moved.expect("the L3s can step somewhere");
        let broke: u32 = events
            .iter()
            .filter_map(|e| match e {
                Event::BrokeDown {
                    piece,
                    percent,
                    points,
                    ..
                } if *piece == tanks => {
                    assert!(*percent > 0);
                    Some(*points)
                }
                _ => None,
            })
            .sum();
        assert!(broke > 0);
        assert_eq!(s.pieces[&tanks].toe, toe - broke);
        // The broken-down tanks stay in the hex the battalion stopped in.
        let stop = s.pieces[&tanks].hex().unwrap();
        assert_eq!(
            s.broken_down[stop].iter().map(|w| w.points).sum::<u32>(),
            broke
        );
        assert_eq!(s.pieces[&tanks].breakdown_checked, Some(8));
    }

    #[test]
    fn a_motorised_unit_collects_the_breakdown_points_of_its_steps_21_2x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let base = movement(&data, Modules::ALL);
        // Some motorised Axis unit's first legal step, priced by the rules
        // core.
        let mut found = false;
        for p in base.pieces.values() {
            let Some(at) = p.hex() else { continue };
            if p.side != Side::Axis
                || p.kind != PieceKind::Combat
                || !base.mobility(&data, p).motorised()
            {
                continue;
            }
            for n in data.map.neighbours(at) {
                let mut s = base.clone();
                let order = Order::Move {
                    pieces: vec![p.id.clone()],
                    path: vec![n.clone()],
                    assault: false,
                    fuel_from: None,
                };
                if s.apply(&data, Side::Axis, order.clone()).is_err() {
                    continue;
                }
                let hexside = data.map.hexside(at, &n);
                let step = cna_rules::movement::Step {
                    from: data.map.hex(at).unwrap(),
                    to: data.map.hex(&n).unwrap(),
                    hexside,
                    climbs: hexside.is_some_and(|hs| hs.climbs(at, &n)),
                    weather: base.weather_at(&n),
                };
                let points = cna_rules::breakdown::step_points(&data.terrain, &step);
                assert!(points > 0.0);
                assert_eq!(f64::from(s.pieces[&p.id].breakdown_halves), points * 2.0);
                // Without the module nothing is collected.
                let mut off = base.clone();
                off.modules.breakdown = false;
                off.apply(&data, Side::Axis, order).unwrap();
                assert_eq!(off.pieces[&p.id].breakdown_halves, 0);
                found = true;
                break;
            }
            if found {
                break;
            }
        }
        assert!(found, "no motorised Axis unit could step anywhere");
    }
}
