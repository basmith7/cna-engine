//! The Morale Modifier Table (SPI 17.4), errata applied.

use crate::close_assault::Range;
use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;

/// One cohesion row: `modifier` maps `+4` … `-4` and `surrender` to the
/// sequential readings that give it, or null.
#[derive(Debug, Clone, Deserialize)]
pub struct MoraleRow {
    pub level: i32,
    pub modifier: BTreeMap<String, Option<Range>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoraleGap {
    pub level: i32,
    pub readings: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MoraleModifier {
    /// From +8 (and better) down to −17 (and worse).
    pub rows: Vec<MoraleRow>,
    #[serde(default)]
    pub known_gaps: Vec<MoraleGap>,
}

impl MoraleModifier {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            "morale-modifier",
        )?)?)
    }

    /// The row a cohesion level reads on, clamped to the end rows (SPI 17.24).
    pub fn row(&self, level: i32) -> &MoraleRow {
        let top = self.rows.iter().map(|r| r.level).max().expect("rows");
        let bottom = self.rows.iter().map(|r| r.level).min().expect("rows");
        let level = level.clamp(bottom, top);
        self.rows
            .iter()
            .find(|r| r.level == level)
            .expect("a row per level")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_clamping() {
        let t = MoraleModifier::load().unwrap();
        assert_eq!(t.rows.len(), 26);
        assert_eq!(t.row(12).level, 8);
        assert_eq!(t.row(-40).level, -17);
        assert_eq!(t.row(0).level, 0);
        assert_eq!(t.known_gaps[0].level, -4);
    }
}
