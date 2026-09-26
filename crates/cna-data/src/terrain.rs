//! The Terrain Effects Chart (SPI 8.37): only the combat column shifts so
//! far. Errata applied.

use anyhow::Result;
use serde::Deserialize;

/// A column shift: negative favours the defender.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "RawShift")]
pub enum Shift {
    Cols(i32),
    /// No fire of that kind at all.
    Prohibited,
    /// Any other printed entry, kept as read.
    Other(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawShift {
    Cols(i32),
    Text(String),
}

impl From<RawShift> for Shift {
    fn from(r: RawShift) -> Self {
        match r {
            RawShift::Cols(n) => Shift::Cols(n),
            RawShift::Text(t) if t == "prohibited" => Shift::Prohibited,
            RawShift::Text(t) => Shift::Other(t),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Shifts {
    pub barrage: Shift,
    pub anti_armour: Shift,
    pub close_assault: Shift,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RowShifts {
    Own(Shifts),
    /// A reference such as `see-fortifications`.
    Ref(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TerrainRow {
    /// `hex`, `hexside`, `fortification` or `minefield`.
    pub kind: String,
    pub terrain: String,
    pub shifts: Option<RowShifts>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Terrain {
    pub rows: Vec<TerrainRow>,
}

impl Terrain {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            "terrain-effects",
        )?)?)
    }

    /// The row's own shifts; none for a missing row or one that refers
    /// elsewhere.
    pub fn shifts(&self, kind: &str, terrain: &str) -> Option<&Shifts> {
        match self
            .rows
            .iter()
            .find(|r| r.kind == kind && r.terrain == terrain)?
            .shifts
            .as_ref()?
        {
            RowShifts::Own(s) => Some(s),
            RowShifts::Ref(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_are_typed() {
        let t = Terrain::load().unwrap();
        let rough = t.shifts("hex", "rough").unwrap();
        assert_eq!(rough.barrage, Shift::Cols(-1));
        let esc = t.shifts("hexside", "up-escarpment").unwrap();
        assert_eq!(esc.anti_armour, Shift::Prohibited);
        assert_eq!(
            t.shifts("fortification", "fortification-2")
                .unwrap()
                .barrage,
            Shift::Cols(-2)
        );
        // Rows without their own shifts.
        assert!(t.shifts("hex", "major-city").is_none());
        assert!(t.shifts("hex", "railroad").is_none());
    }

    #[test]
    fn every_shift_is_a_number_or_prohibited() {
        let t = Terrain::load().unwrap();
        for r in &t.rows {
            if let Some(RowShifts::Own(s)) = &r.shifts {
                for x in [&s.barrage, &s.anti_armour, &s.close_assault] {
                    assert!(
                        !matches!(x, Shift::Other(_)),
                        "{} {}: {x:?}",
                        r.kind,
                        r.terrain
                    );
                }
            }
        }
    }
}
