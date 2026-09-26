//! Anti-armour fire (SPI 14.3, 14.6) as exact expected damage.

use crate::dice::{P, rolls};
pub use cna_data::anti_armour::AntiArmour;

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
}
