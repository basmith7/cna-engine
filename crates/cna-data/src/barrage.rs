//! The Barrage Results Table (SPI 12.6), errata applied.

use crate::close_assault::Range;
use anyhow::Result;
use serde::Deserialize;

/// A barrage-point band, such as `11-12` or `17+`.
#[derive(Debug, Clone, Deserialize)]
pub struct Band {
    pub id: String,
    pub min: u32,
    pub max: Option<u32>,
}

/// One cell: `class` (`infantry`, `armor`, `gun`, `truck`) in band `column`
/// gets `result` (`no-effect`, `pinned`, `lose-1`, `lose-2`) on `dice`.
#[derive(Debug, Clone, Deserialize)]
pub struct Cell {
    pub class: String,
    pub column: String,
    pub result: String,
    pub dice: Range,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Barrage {
    #[serde(rename = "columns")]
    pub bands: Vec<Band>,
    #[serde(rename = "rows")]
    pub cells: Vec<Cell>,
}

impl Barrage {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            "barrage-results",
        )?)?)
    }

    /// The band holding `points`, or none for zero points.
    pub fn band_index(&self, points: u32) -> Option<usize> {
        self.bands
            .iter()
            .position(|b| points >= b.min && b.max.is_none_or(|m| points <= m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands() {
        let t = Barrage::load().unwrap();
        assert_eq!(t.band_index(0), None);
        assert_eq!(t.bands[t.band_index(12).unwrap()].id, "11-12");
        assert_eq!(t.bands[t.band_index(40).unwrap()].id, "17+");
        assert_eq!(t.band_index(1), Some(0));
    }

    #[test]
    fn cells_are_typed() {
        let t = Barrage::load().unwrap();
        let c = t
            .cells
            .iter()
            .find(|c| c.class == "infantry" && c.column == "9-10" && c.result == "lose-1")
            .unwrap();
        assert_eq!((c.dice.from, c.dice.to), (62, 66));
    }
}
