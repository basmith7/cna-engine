//! The Terrain Effects Chart (SPI 8.37): the CP to enter a hex or cross
//! a hexside, the breakdown value, the combat column shifts and the
//! stacking ceiling. Errata applied (E-031: a track halves the hex cost).

use crate::cp::Cp;
use anyhow::Result;
use serde::Deserialize;

/// One CP cell of the chart: a price, or a rule in place of one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "RawCpValue")]
pub enum CpValue {
    Cost(Cp),
    Prohibited,
    /// The unit's whole CPA (an enemy minefield, motorised).
    Cpa,
    Other(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawCpValue {
    Num(f64),
    Text(String),
}

impl From<RawCpValue> for CpValue {
    fn from(r: RawCpValue) -> Self {
        match r {
            RawCpValue::Num(n) => Cp::try_from(n).map_or_else(CpValue::Other, CpValue::Cost),
            RawCpValue::Text(t) if t == "prohibited" => CpValue::Prohibited,
            RawCpValue::Text(t) if t == "cpa" => CpValue::Cpa,
            RawCpValue::Text(t) => CpValue::Other(t),
        }
    }
}

/// The `cp` column: a foot and a motorised price, or a reference such as
/// `as-hex-terrain`, `road-or-railroad-only`, `halves-hex-terrain`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum CpCell {
    Pair { non_mot: CpValue, mot: CpValue },
    Ref(String),
}

impl CpCell {
    /// The price for a unit of one kind; none for a reference cell.
    pub fn for_unit(&self, motorised: bool) -> Option<&CpValue> {
        match self {
            CpCell::Pair { non_mot, mot } => Some(if motorised { mot } else { non_mot }),
            CpCell::Ref(_) => None,
        }
    }
}

/// The breakdown column (SPI 21): points, or a rule (`none`, `halves`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Breakdown {
    Points(f64),
    Rule(String),
}

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
    pub cp: Option<CpCell>,
    pub breakdown: Option<Breakdown>,
    pub shifts: Option<RowShifts>,
    /// The stacking ceiling in SP (9.14); none where the chart prints none.
    pub stacking: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Terrain {
    pub rows: Vec<TerrainRow>,
}

impl Terrain {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            src,
            "terrain-effects",
        )?)?)
    }

    /// The stacking ceiling of a hex terrain (9.14).
    pub fn ceiling(&self, terrain: &str) -> Option<u32> {
        self.row("hex", terrain)?.stacking
    }

    pub fn row(&self, kind: &str, terrain: &str) -> Option<&TerrainRow> {
        self.rows
            .iter()
            .find(|r| r.kind == kind && r.terrain == terrain)
    }

    /// The CP cell of a row; none for a missing row or one with no `cp`.
    pub fn cp(&self, kind: &str, terrain: &str) -> Option<&CpCell> {
        self.row(kind, terrain)?.cp.as_ref()
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
    fn cp_cells_are_typed() {
        let t = Terrain::load().unwrap();
        let price = |kind: &str, name: &str, mot: bool| {
            t.cp(kind, name).unwrap().for_unit(mot).unwrap().clone()
        };
        assert_eq!(price("hex", "clear", false), CpValue::Cost(Cp::whole(2)));
        assert_eq!(price("hex", "rough", true), CpValue::Cost(Cp::whole(4)));
        assert_eq!(price("hex", "road", true), CpValue::Cost(Cp::HALF));
        assert_eq!(price("hexside", "up-escarpment", true), CpValue::Prohibited);
        assert_eq!(
            price("hexside", "up-escarpment", false),
            CpValue::Cost(Cp::whole(6))
        );
        assert_eq!(
            price("hexside", "down-escarpment", true),
            CpValue::Cost(Cp::whole(8))
        );
        assert_eq!(price("minefield", "enemy-minefield", true), CpValue::Cpa);
        // E-031: the track row halves the hex cost instead of pricing it.
        assert_eq!(
            t.cp("hex", "track"),
            Some(&CpCell::Ref("halves-hex-terrain".into()))
        );
        assert_eq!(
            t.cp("hex", "swamp"),
            Some(&CpCell::Ref("road-or-railroad-only".into()))
        );
        assert!(t.cp("hex", "sea").is_none());
        assert_eq!(
            t.row("hex", "desert").unwrap().breakdown,
            Some(Breakdown::Points(24.0))
        );
        assert_eq!(
            t.row("hex", "track").unwrap().breakdown,
            Some(Breakdown::Rule("halves".into()))
        );
    }

    #[test]
    fn every_cp_cell_is_priced_or_a_known_rule() {
        let t = Terrain::load().unwrap();
        for r in &t.rows {
            match &r.cp {
                Some(CpCell::Pair { non_mot, mot }) => {
                    for x in [non_mot, mot] {
                        assert!(
                            !matches!(x, CpValue::Other(_)),
                            "{} {}: {x:?}",
                            r.kind,
                            r.terrain
                        );
                    }
                }
                Some(CpCell::Ref(s)) => assert!(
                    [
                        "as-hex-terrain",
                        "road-or-railroad-only",
                        "halves-hex-terrain"
                    ]
                    .contains(&s.as_str()),
                    "{} {}: {s}",
                    r.kind,
                    r.terrain
                ),
                None => {}
            }
        }
    }

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
