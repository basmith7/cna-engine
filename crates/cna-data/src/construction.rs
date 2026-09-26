//! The Construction Chart (SPI 24.17), errata applied: only the costs so
//! far.

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Supplies {
    pub fuel: Option<u32>,
    pub stores: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConstructionRow {
    pub item: String,
    /// `build`, `rebuild-level` and the like.
    pub situation: String,
    #[serde(default)]
    pub supplies: Supplies,
    /// Capability points (supply dumps).
    pub cp: Option<u32>,
    /// Operations Stages.
    pub stages: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Construction {
    pub rows: Vec<ConstructionRow>,
}

impl Construction {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            "construction",
        )?)?)
    }

    pub fn row(&self, item: &str, situation: &str) -> Option<&ConstructionRow> {
        self.rows
            .iter()
            .find(|r| r.item == item && r.situation == situation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_and_repair_rows() {
        let t = Construction::load().unwrap();
        let real = t.row("real-supply-dump", "build").unwrap();
        assert_eq!((real.cp, real.supplies.stores), (Some(3), Some(10)));
        assert_eq!(t.row("fake-supply-dump", "build").unwrap().cp, Some(2));
        let temp = t.row("temporary-repair-facility", "build").unwrap();
        assert_eq!(
            (temp.supplies.fuel, temp.supplies.stores, temp.stages),
            (Some(50), Some(250), Some(1))
        );
        assert!(t.row("no-such-item", "build").is_none());
    }
}
