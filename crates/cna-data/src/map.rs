//! The map (`vendor/cna/data/map/`): hexes, sheets and adjacency. No game
//! logic lives here; terrain costs stay in `terrain-effects.json`.
//!
//! Adjacency follows cna's `tools/map_geom.py` and the "Hex grid
//! convention" of its `data/README.md`: pointy-top hexes, rows increasing
//! northward, columns eastward, each sheet saying whether its odd rows sit
//! half a hex east or west of the even rows.

use crate::source::{Fs, Source};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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

impl Terrain {
    /// The kebab-case name, as `terrain-effects.json` keys its rows.
    pub fn name(self) -> &'static str {
        match self {
            Terrain::Clear => "clear",
            Terrain::Gravel => "gravel",
            Terrain::SaltMarsh => "salt-marsh",
            Terrain::HeavyVegetation => "heavy-vegetation",
            Terrain::Rough => "rough",
            Terrain::Mountain => "mountain",
            Terrain::Delta => "delta",
            Terrain::Desert => "desert",
            Terrain::MajorCity => "major-city",
            Terrain::Swamp => "swamp",
            Terrain::Sea => "sea",
        }
    }
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
    pub odd_rows_shift: RowShift,
    /// Inclusive `[first, last]`.
    pub rows: [i32; 2],
    pub cols: [i32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RowShift {
    West,
    East,
}

impl Sheet {
    /// True when `row` sits half a hex east of the rows above and below it.
    fn row_shifted_east(&self, row: i32) -> bool {
        (row % 2 == 1) == (self.odd_rows_shift == RowShift::East)
    }
}

/// A hexside feature (cna's `hexsideFeature`, SPI 8.4 and the Terrain Key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HexsideFeature {
    Escarpment,
    Ridge,
    Slope,
    Wadi,
    MajorRiver,
    MinorRiver,
    Road,
    UnfinishedRoad,
    Railroad,
    UnfinishedRailroad,
    Track,
    Coast,
    Lake,
}

/// One of the six sides of a pointy-top hex, for records on a sheet's edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Direction {
    E,
    SE,
    SW,
    W,
    NW,
    NE,
}

/// A hexside with features. A hexside with no record is plain.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Hexside {
    pub features: Vec<HexsideFeature>,
    /// The higher hex of a slope or escarpment, where cna knows it.
    pub up: Option<HexId>,
}

impl Hexside {
    pub fn has(&self, f: HexsideFeature) -> bool {
        self.features.contains(&f)
    }

    /// A road crosses the hexside.
    pub fn road(&self) -> bool {
        self.has(HexsideFeature::Road)
    }

    /// A track crosses the hexside; an unfinished road is a track.
    pub fn track(&self) -> bool {
        self.has(HexsideFeature::Track) || self.has(HexsideFeature::UnfinishedRoad)
    }

    /// True when a move from `from` to `to` goes up the slope or
    /// escarpment: `to` is the recorded `up` hex. Two sheet-B records
    /// (B1705|B1706, B1706|B1805) still carry a slope and no `up`; those
    /// climb both ways until cna reads them.
    pub fn climbs(&self, from: &HexId, to: &HexId) -> bool {
        match &self.up {
            Some(u) => u == to && u != from,
            None => self.has(HexsideFeature::Slope) || self.has(HexsideFeature::Escarpment),
        }
    }
}

/// One row of `hexsides.json`: a pair (`b` set) or an edge (`side` set).
#[derive(Deserialize)]
struct HexsideRecord {
    a: HexId,
    b: Option<HexId>,
    side: Option<Direction>,
    #[serde(flatten)]
    hexside: Hexside,
}

#[derive(Deserialize)]
struct HexsidesFile {
    hexsides: Vec<HexsideRecord>,
}

/// A named place from `places.json`. `kind` is the Terrain Key's class,
/// or `feature` for a named point with no settlement.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "RawPlace")]
pub struct Place {
    pub id: String,
    /// The place's principal hex (where units set up).
    pub hex: HexId,
    /// Every hex the place covers: the optional `hexes` array, else `[hex]`.
    pub hexes: Vec<HexId>,
    pub name: String,
    pub kind: PlaceKind,
    pub port: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceKind {
    MajorCity,
    Village,
    Bir,
    Oasis,
    Feature,
}

#[derive(Deserialize)]
struct RawPlace {
    id: String,
    hex: HexId,
    #[serde(default)]
    hexes: Vec<HexId>,
    name: String,
    kind: PlaceKind,
    port: bool,
}

impl From<RawPlace> for Place {
    fn from(r: RawPlace) -> Self {
        let hexes = if r.hexes.is_empty() {
            vec![r.hex.clone()]
        } else {
            r.hexes
        };
        Place {
            id: r.id,
            hex: r.hex,
            hexes,
            name: r.name,
            kind: r.kind,
            port: r.port,
        }
    }
}

#[derive(Deserialize)]
struct PlacesFile {
    places: Vec<Place>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Country {
    Libya,
    Egypt,
}

/// `regions.json`: whole sheets, and the one sheet the frontier splits.
#[derive(Debug, Clone, Deserialize)]
struct Regions {
    whole_sheets: BTreeMap<char, Country>,
    split_sheet: SplitSheet,
}

#[derive(Debug, Clone, Deserialize)]
struct SplitSheet {
    sheet: char,
    west: Country,
    east: Country,
    /// Per row: the first column in `east`. A value past the sheet's last
    /// column means the whole row is `west`.
    first_east_col: BTreeMap<String, u8>,
}

#[derive(Debug)]
pub struct Map {
    sheets: BTreeMap<char, Sheet>,
    hexes: BTreeMap<HexId, Hex>,
    /// Keyed `(a, b)` with `a < b`.
    hexsides: BTreeMap<(HexId, HexId), Hexside>,
    edges: BTreeMap<(HexId, Direction), Hexside>,
    places: BTreeMap<String, Place>,
    regions: Regions,
    /// Cross-sheet neighbours from `seams.json`, both ways.
    seams: BTreeMap<HexId, Vec<HexId>>,
    seam_pairs: usize,
}

/// `seams.json`: every pair of hexes that touch across a sheet edge, with
/// the features of that hexside and, for a slope or escarpment, the
/// higher hex (cna #235), which both sheets' edge records repeat.
#[derive(Deserialize)]
struct SeamsFile {
    seams: Vec<Seam>,
}

#[derive(Deserialize)]
struct Seam {
    pairs: Vec<SeamPair>,
}

#[derive(Deserialize)]
struct SeamPair {
    west: HexId,
    west_side: Direction,
    east: HexId,
    east_side: Direction,
    features: Vec<HexsideFeature>,
    up: Option<HexId>,
}

#[derive(Deserialize)]
struct SheetsFile {
    sheets: BTreeMap<char, Sheet>,
}

#[derive(Deserialize)]
struct HexesFile {
    hexes: Vec<Hex>,
}

fn read_json<T: serde::de::DeserializeOwned>(src: &dyn Source, name: &str) -> Result<T> {
    crate::source::read_json(src, &format!("map/{name}"))
}

fn pair_key(a: HexId, b: HexId) -> (HexId, HexId) {
    if a <= b { (a, b) } else { (b, a) }
}

impl Map {
    /// Loads `sheets.json`, `hexes.json`, `hexsides.json`, `places.json`
    /// and `regions.json`. Map corrections are already
    /// applied in those files.
    pub fn load() -> Result<Map> {
        Map::load_from(&Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn Source) -> Result<Map> {
        let sheets = read_json::<SheetsFile>(src, "sheets.json")?.sheets;
        let mut hexes = BTreeMap::new();
        for h in read_json::<HexesFile>(src, "hexes.json")?.hexes {
            if !sheets.contains_key(&h.sheet) {
                bail!("{}: sheet {} is not in sheets.json", h.id, h.sheet);
            }
            if hexes.insert(h.id.clone(), h).is_some() {
                bail!("duplicate hex in hexes.json");
            }
        }
        let mut hexsides = BTreeMap::new();
        let mut edges = BTreeMap::new();
        for r in read_json::<HexsidesFile>(src, "hexsides.json")?.hexsides {
            if !hexes.contains_key(&r.a) {
                bail!("hexside {}: {} is not a hex", r.a, r.a);
            }
            match (r.b, r.side) {
                (Some(b), None) => {
                    if !hexes.contains_key(&b) {
                        bail!("hexside {}|{b}: {b} is not a hex", r.a);
                    }
                    if hexsides.insert(pair_key(r.a, b), r.hexside).is_some() {
                        bail!("duplicate hexside in hexsides.json");
                    }
                }
                (None, Some(side)) => {
                    if edges.insert((r.a, side), r.hexside).is_some() {
                        bail!("duplicate edge in hexsides.json");
                    }
                }
                _ => bail!("hexside {}: needs exactly one of b and side", r.a),
            }
        }
        let mut places = BTreeMap::new();
        for p in read_json::<PlacesFile>(src, "places.json")?.places {
            for h in &p.hexes {
                if !hexes.contains_key(h) {
                    bail!("place {}: {} is not a hex", p.id, h);
                }
            }
            if places.insert(p.id.clone(), p).is_some() {
                bail!("duplicate place in places.json");
            }
        }
        let regions: Regions = read_json(src, "regions.json")?;
        for sheet in regions
            .whole_sheets
            .keys()
            .chain(std::iter::once(&regions.split_sheet.sheet))
        {
            if !sheets.contains_key(sheet) {
                bail!("regions.json names sheet {sheet}, which is not in sheets.json");
            }
        }
        let mut seams: BTreeMap<HexId, Vec<HexId>> = BTreeMap::new();
        let mut seam_pairs = 0;
        for seam in read_json::<SeamsFile>(src, "seams.json")?.seams {
            for p in seam.pairs {
                for h in [&p.west, &p.east] {
                    if !hexes.contains_key(h) {
                        bail!("seam {}-{}: {h} is not a hex", p.west, p.east);
                    }
                }
                if p.west.sheet() == p.east.sheet() {
                    bail!("seam {}-{}: same sheet", p.west, p.east);
                }
                seams
                    .entry(p.west.clone())
                    .or_default()
                    .push(p.east.clone());
                seams
                    .entry(p.east.clone())
                    .or_default()
                    .push(p.west.clone());
                seam_pairs += 1;
                let graded = p.features.contains(&HexsideFeature::Slope)
                    || p.features.contains(&HexsideFeature::Escarpment);
                match &p.up {
                    Some(up) if up != &p.west && up != &p.east => {
                        bail!("seam {}-{}: up {up} is neither hex", p.west, p.east);
                    }
                    Some(_) if !graded => {
                        bail!("seam {}-{}: up on no slope or escarpment", p.west, p.east);
                    }
                    None if graded => {
                        bail!(
                            "seam {}-{}: a slope or escarpment with no up",
                            p.west,
                            p.east
                        );
                    }
                    _ => {}
                }
                for (hex, side) in [(&p.west, p.west_side), (&p.east, p.east_side)] {
                    if let Some(edge) = edges.get(&(hex.clone(), side))
                        && edge.up != p.up
                    {
                        bail!(
                            "seam {}-{}: edge record {hex}|{side:?} says up {:?}, the seam {:?}",
                            p.west,
                            p.east,
                            edge.up,
                            p.up
                        );
                    }
                }
                if !p.features.is_empty() {
                    let hs = Hexside {
                        features: p.features,
                        up: p.up,
                    };
                    if hexsides.insert(pair_key(p.west, p.east), hs).is_some() {
                        bail!("seam pair duplicates a hexsides.json record");
                    }
                }
            }
        }
        Ok(Map {
            sheets,
            hexes,
            hexsides,
            edges,
            places,
            regions,
            seams,
            seam_pairs,
        })
    }

    /// How many cross-sheet pairs `seams.json` lists.
    pub fn seam_pairs(&self) -> usize {
        self.seam_pairs
    }

    pub fn place(&self, id: &str) -> Option<&Place> {
        self.places.get(id)
    }

    /// Every place, in id order.
    pub fn places(&self) -> impl Iterator<Item = &Place> {
        self.places.values()
    }

    /// The country a land hex lies in; `None` for sea, Malta and anything
    /// that is not a hex. Follows cna's `tools/map_geom.country`, except
    /// that sea hexes belong to no country.
    pub fn country(&self, id: &HexId) -> Option<Country> {
        let hex = self.hexes.get(id)?;
        if hex.terrain == Terrain::Sea {
            return None;
        }
        if let Some(c) = self.regions.whole_sheets.get(&hex.sheet) {
            return Some(*c);
        }
        let split = &self.regions.split_sheet;
        if hex.sheet != split.sheet {
            return None;
        }
        let first_east = *split.first_east_col.get(&id.row().to_string())?;
        Some(if id.col() >= first_east {
            split.east
        } else {
            split.west
        })
    }

    /// The features between two hexes, in either order; `None` for a plain
    /// hexside (or two hexes that are not adjacent).
    pub fn hexside(&self, a: &HexId, b: &HexId) -> Option<&Hexside> {
        self.hexsides.get(&pair_key(a.clone(), b.clone()))
    }

    /// A sheet-edge record: the features drawn up to the edge of the sheet
    /// on `side` of `a`.
    pub fn edge(&self, a: &HexId, side: Direction) -> Option<&Hexside> {
        self.edges.get(&(a.clone(), side))
    }

    /// Every recorded hexside between two hexes, `a < b`.
    pub fn hexsides(&self) -> impl Iterator<Item = (&HexId, &HexId, &Hexside)> {
        self.hexsides.iter().map(|((a, b), hs)| (a, b, hs))
    }

    /// Every sheet-edge record.
    pub fn edges(&self) -> impl Iterator<Item = (&HexId, Direction, &Hexside)> {
        self.edges.iter().map(|((a, side), hs)| (a, *side, hs))
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

    /// The neighbouring hexes that exist, on this sheet and across a seam,
    /// sorted.
    pub fn neighbours(&self, id: &HexId) -> Vec<HexId> {
        let mut n: Vec<HexId> = self
            .grid_neighbours(id)
            .into_iter()
            .filter(|h| self.hexes.contains_key(h))
            .collect();
        if let Some(across) = self.seams.get(id) {
            n.extend(across.iter().cloned());
        }
        n.sort();
        n
    }

    pub fn adjacent(&self, a: &HexId, b: &HexId) -> bool {
        self.hexes.contains_key(a) && self.neighbours(a).binary_search(b).is_ok()
    }

    /// Every hex within `n` hexes of `centre`, the centre included, by
    /// hex-to-hex steps (seams included).
    pub fn within(&self, centre: &HexId, n: u32) -> BTreeSet<HexId> {
        let mut seen = BTreeSet::new();
        if !self.hexes.contains_key(centre) {
            return seen;
        }
        seen.insert(centre.clone());
        let mut frontier = vec![centre.clone()];
        for _ in 0..n {
            let mut next = Vec::new();
            for h in &frontier {
                for nb in self.neighbours(h) {
                    if seen.insert(nb.clone()) {
                        next.push(nb);
                    }
                }
            }
            frontier = next;
        }
        seen
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

    #[test]
    fn halfaya_escarpment_climbs_south_west() {
        let m = map();
        let (plain, plateau) = (
            HexId::parse("C3922").unwrap(),
            HexId::parse("C3921").unwrap(),
        );
        let hs = m.hexside(&plain, &plateau).unwrap();
        assert!(hs.has(HexsideFeature::Escarpment));
        assert_eq!(hs.up.as_ref(), Some(&plateau));
        assert!(hs.climbs(&plain, &plateau));
        assert!(!hs.climbs(&plateau, &plain));
        // Either order looks up the same record.
        assert_eq!(m.hexside(&plateau, &plain), Some(hs));
    }

    #[test]
    fn seam_slopes_and_escarpments_carry_up_from_seams_json() {
        // cna #235: every slope or escarpment seam pair says which hex is
        // higher, and both sheets' edge records agree with it.
        let m = map();
        for (a, b, up) in [
            ("C0633", "D0701", "D0701"),
            ("A3933", "B3901", "B3901"),
            ("A1233", "B1201", "A1233"),
            ("D2233", "E2301", "E2301"),
        ] {
            let (a, b, up) = (
                HexId::parse(a).unwrap(),
                HexId::parse(b).unwrap(),
                HexId::parse(up).unwrap(),
            );
            let hs = m.hexside(&a, &b).unwrap();
            assert_eq!(hs.up.as_ref(), Some(&up), "{a}|{b}");
            assert_eq!(hs.climbs(&a, &b), b == up, "{a}|{b}");
            assert_eq!(hs.climbs(&b, &a), a == up, "{a}|{b}");
        }
        let graded =
            |hs: &Hexside| hs.has(HexsideFeature::Slope) || hs.has(HexsideFeature::Escarpment);
        let mut seam_graded = 0;
        for (a, b, hs) in m.hexsides() {
            if a.sheet() != b.sheet() && graded(hs) {
                seam_graded += 1;
                let up = hs.up.as_ref().expect("a seam slope or escarpment has up");
                assert!(up == a || up == b, "{a}|{b}: up {up} is neither hex");
            }
        }
        assert_eq!(seam_graded, 25);
        // Every edge record with a slope or escarpment carries up too.
        for (a, side, hs) in m.edges() {
            if graded(hs) {
                assert!(hs.up.is_some(), "{a}|{side:?}");
            }
        }
    }

    #[test]
    fn only_two_sheet_b_slopes_still_climb_both_ways() {
        // hexsides.json still records no up for two sheet-B slopes
        // (B1705|B1706 and B1706|B1805); the engine climbs those both
        // ways until cna reads them. Every other graded hexside climbs
        // one way only.
        let m = map();
        let mut unknown = Vec::new();
        for (a, b, hs) in m.hexsides() {
            let graded = hs.has(HexsideFeature::Slope) || hs.has(HexsideFeature::Escarpment);
            let ways = usize::from(hs.climbs(a, b)) + usize::from(hs.climbs(b, a));
            match (graded, &hs.up) {
                (true, None) => {
                    assert_eq!(ways, 2, "{a}|{b}");
                    unknown.push(format!("{a}|{b}"));
                }
                (true, Some(_)) => assert_eq!(ways, 1, "{a}|{b}"),
                (false, _) => assert_eq!(ways, 0, "{a}|{b}"),
            }
        }
        assert_eq!(unknown, ["B1705|B1706", "B1706|B1805"]);
    }

    #[test]
    fn unfinished_road_either_way_round() {
        let m = map();
        let (a, b) = (
            HexId::parse("C3822").unwrap(),
            HexId::parse("C3922").unwrap(),
        );
        for (x, y) in [(&a, &b), (&b, &a)] {
            let hs = m.hexside(x, y).unwrap();
            assert_eq!(hs.features, vec![HexsideFeature::UnfinishedRoad]);
            assert!(hs.up.is_none());
            assert!(!hs.climbs(x, y));
        }
        // A plain hexside has no record.
        assert!(m.hexside(&b, &HexId::parse("C3923").unwrap()).is_none());
    }

    #[test]
    fn edge_records_are_kept_by_side() {
        let m = map();
        let a1233 = HexId::parse("A1233").unwrap();
        assert!(
            m.edge(&a1233, Direction::E)
                .unwrap()
                .has(HexsideFeature::Slope)
        );
        assert!(m.edge(&a1233, Direction::W).is_none());
    }

    #[test]
    fn every_hexside_joins_adjacent_hexes() {
        let m = map();
        let mut n = 0;
        for (a, b, _) in m.hexsides() {
            assert!(m.hex(a).is_some(), "{a} missing");
            assert!(m.adjacent(a, b), "{a}|{b} not adjacent");
            n += 1;
        }
        assert!(n > 4000, "{n} hexsides");
    }

    #[test]
    fn feature_counts_match_cna() {
        let m = map();
        // hexsides.json counts: pairs on one sheet plus edge records. Seam
        // hexsides are unions of edge records, so they are left out.
        let count = |f: HexsideFeature| {
            m.hexsides()
                .filter(|(a, b, hs)| a.sheet() == b.sheet() && hs.has(f))
                .count()
                + m.edges().filter(|(_, _, hs)| hs.has(f)).count()
        };
        assert_eq!(count(HexsideFeature::Track), 1808);
        assert_eq!(count(HexsideFeature::Road), 354);
        assert_eq!(count(HexsideFeature::Escarpment), 216);
    }

    #[test]
    fn places_resolve_to_hexes() {
        let m = map();
        for (id, hex) in [
            ("tobruk", "C4807"),
            ("sollum", "C4021"),
            ("siwa", "C0127"),
            ("mersa-matruh", "D3714"),
        ] {
            let p = m.place(id).unwrap();
            assert_eq!(p.hex.as_str(), hex, "{id}");
            assert!(m.hex(&p.hex).is_some(), "{id}");
        }
        assert_eq!(m.place("tobruk").unwrap().kind, PlaceKind::MajorCity);
        assert!(m.place("sollum").unwrap().port);
        assert!(!m.place("siwa").unwrap().port);
        assert!(m.place("nowhere").is_none());
    }

    #[test]
    fn cairo_has_five_hexes_and_other_places_one() {
        let m = map();
        let cairo = m.place("cairo").unwrap();
        assert_eq!(cairo.hexes.len(), 5);
        assert!(cairo.hexes.contains(&cairo.hex));
        assert!(cairo.hexes.iter().all(|h| m.hex(h).is_some()));
        let tobruk = m.place("tobruk").unwrap();
        assert_eq!(tobruk.hexes, vec![tobruk.hex.clone()]);
    }

    #[test]
    fn every_place_hex_exists() {
        let m = map();
        assert_eq!(m.places().count(), 51);
        for p in m.places() {
            assert!(m.hex(&p.hex).is_some(), "{}: {}", p.id, p.hex);
        }
    }

    #[test]
    fn whole_sheets_have_one_country() {
        let m = map();
        for h in m.hexes() {
            let want = match h.sheet {
                'A' | 'B' => Some(Country::Libya),
                'D' | 'E' => Some(Country::Egypt),
                _ => continue,
            };
            let want = if h.terrain == Terrain::Sea {
                None
            } else {
                want
            };
            assert_eq!(m.country(&h.id), want, "{}", h.id);
        }
        // Malta is in neither.
        assert_eq!(m.country(&HexId::parse("M0500").unwrap()), None);
    }

    #[test]
    fn frontier_on_sheet_c_matches_map_geom() {
        // Pinned from cna tools/map_geom.country, 2026-10-12.
        let m = map();
        for (hex, want) in [
            ("C2017", Country::Libya),
            ("C2018", Country::Egypt),
            ("C3920", Country::Libya),
            ("C3921", Country::Egypt),
            ("C4020", Country::Libya),
            ("C4021", Country::Egypt),
        ] {
            assert_eq!(m.country(&HexId::parse(hex).unwrap()), Some(want), "{hex}");
        }
        // Sea is in no country: C4233 is sea on a row the frontier never
        // reaches.
        assert_eq!(m.country(&HexId::parse("C4233").unwrap()), None);
    }

    #[test]
    fn c_to_d_is_adjacent() {
        let m = map();
        let (c, d) = (
            HexId::parse("C0633").unwrap(),
            HexId::parse("D0701").unwrap(),
        );
        assert!(m.adjacent(&c, &d));
        assert!(m.adjacent(&d, &c));
        assert!(m.neighbours(&c).contains(&d));
        assert!(m.neighbours(&d).contains(&c));
        assert!(m.hexside(&c, &d).unwrap().has(HexsideFeature::Slope));
        assert!(m.hexside(&d, &c).unwrap().has(HexsideFeature::Slope));
        assert_eq!(m.seam_pairs(), 359);
        // Row 6 is shifted east on sheet C, so C0633's diagonals are column
        // 33 on its own sheet and three hexes of sheet D (SE, E, NE).
        assert_eq!(
            m.neighbours(&c),
            ids(&["C0533", "C0632", "C0733", "D0501", "D0601", "D0701"])
        );
    }

    #[test]
    fn seams_keep_adjacency_symmetric_and_hexsides_adjacent() {
        let m = map();
        for h in m.hexes().filter(|h| matches!(h.id.col(), 1 | 33)) {
            for n in m.neighbours(&h.id) {
                assert!(m.adjacent(&n, &h.id), "{n} -> {}", h.id);
            }
        }
        for (a, b, _) in m.hexsides() {
            assert!(m.adjacent(a, b), "{a}|{b} not adjacent");
        }
    }
}
