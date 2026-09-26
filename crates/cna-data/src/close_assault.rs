//! The Close Assault Results Table (SPI 15.79), errata applied.

use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Attacker,
    Defender,
}

impl Side {
    pub fn key(self) -> &'static str {
        match self {
            Side::Attacker => "attacker",
            Side::Defender => "defender",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Column {
    pub id: String,
    pub min: Option<i32>,
    pub max: Option<i32>,
    pub et: bool,
    pub overrun: bool,
}

/// An inclusive range of sequential two-dice readings (11..=66).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Range {
    pub from: u8,
    pub to: u8,
}

impl Range {
    pub fn contains(self, reading: u8) -> bool {
        (self.from..=self.to).contains(&reading)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct KnownGap {
    pub side: String,
    pub column: String,
    pub readings: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CloseAssault {
    pub columns: Vec<Column>,
    /// `losses[side][pct][column]`.
    pub losses: BTreeMap<String, BTreeMap<String, BTreeMap<String, Option<Range>>>>,
    /// `sums[line][column]`: the dice sums (2..=12) that land on that line.
    pub sums: BTreeMap<String, BTreeMap<String, Option<Vec<u8>>>>,
    #[serde(default)]
    pub known_gaps: Vec<KnownGap>,
}

impl CloseAssault {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            "close-assault-results",
        )?)?)
    }

    pub fn column_index(&self, id: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e008_is_applied() {
        let t = CloseAssault::load().unwrap();
        let r = t.losses["defender"]["10"]["+4"].unwrap();
        assert_eq!((r.from, r.to), (34, 45)); // printed 24-45, errata E-008
    }

    #[test]
    fn eighteen_columns_in_order() {
        let t = CloseAssault::load().unwrap();
        let ids: Vec<_> = t.columns.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "-11", "-8", "-6", "-4", "-3", "-2", "-1", "0", "+1", "+2", "+3", "+4", "+5", "+7",
                "+9", "+11", "+14", "+17"
            ]
        );
        assert!(t.columns[15].overrun && !t.columns[14].overrun);
    }
}
