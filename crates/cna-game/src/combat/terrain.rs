//! Terrain column shifts for combat (Terrain Effects Chart): a hex's own
//! terrain, and a hexside crossed by the fire. Fortifications and
//! minefields are not simulated, so a major city's "see fortifications"
//! reads as no shift.

use super::strength::Kind;
use crate::data::Data;
use cna_data::map::{HexId, Hexside, HexsideFeature};
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

/// 14.34: why no anti-armour fire, by either side, crosses `side` when
/// the phasing points assault over it from `from` into `to`; none where
/// the fire is allowed. A ridge has no direction; a slope or escarpment
/// with no recorded `up` (a seam pair) climbs both ways, as for movement.
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
    fn seam_slopes_and_escarpments_climb_both_ways_for_anti_armour_fire_too() {
        // seams.json records no `up`, so the engine climbs them in either
        // direction, as it does for movement: an escarpment is banned
        // both ways even with its track, a slope is L1 both ways, and a
        // ridge (which has no direction) follows its road or track.
        let data = Data::load("grazianis-offensive").unwrap();
        assert_eq!(aa(&data, "A3933", "B3901"), None);
        assert_eq!(aa(&data, "B3901", "A3933"), None);
        assert_eq!(aa(&data, "A1233", "B1201"), Some(-1));
        assert_eq!(aa(&data, "B1201", "A1233"), Some(-1));
        assert_eq!(aa(&data, "D2833", "E2901"), None);
    }
}
