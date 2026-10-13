//! The Off-Map Land Unit Movement Distance Chart
//! (`off-map-distances.json`, SPI 8.89): OpStages of continuous movement
//! between the Tunis, Gabes, Tripoli and Tripolitania boxes and Nofilia
//! on sheet A, by the unit's CPA band.

use crate::errata::load_table;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// An end of an off-map journey: a box, or the map at Nofilia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OffMapPlace {
    Tunis,
    Gabes,
    Tripoli,
    Tripolitania,
    Nofilia,
}

impl fmt::Display for OffMapPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OffMapPlace::Tunis => "Tunis",
            OffMapPlace::Gabes => "Gabes",
            OffMapPlace::Tripoli => "Tripoli",
            OffMapPlace::Tripolitania => "Tripolitania",
            OffMapPlace::Nofilia => "Nofilia",
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Band {
    min: Option<u32>,
    max: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
struct Row {
    a: OffMapPlace,
    b: OffMapPlace,
    stages: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OffMapDistances {
    cpa_bands: Vec<Band>,
    rows: Vec<Row>,
}

impl OffMapDistances {
    pub fn load() -> Result<OffMapDistances> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<OffMapDistances> {
        serde_json::from_value(load_table(src, "off-map-distances")?)
            .context("parsing off-map-distances")
    }

    /// The chart's column for a CPA: the band it falls in, or, in the
    /// gap the chart leaves between its printed bands (21–24), the next
    /// slower band.
    fn band(&self, cpa: u32) -> Option<usize> {
        let fits = |b: &Band| b.min.is_none_or(|m| cpa >= m) && b.max.is_none_or(|m| cpa <= m);
        if let Some(i) = self.cpa_bands.iter().position(fits) {
            return Some(i);
        }
        self.cpa_bands
            .iter()
            .position(|b| b.max.is_some_and(|m| cpa > m))
    }

    /// OpStages of continuous movement between two places for a unit of
    /// this CPA; none for the same place or a pair the chart lacks.
    pub fn stages(&self, a: OffMapPlace, b: OffMapPlace, cpa: u32) -> Option<u8> {
        let column = self.band(cpa)?;
        self.rows
            .iter()
            .find(|r| (r.a == a && r.b == b) || (r.a == b && r.b == a))
            .and_then(|r| r.stages.get(column).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_by_pair_and_cpa_band() {
        let t = OffMapDistances::load().unwrap();
        use OffMapPlace::*;
        assert_eq!(t.stages(Tripoli, Tripolitania, 30), Some(1));
        assert_eq!(t.stages(Tripolitania, Tripoli, 25), Some(1));
        assert_eq!(t.stages(Tripoli, Tripolitania, 10), Some(3));
        assert_eq!(t.stages(Nofilia, Tunis, 18), Some(5));
        assert_eq!(t.stages(Nofilia, Tunis, 14), Some(14));
        assert_eq!(t.stages(Tripolitania, Nofilia, 20), Some(1));
        // The chart's gap: 22 reads as the 15–20 column.
        assert_eq!(t.stages(Tripoli, Tunis, 22), Some(3));
        assert_eq!(t.stages(Tripoli, Tripoli, 30), None);
    }
}
