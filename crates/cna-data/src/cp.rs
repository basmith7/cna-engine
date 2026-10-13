//! Capability points (SPI 6.1x) as an exact quantity: the Terrain Effects
//! Chart prices a motorised unit on a road at ½ CP, so a CP figure is
//! kept in half points, never as a float.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Sub};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(try_from = "f64", into = "f64")]
pub struct Cp(u32);

impl Cp {
    pub const ZERO: Cp = Cp(0);
    pub const HALF: Cp = Cp(1);

    pub const fn whole(n: u32) -> Cp {
        Cp(n * 2)
    }

    pub const fn halves(n: u32) -> Cp {
        Cp(n)
    }

    pub fn in_halves(self) -> u32 {
        self.0
    }

    /// Whole points, any half dropped.
    pub fn floor(self) -> u32 {
        self.0 / 2
    }

    /// Half of this, rounded up to the next half point.
    pub fn halved(self) -> Cp {
        Cp(self.0.div_ceil(2))
    }

    pub fn doubled(self) -> Cp {
        Cp(self.0 * 2)
    }

    pub fn saturating_sub(self, other: Cp) -> Cp {
        Cp(self.0.saturating_sub(other.0))
    }
}

impl Add for Cp {
    type Output = Cp;
    fn add(self, o: Cp) -> Cp {
        Cp(self.0 + o.0)
    }
}

impl AddAssign for Cp {
    fn add_assign(&mut self, o: Cp) {
        self.0 += o.0;
    }
}

impl Sub for Cp {
    type Output = Cp;
    fn sub(self, o: Cp) -> Cp {
        Cp(self.0 - o.0)
    }
}

impl Sum for Cp {
    fn sum<I: Iterator<Item = Cp>>(iter: I) -> Cp {
        iter.fold(Cp::ZERO, Add::add)
    }
}

impl From<Cp> for f64 {
    fn from(c: Cp) -> f64 {
        f64::from(c.0) / 2.0
    }
}

impl TryFrom<f64> for Cp {
    type Error = String;
    fn try_from(v: f64) -> Result<Cp, String> {
        let halves = v * 2.0;
        if !(0.0..=f64::from(u32::MAX)).contains(&halves) || halves.fract() != 0.0 {
            return Err(format!("{v}: not a non-negative multiple of ½ CP"));
        }
        Ok(Cp(halves as u32))
    }
}

impl fmt::Display for Cp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.0 / 2, self.0 % 2) {
            (0, 1) => f.write_str("½"),
            (n, 0) => write!(f, "{n}"),
            (n, _) => write!(f, "{n}½"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_are_exact() {
        assert_eq!(Cp::whole(2) + Cp::HALF, Cp::halves(5));
        assert_eq!(Cp::whole(3).halved(), Cp::halves(3));
        assert_eq!(Cp::halves(3).halved(), Cp::whole(1));
        assert_eq!(Cp::halves(5).floor(), 2);
        assert_eq!(Cp::halves(5).to_string(), "2½");
        assert_eq!(Cp::HALF.to_string(), "½");
        assert_eq!(Cp::whole(4).to_string(), "4");
    }

    #[test]
    fn json_is_a_number_of_points() {
        assert_eq!(serde_json::to_string(&Cp::halves(5)).unwrap(), "2.5");
        assert_eq!(serde_json::to_string(&Cp::whole(2)).unwrap(), "2.0");
        assert_eq!(serde_json::from_str::<Cp>("0.5").unwrap(), Cp::HALF);
        assert_eq!(serde_json::from_str::<Cp>("3").unwrap(), Cp::whole(3));
        assert!(serde_json::from_str::<Cp>("0.3").is_err());
        assert!(serde_json::from_str::<Cp>("-1").is_err());
    }
}
