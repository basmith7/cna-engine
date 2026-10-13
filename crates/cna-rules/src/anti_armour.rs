//! Anti-armour fire (SPI 14.3, 14.6) as exact expected damage.

use crate::dice::{P, rolls};
use crate::ruleset::{R015, Ruleset};
pub use cna_data::anti_armour::AntiArmour;

/// The column shift for anti-armour fire (R-015). For phasing fire the
/// target hex is the defended hex; for non-phasing fire it is the hex the
/// assaulting armour stands in. `hexside_shift` is the hexside the phasing
/// points assault across. In-hex effects are the best single one; a
/// hexside adds to it (SPI 14.32).
pub fn terrain_shift(
    rules: &Ruleset,
    firer_phasing: bool,
    target_hex_shift: i32,
    hexside_shift: i32,
) -> i32 {
    if firer_phasing {
        return target_hex_shift + hexside_shift;
    }
    match rules.r015 {
        R015::OwnHexBoth => target_hex_shift,
        R015::PhasingOnly => 0,
        R015::HexAndHexsideBoth => target_hex_shift + hexside_shift,
    }
}

/// The column `points` actual anti-armour points read after `shift`
/// (below column 0 reads column 0, SPI 14.33).
fn column(t: &AntiArmour, points: u32, shift: i32) -> usize {
    let last = t.columns.len() as i32 - 1;
    (points.min(last as u32) as i32 + shift).clamp(0, last) as usize
}

/// Damage points on one sequential reading (11..=66): the game's
/// sampling entry point. The phasing player reads one row lower than
/// rolled, except on the lowest row (SPI 14.6).
pub fn damage_for(t: &AntiArmour, points: u32, shift: i32, phasing: bool, reading: u8) -> u32 {
    let row = AntiArmour::row_index(reading);
    let row = if phasing { row.saturating_sub(1) } else { row };
    t.damage(row, column(t, points, shift))
}

/// Expected damage points from `points` actual anti-armour points, the
/// column moved by `shift` (negative toward the defender; below column 0
/// reads column 0, SPI 14.33). The phasing player reads one row lower than
/// rolled, except on the lowest row (SPI 14.6).
pub fn expected_damage(t: &AntiArmour, points: u32, shift: i32, phasing: bool) -> f64 {
    let last = t.columns.len() as i32 - 1;
    let column = (points.min(last as u32) as i32 + shift).clamp(0, last) as usize;
    rolls()
        .map(|(a, b)| {
            let row = AntiArmour::row_index(a * 10 + b);
            let row = if phasing { row.saturating_sub(1) } else { row };
            P * t.damage(row, column) as f64
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> AntiArmour {
        AntiArmour::load().unwrap()
    }

    #[test]
    fn hand_computed_16_plus_non_phasing() {
        // Sum of the 16+ column over the 18 rows, each row two readings.
        let d = expected_damage(&t(), 16, 0, false);
        assert!((d - 952.0 / 36.0).abs() < 1e-9, "{d}");
        assert_eq!(expected_damage(&t(), 30, 0, false), d);
    }

    #[test]
    fn phasing_reads_a_row_lower_and_never_does_better() {
        let t = t();
        for p in 0..=16 {
            assert!(expected_damage(&t, p, 0, true) <= expected_damage(&t, p, 0, false));
        }
        // Row 0 stays put: at 16+ the phasing player loses exactly the
        // difference between the top row and row 0, over 2 readings each.
        let diff = expected_damage(&t, 16, 0, false) - expected_damage(&t, 16, 0, true);
        let expected = (t.damage(17, 16) as f64 - t.damage(0, 16) as f64) * 2.0 / 36.0;
        assert!((diff - expected).abs() < 1e-9, "{diff} {expected}");
    }

    #[test]
    fn a_shift_below_zero_reads_column_0() {
        let t = t();
        assert_eq!(
            expected_damage(&t, 5, -20, false),
            expected_damage(&t, 0, 0, false)
        );
        assert_eq!(
            expected_damage(&t, 9, -1, false),
            expected_damage(&t, 8, 0, false)
        );
    }

    #[test]
    fn damage_for_agrees_with_the_expectation() {
        let t = t();
        for points in [0, 3, 5, 9, 16, 30] {
            for shift in [-3, 0, 2] {
                for phasing in [false, true] {
                    let mean: f64 = crate::dice::rolls()
                        .map(|(a, b)| P * damage_for(&t, points, shift, phasing, a * 10 + b) as f64)
                        .sum();
                    let exact = expected_damage(&t, points, shift, phasing);
                    assert!((mean - exact).abs() < 1e-9, "{points} {shift} {phasing}");
                }
            }
        }
    }

    #[test]
    fn ten_thousand_seeded_samples_match_the_expectation() {
        use crate::dice::sample;
        let t = t();
        let exact = expected_damage(&t, 9, 0, false);
        let mut rng = sample::rng(14);
        let n = 10_000;
        let total: u32 = (0..n)
            .map(|_| damage_for(&t, 9, 0, false, sample::reading(&mut rng)))
            .sum();
        assert!((total as f64 / n as f64 - exact).abs() < 0.2, "{exact}");
        let mut rng = sample::rng(14);
        let r = sample::reading(&mut rng);
        assert_eq!(r, 55);
        assert_eq!(
            damage_for(&t, 9, 0, false, r),
            t.damage(AntiArmour::row_index(55), 9)
        );
    }

    #[test]
    fn r015_decides_which_fire_terrain_weakens() {
        use crate::ruleset::{R015, Ruleset};
        let r = |r015| Ruleset {
            r015,
            ..Ruleset::default()
        };
        // (option, firer phasing) -> shift, for a -1 target hex and a -1 hexside
        for (o, phasing, want) in [
            (R015::OwnHexBoth, true, -2),
            (R015::OwnHexBoth, false, -1),
            (R015::PhasingOnly, true, -2),
            (R015::PhasingOnly, false, 0),
            (R015::HexAndHexsideBoth, true, -2),
            (R015::HexAndHexsideBoth, false, -2),
        ] {
            assert_eq!(
                terrain_shift(&r(o), phasing, -1, -1),
                want,
                "{o:?} {phasing}"
            );
        }
    }
}
