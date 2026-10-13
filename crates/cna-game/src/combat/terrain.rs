//! Terrain column shifts for combat (Terrain Effects Chart): a hex's own
//! terrain, and a hexside crossed by the fire. Fortifications and
//! minefields are not simulated, so a major city's "see fortifications"
//! reads as no shift.

use super::strength::Kind;
use crate::data::Data;
use cna_data::map::{HexId, HexsideFeature};
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

/// The shift for fire from `from` into `to` across their hexside; none
/// where that fire is prohibited (an escarpment climbed, 14.35). A
/// hexside carrying two features gives both (15.3x: a wadi along a
/// slope), and assaulting down a slope or escarpment is the attacker's
/// +1.
pub fn hexside_shift(data: &Data, from: &HexId, to: &HexId, kind: Kind) -> Option<i32> {
    let Some(side) = data.map.hexside(from, to) else {
        return Some(0);
    };
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
        // An escarpment climbed prohibits anti-armour fire; barrage is
        // never shifted by a hexside.
        let (a, b, _) = data
            .map
            .hexsides()
            .find(|(_, _, s)| s.has(HexsideFeature::Escarpment) && s.up.is_some())
            .unwrap();
        let (down, up) = if data.map.hexside(a, b).unwrap().climbs(a, b) {
            (a, b)
        } else {
            (b, a)
        };
        assert_eq!(hexside_shift(&data, down, up, Kind::AntiArmour), None);
        assert_eq!(hexside_shift(&data, down, up, Kind::Barrage), Some(0));
        assert_eq!(hexside_shift(&data, down, up, Kind::Offence), Some(-3));
        assert_eq!(hexside_shift(&data, up, down, Kind::AntiArmour), Some(-2));
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
        // Anti-armour fire down a slope is -1 either way.
        assert_eq!(
            hexside_shift(&data, &hex("A0822"), &hex("A0923"), Kind::AntiArmour),
            Some(-1)
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
        // A4630|A4730: a wadi along an escarpment; climbing it still
        // prohibits anti-armour fire.
        assert_eq!(
            hexside_shift(&data, &hex("A4730"), &hex("A4630"), Kind::AntiArmour),
            None
        );
        assert_eq!(
            hexside_shift(&data, &hex("A4630"), &hex("A4730"), Kind::AntiArmour),
            Some(-2)
        );
    }
}
