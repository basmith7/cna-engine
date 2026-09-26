//! Construction costs (SPI 24.9, 24.17).

use crate::ruleset::{R018, Ruleset};
pub use cna_data::construction::Construction;

/// A real dump's CP by the text of SPI 24.9.
pub const TEXT_REAL_DUMP_CP: u32 = 3;
/// A real dump's stores by the text of SPI 24.9.
pub const TEXT_REAL_DUMP_STORES: u32 = 20;
/// A dummy dump's CP by the text of SPI 24.9.
pub const TEXT_DUMMY_DUMP_CP: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DumpCost {
    pub cp: u32,
    pub stores: u32,
}

/// What building a supply dump costs (R-018). Chart figures are read from
/// the Construction Chart; text figures are the constants above.
pub fn dump_cost(rules: &Ruleset, t: &Construction, dummy: bool) -> DumpCost {
    let item = if dummy {
        "fake-supply-dump"
    } else {
        "real-supply-dump"
    };
    let chart = t.row(item, "build").expect("dump row on the chart");
    let chart_cp = chart.cp.expect("dump CP on the chart");
    let chart_stores = chart.supplies.stores.unwrap_or(0);
    let (text_cp, text_stores) = if dummy {
        (TEXT_DUMMY_DUMP_CP, 0)
    } else {
        (TEXT_REAL_DUMP_CP, TEXT_REAL_DUMP_STORES)
    };
    let (cp, stores) = match rules.r018 {
        R018::Text => (text_cp, text_stores),
        R018::Charts => (chart_cp, chart_stores),
        R018::CpChartsStoresText => (chart_cp, text_stores),
    };
    DumpCost { cp, stores }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset::{R018, Ruleset};

    #[test]
    fn r018_decides_the_dump_cost() {
        let t = Construction::load().unwrap();
        let r = |r018| Ruleset {
            r018,
            ..Ruleset::default()
        };
        // (option, dummy) -> (cp, stores)
        for (o, dummy, cp, stores) in [
            (R018::Text, false, 3, 20),
            (R018::Text, true, 3, 0),
            (R018::Charts, false, 3, 10),
            (R018::Charts, true, 2, 0),
            (R018::CpChartsStoresText, false, 3, 20),
            (R018::CpChartsStoresText, true, 2, 0),
        ] {
            assert_eq!(
                dump_cost(&r(o), &t, dummy),
                DumpCost { cp, stores },
                "{o:?} {dummy}"
            );
        }
        assert_eq!(Ruleset::default().r018, R018::CpChartsStoresText);
    }
}
