//! The Raid on Rommel Table (`raid-on-rommel.json`, SPI 27.92): two dice
//! when a Commonwealth LRDG raids Rommel's hex (27.6x).

use crate::errata::load_table;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Dice {
    pub from: u8,
    pub to: u8,
}

/// What a raid on Rommel does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RaidResult {
    /// The Axis initiative rating is 3 for the row's `turns`.
    #[serde(rename = "axis-initiative-3-temporary")]
    AxisInitiative3Temporary,
    /// The LRDG is eliminated, unless Rommel's hex held no combat or HQ
    /// unit (the table's `lrdg_eliminated_unless_hex_empty`).
    LrdgEliminated,
    None,
    /// Rommel is removed for good.
    RommelRemoved,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    pub dice: Dice,
    pub result: RaidResult,
    #[serde(default)]
    pub turns: Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaidOnRommel {
    pub lrdg_eliminated_unless_hex_empty: bool,
    pub rows: Vec<Row>,
}

impl RaidOnRommel {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(load_table(src, "raid-on-rommel")?)?)
    }

    /// The row for a two-dice total.
    pub fn row(&self, total: u8) -> Option<&Row> {
        self.rows
            .iter()
            .find(|r| (r.dice.from..=r.dice.to).contains(&total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_total_from_two_to_twelve_has_a_row() {
        let t = RaidOnRommel::load().unwrap();
        let result = |n| t.row(n).unwrap().result;
        assert_eq!(result(2), RaidResult::AxisInitiative3Temporary);
        assert_eq!(t.row(2).unwrap().turns, Some(2));
        for n in [3, 9, 11] {
            assert_eq!(result(n), RaidResult::LrdgEliminated, "{n}");
        }
        for n in [4, 5, 6, 7, 8, 10] {
            assert_eq!(result(n), RaidResult::None, "{n}");
        }
        assert_eq!(result(12), RaidResult::RommelRemoved);
        assert!(t.row(13).is_none());
        assert!(t.lrdg_eliminated_unless_hex_empty);
    }
}
