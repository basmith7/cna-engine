//! The Game-Turn calendar (`calendar.json`): which year, month and week
//! each Game-Turn falls in. Every table keyed by month is read through
//! it.

use anyhow::Result;
use serde::Deserialize;

/// One Game-Turn's week: `week` is 1-4 for I-IV.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Week {
    pub game_turn: u32,
    pub year: u32,
    pub month: u32,
    pub week: u32,
}

impl Week {
    /// The month as the monthly tables key it, `YYYY-MM`.
    pub fn month_key(&self) -> String {
        format!("{}-{:02}", self.year, self.month)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Calendar {
    pub rows: Vec<Week>,
}

impl Calendar {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            src, "calendar",
        )?)?)
    }

    /// The week of `game_turn`, if the campaign has one.
    pub fn week(&self, game_turn: u32) -> Option<Week> {
        self.rows.iter().copied().find(|w| w.game_turn == game_turn)
    }

    /// Whether `game_turn` is the first of its month.
    pub fn starts_month(&self, game_turn: u32) -> bool {
        match (self.week(game_turn), self.week(game_turn.wrapping_sub(1))) {
            (Some(now), Some(before)) => now.month_key() != before.month_key(),
            (Some(_), None) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_turn_1_is_september_iii_1940_and_june_i_1941_is_35() {
        let c = Calendar::load().unwrap();
        let w = c.week(1).unwrap();
        assert_eq!((w.year, w.month, w.week), (1940, 9, 3));
        assert_eq!(w.month_key(), "1940-09");
        let w = c.week(35).unwrap();
        assert_eq!((w.year, w.month, w.week), (1941, 6, 1));
        assert_eq!(c.week(112), None);
    }

    #[test]
    fn a_month_starts_at_its_first_week() {
        let c = Calendar::load().unwrap();
        assert!(c.starts_month(1));
        assert!(!c.starts_month(2));
        assert!(c.starts_month(3));
        assert!(c.starts_month(15));
        assert!(!c.starts_month(16));
    }
}

/// One month of the Abstract Motorisation Point Loss Chart
/// (`motorisation-losses.json`, SPI 32.59).
#[derive(Debug, Clone, Deserialize)]
pub struct MonthlyLoss {
    /// `YYYY-MM`.
    pub month: String,
    pub cw_percent: u32,
    pub axis_percent: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MotorisationLosses {
    pub rows: Vec<MonthlyLoss>,
}

impl MotorisationLosses {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            src,
            "motorisation-losses",
        )?)?)
    }

    /// The month's row (`YYYY-MM`); none for a month the chart prints as
    /// a dash.
    pub fn month(&self, key: &str) -> Option<&MonthlyLoss> {
        self.rows.iter().find(|r| r.month == key)
    }
}

/// The `YYYY-MM` key of the month before `week`'s.
pub fn previous_month(week: &Week) -> String {
    if week.month == 1 {
        format!("{}-12", week.year - 1)
    } else {
        format!("{}-{:02}", week.year, week.month - 1)
    }
}

#[cfg(test)]
mod loss_tests {
    use super::*;

    #[test]
    fn the_chart_starts_in_october_1940() {
        let t = MotorisationLosses::load().unwrap();
        assert!(t.month("1940-09").is_none());
        let oct = t.month("1940-10").unwrap();
        assert_eq!((oct.cw_percent, oct.axis_percent), (6, 2));
        let c = Calendar::load().unwrap();
        assert_eq!(previous_month(&c.week(15).unwrap()), "1940-12");
        assert_eq!(previous_month(&c.week(7).unwrap()), "1940-10");
    }
}

/// One die row of a Simplified Supply Availability Table: units by
/// column (tonnage letter or supply period).
#[derive(Debug, Clone, Deserialize)]
pub struct SupplyRow {
    pub die: u8,
    pub units: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AxisSupplyTable {
    pub letters: Vec<String>,
    pub arrive_turns_later: u32,
    pub rows: Vec<SupplyRow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SupplyPeriod {
    pub period: String,
    /// `YYYY-MM`, inclusive.
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CwSupplyTable {
    pub periods: Vec<SupplyPeriod>,
    pub arrive_turns_later: u32,
    pub rows: Vec<SupplyRow>,
}

/// The Simplified Supply Availability Tables (`simplified-supply.json`,
/// SPI 32.46, 32.47) and the Axis Naval Convoy Level Chart
/// (`axis-convoy-level.json`, SPI 56.4) whose letters the Axis table
/// reads.
#[derive(Debug, Clone)]
pub struct SimplifiedSupply {
    pub axis: AxisSupplyTable,
    pub cw: CwSupplyTable,
    /// Letters by year, January first; none for a dash.
    pub levels: std::collections::BTreeMap<String, Vec<Option<String>>>,
}

#[derive(Deserialize)]
struct SupplyFile {
    axis: AxisSupplyTable,
    cw: CwSupplyTable,
}

#[derive(Deserialize)]
struct LevelFile {
    levels: std::collections::BTreeMap<String, Vec<Option<String>>>,
}

impl SimplifiedSupply {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        let f: SupplyFile =
            serde_json::from_value(crate::errata::load_table(src, "simplified-supply")?)?;
        let l: LevelFile =
            serde_json::from_value(crate::errata::load_table(src, "axis-convoy-level")?)?;
        Ok(SimplifiedSupply {
            axis: f.axis,
            cw: f.cw,
            levels: l.levels,
        })
    }

    /// The tonnage letter of a month (`YYYY-MM`); none before September
    /// 1940 or after the chart.
    pub fn letter(&self, month: &str) -> Option<&str> {
        let (year, m) = month.split_once('-')?;
        let m: usize = m.parse().ok()?;
        self.levels.get(year)?.get(m.checked_sub(1)?)?.as_deref()
    }

    /// Axis units on `die` under the month's letter (32.46); none
    /// without a letter.
    pub fn axis_units(&self, month: &str, die: u8) -> Option<u32> {
        let letter = self.letter(month)?;
        let col = self.axis.letters.iter().position(|l| l == letter)?;
        self.axis
            .rows
            .iter()
            .find(|r| r.die == die)?
            .units
            .get(col)
            .copied()
    }

    /// Commonwealth units on `die` in the period holding the month
    /// (32.47).
    pub fn cw_units(&self, month: &str, die: u8) -> Option<u32> {
        let col =
            self.cw.periods.iter().position(|p| {
                p.from.as_str() <= month && p.to.as_deref().is_none_or(|t| month <= t)
            })?;
        self.cw
            .rows
            .iter()
            .find(|r| r.die == die)?
            .units
            .get(col)
            .copied()
    }
}

#[cfg(test)]
mod supply_tests {
    use super::*;

    #[test]
    fn letters_and_periods_pick_the_columns_32_46_32_47() {
        let t = SimplifiedSupply::load().unwrap();
        assert_eq!(t.letter("1940-08"), None);
        assert_eq!(t.letter("1940-09"), Some("B"));
        assert_eq!(t.letter("1941-11"), Some("E"));
        // B: a 1 brings nothing, a 6 at least one.
        assert_eq!(t.axis_units("1940-09", 1), Some(0));
        assert_eq!(t.axis_units("1940-08", 6), None);
        assert_eq!(t.cw_units("1940-10", 1), Some(1));
        assert_eq!(t.cw_units("1941-05", 6), Some(4));
        assert_eq!(t.cw_units("1942-06", 6), Some(7));
    }
}
