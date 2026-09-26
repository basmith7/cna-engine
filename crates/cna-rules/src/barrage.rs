//! Barrage resolution (SPI 12.3, 12.4) as exact dice distributions.

use crate::dice::{P, rolls};
use crate::ruleset::{R009, Ruleset};
pub use cna_data::barrage::Barrage;

/// The band shift for a barrage at a target whose terrain gives
/// `target_shift` (the best single in-hex benefit), fired by the phasing
/// player or not (R-009).
pub fn terrain_shift(rules: &Ruleset, target_shift: i32, firer_phasing: bool) -> i32 {
    match rules.r009 {
        R009::EitherSide => target_shift,
        R009::NonPhasingOnly if firer_phasing => target_shift,
        R009::NonPhasingOnly => 0,
    }
}

/// One barrage against one target, over all 36 readings.
#[derive(Debug, Clone, Copy)]
pub struct BarrageOutcome {
    /// Chance of a pin or a loss.
    pub p_effect: f64,
    /// Expected TOE strength points destroyed.
    pub expected_loss: f64,
    /// Chance of a reading in no cell, or in several.
    pub unresolved: f64,
}

/// `points` actual barrage points against a target of `class`, the band
/// moved by `shift` (negative toward the defender, SPI 12.33). A shift
/// below the lowest band has no effect.
pub fn outcome(t: &Barrage, class: &str, points: u32, shift: i32) -> BarrageOutcome {
    let mut o = BarrageOutcome {
        p_effect: 0.0,
        expected_loss: 0.0,
        unresolved: 0.0,
    };
    let Some(band) = t
        .band_index(points)
        .map(|i| i as i32 + shift)
        .filter(|i| *i >= 0)
    else {
        return o;
    };
    let band = &t.bands[(band as usize).min(t.bands.len() - 1)].id;
    for (a, b) in rolls() {
        let r = a * 10 + b;
        let hits: Vec<&str> = t
            .cells
            .iter()
            .filter(|c| c.class == class && &c.column == band && c.dice.contains(r))
            .map(|c| c.result.as_str())
            .collect();
        match hits.as_slice() {
            ["no-effect"] => {}
            ["pinned"] => o.p_effect += P,
            ["lose-1"] => {
                o.p_effect += P;
                o.expected_loss += P;
            }
            ["lose-2"] => {
                o.p_effect += P;
                o.expected_loss += 2.0 * P;
            }
            _ => o.unresolved += P,
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::rolls;

    fn t() -> Barrage {
        Barrage::load().unwrap()
    }

    #[test]
    fn every_class_and_band_covers_the_36_readings_once() {
        let t = t();
        for c in &t.cells {
            assert!(
                ["no-effect", "pinned", "lose-1", "lose-2"].contains(&c.result.as_str()),
                "unknown result {}",
                c.result
            );
        }
        for class in ["infantry", "armor", "gun", "truck"] {
            for b in &t.bands {
                for (a, d) in rolls() {
                    let r = a * 10 + d;
                    let n = t
                        .cells
                        .iter()
                        .filter(|c| c.class == class && c.column == b.id && c.dice.contains(r))
                        .count();
                    assert_eq!(n, 1, "{class} {} {r}", b.id);
                }
            }
        }
    }

    #[test]
    fn hand_computed_infantry_9_10() {
        // 9-10 infantry: no effect 11-24 (10 readings), pinned 25-61 (21),
        // lose 1 on 62-66 (5).
        let o = outcome(&t(), "infantry", 9, 0);
        assert!((o.p_effect - 26.0 / 36.0).abs() < 1e-12, "{o:?}");
        assert!((o.expected_loss - 5.0 / 36.0).abs() < 1e-12, "{o:?}");
        assert_eq!(o.unresolved, 0.0);
    }

    #[test]
    fn a_shift_moves_whole_bands() {
        let t = t();
        let a = outcome(&t, "infantry", 12, -2);
        let b = outcome(&t, "infantry", 7, 0);
        assert_eq!((a.p_effect, a.expected_loss), (b.p_effect, b.expected_loss));
    }

    #[test]
    fn a_shift_below_the_table_has_no_effect() {
        let o = outcome(&t(), "infantry", 2, -1);
        assert_eq!((o.p_effect, o.expected_loss), (0.0, 0.0));
    }

    #[test]
    fn r009_decides_whose_barrage_is_shifted() {
        use crate::ruleset::{R009, Ruleset};
        let r = |r009| Ruleset {
            r009,
            ..Ruleset::default()
        };
        // (option, firer phasing) -> shift applied to a -2 target
        for (o, phasing, want) in [
            (R009::EitherSide, true, -2),
            (R009::EitherSide, false, -2),
            (R009::NonPhasingOnly, true, -2),
            (R009::NonPhasingOnly, false, 0),
        ] {
            assert_eq!(terrain_shift(&r(o), -2, phasing), want, "{o:?} {phasing}");
        }
    }
}
