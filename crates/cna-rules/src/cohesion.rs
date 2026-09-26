//! Cohesion (SPI 6.2–6.26): one signed level per unit, lowered by DP and
//! raised by RP the moment they are earned.

use crate::ruleset::{R001, Ruleset};

/// No cohesion level may exceed this (SPI 6.23).
pub const MAX_LEVEL: i32 = 10;
/// Rest earns this many RP (SPI 6.24).
pub const REST_RP: i32 = 5;
/// A victory earns this many RP (SPI 6.24).
pub const VICTORY_RP: i32 = 3;
/// The collapse threshold, a level or a DP count by R-001 (SPI 6.26).
pub const COLLAPSE: i32 = 26;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Unit {
    pub level: i32,
    /// Every DP ever earned, never reduced: the count R-001 option 2 needs.
    /// The rules give it no reset, so none is invented.
    pub dp_tally: u32,
}

impl Unit {
    pub fn earn_dp(&mut self, n: u32) {
        self.level -= n as i32;
        self.dp_tally += n;
    }

    fn earn_rp(&mut self, n: i32, ceiling: i32) {
        if self.level < ceiling {
            self.level = (self.level + n).min(ceiling);
        }
    }

    /// A whole Operations Stage without spending CP; never lifts above 0.
    pub fn rest(&mut self) {
        self.earn_rp(REST_RP, 0);
    }

    pub fn victory(&mut self) {
        self.earn_rp(VICTORY_RP, MAX_LEVEL);
    }

    /// Barred from moving, attacking or defending, and surrendering to any
    /// adjacent enemy combat unit (R-001).
    pub fn collapsed(&self, rules: &Ruleset) -> bool {
        match rules.r001 {
            R001::CohesionLevel => self.level <= -COLLAPSE,
            R001::DpTally => self.dp_tally >= COLLAPSE as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset::{R001, Ruleset};

    fn at(level: i32) -> Unit {
        Unit { level, dp_tally: 0 }
    }

    #[test]
    fn rest_never_lifts_above_zero() {
        let mut u = at(-1);
        u.rest();
        assert_eq!(u.level, 0);
        let mut u = at(-8);
        u.rest();
        assert_eq!(u.level, -3);
        let mut u = at(2);
        u.rest();
        assert_eq!(u.level, 2);
    }

    #[test]
    fn victory_caps_at_ten() {
        let mut u = at(9);
        u.victory();
        assert_eq!(u.level, 10);
    }

    #[test]
    fn rp_raise_the_level_but_not_the_tally() {
        let mut u = Unit::default();
        u.earn_dp(4);
        assert_eq!((u.level, u.dp_tally), (-4, 4));
        u.rest();
        assert_eq!((u.level, u.dp_tally), (0, 4));
    }

    #[test]
    fn r001_decides_what_collapses() {
        let one = Ruleset::default();
        let two = Ruleset {
            r001: R001::DpTally,
            ..one
        };
        let deep = Unit {
            level: -26,
            dp_tally: 25,
        };
        let worn = Unit {
            level: -3,
            dp_tally: 26,
        };
        assert!(deep.collapsed(&one) && !deep.collapsed(&two));
        assert!(!worn.collapsed(&one) && worn.collapsed(&two));
    }
}
