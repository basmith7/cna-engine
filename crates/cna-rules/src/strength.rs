//! Actual strength from raw (SPI 11.3x; cna `60-combat.md`, *Actual
//! strength*): raw ÷ 10 rounded to nearest, .5 up; four raw or fewer is
//! nothing; and the close-assault exception for two weak sides. Pure.

/// Raw points at or below this are zero actual (11.3x).
pub const FLOOR: u32 = 4;
/// Below this raw, both sides of a close assault compare raw as actual.
pub const RAW_AS_ACTUAL: u32 = 10;

/// Actual points from a raw total.
pub fn actual(raw: u32) -> u32 {
    if raw <= FLOOR { 0 } else { (raw + 5) / 10 }
}

/// Both sides' actual points in a close assault or probe (15.51): the
/// floor first, then raw as actual when neither side reaches 10 raw.
pub fn assault_actual(attacker_raw: u32, defender_raw: u32) -> (u32, u32) {
    let floor = |r: u32| if r <= FLOOR { 0 } else { r };
    let (a, d) = (floor(attacker_raw), floor(defender_raw));
    if a < RAW_AS_ACTUAL && d < RAW_AS_ACTUAL {
        (a, d)
    } else {
        (actual(a), actual(d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_nearest_half_up_and_floors_at_four() {
        assert_eq!(actual(84), 8);
        assert_eq!(actual(85), 9);
        assert_eq!(actual(15), 2);
        assert_eq!(actual(14), 1);
        assert_eq!(actual(5), 1);
        assert_eq!(actual(4), 0);
        assert_eq!(actual(0), 0);
    }

    #[test]
    fn two_weak_sides_compare_raw() {
        assert_eq!(assault_actual(6, 8), (6, 8));
        assert_eq!(assault_actual(6, 30), (1, 3));
        assert_eq!(assault_actual(3, 8), (0, 8));
        assert_eq!(assault_actual(9, 10), (1, 1));
    }
}
