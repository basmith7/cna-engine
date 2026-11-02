//! The Formation Organisation Charts (`formation-organisation.json`,
//! SPI 19.31-19.33): what each parent formation is built of, and which
//! of those components count toward a shell (9.26, R-141).

use crate::errata::load_table;
use crate::max_attachments::Turns;
use crate::oa::Nation;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Component {
    pub count: u32,
    /// The counter kind; `None` where the chart gives a choice
    /// (`any_of`).
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub any_of: Vec<String>,
    /// The printed stacking points; `None` where none is printed.
    pub sp: Option<u32>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    /// One of the brigade-equivalents or battalions 9.26 counts (R-141).
    pub counts_for_shell: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Row {
    pub nation: Nation,
    /// The key an OA HQ names in its `organisation`.
    pub parent: String,
    pub sp: u32,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub turns: Option<Turns>,
    pub components: Vec<Component>,
}

impl Row {
    pub fn covers(&self, turn: u32) -> bool {
        self.turns
            .is_none_or(|t| t.from <= turn && t.to.is_none_or(|to| turn <= to))
    }

    /// How many components 9.26 counts (R-141): each counted component
    /// `count` times; an `any_of` slot once per count, whichever unit
    /// fills it.
    pub fn shell_count(&self) -> u32 {
        self.components
            .iter()
            .filter(|c| c.counts_for_shell)
            .map(|c| c.count)
            .sum()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Organisation {
    pub rows: Vec<Row>,
}

impl Organisation {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(load_table(
            src,
            "formation-organisation",
        )?)?)
    }

    /// The row keyed `parent` (an HQ's `organisation`) covering `turn`.
    pub fn row(&self, nation: Nation, parent: &str, turn: u32) -> Option<&Row> {
        self.rows
            .iter()
            .find(|r| r.nation == nation && r.parent == parent && r.covers(turn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oa::Oa;

    #[test]
    fn every_hq_organisation_names_a_row_of_its_nation() {
        let t = Organisation::load().unwrap();
        let oa = Oa::load_all().unwrap();
        let mut hqs = 0;
        for u in oa.units() {
            let Some(key) = &u.organisation else { continue };
            hqs += 1;
            assert!(
                t.rows
                    .iter()
                    .any(|r| r.nation == u.id.nation() && &r.parent == key),
                "{}: {key}",
                u.id
            );
        }
        assert!(hqs > 100, "{hqs}");
    }

    #[test]
    fn rows_of_one_parent_never_overlap_in_turns() {
        let t = Organisation::load().unwrap();
        for a in &t.rows {
            for turn in 1..=111 {
                let n = t
                    .rows
                    .iter()
                    .filter(|b| b.nation == a.nation && b.parent == a.parent && b.covers(turn))
                    .count();
                assert!(n <= 1, "{:?} {} turn {turn}", a.nation, a.parent);
            }
        }
    }

    #[test]
    fn the_counted_components_follow_r141() {
        let t = Organisation::load().unwrap();
        let count = |n, p: &str, turn| t.row(n, p, turn).unwrap().shell_count();
        // A Commonwealth infantry division its three brigades; an
        // armoured division three, then two from turn 71.
        assert_eq!(count(Nation::Cw, "infantry-division", 1), 3);
        assert_eq!(count(Nation::Cw, "armoured-division", 30), 3);
        assert_eq!(count(Nation::Cw, "armoured-division", 71), 2);
        assert_eq!(count(Nation::Cw, "infantry-brigade-3sp", 1), 5);
        assert_eq!(count(Nation::De, "5-leichte-panzer-division", 1), 1);
        assert_eq!(count(Nation::De, "15-infantry-brigade", 1), 3);
        // Armoured brigade type II applies in two periods, as two rows.
        let kind = |turn| {
            t.row(Nation::Cw, "armoured-brigade", turn)
                .unwrap()
                .kind
                .clone()
        };
        assert_eq!(kind(80).as_deref(), Some("III"));
        assert_eq!(kind(95).as_deref(), Some("II"));
        // Never a component of 0 SP or with no printed figure.
        for r in &t.rows {
            for c in r.components.iter().filter(|c| c.counts_for_shell) {
                let floor = if r.sp >= 5 { 2 } else { 1 };
                assert!(c.sp.is_some_and(|sp| sp >= floor), "{}: {c:?}", r.parent);
            }
        }
    }
}
