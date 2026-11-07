//! Captured equipment (28.3, R-152): the gun and vehicle TOE points of
//! units that surrender become the captor's replacement points at once,
//! with no repair, waiting in the captor's hex for a unit that may
//! rebuild with their class to absorb them in phase C (20.41). They
//! share `GameState::repaired` with repaired points, under a key per
//! class (`captured tank` and so on) that no weapon system is named.

use crate::data::Data;
use crate::event::Event;
use crate::piece::{Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::oa::{Nation, ReconVehicle};
use cna_data::scenario::Side;
use cna_data::weapons::WeaponClass;
use std::collections::{BTreeMap, BTreeSet};

/// What a captured point of a gun or vehicle row becomes (R-152).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Spoil {
    Tank,
    Artillery,
    AntiTank,
    AntiAir,
    /// A recce unit's point: what it gives turns on its vehicle class.
    Recce,
}

/// The replacement points a captured recce point gives, by class.
const ARMOURED_RECCE: &str = "captured armoured recce";

impl Spoil {
    /// By a characteristics row's unit type; none for infantry-type rows,
    /// HQs and engineers, whose points are prisoners.
    fn of_row(unit_type: &str) -> Option<Spoil> {
        if unit_type.contains("Anti-air") {
            Some(Spoil::AntiAir)
        } else if unit_type.contains("Anti-tank") {
            Some(Spoil::AntiTank)
        } else if unit_type.contains("Artillery") {
            Some(Spoil::Artillery)
        } else if unit_type.contains("Tank") {
            Some(Spoil::Tank)
        } else if unit_type.contains("Recce") || unit_type.contains("Reconnaissance") {
            Some(Spoil::Recce)
        } else {
            None
        }
    }

    /// The replacement key, for every spoil but a recce point's.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Spoil::Tank => "captured tank",
            Spoil::Artillery => "captured artillery",
            Spoil::AntiTank => "captured anti-tank",
            Spoil::AntiAir => "captured anti-air",
            Spoil::Recce => ARMOURED_RECCE,
        }
    }

    /// The spoil a replacement key names, if it names one.
    fn from_key(key: &str) -> Option<Spoil> {
        [
            Spoil::Tank,
            Spoil::Artillery,
            Spoil::AntiTank,
            Spoil::AntiAir,
            Spoil::Recce,
        ]
        .into_iter()
        .find(|s| s.key() == key)
    }
}

/// What one repaired point of a captured broken-down vehicle gives its
/// captor (R-152): a point of its weapon row's class, or for a recce
/// unit's points (R-125) what its vehicle class gives, as `spoil_of`
/// judges a surrendering recce unit; none for what neither names.
pub(crate) fn wreck_spoil(
    data: &Data,
    nation: Nation,
    weapon: &str,
) -> Option<(&'static str, u32)> {
    if let Some(row) = data.weapons.resolve(nation, weapon) {
        let spoil = match row.class {
            WeaponClass::Tank => Spoil::Tank,
            WeaponClass::Artillery => Spoil::Artillery,
            WeaponClass::AntiTank => Spoil::AntiTank,
            WeaponClass::AntiAir => Spoil::AntiAir,
        };
        return Some((spoil.key(), 1));
    }
    if weapon == ReconVehicle::ArmouredCar.as_str() {
        Some((ARMOURED_RECCE, 2))
    } else if weapon == ReconVehicle::ArmouredRecce.as_str() {
        Some((ARMOURED_RECCE, 1))
    } else {
        None
    }
}

/// The side a nation's units fight for.
pub(crate) fn side_of(nation: Nation) -> Side {
    match nation {
        Nation::Cw => Side::Cw,
        Nation::De | Nation::It => Side::Axis,
    }
}

fn vehicle(data: &Data, piece: &Piece) -> Option<ReconVehicle> {
    piece
        .id
        .unit()
        .and_then(|u| data.oa.unit(&u))
        .and_then(|u| u.vehicle)
}

impl GameState {
    /// What one TOE point of `piece` gives its captor, judged by its
    /// unit type (R-152, 20.3's first alternative): one point of its
    /// class for a gun or tank unit, one armoured-recce point for an
    /// armoured-recce unit and two for an armoured car's. None for an
    /// infantry-type unit or an HQ, whose points (heavy weapons
    /// included) are prisoners.
    pub(crate) fn spoil_of(&self, data: &Data, piece: &Piece) -> Option<(&'static str, u32)> {
        if piece.kind != PieceKind::Combat {
            return None;
        }
        let row = data.chars.row(piece.nation?, piece.id_code.as_deref()?)?;
        match Spoil::of_row(&row.unit_type)? {
            Spoil::Recce => match vehicle(data, piece) {
                Some(ReconVehicle::ArmouredCar) => Some((ARMOURED_RECCE, 2)),
                Some(ReconVehicle::ArmouredRecce) => Some((ARMOURED_RECCE, 1)),
                _ => None,
            },
            s => Some((s.key(), 1)),
        }
    }

    /// The gun and vehicle units' TOE points among `ids` by replacement
    /// key, with the replacement points they give (`spoil_of`). A piece
    /// listed twice counts once.
    pub(crate) fn spoil_toe(
        &self,
        data: &Data,
        ids: &[PieceId],
    ) -> BTreeMap<&'static str, (u32, u32)> {
        let mut out: BTreeMap<&'static str, (u32, u32)> = BTreeMap::new();
        let ids: BTreeSet<&PieceId> = ids.iter().collect();
        for id in ids {
            let p = &self.pieces[id];
            if let Some((key, each)) = self.spoil_of(data, p) {
                let e = out.entry(key).or_default();
                e.0 += p.toe;
                e.1 += p.toe * each;
            }
        }
        out
    }

    /// The replacement points `ids` give their captor on surrendering
    /// (R-152). Read before their losses.
    pub(crate) fn spoils(&self, data: &Data, ids: &[PieceId]) -> BTreeMap<&'static str, u32> {
        self.spoil_toe(data, ids)
            .into_iter()
            .map(|(k, (_, points))| (k, points))
            .collect()
    }

    /// The captured share of close-assault losses (15.89, R-152): of
    /// `lost` TOE points, `lost_spoils` (from `spoil_toe` before less
    /// after) were guns and vehicles. `pct` of each class's lost points,
    /// rounded up, is captured: the infantry-type share as prisoner
    /// points, the rest as the captor's replacement points.
    pub(crate) fn captured_share(
        pct: u32,
        lost: u32,
        lost_spoils: &BTreeMap<&'static str, (u32, u32)>,
    ) -> (u32, BTreeMap<&'static str, u32>) {
        let vehicles: u32 = lost_spoils.values().map(|(toe, _)| toe).sum();
        let prisoners = (pct * lost.saturating_sub(vehicles)).div_ceil(100);
        let spoils = lost_spoils
            .iter()
            .map(|(k, (toe, points))| {
                let taken = (pct * toe).div_ceil(100);
                // Each lost point of the class gives `points / toe`.
                (*k, taken * points / (*toe).max(1))
            })
            .filter(|(_, n)| *n > 0)
            .collect();
        (prisoners, spoils)
    }

    /// `captor` takes `spoils` in `hex` as replacement points (R-152),
    /// with the captured-equipment module on. Under abstract supply the
    /// Commonwealth removes captured Axis anti-air points at once (32.81).
    pub(crate) fn capture_spoils(
        &mut self,
        captor: Side,
        hex: &HexId,
        spoils: BTreeMap<&'static str, u32>,
        events: &mut Vec<Event>,
    ) {
        if !self.modules.captured_equipment {
            return;
        }
        for (key, points) in spoils {
            if points == 0
                || (key == Spoil::AntiAir.key()
                    && captor == Side::Cw
                    && self.modules.abstract_supply)
            {
                continue;
            }
            events.push(Event::EquipmentCaptured {
                side: captor,
                hex: hex.clone(),
                weapon: key.to_string(),
                points,
            });
            *self
                .repaired
                .entry(captor)
                .or_default()
                .entry(hex.clone())
                .or_default()
                .entry(key.to_string())
                .or_default() += points;
        }
    }

    /// For a captured key, whether `piece` may rebuild with it (R-152,
    /// 19.6): a unit of that type, by its ID code, whatever it holds
    /// now; armoured-recce points go to an armoured-recce unit, not to
    /// an armoured car's (which 20.3 rebuilds at two a point). `None`
    /// when `key` names no captured class.
    pub(crate) fn fits_captured(&self, data: &Data, piece: &Piece, key: &str) -> Option<bool> {
        Some(self.spoil_of(data, piece) == Some((key_static(key)?, 1)))
    }

    /// The weapon entry captured points join when `piece` absorbs them:
    /// its biggest listed entry, so they take its ratings; none when its
    /// points are all its ID code's.
    pub(crate) fn captured_joins(&self, piece: &Piece, key: &str) -> Option<String> {
        Spoil::from_key(key)?;
        piece
            .weapons
            .iter()
            .max_by_key(|w| w.points)
            .map(|w| w.weapon.clone())
    }
}

/// The static key equal to `key`, if it is a captured one.
fn key_static(key: &str) -> Option<&'static str> {
    Spoil::from_key(key).map(Spoil::key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Phase;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;
    use crate::piece::Location;
    use crate::state::Modules;
    use crate::state::test_support::auto_place;

    const TANKS: &str = "unit:it:iim-tank-bn";

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// The Italian tank battalion, collapsed beside C3923, surrenders when
    /// the 1st KRRC steps in (as prisoners.rs's 28.11 test), with the
    /// captured-equipment module as given.
    fn surrender(data: &Data, captured_equipment: bool) -> (GameState, Vec<Event>, u32) {
        let mut state = moving(data, 3, Side::Cw);
        state.modules.captured_equipment = captured_equipment;
        let spot = data
            .map
            .neighbours(&hex("C3924"))
            .into_iter()
            .find(|h| {
                !data.map.neighbours(&hex("C3922")).contains(h)
                    && *h != hex("C3922")
                    && *h != hex("C3923")
                    && state.holder(h).is_none()
            })
            .unwrap();
        let tanks = pid(TANKS);
        let p = state.pieces.get_mut(&tanks).unwrap();
        p.location = Location::Hex(spot);
        p.cohesion.level = -26;
        let spoils = state.spoils(data, &[tanks]);
        let points = spoils["captured tank"];
        assert!(points > 0, "{spoils:?}");
        let events = state
            .apply(data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923", "C3924"]))
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Surrendered { .. }))
        );
        (state, events, points)
    }

    #[test]
    fn a_surrendered_tank_battalion_s_tanks_wait_as_the_captor_s_replacements_r152() {
        let data = graziani();
        let (state, events, points) = surrender(&data, true);
        assert!(
            events.contains(&Event::EquipmentCaptured {
                side: Side::Cw,
                hex: hex("C3923"),
                weapon: "captured tank".into(),
                points,
            }),
            "{events:?}"
        );
        assert_eq!(
            state.repaired[&Side::Cw][&hex("C3923")]["captured tank"],
            points
        );
    }

    #[test]
    fn with_the_module_off_surrendered_tanks_are_lost_as_before() {
        let data = graziani();
        let (state, events, _) = surrender(&data, false);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::EquipmentCaptured { .. }))
        );
        assert!(state.repaired.is_empty());
    }

    #[test]
    fn infantry_gives_no_equipment_and_an_armoured_car_two_recce_points() {
        let data = graziani();
        let mut state = GameState::new(&data, 3).unwrap();
        auto_place(&data, &mut state);
        let foot = pid("unit:it:viii-libyan-bn");
        assert!(state.spoils(&data, &[foot]).is_empty());
        let car = state
            .pieces
            .values()
            .find(|p| vehicle(&data, p) == Some(ReconVehicle::ArmouredCar) && p.toe > 0)
            .expect("an armoured-car unit in Graziani's")
            .clone();
        let recce = car.toe;
        assert!(recce > 0);
        assert_eq!(
            state
                .spoils(&data, std::slice::from_ref(&car.id))
                .get(ARMOURED_RECCE),
            Some(&(2 * recce))
        );
    }

    #[test]
    fn under_abstract_supply_the_commonwealth_drops_captured_anti_air_32_81() {
        let data = graziani();
        let mut state = GameState::with_modules(&data, 3, Modules::ALL).unwrap();
        let spoils = BTreeMap::from([("captured anti-air", 2), ("captured artillery", 3)]);
        let mut events = Vec::new();
        state.capture_spoils(Side::Cw, &hex("C3923"), spoils.clone(), &mut events);
        assert_eq!(
            state.repaired[&Side::Cw][&hex("C3923")],
            BTreeMap::from([("captured artillery".to_string(), 3)])
        );
        state.capture_spoils(Side::Axis, &hex("C3923"), spoils, &mut events);
        assert_eq!(state.repaired[&Side::Axis][&hex("C3923")].len(), 2);
    }

    #[test]
    fn a_tank_unit_absorbs_captured_tanks_into_its_own_model_and_an_hq_may_not() {
        let data = graziani();
        let mut state = GameState::with_modules(&data, 3, Modules::ALL).unwrap();
        auto_place(&data, &mut state);
        let tanks = pid(TANKS);
        let p = state.pieces.get_mut(&tanks).unwrap();
        let w = p.weapons.iter_mut().find(|w| w.points >= 2).unwrap();
        w.points -= 2;
        let model = w.weapon.clone();
        let before = w.points;
        p.toe -= 2;
        let toe = p.toe;
        let at = p.hex().unwrap().clone();
        state.repaired.insert(
            Side::Axis,
            [(at.clone(), [("captured tank".to_string(), 2)].into())].into(),
        );
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        assert_eq!(state.clock.phase, Phase::Organisation);
        let absorb = |piece: &PieceId, points| Order::Absorb {
            piece: piece.clone(),
            weapon: "captured tank".into(),
            points,
        };
        let hq = state
            .pieces
            .values()
            .find(|q| q.side == Side::Axis && q.hex() == Some(&at) && q.id != tanks)
            .filter(|q| state.fits_captured(&data, q, "captured tank") == Some(false))
            .map(|q| q.id.clone());
        if let Some(hq) = hq {
            let err = state.apply(&data, Side::Axis, absorb(&hq, 1)).unwrap_err();
            assert_eq!(err.case, "19.6x");
        }
        state.apply(&data, Side::Axis, absorb(&tanks, 2)).unwrap();
        let p = &state.pieces[&tanks];
        assert_eq!(p.toe, toe + 2);
        let w = p.weapons.iter().find(|w| w.weapon == model).unwrap();
        assert_eq!(w.points, before + 2);
        assert!(!p.weapons.iter().any(|w| w.weapon.starts_with("captured")));
        assert!(state.repaired.is_empty());
    }

    #[test]
    fn the_15_89_share_splits_into_prisoners_and_equipment_by_class_r152() {
        let lost = BTreeMap::from([("captured tank", (4, 4)), (ARMOURED_RECCE, (3, 6))]);
        // 10 TOE points lost, 7 of them vehicles; 50 % of each class.
        let (prisoners, spoils) = GameState::captured_share(50, 10, &lost);
        assert_eq!(prisoners, 2);
        assert_eq!(
            spoils,
            BTreeMap::from([("captured tank", 2), (ARMOURED_RECCE, 4)])
        );
        // No vehicles lost: the old count, all prisoners.
        assert_eq!(
            GameState::captured_share(30, 7, &BTreeMap::new()),
            (3, BTreeMap::new())
        );
        assert_eq!(GameState::captured_share(0, 7, &lost), (0, BTreeMap::new()));
    }

    #[test]
    fn an_hq_s_guns_and_an_infantry_unit_s_heavy_weapons_are_prisoners_r152() {
        let data = graziani();
        let mut state = GameState::with_modules(&data, 3, Modules::ALL).unwrap();
        auto_place(&data, &mut state);
        let armed = |kind| {
            state
                .pieces
                .values()
                .find(|p| {
                    p.kind == kind
                        && p.toe > 0
                        && !p.weapons.is_empty()
                        && state.spoil_of(&data, p).is_none()
                })
                .map(|p| p.id.clone())
        };
        for id in [armed(PieceKind::Hq), armed(PieceKind::Combat)]
            .into_iter()
            .flatten()
        {
            let p = &state.pieces[&id];
            assert!(
                state.spoils(&data, std::slice::from_ref(&id)).is_empty(),
                "{id}"
            );
            assert_eq!(
                state.prisoner_points(&data, std::slice::from_ref(&id)),
                p.toe,
                "{id}"
            );
            assert_eq!(state.fits_captured(&data, p, "captured tank"), Some(false));
        }
        assert!(armed(PieceKind::Hq).is_some(), "an HQ listing weapons");
        // A tank battalion gives no prisoners, and counts once however
        // often a split assault lists it.
        let tanks = pid(TANKS);
        let toe = state.pieces[&tanks].toe;
        assert_eq!(
            state.prisoner_points(&data, std::slice::from_ref(&tanks)),
            0
        );
        assert_eq!(
            state.spoil_toe(&data, &[tanks.clone(), tanks.clone()]),
            BTreeMap::from([("captured tank", (toe, toe))])
        );
    }
}
