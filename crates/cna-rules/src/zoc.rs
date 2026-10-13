//! Zones of control (SPI 10.1x, 10.2x): whether a hex's contents can
//! exert one, and which hexsides none reaches across. Pure.

use cna_data::map::{Hexside, HexsideFeature};

/// A hex exerts a ZOC when it holds more than 1 SP of combat units, not
/// collapsed, with at least this many raw close-assault defence points
/// (10.11–10.15).
pub const MIN_DEFENCE: u32 = 10;

/// Whether combat units of `sp` stacking points and `raw_defence`
/// defence points, none collapsed, exert a ZOC (10.1x). Collapsed units
/// (−26, R-001) are left out of both sums by the caller.
pub fn exerts(sp: u32, raw_defence: u32) -> bool {
    sp > 1 && raw_defence >= MIN_DEFENCE
}

/// Whether a ZOC reaches across this hexside (10.21): not across a
/// major river, lake or escarpment. A plain hexside is `None`.
pub fn crosses(hexside: Option<&Hexside>) -> bool {
    !hexside.is_some_and(|hs| {
        hs.has(HexsideFeature::MajorRiver)
            || hs.has(HexsideFeature::Lake)
            || hs.has(HexsideFeature::Escarpment)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_more_than_one_sp_and_ten_points() {
        assert!(exerts(2, 10));
        assert!(!exerts(1, 30));
        assert!(!exerts(2, 9));
        assert!(!exerts(0, 0));
    }

    #[test]
    fn blocked_hexsides() {
        let hs = |f: HexsideFeature| Hexside {
            features: vec![f],
            up: None,
        };
        assert!(crosses(None));
        assert!(crosses(Some(&hs(HexsideFeature::Slope))));
        assert!(crosses(Some(&hs(HexsideFeature::Wadi))));
        assert!(!crosses(Some(&hs(HexsideFeature::Escarpment))));
        assert!(!crosses(Some(&hs(HexsideFeature::MajorRiver))));
        assert!(!crosses(Some(&hs(HexsideFeature::Lake))));
    }
}
