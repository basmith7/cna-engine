//! The Organisation at Arrival sheets (`data/oa/{it,cw,de}.json`, SPI
//! 4.44): every counter of each nation, grouped by the sheet it is printed
//! on, with its parent on that sheet. Values are as printed; the engine's
//! reading of them (TOE points, pieces) lives in `cna-game`.

use crate::characteristics::Equivalent;
use crate::source::{Fs, Source};
use crate::time::GameTime;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Nation {
    It,
    Cw,
    De,
}

impl Nation {
    pub fn as_str(self) -> &'static str {
        match self {
            Nation::It => "it",
            Nation::Cw => "cw",
            Nation::De => "de",
        }
    }

    fn parse(s: &str) -> Option<Nation> {
        match s {
            "it" => Some(Nation::It),
            "cw" => Some(Nation::Cw),
            "de" => Some(Nation::De),
            _ => None,
        }
    }
}

impl fmt::Display for Nation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A counter on an OA sheet: `unit:<nation>:<slug>`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct UnitId(String);

impl UnitId {
    /// Checks the grammar `^unit:(cw|it|de):[a-z0-9][a-z0-9-]*$`.
    pub fn parse(s: &str) -> Result<UnitId> {
        let parts: Vec<&str> = s.splitn(3, ':').collect();
        let ok = parts.len() == 3
            && parts[0] == "unit"
            && Nation::parse(parts[1]).is_some()
            && slug_ok(parts[2]);
        if !ok {
            bail!("bad unit id {s:?}");
        }
        Ok(UnitId(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn nation(&self) -> Nation {
        Nation::parse(&self.0[5..7]).expect("checked at parse")
    }
}

fn slug_ok(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b[0].is_ascii_lowercase() | b[0].is_ascii_digit()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

impl fmt::Display for UnitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for UnitId {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self> {
        UnitId::parse(&s)
    }
}

impl From<UnitId> for String {
    fn from(id: UnitId) -> String {
        id.0
    }
}

/// The sheet's arrival column: a game time, or `D` for deployed at start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ArrivalCell", into = "ArrivalCell")]
pub enum Arrival {
    At(GameTime),
    Deployed,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ArrivalCell {
    At(GameTime),
    Word(String),
}

impl TryFrom<ArrivalCell> for Arrival {
    type Error = anyhow::Error;
    fn try_from(c: ArrivalCell) -> Result<Arrival> {
        match c {
            ArrivalCell::At(t) => Ok(Arrival::At(t)),
            ArrivalCell::Word(w) if w == "deployed" => Ok(Arrival::Deployed),
            ArrivalCell::Word(w) => bail!("bad arrives {w:?}"),
        }
    }
}

impl From<Arrival> for ArrivalCell {
    fn from(a: Arrival) -> ArrivalCell {
        match a {
            Arrival::At(t) => ArrivalCell::At(t),
            Arrival::Deployed => ArrivalCell::Word("deployed".into()),
        }
    }
}

/// The sheet's TOE column where it prints a mark instead of weapons:
/// `N` (full TOE for the ID code), `U@x` (understrength at x), `O@x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum ToeMark {
    Full,
    Under(u32),
    Over(u32),
}

impl TryFrom<String> for ToeMark {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<ToeMark> {
        if s == "N" {
            return Ok(ToeMark::Full);
        }
        let at = |p: &str| -> Option<u32> { s.strip_prefix(p)?.parse().ok() };
        if let Some(x) = at("U@") {
            return Ok(ToeMark::Under(x));
        }
        if let Some(x) = at("O@") {
            return Ok(ToeMark::Over(x));
        }
        bail!("bad toe_mark {s:?}")
    }
}

impl From<ToeMark> for String {
    fn from(m: ToeMark) -> String {
        match m {
            ToeMark::Full => "N".into(),
            ToeMark::Under(x) => format!("U@{x}"),
            ToeMark::Over(x) => format!("O@{x}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToeEntry {
    pub weapon: String,
    pub points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub name: String,
    pub abbreviation: Option<String>,
    pub id_code: Option<String>,
    pub parent: Option<UnitId>,
    /// An HQ's level (R-124): none for a unit that is not an HQ.
    #[serde(default)]
    pub echelon: Option<Echelon>,
    /// A recce unit's vehicles for breakdown (R-125): none off the
    /// Recce and Reconnaissance rows.
    #[serde(default)]
    pub vehicle: Option<ReconVehicle>,
    #[serde(default)]
    pub toe: Vec<ToeEntry>,
    pub arrives: Option<Arrival>,
    pub toe_mark: Option<ToeMark>,
    /// Overrides the formation's `basic_morale`.
    pub morale: Option<i32>,
    /// Only where the characteristics row says `by-unit`.
    pub equivalent: Option<Equivalent>,
    /// An HQ's Maximum Attachment Chart rows (19.5, R-130), keys of
    /// `max-attachments.json`; the row whose turns hold the current turn
    /// applies. Empty for a non-HQ unit (its nation's battalion row).
    #[serde(default)]
    pub attachment_parent: Vec<String>,
    /// An HQ's Formation Organisation Chart row (19.3); none where the
    /// O/A notes govern or for a non-HQ unit.
    #[serde(default)]
    pub organisation: Option<String>,
    /// The counter this one replaces in a historical reorganisation:
    /// at this one's arrival it takes the other's hex, TOE and units
    /// (R-096's scenarios start with the replacement made).
    #[serde(default)]
    pub replaces: Option<UnitId>,
    /// With `replaces`: units from other sheets assigned to this counter
    /// when the replacement is made.
    #[serde(default)]
    pub assigns: Vec<UnitId>,
    /// Rommel's counter only (31.0, R-135): a piece of its own, neither
    /// HQ nor combat unit, whose values stand in for a characteristics
    /// row.
    #[serde(default)]
    pub commander: Option<Commander>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// 31.0's values for Rommel (R-135).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Commander {
    pub cpa: u32,
    /// A truck row (`truck-characteristics.json`) for movement and
    /// breakdown.
    pub moves_as: String,
    pub combat_ratings: bool,
    /// Added to the morale of every unit in his hex in combat.
    pub stack_morale_bonus: i32,
    /// Added to the CPA of a unit he starts and spends a whole OpStage
    /// with.
    pub escort_cpa_bonus: u32,
    /// The two-dice total at a Game-Turn's start that recalls him.
    pub recall_roll: u32,
    pub initiative_while_recalled: i32,
}

/// An HQ's organisational level (R-124, 9.22): a regiment HQ is a
/// brigade equivalent; battle groups (19.73) are their own class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Echelon {
    Corps,
    Division,
    Brigade,
    BattleGroup,
}

/// What a reconnaissance unit's TOE points break down as (R-125, 21.1x):
/// keyed into `breakdown-adjustments.json`; camels and motorcycles are
/// exempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReconVehicle {
    ArmouredCar,
    ArmouredRecce,
    Truck,
    #[serde(rename = "none")]
    Exempt,
}

impl ReconVehicle {
    /// The adjustment row's and the wreck's name.
    pub fn as_str(self) -> &'static str {
        match self {
            ReconVehicle::ArmouredCar => "armoured-car",
            ReconVehicle::ArmouredRecce => "armoured-recce",
            ReconVehicle::Truck => "truck",
            ReconVehicle::Exempt => "none",
        }
    }
}

/// A unit printed on one sheet but attached to an HQ of another.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Attachment {
    pub unit: UnitId,
    pub to: UnitId,
}

/// One OA sheet. `id` is `<nation>:<slug>`, as scenarios cite it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Formation {
    pub id: String,
    pub name: String,
    pub basic_morale: Option<i32>,
    #[serde(default)]
    pub attached: Vec<Attachment>,
    pub units: Vec<Unit>,
}

impl Formation {
    /// The hex a garrison sheet garrisons (SPI 9.16's `G`): the sheet's
    /// title names it, as in `Bardia Garrison (Hex: C4321)`; none for
    /// any other sheet.
    pub fn garrison_hex(&self) -> Option<crate::map::HexId> {
        let rest = self.name.split("Garrison (Hex:").nth(1)?;
        let inside = rest.split(')').next()?;
        let compact: String = inside.chars().filter(|c| !c.is_whitespace()).collect();
        crate::map::HexId::parse(&compact).ok()
    }
}

#[derive(Deserialize)]
struct OaFile {
    nation: Nation,
    formations: Vec<Formation>,
}

#[derive(Debug)]
pub struct Oa {
    formations: Vec<Formation>,
    by_formation_id: BTreeMap<String, usize>,
    /// Unit id → (formation index, unit index).
    index: BTreeMap<UnitId, (usize, usize)>,
    children: BTreeMap<UnitId, Vec<UnitId>>,
}

impl Oa {
    /// Loads all three nations' sheets and checks every id is unique and
    /// every parent is a unit on the same sheet.
    pub fn load_all() -> Result<Oa> {
        Oa::load_from(&Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn Source) -> Result<Oa> {
        let mut oa = Oa {
            formations: Vec::new(),
            by_formation_id: BTreeMap::new(),
            index: BTreeMap::new(),
            children: BTreeMap::new(),
        };
        for nation in [Nation::It, Nation::Cw, Nation::De] {
            let path = format!("oa/{nation}.json");
            let file: OaFile = serde_json::from_str(&src.read(&path)?)
                .with_context(|| format!("parsing {path}"))?;
            if file.nation != nation {
                bail!("{path}: nation is {}", file.nation);
            }
            for mut f in file.formations {
                f.id = format!("{nation}:{}", f.id);
                let fi = oa.formations.len();
                if oa.by_formation_id.insert(f.id.clone(), fi).is_some() {
                    bail!("duplicate formation {}", f.id);
                }
                for (ui, u) in f.units.iter().enumerate() {
                    if u.id.nation() != nation {
                        bail!("{}: on a {nation} sheet", u.id);
                    }
                    if oa.index.insert(u.id.clone(), (fi, ui)).is_some() {
                        bail!("duplicate unit {}", u.id);
                    }
                }
                for u in &f.units {
                    if let Some(p) = &u.parent {
                        if !f.units.iter().any(|x| &x.id == p) {
                            bail!("{}: parent {p} is not on sheet {}", u.id, f.id);
                        }
                        oa.children.entry(p.clone()).or_default().push(u.id.clone());
                    }
                }
                oa.formations.push(f);
            }
        }
        Ok(oa)
    }

    pub fn unit(&self, id: &UnitId) -> Option<&Unit> {
        let &(fi, ui) = self.index.get(id)?;
        Some(&self.formations[fi].units[ui])
    }

    /// The units whose `parent` is `id`, in sheet order.
    pub fn children(&self, id: &UnitId) -> &[UnitId] {
        self.children.get(id).map_or(&[], Vec::as_slice)
    }

    /// `id` is `<nation>:<slug>`.
    pub fn formation(&self, id: &str) -> Option<&Formation> {
        Some(&self.formations[*self.by_formation_id.get(id)?])
    }

    /// The sheet a unit is printed on. Panics for an id not in the OA;
    /// check with `unit` first.
    pub fn formation_of(&self, id: &UnitId) -> &Formation {
        let &(fi, _) = self.index.get(id).expect("unit is in the OA");
        &self.formations[fi]
    }

    /// The unit's own morale, else its sheet's basic morale, else none
    /// (a sheet with no morale printed).
    pub fn morale(&self, id: &UnitId) -> Option<i32> {
        let u = self.unit(id)?;
        u.morale.or(self.formation_of(id).basic_morale)
    }

    pub fn formations(&self) -> impl Iterator<Item = &Formation> {
        self.formations.iter()
    }

    pub fn units(&self) -> impl Iterator<Item = &Unit> {
        self.formations.iter().flat_map(|f| f.units.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::characteristics::{Characteristics, Equivalent};
    use crate::time::GameTime;

    #[test]
    fn unit_id_grammar() {
        let id = UnitId::parse("unit:it:maletti-div-hq").unwrap();
        assert_eq!(id.nation(), Nation::It);
        assert_eq!(id.to_string(), "unit:it:maletti-div-hq");
        assert!(UnitId::parse("it:maletti-div-hq").is_err());
        assert!(UnitId::parse("unit:fr:x").is_err());
        assert!(UnitId::parse("unit:it:Bad").is_err());
    }

    #[test]
    fn hqs_carry_their_echelon_r_124() {
        let oa = Oa::load_all().unwrap();
        let echelon = |id: &str| oa.unit(&UnitId::parse(id).unwrap()).unwrap().echelon;
        assert_eq!(
            echelon("unit:cw:7th-armored-div-hq"),
            Some(Echelon::Division)
        );
        assert_eq!(
            echelon("unit:cw:4th-armored-bde-hq"),
            Some(Echelon::Brigade)
        );
        assert_eq!(echelon("unit:cw:6-rtr"), None);
        assert!(oa.units().any(|u| u.echelon == Some(Echelon::BattleGroup)));
        assert!(oa.units().any(|u| u.echelon == Some(Echelon::Corps)));
    }

    #[test]
    fn counts_per_nation() {
        let oa = Oa::load_all().unwrap();
        let count = |n| oa.units().filter(|u| u.id.nation() == n).count();
        assert_eq!(count(Nation::It), 436);
        assert_eq!(count(Nation::Cw), 436);
        // 149 with Rommel's own counter (R-135).
        assert_eq!(count(Nation::De), 149);
        assert_eq!(oa.units().count(), 1021);
    }

    #[test]
    fn every_parent_resolves_in_its_own_formation() {
        let oa = Oa::load_all().unwrap();
        for u in oa.units() {
            if let Some(p) = &u.parent {
                let parent = oa
                    .unit(p)
                    .unwrap_or_else(|| panic!("{}: parent {p} missing", u.id));
                assert_eq!(oa.formation_of(&parent.id).id, oa.formation_of(&u.id).id);
                assert!(oa.children(p).contains(&u.id));
            }
        }
    }

    #[test]
    fn every_id_code_has_a_characteristics_row() {
        let oa = Oa::load_all().unwrap();
        let c = Characteristics::load().unwrap();
        for u in oa.units() {
            if let Some(code) = &u.id_code {
                assert!(
                    c.row(u.id.nation(), code).is_some(),
                    "{}: no characteristics row for {} {code}",
                    u.id,
                    u.id.nation()
                );
            }
        }
    }

    #[test]
    fn garrison_sheets_name_their_hex() {
        let oa = Oa::load_all().unwrap();
        let hex = |f: &str| oa.formation(f).unwrap().garrison_hex();
        assert_eq!(
            hex("it:bardia-garrison-hex-c4321").unwrap().as_str(),
            "C4321"
        );
        assert_eq!(
            hex("it:giariabub-oasis-complex-garrison-hex-c-1014")
                .unwrap()
                .as_str(),
            "C1014"
        );
        assert_eq!(hex("it:minor-garrison-units"), None);
        assert_eq!(hex("it:gruppo-maletti"), None);
    }

    #[test]
    fn unit_morale_overrides_formation() {
        let oa = Oa::load_all().unwrap();
        let bn = UnitId::parse("unit:it:xviii-libyan-bn").unwrap();
        assert_eq!(oa.unit(&bn).unwrap().morale, Some(-2));
        assert_eq!(oa.morale(&bn), Some(-2));
        let hq = UnitId::parse("unit:it:30th-gaf-s-di-c-hq").unwrap();
        assert_eq!(oa.unit(&hq).unwrap().morale, None);
        assert_eq!(oa.morale(&hq), Some(-3));
        let selby = UnitId::parse("unit:cw:selby-force-the-matruh-garrison").unwrap();
        assert_eq!(oa.morale(&selby), None);
    }

    #[test]
    fn unit_fields() {
        let oa = Oa::load_all().unwrap();
        let hq = oa.unit(&UnitId::parse("unit:cw:7-in-hq").unwrap()).unwrap();
        assert_eq!(hq.arrives, Some(Arrival::At(GameTime::new(5, 1))));
        assert_eq!(hq.toe_mark, Some(ToeMark::Full));
        assert_eq!(hq.parent.as_ref().unwrap().as_str(), "unit:cw:4-in-hq");
        let mg = oa
            .unit(&UnitId::parse("unit:it:giariabub-oasis-mg-bn").unwrap())
            .unwrap();
        assert_eq!(mg.toe_mark, Some(ToeMark::Under(4)));
        assert_eq!(mg.arrives, Some(Arrival::Deployed));
        let sarc = oa.unit(&UnitId::parse("unit:cw:1-sarc").unwrap()).unwrap();
        assert_eq!(sarc.equivalent, Some(Equivalent::Battalion));
        let f = oa.formation("it:bardia-garrison-hex-c4321").unwrap();
        assert_eq!(f.units.len(), 11);
        assert_eq!(f.basic_morale, Some(-3));
        assert!(oa.formation("it:nowhere").is_none());
    }
}
