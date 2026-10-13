//! The map (`vendor/cna/data/map/`): hexes, sheets and adjacency. No game
//! logic lives here; terrain costs stay in `terrain-effects.json`.
//!
//! Adjacency follows cna's `tools/map_geom.py` and the "Hex grid
//! convention" of its `data/README.md`: pointy-top hexes, rows increasing
//! northward, columns eastward, each sheet saying whether its odd rows sit
//! half a hex east or west of the even rows.

use crate::cna_root;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// A hex id as cna writes it: sheet letter (A–E, M = Malta), then row and
/// column as two digits each, e.g. `C3922`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct HexId(String);

impl HexId {
    /// Checks the grammar `^[A-EM][0-9]{4}$`.
    pub fn parse(s: &str) -> Result<HexId> {
        let b = s.as_bytes();
        let ok = b.len() == 5
            && matches!(b[0], b'A'..=b'E' | b'M')
            && b[1..].iter().all(u8::is_ascii_digit);
        if !ok {
            bail!("bad hex id {s:?}");
        }
        Ok(HexId(s.to_owned()))
    }

    /// Builds an id from its parts; none for a row or column outside 0–99.
    fn from_parts(sheet: char, row: i32, col: i32) -> Option<HexId> {
        if !(0..100).contains(&row) || !(0..100).contains(&col) {
            return None;
        }
        Some(HexId(format!("{sheet}{row:02}{col:02}")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn sheet(&self) -> char {
        self.0.as_bytes()[0] as char
    }

    pub fn row(&self) -> u8 {
        self.0[1..3].parse().expect("checked at parse")
    }

    pub fn col(&self) -> u8 {
        self.0[3..5].parse().expect("checked at parse")
    }
}

impl fmt::Display for HexId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for HexId {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self> {
        HexId::parse(&s)
    }
}

impl From<HexId> for String {
    fn from(id: HexId) -> String {
        id.0
    }
}

/// A hex fill of the Terrain Key (cna's `mapTerrain`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Terrain {
    Clear,
    Gravel,
    SaltMarsh,
    HeavyVegetation,
    Rough,
    Mountain,
    Delta,
    Desert,
    MajorCity,
    Swamp,
    Sea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Settlement {
    MajorCity,
    Village,
    Bir,
    Oasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Hex {
    pub id: HexId,
    pub sheet: char,
    pub terrain: Terrain,
    pub settlement: Option<Settlement>,
    pub coastal: bool,
}

/// One map sheet's extent and stagger, from `sheets.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Sheet {
    pub zone: String,
    pub odd_rows_shift: Shift,
    /// Inclusive `[first, last]`.
    pub rows: [i32; 2],
    pub cols: [i32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Shift {
    West,
    East,
}

impl Sheet {
    /// True when `row` sits half a hex east of the rows above and below it.
    fn row_shifted_east(&self, row: i32) -> bool {
        (row % 2 == 1) == (self.odd_rows_shift == Shift::East)
    }
}

#[derive(Debug)]
pub struct Map {
    sheets: BTreeMap<char, Sheet>,
    hexes: BTreeMap<HexId, Hex>,
}

#[derive(Deserialize)]
struct SheetsFile {
    sheets: BTreeMap<char, Sheet>,
}

#[derive(Deserialize)]
struct HexesFile {
    hexes: Vec<Hex>,
}

fn read_json<T: serde::de::DeserializeOwned>(name: &str) -> Result<T> {
    let path = cna_root().join("data/map").join(name);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

impl Map {
    /// Loads `sheets.json` and `hexes.json`. Map corrections are already
    /// applied in those files.
    pub fn load() -> Result<Map> {
        let sheets = read_json::<SheetsFile>("sheets.json")?.sheets;
        let mut hexes = BTreeMap::new();
        for h in read_json::<HexesFile>("hexes.json")?.hexes {
            if !sheets.contains_key(&h.sheet) {
                bail!("{}: sheet {} is not in sheets.json", h.id, h.sheet);
            }
            if hexes.insert(h.id.clone(), h).is_some() {
                bail!("duplicate hex in hexes.json");
            }
        }
        Ok(Map { sheets, hexes })
    }

    pub fn hex(&self, id: &HexId) -> Option<&Hex> {
        self.hexes.get(id)
    }

    /// Every hex, in id order.
    pub fn hexes(&self) -> impl Iterator<Item = &Hex> {
        self.hexes.values()
    }

    pub fn sheet(&self, letter: char) -> Option<&Sheet> {
        self.sheets.get(&letter)
    }

    /// The six grid positions around `id` on its own sheet, whether or not
    /// they are hexes.
    fn grid_neighbours(&self, id: &HexId) -> Vec<HexId> {
        let Some(sheet) = self.sheets.get(&id.sheet()) else {
            return Vec::new();
        };
        let (r, c) = (i32::from(id.row()), i32::from(id.col()));
        let (lo, hi) = if sheet.row_shifted_east(r) {
            (c, c + 1)
        } else {
            (c - 1, c)
        };
        [
            (r, c - 1),
            (r, c + 1),
            (r - 1, lo),
            (r - 1, hi),
            (r + 1, lo),
            (r + 1, hi),
        ]
        .into_iter()
        .filter_map(|(r, c)| HexId::from_parts(id.sheet(), r, c))
        .collect()
    }

    /// The neighbouring hexes that exist, sorted.
    pub fn neighbours(&self, id: &HexId) -> Vec<HexId> {
        let mut n: Vec<HexId> = self
            .grid_neighbours(id)
            .into_iter()
            .filter(|h| self.hexes.contains_key(h))
            .collect();
        n.sort();
        n
    }

    pub fn adjacent(&self, a: &HexId, b: &HexId) -> bool {
        self.hexes.contains_key(a) && self.neighbours(a).binary_search(b).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> Map {
        Map::load().unwrap()
    }

    fn ids(v: &[&str]) -> Vec<HexId> {
        v.iter().map(|s| HexId::parse(s).unwrap()).collect()
    }

    #[test]
    fn all_hexes_load() {
        let m = map();
        assert_eq!(m.hexes().count(), 7073);
        let h = m.hex(&HexId::parse("C3922").unwrap()).unwrap();
        assert_eq!(h.sheet, 'C');
        assert_eq!(h.terrain, Terrain::Clear);
    }

    #[test]
    fn hand_checked_neighbours() {
        let m = map();
        assert_eq!(
            m.neighbours(&HexId::parse("C3922").unwrap()),
            ids(&["C3821", "C3822", "C3921", "C3923", "C4021", "C4022"])
        );
        assert_eq!(
            m.neighbours(&HexId::parse("E1730").unwrap()),
            ids(&["E1629", "E1630", "E1729", "E1731", "E1829", "E1830"])
        );
    }

    #[test]
    fn corner_hex_neighbours_exist_on_its_sheet() {
        let m = map();
        let a0101 = HexId::parse("A0101").unwrap();
        let n = m.neighbours(&a0101);
        assert_eq!(n, ids(&["A0102", "A0201"]));
        for id in &n {
            assert!(m.hex(id).is_some());
            assert_eq!(m.hex(id).unwrap().sheet, 'A');
        }
    }

    #[test]
    fn adjacency_is_symmetric() {
        let m = map();
        for h in m.hexes().step_by(7).take(1000) {
            for n in m.neighbours(&h.id) {
                assert!(m.adjacent(&h.id, &n), "{} -> {n}", h.id);
                assert!(m.adjacent(&n, &h.id), "{n} -> {}", h.id);
                assert!(m.hex(&n).is_some(), "{n} missing");
            }
        }
    }

    #[test]
    fn malformed_ids_are_rejected() {
        for bad in ["", "C392", "C39222", "F0101", "c3922", "C39A2", "3922"] {
            assert!(HexId::parse(bad).is_err(), "{bad:?} accepted");
        }
        let ok = HexId::parse("M0500").unwrap();
        assert_eq!(ok.to_string(), "M0500");
        assert_eq!((ok.sheet(), ok.row(), ok.col()), ('M', 5, 0));
    }

    #[test]
    fn hex_id_round_trips_through_json() {
        let id = HexId::parse("D0701").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"D0701\"");
        assert_eq!(serde_json::from_str::<HexId>(&json).unwrap(), id);
        assert!(serde_json::from_str::<HexId>("\"Z0000\"").is_err());
    }
}
