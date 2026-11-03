//! The three patrol tables (SPI 16.6–16.8): Patrol Survival
//! (`patrol-survival.json`), Reconnaissance (`patrol-reconnaissance.json`)
//! and Objective Loss (`objective-loss.json`).

use crate::close_assault::Range;
use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;

/// A die range's TOE points killed and captured.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct LossRow {
    pub die: Range,
    pub killed: u32,
    pub captured: u32,
}

/// Patrol Survival (16.6) or Objective Loss (16.8).
#[derive(Debug, Clone, Deserialize)]
pub struct LossTable {
    /// Added to the die when every patrol point is recce (survival only).
    #[serde(default)]
    pub recce_only_modifier: i32,
    pub rows: Vec<LossRow>,
}

impl LossTable {
    /// The row for a modified die, if any.
    pub fn row(&self, die: i32) -> Option<LossRow> {
        let die = u8::try_from(die).ok()?;
        self.rows.iter().copied().find(|r| r.die.contains(die))
    }
}

/// A Reconnaissance Table cell: a number of battalion-equivalents, or all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum Revealed {
    Count(u32),
    /// `"all"`.
    All(AllMark),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AllMark {
    All,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReconRow {
    pub die: u8,
    pub units_revealed: BTreeMap<String, Revealed>,
}

/// The Reconnaissance Table (16.7): die × net patrol points (1–3).
#[derive(Debug, Clone, Deserialize)]
pub struct Reconnaissance {
    pub rows: Vec<ReconRow>,
}

impl Reconnaissance {
    /// How many units `points` surviving patrol points reveal on `die`;
    /// `None` for all of them.
    pub fn revealed(&self, die: u8, points: u32) -> Option<u32> {
        if points == 0 {
            return Some(0);
        }
        let row = self.rows.iter().find(|r| r.die == die)?;
        match row.units_revealed.get(&points.min(3).to_string())? {
            Revealed::Count(n) => Some(*n),
            Revealed::All(_) => None,
        }
    }
}

/// The three tables together.
#[derive(Debug, Clone)]
pub struct Patrol {
    pub survival: LossTable,
    pub reconnaissance: Reconnaissance,
    pub objective_loss: LossTable,
}

impl Patrol {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        let table = |name| crate::errata::load_table(src, name);
        Ok(Patrol {
            survival: serde_json::from_value(table("patrol-survival")?)?,
            reconnaissance: serde_json::from_value(table("patrol-reconnaissance")?)?,
            objective_loss: serde_json::from_value(table("objective-loss")?)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_patrol_tables_read_as_printed_16_6_to_16_8() {
        let t = Patrol::load().unwrap();
        let s = |d| t.survival.row(d).map(|r| (r.killed, r.captured));
        assert_eq!(t.survival.recce_only_modifier, -1);
        assert_eq!(s(0), Some((0, 0)));
        assert_eq!(s(3), Some((0, 0)));
        assert_eq!(s(4), Some((0, 1)));
        assert_eq!(s(5), Some((1, 0)));
        assert_eq!(s(6), Some((1, 1)));
        assert_eq!(t.reconnaissance.revealed(1, 1), Some(0));
        assert_eq!(t.reconnaissance.revealed(4, 2), Some(2));
        assert_eq!(t.reconnaissance.revealed(6, 3), None);
        assert_eq!(t.reconnaissance.revealed(6, 0), Some(0));
        let o = |d| t.objective_loss.row(d).map(|r| (r.killed, r.captured));
        assert_eq!(o(4), Some((0, 0)));
        assert_eq!(o(5), Some((0, 1)));
        assert_eq!(o(6), Some((1, 0)));
    }
}
