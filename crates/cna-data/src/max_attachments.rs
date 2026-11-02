//! The Maximum Attachment Chart (`max-attachments.json`, SPI 19.5,
//! R-130): how many battalion-equivalents a parent may carry attached
//! beyond its assigned structure, and of what kinds.

use crate::errata::load_table;
use crate::oa::Nation;
use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Turns {
    pub from: u32,
    pub to: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Row {
    pub nation: Nation,
    pub parent: String,
    #[serde(default)]
    pub turns: Option<Turns>,
    pub max_units: u32,
    /// `company` or `artillery`: the only kind the row takes.
    #[serde(default)]
    pub unit_kind: Option<String>,
    /// Caps on a type within `max_units` (`tank`, `infantry`, ...).
    #[serde(default)]
    pub restrictions: BTreeMap<String, u32>,
    /// `each` or `combined` (see the table's notes).
    #[serde(default)]
    pub restriction_mode: Option<String>,
    /// Ways to fill the row with brigades and units (the German
    /// divisions'); empty where the row counts units only.
    #[serde(default)]
    pub alternatives: Vec<Alternative>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Alternative {
    pub brigades: u32,
    pub units: u32,
}

impl Row {
    pub fn covers(&self, turn: u32) -> bool {
        self.turns
            .is_none_or(|t| t.from <= turn && t.to.is_none_or(|to| turn <= to))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MaxAttachments {
    /// Small non-SP company-equivalents any division or brigade may
    /// carry on top of its maximum.
    pub free_company_equivalents: u32,
    pub rows: Vec<Row>,
}

impl MaxAttachments {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(load_table(src, "max-attachments")?)?)
    }

    /// The row among `keys` (an HQ's `attachment_parent`) covering
    /// `turn`.
    pub fn row<'a>(&'a self, nation: Nation, keys: &[String], turn: u32) -> Option<&'a Row> {
        self.rows
            .iter()
            .find(|r| r.nation == nation && keys.contains(&r.parent) && r.covers(turn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oa::Oa;

    #[test]
    fn every_hq_row_names_a_row_of_its_nation() {
        let t = MaxAttachments::load().unwrap();
        let oa = Oa::load_all().unwrap();
        let mut hqs = 0;
        for u in oa.units().filter(|u| !u.attachment_parent.is_empty()) {
            hqs += 1;
            for key in &u.attachment_parent {
                assert!(
                    t.rows
                        .iter()
                        .any(|r| r.nation == u.id.nation() && &r.parent == key),
                    "{}: {key}",
                    u.id
                );
            }
        }
        assert!(hqs > 100, "{hqs}");
    }

    #[test]
    fn the_commonwealth_armoured_division_row_splits_at_turn_68() {
        let t = MaxAttachments::load().unwrap();
        let keys = [
            "armour-division".to_string(),
            "armour-division-late".to_string(),
        ];
        assert_eq!(t.row(Nation::Cw, &keys, 10).unwrap().max_units, 2);
        assert_eq!(t.row(Nation::Cw, &keys, 68).unwrap().max_units, 3);
        assert_eq!(
            t.row(Nation::It, &["battalion".into()], 1)
                .unwrap()
                .max_units,
            0
        );
        assert_eq!(t.free_company_equivalents, 2);
    }
}
