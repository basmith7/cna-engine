//! Terrain column shifts for combat (Terrain Effects Chart): a hex's own
//! terrain or its fortification, whichever is better, and a hexside
//! crossed by the fire. Minefields are not simulated.

use super::strength::Kind;
use crate::data::Data;
use crate::state::GameState;
use cna_data::map::{HexId, Hexside, HexsideFeature};
use cna_data::scenario::Side;
use cna_data::terrain::{Shift, Shifts};

fn pick(s: &Shifts, kind: Kind) -> Option<i32> {
    let shift = match kind {
        Kind::Barrage => &s.barrage,
        Kind::AntiArmour => &s.anti_armour,
        Kind::Offence | Kind::Defence => &s.close_assault,
    };
    match shift {
        Shift::Cols(n) => Some(*n),
        Shift::Prohibited => None,
        Shift::Other(_) => Some(0),
    }
}

/// The shift a target hex's terrain gives; none where that fire is
/// prohibited.
pub fn hex_shift(data: &Data, hex: &HexId, kind: Kind) -> Option<i32> {
    let Some(h) = data.map.hex(hex) else {
        return Some(0);
    };
    match data.terrain.shifts("hex", h.terrain.name()) {
        Some(s) => pick(s, kind),
        None => Some(0),
    }
}

/// The shift a fortification of `level` gives (TEC); none at level 0.
fn fortification_shift(data: &Data, level: u8, kind: Kind) -> Option<i32> {
    if level == 0 {
        return None;
    }
    data.terrain
        .shifts("fortification", &format!("fortification-{level}"))
        .and_then(|s| pick(s, kind))
}

impl GameState {
    /// A hex's fortification level (25.1x): what bombing, barrage or
    /// building left it at, else a major city's own, 3 for Alexandria and
    /// Cairo and 2 for the rest (TEC footnote 4, E-031), else none.
    pub fn fortification_level(&self, data: &Data, hex: &HexId) -> u8 {
        if let Some(level) = self.fortifications.get(hex) {
            return *level;
        }
        city_level(data, hex)
    }

    /// The in-hex shift for fire of `kind` into `hex` (12.33, 14.3x,
    /// 15.3x): the better of its terrain's and its fortification's, never
    /// their sum; armour targets take a fortification's only in a major
    /// city (TEC footnote 12). None where the terrain prohibits the fire.
    pub fn in_hex_shift(&self, data: &Data, hex: &HexId, kind: Kind, armour: bool) -> Option<i32> {
        let terrain = hex_shift(data, hex, kind)?;
        if armour && city_level(data, hex) == 0 {
            return Some(terrain);
        }
        let fort = fortification_shift(data, self.fortification_level(data, hex), kind);
        Some(fort.map_or(terrain, |f| f.min(terrain)))
    }

    /// The minefield shift for `defender` against fire of `kind` into
    /// `target` from `from` (26.26, TEC footnote 13): one column for the
    /// defender when it stands in its own field or an attacker stands in
    /// one of the defender's, never two; barrage is unaffected.
    pub fn minefield_shift(
        &self,
        target: &HexId,
        defender: Side,
        from: &[HexId],
        kind: Kind,
    ) -> i32 {
        if kind == Kind::Barrage {
            return 0;
        }
        let theirs = |h: &HexId| self.minefields.get(h).is_some_and(|f| f.side == defender);
        if theirs(target) || from.iter().any(theirs) {
            -1
        } else {
            0
        }
    }

    /// Units in a level-3 city ignore pinned results (25.24).
    pub fn ignores_pins(&self, data: &Data, hex: &HexId) -> bool {
        city_level(data, hex) > 0 && self.fortification_level(data, hex) >= 3
    }
}

/// A major city's own fortification level: 3 for Alexandria and Cairo,
/// 2 for the others, 0 elsewhere.
pub(crate) fn city_level(data: &Data, hex: &HexId) -> u8 {
    let city = data
        .map
        .hex(hex)
        .is_some_and(|h| h.terrain == cna_data::map::Terrain::MajorCity);
    if !city {
        return 0;
    }
    let three = ["cairo", "alexandria-east", "alexandria-west"]
        .iter()
        .filter_map(|id| data.map.place(id))
        .any(|p| p.hexes.contains(hex));
    if three { 3 } else { 2 }
}

/// 14.34: why no anti-armour fire, by either side, crosses `side` when
/// the phasing points assault over it from `from` into `to`; none where
/// the fire is allowed. A ridge has no direction; a slope or escarpment
/// climbs towards its recorded `up` hex, as for movement.
pub fn anti_armour_ban(side: &Hexside, from: &HexId, to: &HexId) -> Option<&'static str> {
    let crossing = side.road() || side.track();
    let climbs = side.climbs(from, to);
    for f in &side.features {
        let ban = match f {
            HexsideFeature::Ridge if !crossing => "a ridge with no road or track",
            HexsideFeature::Slope if !climbs && !crossing => "down a slope with no road or track",
            HexsideFeature::Escarpment if climbs => "up an escarpment",
            HexsideFeature::Escarpment if !side.track() => "down an escarpment with no track",
            _ => continue,
        };
        return Some(ban);
    }
    None
}

/// The shift for fire from `from` into `to` across their hexside; none
/// where that fire is prohibited (anti-armour fire across a hexside
/// 14.34 bans). A hexside carrying two features gives both (15.3x: a
/// wadi along a slope), and assaulting down a slope or escarpment is the
/// attacker's +1.
pub fn hexside_shift(data: &Data, from: &HexId, to: &HexId, kind: Kind) -> Option<i32> {
    let Some(side) = data.map.hexside(from, to) else {
        return Some(0);
    };
    if kind == Kind::AntiArmour && anti_armour_ban(side, from, to).is_some() {
        return None;
    }
    let mut total = 0;
    for f in &side.features {
        let name = match f {
            HexsideFeature::Ridge => "ridge",
            HexsideFeature::Slope if side.climbs(from, to) => "up-slope",
            HexsideFeature::Slope => "down-slope",
            HexsideFeature::Escarpment if side.climbs(from, to) => "up-escarpment",
            HexsideFeature::Escarpment => "down-escarpment",
            HexsideFeature::Wadi => "wadi",
            HexsideFeature::MajorRiver => "major-river",
            HexsideFeature::MinorRiver => "minor-river",
            _ => continue,
        };
        let Some(s) = data.terrain.shifts("hexside", name) else {
            continue;
        };
        total += pick(s, kind)?;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::hex;

    fn aa(data: &Data, from: &str, to: &str) -> Option<i32> {
        hexside_shift(data, &hex(from), &hex(to), Kind::AntiArmour)
    }

    #[test]
    fn hex_and_hexside_shifts() {
        let data = Data::load("grazianis-offensive").unwrap();
        // C4020 is clear.
        assert_eq!(hex_shift(&data, &hex("C4020"), Kind::Barrage), Some(0));
        assert_eq!(hex_shift(&data, &hex("C4020"), Kind::Offence), Some(0));
        // Some rough hex on sheet C.
        let rough = data
            .map
            .hexes()
            .find(|h| h.terrain == cna_data::map::Terrain::Rough)
            .unwrap();
        assert_eq!(hex_shift(&data, &rough.id, Kind::Barrage), Some(-1));
        assert_eq!(hex_shift(&data, &rough.id, Kind::Defence), Some(-2));
        // C4020|C4121: an escarpment with a track, C4020 above. Climbing
        // it prohibits anti-armour fire; down it by the track is L2.
        // Barrage is never shifted by a hexside.
        assert_eq!(aa(&data, "C4121", "C4020"), None);
        assert_eq!(aa(&data, "C4020", "C4121"), Some(-2));
        assert_eq!(
            hexside_shift(&data, &hex("C4121"), &hex("C4020"), Kind::Barrage),
            Some(0)
        );
        assert_eq!(
            hexside_shift(&data, &hex("C4121"), &hex("C4020"), Kind::Offence),
            Some(-3)
        );
    }

    fn clear_hex(data: &Data) -> HexId {
        let h = hex("C4020");
        assert_eq!(
            data.map.hex(&h).unwrap().terrain,
            cna_data::map::Terrain::Clear
        );
        h
    }

    #[test]
    fn a_major_city_defends_at_its_fortification_level_25_11() {
        let data = Data::load("grazianis-offensive").unwrap();
        let state = crate::state::test_support::set_up(&data, 3);
        let tobruk = hex("C4807");
        assert_eq!(state.fortification_level(&data, &tobruk), 2);
        assert_eq!(
            state.in_hex_shift(&data, &tobruk, Kind::Barrage, false),
            Some(-2)
        );
        assert_eq!(
            state.in_hex_shift(&data, &tobruk, Kind::Defence, false),
            Some(-3)
        );
        // Armour in a city takes the benefit (TEC footnote 12).
        assert_eq!(
            state.in_hex_shift(&data, &tobruk, Kind::AntiArmour, true),
            Some(-2)
        );
        let alexandria = hex("E3613");
        assert_eq!(state.fortification_level(&data, &alexandria), 3);
        assert_eq!(
            state.in_hex_shift(&data, &alexandria, Kind::Defence, false),
            Some(-4)
        );
        assert!(state.ignores_pins(&data, &alexandria));
        assert!(!state.ignores_pins(&data, &tobruk));
        // Rebuilt or reduced levels are state.
        let mut reduced = state.clone();
        reduced.fortifications.insert(tobruk.clone(), 1);
        assert_eq!(
            reduced.in_hex_shift(&data, &tobruk, Kind::Defence, false),
            Some(-2)
        );
    }

    #[test]
    fn a_built_fortification_is_the_best_single_benefit_not_a_sum_12_33() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = crate::state::test_support::set_up(&data, 3);
        let rough = data
            .map
            .hexes()
            .find(|h| h.terrain == cna_data::map::Terrain::Rough)
            .unwrap()
            .id
            .clone();
        state.fortifications.insert(rough.clone(), 2);
        // Rough is L1/L2; fortification 2 is L2/L3: the better, not L3/L5.
        assert_eq!(
            state.in_hex_shift(&data, &rough, Kind::Barrage, false),
            Some(-2)
        );
        assert_eq!(
            state.in_hex_shift(&data, &rough, Kind::Defence, false),
            Some(-3)
        );
        let clear = clear_hex(&data);
        state.fortifications.insert(clear.clone(), 1);
        assert_eq!(
            state.in_hex_shift(&data, &clear, Kind::Defence, false),
            Some(-2)
        );
        // Armour outside a major city gets no fortification benefit
        // (TEC footnote 12), in barrage or anti-armour fire.
        assert_eq!(
            state.in_hex_shift(&data, &clear, Kind::AntiArmour, true),
            Some(0)
        );
        assert_eq!(
            state.in_hex_shift(&data, &clear, Kind::Barrage, true),
            Some(0)
        );
        assert_eq!(
            state.in_hex_shift(&data, &clear, Kind::Barrage, false),
            Some(-1)
        );
    }

    #[test]
    fn minefields_shift_anti_armour_and_close_assault_one_for_the_defender_26_26() {
        use crate::construction::Minefield;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = crate::state::test_support::set_up(&data, 3);
        let (target, from) = (hex("C3923"), hex("C4022"));
        let defender = Side::Cw;
        let field = |side| Minefield {
            side,
            real: false,
            revealed: false,
        };
        assert_eq!(
            state.minefield_shift(
                &target,
                defender,
                std::slice::from_ref(&from),
                Kind::Defence
            ),
            0
        );
        // Attackers standing in the defender's field.
        state.minefields.insert(from.clone(), field(defender));
        assert_eq!(
            state.minefield_shift(
                &target,
                defender,
                std::slice::from_ref(&from),
                Kind::Defence
            ),
            -1
        );
        assert_eq!(
            state.minefield_shift(
                &target,
                defender,
                std::slice::from_ref(&from),
                Kind::AntiArmour
            ),
            -1
        );
        assert_eq!(
            state.minefield_shift(
                &target,
                defender,
                std::slice::from_ref(&from),
                Kind::Barrage
            ),
            0
        );
        // The defender in its own field as well: still one, not two.
        state.minefields.insert(target.clone(), field(defender));
        assert_eq!(
            state.minefield_shift(
                &target,
                defender,
                std::slice::from_ref(&from),
                Kind::Defence
            ),
            -1
        );
        // A field of the attacker's own gives the defender nothing.
        state.minefields.clear();
        state
            .minefields
            .insert(target.clone(), field(defender.other()));
        state
            .minefields
            .insert(from.clone(), field(defender.other()));
        assert_eq!(
            state.minefield_shift(&target, defender, &[from], Kind::Defence),
            0
        );
    }

    #[test]
    fn assaulting_down_a_slope_favours_the_attacker() {
        let data = Data::load("grazianis-offensive").unwrap();
        // A0822 is the top of the slope to A0923.
        assert_eq!(
            hexside_shift(&data, &hex("A0822"), &hex("A0923"), Kind::Offence),
            Some(1)
        );
        assert_eq!(
            hexside_shift(&data, &hex("A0923"), &hex("A0822"), Kind::Offence),
            Some(-2)
        );
    }

    #[test]
    fn a_hexside_carrying_two_features_gives_both() {
        let data = Data::load("grazianis-offensive").unwrap();
        // B3801|B3901: a wadi along a slope, B3801 above. Down the slope
        // (+1) across the wadi (-1); up it (-2) across the wadi (-1).
        assert_eq!(
            hexside_shift(&data, &hex("B3801"), &hex("B3901"), Kind::Offence),
            Some(0)
        );
        assert_eq!(
            hexside_shift(&data, &hex("B3901"), &hex("B3801"), Kind::Offence),
            Some(-3)
        );
        // A4630|A4730: a wadi along an escarpment with no track; no
        // anti-armour fire crosses it either way (14.34).
        assert_eq!(aa(&data, "A4730", "A4630"), None);
        assert_eq!(aa(&data, "A4630", "A4730"), None);
    }

    // 14.34: the hexsides the phasing points assault across that ban
    // anti-armour fire outright, and the road or track exceptions.

    #[test]
    fn a_ridge_bans_anti_armour_fire_unless_a_road_or_track_crosses_it() {
        let data = Data::load("grazianis-offensive").unwrap();
        // C4213|C4313: a bare ridge. A ridge has no direction.
        assert_eq!(aa(&data, "C4213", "C4313"), None);
        assert_eq!(aa(&data, "C4313", "C4213"), None);
        // Close assault and barrage are not banned by 14.34.
        assert_eq!(
            hexside_shift(&data, &hex("C4213"), &hex("C4313"), Kind::Offence),
            Some(-2)
        );
        assert_eq!(
            hexside_shift(&data, &hex("C4213"), &hex("C4313"), Kind::Barrage),
            Some(0)
        );
        // C3427|C3527: a ridge with a track, L2 either way.
        assert_eq!(aa(&data, "C3427", "C3527"), Some(-2));
        assert_eq!(aa(&data, "C3527", "C3427"), Some(-2));
        // C4506|C4605: an unfinished road counts as a track, as it does
        // for movement.
        assert_eq!(aa(&data, "C4506", "C4605"), Some(-2));
    }

    #[test]
    fn a_slope_bans_anti_armour_fire_downward_unless_a_road_or_track_crosses_it() {
        let data = Data::load("grazianis-offensive").unwrap();
        // A0822|A0923, A0822 above, nothing over it: no fire down it;
        // up it is never banned, L1.
        assert_eq!(aa(&data, "A0822", "A0923"), None);
        assert_eq!(aa(&data, "A0923", "A0822"), Some(-1));
        // A2329|A2429, A2329 above, a road over it: down it is L1.
        assert_eq!(aa(&data, "A2329", "A2429"), Some(-1));
        // A1629|A1729, A1729 above, a track over it: down it is L1.
        assert_eq!(aa(&data, "A1729", "A1629"), Some(-1));
    }

    #[test]
    fn an_escarpment_bans_anti_armour_fire_downward_unless_a_track_crosses_it() {
        let data = Data::load("grazianis-offensive").unwrap();
        // A3933|A4033, A4033 above, nothing over it: banned both ways.
        assert_eq!(aa(&data, "A4033", "A3933"), None);
        assert_eq!(aa(&data, "A3933", "A4033"), None);
        // C4020|C4121, C4020 above, a track over it: down it is L2; up
        // it is still banned.
        assert_eq!(aa(&data, "C4020", "C4121"), Some(-2));
        assert_eq!(aa(&data, "C4121", "C4020"), None);
    }

    #[test]
    fn a_road_alone_does_not_open_an_escarpment_to_anti_armour_fire() {
        // The map has no escarpment with a road and no track, so the
        // hexside is made up: a and b, a above.
        let (a, b) = (hex("A0101"), hex("A0102"));
        let road = Hexside {
            features: vec![HexsideFeature::Escarpment, HexsideFeature::Road],
            up: Some(a.clone()),
        };
        assert!(anti_armour_ban(&road, &a, &b).is_some());
        assert!(anti_armour_ban(&road, &b, &a).is_some());
        let track = Hexside {
            features: vec![HexsideFeature::Escarpment, HexsideFeature::Track],
            up: Some(a.clone()),
        };
        assert_eq!(anti_armour_ban(&track, &a, &b), None);
        assert!(anti_armour_ban(&track, &b, &a).is_some());
        // A road does lift a slope's ban.
        let slope = Hexside {
            features: vec![HexsideFeature::Slope, HexsideFeature::Road],
            up: Some(a.clone()),
        };
        assert_eq!(anti_armour_ban(&slope, &a, &b), None);
    }

    #[test]
    fn seam_slopes_and_escarpments_follow_their_recorded_up() {
        // seams.json says which hex is higher (cna #235), so a seam
        // hexside bans and shifts like any other: A3933|B3901 is an
        // escarpment with a track, B3901 above, banned up and L2 down;
        // A1233|B1201 a bare slope, A1233 above, banned down and L1 up;
        // D2833|E2901 a bare ridge, banned both ways.
        let data = Data::load("grazianis-offensive").unwrap();
        assert_eq!(aa(&data, "A3933", "B3901"), None);
        assert_eq!(aa(&data, "B3901", "A3933"), Some(-2));
        assert_eq!(aa(&data, "A1233", "B1201"), None);
        assert_eq!(aa(&data, "B1201", "A1233"), Some(-1));
        assert_eq!(aa(&data, "D2833", "E2901"), None);
        assert_eq!(aa(&data, "E2901", "D2833"), None);
    }
}
