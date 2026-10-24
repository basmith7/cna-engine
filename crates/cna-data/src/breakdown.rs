//! The Breakdown Table (`breakdown.json`, SPI 21.38) and the breakdown
//! adjustment ratings (`breakdown-adjustments.json`, SPI 21.14).

use crate::barrage::Band;
use crate::close_assault::Range;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Cell {
    pub column: String,
    /// The percentage of the group's TOE points that break down.
    pub percent: u32,
    pub dice: Range,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BreakdownTable {
    #[serde(rename = "columns")]
    pub bands: Vec<Band>,
    #[serde(rename = "rows")]
    pub cells: Vec<Cell>,
}

/// One vehicle type's BAR (21.14): columns left (negative) or right; a
/// range where the model's own rating on the characteristics charts
/// decides.
#[derive(Debug, Clone, Deserialize)]
pub struct Adjustment {
    pub vehicle: String,
    pub shift: i32,
    #[serde(default)]
    pub shift_max: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct AdjustmentFile {
    rows: Vec<Adjustment>,
}

#[derive(Debug, Clone)]
pub struct Breakdown {
    pub table: BreakdownTable,
    pub adjustments: Vec<Adjustment>,
}

impl Breakdown {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        let table = serde_json::from_value(crate::errata::load_table(src, "breakdown")?)?;
        let adj: AdjustmentFile =
            serde_json::from_value(crate::errata::load_table(src, "breakdown-adjustments")?)?;
        Ok(Breakdown {
            table,
            adjustments: adj.rows,
        })
    }

    /// The column index for `points` accumulated (21.31: fractions are
    /// rounded up before this).
    pub fn band_index(&self, points: u32) -> Option<usize> {
        self.table
            .bands
            .iter()
            .position(|b| points >= b.min && b.max.is_none_or(|m| points <= m))
    }

    pub fn adjustment(&self, vehicle: &str) -> Option<&Adjustment> {
        self.adjustments.iter().find(|a| a.vehicle == vehicle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_columns_and_every_reading_once_per_column() {
        let t = Breakdown::load().unwrap();
        assert_eq!(t.table.bands.len(), 9);
        assert_eq!(t.band_index(3), Some(0));
        assert_eq!(t.band_index(4), Some(1));
        assert_eq!(t.band_index(35), Some(4));
        assert_eq!(t.band_index(500), Some(8));
        for band in &t.table.bands {
            for a in 1..=6u8 {
                for b in 1..=6u8 {
                    let reading = a * 10 + b;
                    let hits = t
                        .table
                        .cells
                        .iter()
                        .filter(|c| c.column == band.id && c.dice.contains(reading))
                        .count();
                    assert_eq!(hits, 1, "{} at {reading}", band.id);
                }
            }
        }
        assert_eq!(t.adjustment("truck").unwrap().shift, -2);
    }
}
