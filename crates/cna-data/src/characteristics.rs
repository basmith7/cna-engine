//! The Unit Characteristics Chart (`unit-characteristics.json`, SPI 4.46):
//! one row per nation and ID code. Cells keep the printed marks, so a
//! rating is a number, a mark, and the raw text, never a retyped value.

use crate::errata::load_table;
use crate::oa::Nation;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A printed mark after a rating. The chart's key gives the first three;
/// anything else is kept verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mark {
    /// `+`: historically had the trucks to motorise fully (4.46).
    Motorisable,
    /// `*` or `**`: keeps its CPA whatever TOE points it carries.
    KeepsCpa,
    /// `(n)`: usable only when no unit in the hex has the plain rating.
    Parenthesised,
    Other(String),
}

/// One chart cell: a value, an optional mark, and the cell as printed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Cell", into = "String")]
pub struct Rating {
    pub value: Option<i32>,
    pub mark: Option<Mark>,
    pub raw: String,
}

/// The JSON forms a cell takes: an integer or a string with marks.
#[derive(Deserialize)]
#[serde(untagged)]
enum Cell {
    Int(i64),
    Str(String),
}

impl TryFrom<Cell> for Rating {
    type Error = anyhow::Error;
    fn try_from(c: Cell) -> Result<Rating> {
        match c {
            Cell::Int(n) => Ok(Rating {
                value: Some(i32::try_from(n)?),
                mark: None,
                raw: n.to_string(),
            }),
            Cell::Str(s) => Rating::parse(&s),
        }
    }
}

impl From<Rating> for String {
    fn from(r: Rating) -> String {
        r.raw
    }
}

impl Rating {
    /// Parses a printed cell: an optional leading integer, then a mark.
    /// `10+` is 10 motorisable; `(1)` is 1 parenthesised; `9+3` keeps
    /// `+3` as an `Other` mark, since it is not the motorisable `+`.
    pub fn parse(raw: &str) -> Result<Rating> {
        let raw = raw.trim();
        if raw.is_empty() {
            bail!("empty rating cell");
        }
        if let Some(inner) = raw.strip_prefix('(').and_then(|s| s.strip_suffix(')'))
            && let Ok(v) = inner.parse::<i32>()
        {
            return Ok(Rating {
                value: Some(v),
                mark: Some(Mark::Parenthesised),
                raw: raw.to_owned(),
            });
        }
        let digits = raw.len() - raw.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let value = (digits > 0)
            .then(|| raw[..digits].parse::<i32>())
            .transpose()?;
        let rest = &raw[digits..];
        let mark = match rest {
            "" => None,
            _ if value.is_none() => Some(Mark::Other(rest.to_owned())),
            _ if rest.starts_with('+') && !rest.chars().any(|c| c.is_ascii_digit()) => {
                Some(Mark::Motorisable)
            }
            _ if rest.starts_with('*') => Some(Mark::KeepsCpa),
            _ => Some(Mark::Other(rest.to_owned())),
        };
        Ok(Rating {
            value,
            mark,
            raw: raw.to_owned(),
        })
    }
}

/// The stacking equivalent of a row (SPI 9.4). `ByUnit` rows defer to the
/// OA unit's own `equivalent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Equivalent {
    Brigade,
    Battalion,
    Company,
    Hq,
    ByUnit,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Row {
    pub nation: Nation,
    pub id_code: String,
    pub unit_type: String,
    pub equivalent: Equivalent,
    pub cpa: Option<Rating>,
    pub anti_air: Option<Rating>,
    pub barrage: Option<Rating>,
    pub anti_armour: Option<Rating>,
    pub vulnerability: Option<Rating>,
    pub armour_protection: Option<Rating>,
    pub close_assault_offence: Option<Rating>,
    pub close_assault_defence: Option<Rating>,
    pub max_toe: Option<Rating>,
    pub assign_note: Option<String>,
}

#[derive(Deserialize)]
struct Table {
    rows: Vec<Row>,
}

#[derive(Debug)]
pub struct Characteristics {
    rows: BTreeMap<(Nation, String), Row>,
}

impl Characteristics {
    /// Loads the chart with its errata applied.
    pub fn load() -> Result<Characteristics> {
        let table: Table = serde_json::from_value(load_table("unit-characteristics")?)
            .context("parsing unit-characteristics")?;
        let mut rows = BTreeMap::new();
        for r in table.rows {
            let key = (r.nation, r.id_code.clone());
            if rows.insert(key, r).is_some() {
                bail!("unit-characteristics: duplicate row");
            }
        }
        Ok(Characteristics { rows })
    }

    pub fn row(&self, nation: Nation, id_code: &str) -> Option<&Row> {
        self.rows.get(&(nation, id_code.to_owned()))
    }

    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oa::Nation;

    #[test]
    fn motorisable_mark_parses() {
        let r = Rating::parse("10+").unwrap();
        assert_eq!(r.value, Some(10));
        assert_eq!(r.mark, Some(Mark::Motorisable));
        assert_eq!(r.raw, "10+");
    }

    #[test]
    fn other_marks_parse() {
        assert_eq!(Rating::parse("30*").unwrap().mark, Some(Mark::KeepsCpa));
        assert_eq!(Rating::parse("15**").unwrap().mark, Some(Mark::KeepsCpa));
        let p = Rating::parse("(1)").unwrap();
        assert_eq!((p.value, p.mark), (Some(1), Some(Mark::Parenthesised)));
        let plain = Rating::parse("12").unwrap();
        assert_eq!((plain.value, plain.mark), (Some(12), None));
        let odd = Rating::parse("9+3").unwrap();
        assert_eq!(
            (odd.value, odd.mark),
            (Some(9), Some(Mark::Other("+3".into())))
        );
        let star = Rating::parse("*").unwrap();
        assert_eq!(
            (star.value, star.mark),
            (None, Some(Mark::Other("*".into())))
        );
        assert!(Rating::parse("").is_err());
    }

    #[test]
    fn loads_every_row_once() {
        let c = Characteristics::load().unwrap();
        assert_eq!(c.rows().count(), 124);
        let a = c.row(Nation::Cw, "a").unwrap();
        assert_eq!(a.equivalent, Equivalent::Hq);
        assert_eq!(a.cpa.as_ref().unwrap().value, Some(30));
        assert!(a.barrage.is_none());
        let g = c.row(Nation::Cw, "g").unwrap();
        assert_eq!(g.max_toe.as_ref().unwrap().value, Some(10));
        assert_eq!(
            c.row(Nation::Cw, "uu").unwrap().equivalent,
            Equivalent::ByUnit
        );
        assert!(c.row(Nation::De, "zzz").is_none());
    }
}
