//! The Anti-Armour Combat Results Table (SPI 14.6), errata applied.

use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Column {
    pub id: String,
}

/// One dice-pair row: `"11"` covers readings 11 and 12, `"13"` 13 and 14.
#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    pub dice: String,
    /// Damage points by column id; null is none.
    pub damage: BTreeMap<String, Option<u32>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AntiArmour {
    /// Actual anti-armour points `0` to `16+`, in order.
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

impl AntiArmour {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            "anti-armour-results",
        )?)?)
    }

    /// The row a sequential reading (11..=66) falls in.
    pub fn row_index(reading: u8) -> usize {
        let (tens, units) = (reading / 10, reading % 10);
        (tens as usize - 1) * 3 + (units as usize - 1) / 2
    }

    pub fn damage(&self, row: usize, column: usize) -> u32 {
        self.rows[row].damage[&self.columns[column].id].unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_and_damage() {
        let t = AntiArmour::load().unwrap();
        assert_eq!(t.rows.len(), 18);
        assert_eq!(t.columns.len(), 17);
        assert_eq!(AntiArmour::row_index(11), 0);
        assert_eq!(AntiArmour::row_index(12), 0);
        assert_eq!(AntiArmour::row_index(13), 1);
        assert_eq!(AntiArmour::row_index(66), 17);
        assert_eq!(t.damage(0, 16), 22);
        assert_eq!(t.damage(0, 0), 0); // null
        assert_eq!(t.damage(0, 4), 2);
    }
}
