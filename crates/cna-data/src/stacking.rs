//! The Stacking Point Values chart (`stacking-point-values.json`, SPI 9.4):
//! SP per unit equivalent at full strength.

use crate::characteristics::Equivalent;
use crate::errata::load_table;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    pub equivalent: String,
    pub sp: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StackingPoints {
    pub rows: Vec<Row>,
}

impl StackingPoints {
    pub fn load() -> Result<StackingPoints> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<StackingPoints> {
        Ok(serde_json::from_value(load_table(
            src,
            "stacking-point-values",
        )?)?)
    }

    /// The chart's SP for a characteristics equivalent; none for `hq`
    /// (the rules decide, 9.1x) and `by-unit`.
    pub fn sp(&self, e: Equivalent) -> Option<u32> {
        let name = match e {
            Equivalent::Brigade => "brigade",
            Equivalent::Battalion => "battalion",
            Equivalent::Company => "company",
            Equivalent::Hq | Equivalent::ByUnit => return None,
        };
        self.rows
            .iter()
            .find(|r| r.equivalent == name)
            .map(|r| r.sp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_values() {
        let t = StackingPoints::load().unwrap();
        assert_eq!(t.sp(Equivalent::Brigade), Some(2));
        assert_eq!(t.sp(Equivalent::Battalion), Some(1));
        assert_eq!(t.sp(Equivalent::Company), Some(0));
        assert_eq!(t.sp(Equivalent::Hq), None);
    }
}
