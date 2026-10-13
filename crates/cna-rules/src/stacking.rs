//! Stacking points of a piece (SPI 9.1x, 9.4).

use cna_data::characteristics::Equivalent;
use cna_data::stacking::StackingPoints;

/// The SP a counter counts toward the terrain ceiling: the chart's value
/// for its equivalent; an HQ with nothing attached is 0 (9.25), as is a
/// counter with no equivalent (supply units, 9.11).
pub fn points(chart: &StackingPoints, equivalent: Option<Equivalent>) -> u32 {
    equivalent.and_then(|e| chart.sp(e)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hq_and_none_are_zero() {
        let chart = StackingPoints::load().unwrap();
        assert_eq!(points(&chart, Some(Equivalent::Hq)), 0);
        assert_eq!(points(&chart, None), 0);
        assert_eq!(points(&chart, Some(Equivalent::Brigade)), 2);
    }
}
