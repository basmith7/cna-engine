//! A scenario (`data/scenarios/<slug>.json`, SPI 59): its clock, initiative,
//! victory block and each side's set-up. A child scenario `extends` another
//! and takes every top-level block it does not give. `land_only` applies the
//! Land-Game-only abstraction (60.92 for Group One).

use crate::map::{Country, HexId};
use crate::oa::{ToeEntry, UnitId};
use crate::source::{Fs, Source};
use crate::time::GameTime;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

/// A player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Axis,
    Cw,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::Axis => Side::Cw,
            Side::Cw => Side::Axis,
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Side::Axis => "axis",
            Side::Cw => "cw",
        })
    }
}

/// An off-map holding box (8.8x).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OffMapBox {
    Tripoli,
    Tunisia,
    Tripolitania,
    Malta,
    ItalySicily,
    Crete,
}

/// One of the schema's locators. A placement holds one or more; a hex
/// must satisfy every one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locator {
    /// Any of these hexes, divided as the owner likes (59.2).
    Hexes(Vec<HexId>),
    /// `places.json` ids: any of them, divided as the owner likes.
    Place(Vec<String>),
    Box(OffMapBox),
    Within {
        of: HexId,
        hexes: u32,
    },
    Region(Country),
    Sheets(Vec<char>),
    /// West of this hex: an earlier sheet, or its sheet with a lower
    /// column (a column bound; the same column is not west).
    WestOf(HexId),
    AirFacility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotWithin {
    /// Only `enemy-unit` today.
    pub of: String,
    pub hexes: u32,
}

/// A placement: one or more locators (a hex must satisfy every one, in
/// schema order), narrowed by `not_within`, by `together` and by
/// `constraint`, a sentence the grammar cannot check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "PlacementRaw", into = "PlacementRaw")]
pub struct Placement {
    pub locators: Vec<Locator>,
    pub not_within: Option<NotWithin>,
    /// Every unit, dump or supply unit of the block goes into one hex.
    pub together: bool,
    pub constraint: Option<String>,
}

impl Placement {
    /// A placement with a single locator and nothing narrowing it.
    pub fn of(locator: Locator) -> Placement {
        Placement {
            locators: vec![locator],
            not_within: None,
            together: false,
            constraint: None,
        }
    }

    /// The only locator, where there is exactly one.
    pub fn sole(&self) -> Option<&Locator> {
        match self.locators.as_slice() {
            [l] => Some(l),
            _ => None,
        }
    }
}

/// A place locator: one id or a choice of several.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum PlaceRaw {
    One(String),
    Any(Vec<String>),
}

/// The JSON shape: locator keys side by side with the narrowing fields.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlacementRaw {
    #[serde(skip_serializing_if = "Option::is_none")]
    hexes: Option<Vec<HexId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    place: Option<PlaceRaw>,
    #[serde(rename = "box", skip_serializing_if = "Option::is_none")]
    off_map_box: Option<OffMapBox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    within: Option<Within>,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<Country>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sheets: Option<Vec<char>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    west_of: Option<HexId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    air_facility: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    not_within: Option<NotWithin>,
    #[serde(skip_serializing_if = "Option::is_none")]
    together: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    constraint: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Within {
    of: HexId,
    hexes: u32,
}

impl TryFrom<PlacementRaw> for Placement {
    type Error = anyhow::Error;
    fn try_from(r: PlacementRaw) -> Result<Placement> {
        let mut found = Vec::new();
        if let Some(h) = r.hexes {
            if h.is_empty() {
                bail!("placement: empty hexes");
            }
            found.push(Locator::Hexes(h));
        }
        if let Some(p) = r.place {
            let ids = match p {
                PlaceRaw::One(id) => vec![id],
                PlaceRaw::Any(ids) if ids.len() >= 2 => ids,
                PlaceRaw::Any(ids) => {
                    bail!("placement: a place list needs two or more, has {ids:?}")
                }
            };
            found.push(Locator::Place(ids));
        }
        if let Some(b) = r.off_map_box {
            found.push(Locator::Box(b));
        }
        if let Some(w) = r.within {
            found.push(Locator::Within {
                of: w.of,
                hexes: w.hexes,
            });
        }
        if let Some(c) = r.region {
            found.push(Locator::Region(c));
        }
        if let Some(s) = r.sheets {
            if s.is_empty() || s.iter().any(|c| !('A'..='E').contains(c)) {
                bail!("placement: bad sheets {s:?}");
            }
            found.push(Locator::Sheets(s));
        }
        if let Some(h) = r.west_of {
            found.push(Locator::WestOf(h));
        }
        if r.air_facility == Some(true) {
            found.push(Locator::AirFacility);
        }
        if found.is_empty() {
            bail!("placement: needs a locator");
        }
        Ok(Placement {
            locators: found,
            not_within: r.not_within,
            together: r.together == Some(true),
            constraint: r.constraint,
        })
    }
}

impl From<Placement> for PlacementRaw {
    fn from(p: Placement) -> PlacementRaw {
        let mut r = PlacementRaw {
            not_within: p.not_within,
            together: p.together.then_some(true),
            constraint: p.constraint,
            ..PlacementRaw::default()
        };
        for l in p.locators {
            match l {
                Locator::Hexes(h) => r.hexes = Some(h),
                Locator::Place(mut ids) => {
                    r.place = Some(if ids.len() == 1 {
                        PlaceRaw::One(ids.remove(0))
                    } else {
                        PlaceRaw::Any(ids)
                    })
                }
                Locator::Box(b) => r.off_map_box = Some(b),
                Locator::Within { of, hexes } => r.within = Some(Within { of, hexes }),
                Locator::Region(c) => r.region = Some(c),
                Locator::Sheets(s) => r.sheets = Some(s),
                Locator::WestOf(h) => r.west_of = Some(h),
                Locator::AirFacility => r.air_facility = Some(true),
            }
        }
        r
    }
}

/// What a set-up entry names: one OA unit (with what its sheet assigns to
/// it) or a whole sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subject {
    Unit(UnitId),
    /// `<nation>:<formation id>`.
    Formation(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    InTraining,
    BrokenDown,
}

/// One line of a set-up, with the 59.2 indicators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "UnitEntryRaw")]
pub struct UnitEntry {
    pub subject: Subject,
    /// Not present (arrives elsewhere or later).
    pub less: Vec<UnitId>,
    /// Reassigned to this entry.
    pub assigned: Vec<UnitId>,
    /// Present, but placed by another entry.
    pub detached: Vec<UnitId>,
    /// Attached to this entry from another sheet.
    pub attached: Vec<UnitId>,
    /// The entry is the named unit and exactly these.
    pub consists_of: Option<Vec<UnitId>>,
    /// The counter only, not its subordinates.
    pub alone: bool,
    pub status: Option<Status>,
    /// A TOE other than the sheet's.
    pub toe: Option<Vec<ToeEntry>>,
    pub notes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitEntryRaw {
    unit: Option<UnitId>,
    formation: Option<String>,
    #[serde(default)]
    less: Vec<UnitId>,
    #[serde(default)]
    assigned: Vec<UnitId>,
    #[serde(default)]
    detached: Vec<UnitId>,
    #[serde(default)]
    attached: Vec<UnitId>,
    consists_of: Option<Vec<UnitId>>,
    #[serde(default)]
    alone: bool,
    status: Option<Status>,
    toe: Option<Vec<ToeEntry>>,
    #[serde(default)]
    notes: Vec<String>,
}

impl TryFrom<UnitEntryRaw> for UnitEntry {
    type Error = anyhow::Error;
    fn try_from(r: UnitEntryRaw) -> Result<UnitEntry> {
        let subject = match (r.unit, r.formation) {
            (Some(u), None) => Subject::Unit(u),
            (None, Some(f)) => Subject::Formation(f),
            _ => bail!("unit entry: needs exactly one of unit and formation"),
        };
        Ok(UnitEntry {
            subject,
            less: r.less,
            assigned: r.assigned,
            detached: r.detached,
            attached: r.attached,
            consists_of: r.consists_of,
            alone: r.alone,
            status: r.status,
            toe: r.toe,
            notes: r.notes,
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trucks {
    #[serde(default)]
    pub light: u32,
    #[serde(default)]
    pub medium: u32,
    #[serde(default)]
    pub heavy: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deployment {
    pub placement: Placement,
    pub units: Vec<UnitEntry>,
    pub trucks: Option<Trucks>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupplyKind {
    Dump,
    DummyDump,
    AirFacilities,
    SupplyUnit,
    DummySupplyUnit,
}

fn one() -> u32 {
    1
}

/// A supply block: `count` dumps or supply units of one kind in one place.
/// Unless `pooled`, `ammo`, `fuel`, `stores` and `water` are each dump's
/// or supply unit's own (scenario schema, `supplyBlock.count`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyBlock {
    pub placement: Placement,
    #[serde(default = "one")]
    pub count: u32,
    pub kind: Option<SupplyKind>,
    /// The figures are one pool for the whole block.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pooled: bool,
    pub ammo: Option<u32>,
    pub fuel: Option<u32>,
    pub stores: Option<u32>,
    pub water: Option<u32>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// One side's set-up. The air, fleet, repair and truck blocks are not
/// read: Mission 2 plays the Land Game only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideSetup {
    #[serde(default)]
    pub deployments: Vec<Deployment>,
    #[serde(default)]
    pub supply: Vec<SupplyBlock>,
    #[serde(default)]
    pub reinforcements: Vec<String>,
    #[serde(default)]
    pub special: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Initiative {
    pub side: Side,
    /// Held until the end of this OpStage; then the normal rule (7.0).
    pub through: GameTime,
}

/// The blocks an abstraction gives a side; only `supply` is read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideOverride {
    pub supply: Option<Vec<SupplyBlock>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abstraction {
    #[serde(default)]
    pub replaces: BTreeMap<Side, SideOverride>,
    #[serde(default)]
    pub adds: BTreeMap<Side, SideOverride>,
    pub motorisation: Option<Motorisation>,
}

/// A truck class (59.63).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TruckClass {
    Light,
    Medium,
    Heavy,
}

/// How truck points become motorisation points (32.5, 59.63): each truck
/// point of a class in `from` is one motorisation point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Motorisation {
    pub from: Vec<TruckClass>,
    /// Printed unattached starting points; they replace the second-line
    /// truck blocks. Kept as JSON: no scenario the engine plays has them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unattached: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abstractions {
    pub air: Option<Abstraction>,
    pub air_and_logistics: Option<Abstraction>,
    pub logistics: Option<Abstraction>,
}

/// How a scenario is won: a ladder of levels, or points for places held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VictoryKind {
    Levels,
    Points,
}

/// A level of victory, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VictoryLevel {
    Tactical,
    Decisive,
    Strategic,
}

impl fmt::Display for VictoryLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            VictoryLevel::Tactical => "tactical",
            VictoryLevel::Decisive => "decisive",
            VictoryLevel::Strategic => "strategic",
        })
    }
}

/// A place to hold at the end: a placement (its `constraint` is a
/// sentence the engine cannot check); `any_hex` means one hex of it is
/// enough.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Objective {
    pub hold: Placement,
    #[serde(default)]
    pub any_hex: bool,
}

/// Where a holding unit's route must reach (R-110): any hex of these
/// places, these hexes, or this sheet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyTrace {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hexes: Vec<HexId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet: Option<char>,
}

/// One rung of a levels ladder: every `all_of` objective, at least one
/// `any_of` if any are listed, by units meeting the supply condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub side: Side,
    pub level: VictoryLevel,
    pub all_of: Vec<Objective>,
    #[serde(default)]
    pub any_of: Vec<Objective>,
    /// The supply sentence as cna wrote it.
    pub supply: Option<String>,
    pub supply_trace: Option<SupplyTrace>,
}

/// A place scored in a points victory, by whichever side holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointsObjective {
    pub place: Option<String>,
    #[serde(default)]
    pub hexes: Vec<HexId>,
    pub axis: i32,
    pub cw: i32,
}

/// A points victory's level by margin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Margin {
    pub min: i32,
    pub level: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Victory {
    pub kind: VictoryKind,
    #[serde(default)]
    pub levels: Vec<Level>,
    #[serde(default)]
    pub points: Vec<PointsObjective>,
    #[serde(default)]
    pub margins: Vec<Margin>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub group: u8,
    pub extends: Option<String>,
    pub start: GameTime,
    pub end: GameTime,
    pub initiative: Option<Initiative>,
    /// Typed by the slice that reads it.
    pub construction: Option<Value>,
    pub victory: Option<Victory>,
    #[serde(default)]
    pub sides: BTreeMap<Side, SideSetup>,
    pub abstractions: Option<Abstractions>,
}

fn read_value(src: &dyn Source, slug: &str) -> Result<Value> {
    crate::source::read_json(src, &format!("scenarios/{slug}.json"))
}

/// The file as a JSON object with its parent's blocks filled in.
fn read_merged(src: &dyn Source, slug: &str, depth: u8) -> Result<Value> {
    if depth > 8 {
        bail!("scenario {slug}: extends chain too deep");
    }
    let mut v = read_value(src, slug)?;
    if let Some(parent) = v.get("extends").and_then(Value::as_str) {
        let parent_slug = parent
            .strip_prefix("scenario:")
            .with_context(|| format!("scenario {slug}: bad extends {parent:?}"))?
            .to_owned();
        let base = read_merged(src, &parent_slug, depth + 1)?;
        let (Some(obj), Some(base)) = (v.as_object_mut(), base.as_object()) else {
            bail!("scenario {slug}: not an object");
        };
        for (k, val) in base {
            obj.entry(k.clone()).or_insert_with(|| val.clone());
        }
    }
    Ok(v)
}

impl Scenario {
    /// Loads `data/scenarios/<slug>.json`, following `extends`.
    pub fn load(slug: &str) -> Result<Scenario> {
        Scenario::load_from(&Fs, slug)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn Source, slug: &str) -> Result<Scenario> {
        serde_json::from_value(read_merged(src, slug, 0)?)
            .with_context(|| format!("scenario {slug}"))
    }

    /// A side's set-up, empty if the file gives none.
    pub fn side(&self, side: Side) -> &SideSetup {
        static EMPTY: SideSetup = SideSetup {
            deployments: Vec::new(),
            supply: Vec::new(),
            reinforcements: Vec::new(),
            special: Vec::new(),
        };
        self.sides.get(&side).unwrap_or(&EMPTY)
    }

    /// The scenario as the Land Game alone plays it: each side block in
    /// `abstractions.air_and_logistics.replaces` replaces that side's block
    /// of the same name, and `adds` appends. Only `supply` is such a block
    /// today. A scenario with no such abstraction is returned unchanged.
    pub fn land_only(&self) -> Scenario {
        let mut s = self.clone();
        let Some(abs) = self
            .abstractions
            .as_ref()
            .and_then(|a| a.air_and_logistics.as_ref())
        else {
            return s;
        };
        for (side, over) in &abs.replaces {
            if let Some(supply) = &over.supply {
                s.sides.entry(*side).or_default().supply = supply.clone();
            }
        }
        for (side, over) in &abs.adds {
            if let Some(supply) = &over.supply {
                s.sides
                    .entry(*side)
                    .or_default()
                    .supply
                    .extend(supply.iter().cloned());
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Country;
    use crate::time::GameTime;

    fn graziani() -> Scenario {
        Scenario::load("grazianis-offensive").unwrap()
    }

    #[test]
    fn graziani_loads() {
        let s = graziani();
        assert_eq!(s.id, "scenario:grazianis-offensive");
        assert_eq!(s.start, GameTime::new(1, 1));
        assert_eq!(s.end, GameTime::new(6, 3));
        let init = s.initiative.as_ref().unwrap();
        assert_eq!((init.side, init.through), (Side::Axis, GameTime::new(1, 3)));
        assert_eq!(s.side(Side::Axis).deployments.len(), 26);
        assert_eq!(s.side(Side::Cw).deployments.len(), 16);
        let entries =
            |side| -> usize { s.side(side).deployments.iter().map(|d| d.units.len()).sum() };
        assert_eq!(entries(Side::Axis), 48);
        assert_eq!(entries(Side::Cw), 32);
        assert_eq!(s.side(Side::Axis).supply.len(), 10);
        assert_eq!(s.side(Side::Cw).supply.len(), 5);
    }

    #[test]
    fn italian_campaign_extends_graziani() {
        let s = Scenario::load("italian-campaign").unwrap();
        let g = graziani();
        assert_eq!(s.id, "scenario:italian-campaign");
        assert_eq!(s.extends.as_deref(), Some("scenario:grazianis-offensive"));
        assert_eq!(s.end, GameTime::new(20, 3));
        assert_eq!(s.sides, g.sides);
        assert_eq!(s.initiative, g.initiative);
        assert_eq!(s.abstractions, g.abstractions);
        assert_eq!(s.victory.as_ref().unwrap().kind, VictoryKind::Points);
        assert_eq!(g.victory.as_ref().unwrap().kind, VictoryKind::Levels);
    }

    #[test]
    fn graziani_victory_levels_are_typed() {
        let v = graziani().victory.unwrap();
        assert_eq!(v.kind, VictoryKind::Levels);
        assert_eq!(v.levels.len(), 6);
        let axis_strategic = &v.levels[0];
        assert_eq!(
            (axis_strategic.side, axis_strategic.level),
            (Side::Axis, VictoryLevel::Strategic)
        );
        assert_eq!(axis_strategic.all_of.len(), 1);
        assert!(axis_strategic.all_of[0].any_hex);
        assert!(matches!(
            &axis_strategic.all_of[0].hold.locators[0],
            Locator::Hexes(h) if h.len() == 7
        ));
        assert_eq!(
            axis_strategic.supply_trace,
            Some(SupplyTrace {
                places: vec![],
                hexes: vec![],
                sheet: Some('D'),
            })
        );
        let cw_tactical = &v.levels[3];
        assert_eq!(
            (cw_tactical.side, cw_tactical.level),
            (Side::Cw, VictoryLevel::Tactical)
        );
        assert_eq!(cw_tactical.all_of.len(), 3);
        assert!(!cw_tactical.all_of[0].any_hex);
        assert_eq!(
            cw_tactical.all_of[0].hold.locators[0],
            Locator::Place(vec!["sollum".into()])
        );
        assert_eq!(
            cw_tactical.supply_trace,
            Some(SupplyTrace {
                places: vec!["cairo".into()],
                hexes: vec![
                    HexId::parse("E3613").unwrap(),
                    HexId::parse("E3714").unwrap()
                ],
                sheet: None,
            })
        );
        assert!(cw_tactical.supply.is_some());
        assert!(VictoryLevel::Strategic > VictoryLevel::Decisive);
        assert!(VictoryLevel::Decisive > VictoryLevel::Tactical);
        // Rommel's Arrival: a hold the grammar cannot check carries its sentence.
        let r = Scenario::load("rommels-arrival").unwrap().victory.unwrap();
        let region = &r.levels[2].all_of[3].hold;
        assert_eq!(region.locators[0], Locator::Region(Country::Egypt));
        assert!(region.constraint.is_some());
        assert_eq!(r.levels[2].supply_trace, None);
    }

    #[test]
    fn every_scenario_file_deserialises() {
        for slug in [
            "campaign-game",
            "desert-fox-campaign",
            "italian-campaign",
            "operation-crusader",
            "rommels-arrival",
            "short-campaign-game",
            "the-last-chance",
            "the-long-retreat",
        ] {
            Scenario::load(slug).unwrap_or_else(|e| panic!("{slug}: {e:#}"));
        }
        assert!(Scenario::load("no-such-scenario").is_err());
    }

    #[test]
    fn land_only_swaps_supply_for_supply_units() {
        let g = graziani();
        let s = g.land_only();
        let axis = &s.side(Side::Axis).supply;
        assert_eq!(axis.len(), 10);
        let count = |blocks: &[SupplyBlock], kind| -> u32 {
            blocks
                .iter()
                .filter(|b| b.kind == Some(kind))
                .map(|b| b.count)
                .sum()
        };
        assert_eq!(count(axis, SupplyKind::SupplyUnit), 22);
        assert_eq!(count(axis, SupplyKind::DummySupplyUnit), 6);
        let cw = &s.side(Side::Cw).supply;
        assert_eq!(cw.len(), 6);
        assert_eq!(count(cw, SupplyKind::SupplyUnit), 15);
        assert_eq!(count(cw, SupplyKind::DummySupplyUnit), 3);
        // The two `adds` blocks come last.
        assert!(matches!(cw[4].placement.locators[0], Locator::Hexes(ref h) if h.len() == 2));
        assert_eq!(
            cw[5].placement.locators[0],
            Locator::Place(vec!["cairo".into()])
        );
        // Everything else is untouched.
        assert_eq!(
            s.sides[&Side::Axis].deployments,
            g.sides[&Side::Axis].deployments
        );
        assert_eq!(s.start, g.start);
    }

    #[test]
    fn every_placement_variant_in_the_file_deserialises() {
        let s = graziani();
        let mut seen = std::collections::HashSet::new();
        for side in [Side::Axis, Side::Cw] {
            for d in &s.side(side).deployments {
                seen.insert(std::mem::discriminant(&d.placement.locators[0]));
            }
        }
        assert_eq!(seen.len(), 6, "hexes, place, box, within, region, sheets");
        let axis = &s.side(Side::Axis).deployments;
        assert_eq!(
            axis[22].placement.locators[0],
            Locator::Within {
                of: HexId::parse("C0716").unwrap(),
                hexes: 3
            }
        );
        assert_eq!(
            axis[21].placement.locators[0],
            Locator::Region(Country::Libya)
        );
        assert_eq!(
            axis[23].placement.locators[0],
            Locator::Sheets(vec!['A', 'B'])
        );
        assert_eq!(
            axis[25].placement.locators[0],
            Locator::Box(OffMapBox::Tripolitania)
        );
        let dump = &s.side(Side::Axis).supply[5];
        assert_eq!(dump.placement.not_within.as_ref().unwrap().hexes, 4);
        // Typed now (cna bd108b6): the Libyan part of sheet C.
        assert_eq!(
            dump.placement.locators,
            vec![Locator::Region(Country::Libya), Locator::Sheets(vec!['C'])]
        );
        assert_eq!(dump.placement.constraint, None);
        assert_eq!(dump.kind, Some(SupplyKind::Dump));
        // A supply-unit group in one hex, west of Mersa Matruh.
        let land = s.land_only();
        let west = land
            .side(Side::Cw)
            .supply
            .iter()
            .find(|b| {
                b.placement
                    .locators
                    .contains(&Locator::WestOf(HexId::parse("D3714").unwrap()))
            })
            .expect("a west-of block");
        assert!(west.placement.together);
        assert!(
            west.placement
                .locators
                .contains(&Locator::Sheets(vec!['D']))
        );
    }

    #[test]
    fn unit_entry_indicators() {
        let s = graziani();
        let cw = &s.side(Side::Cw).deployments;
        let aus = &cw[12].units[0];
        assert_eq!(
            aus.subject,
            Subject::Unit(UnitId::parse("unit:cw:6-aus-hq").unwrap())
        );
        assert_eq!(aus.status, Some(Status::InTraining));
        assert_eq!(aus.less.len(), 3);
        assert_eq!(
            cw[12].placement.locators,
            vec![Locator::Place(vec!["cairo".into(), "helwan".into()])]
        );
        let nz = &cw[11].units[0];
        assert!(nz.alone);
        let bardia = &s.side(Side::Axis).deployments[9].units[1];
        assert_eq!(
            bardia.subject,
            Subject::Formation("it:bardia-garrison-hex-c4321".into())
        );
        let trucks = s.side(Side::Axis).deployments[0].trucks.as_ref().unwrap();
        assert_eq!((trucks.light, trucks.medium, trucks.heavy), (10, 25, 10));
    }

    #[test]
    fn placement_round_trips_through_json() {
        let p = Placement {
            locators: vec![
                Locator::Place(vec!["a".into(), "b".into()]),
                Locator::Region(Country::Libya),
                Locator::Sheets(vec!['C']),
                Locator::WestOf(HexId::parse("C3714").unwrap()),
            ],
            not_within: Some(NotWithin {
                of: "enemy-unit".into(),
                hexes: 4,
            }),
            together: true,
            constraint: Some("x".into()),
        };
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<Placement>(&json).unwrap(), p);
        let one = serde_json::from_str::<Placement>(r#"{"place":"a"}"#).unwrap();
        assert_eq!(one, Placement::of(Locator::Place(vec!["a".into()])));
        assert_eq!(serde_json::to_string(&one).unwrap(), r#"{"place":"a"}"#);
        assert!(serde_json::from_str::<Placement>(r#"{"place":["a"]}"#).is_err());
        assert!(serde_json::from_str::<Placement>(r#"{"constraint":"x"}"#).is_err());
    }
}
