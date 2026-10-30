//! Bombing simplified Axis convoys (32.6x): the Simple Axis Naval Convoy
//! Bombing Chart (`axis-convoy-bombing.json`, 32.66), which turns the
//! westernmost full Commonwealth division into bomb points per lane, and
//! the Axis naval convoy rows of the Air Bombardment Table
//! (`air-bombardment.json`, 41.5), which turn bomb points and two dice
//! into the percentage of the convoy lost.

use crate::close_assault::Range;
use anyhow::{Context, Result};
use serde::Deserialize;

/// A map sheet and an east-west column (any row): the chart's "Cxx01".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Threshold {
    pub map: char,
    pub column: u8,
}

impl Threshold {
    /// West to east: the sheet, then the column.
    fn key(self) -> (char, u8) {
        (self.map, self.column)
    }
}

/// Bomb points, `to` none for "and more".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct BombPoints {
    pub from: u32,
    pub to: Option<u32>,
}

impl BombPoints {
    pub fn contains(self, n: u32) -> bool {
        n >= self.from && self.to.is_none_or(|t| n <= t)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BombingRow {
    pub bomb_points: BombPoints,
    /// One per lane in `routes` order; none is the chart's dash.
    pub thresholds: Vec<Option<Threshold>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConvoyBombing {
    pub routes: Vec<u8>,
    pub rows: Vec<BombingRow>,
}

impl ConvoyBombing {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            src,
            "axis-convoy-bombing",
        )?)?)
    }

    /// The bomb points against a convoy on `lane` when the westernmost
    /// full Commonwealth division stands in `sheet` and `column`: down
    /// the lane's column while the division is in or west of each row's
    /// threshold, the last such row (32.66). None when it is east of the
    /// first, or the lane is not on the chart.
    pub fn bomb_points(&self, lane: u8, sheet: char, column: u8) -> Option<BombPoints> {
        let i = self.routes.iter().position(|r| *r == lane)?;
        self.rows
            .iter()
            .take_while(|row| row.thresholds[i].is_some_and(|t| (sheet, column) <= t.key()))
            .last()
            .map(|row| row.bomb_points)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Column {
    bomb: BombPoints,
}

#[derive(Debug, Clone, Deserialize)]
struct ResultRow {
    result: serde_json::Value,
    dice: Vec<Option<Range>>,
}

#[derive(Debug, Clone, Deserialize)]
struct Group {
    id: String,
    rows: Vec<ResultRow>,
}

#[derive(Debug, Clone, Deserialize)]
struct BombardmentFile {
    columns: Vec<Column>,
    groups: Vec<Group>,
}

/// The Air Bombardment Table's Axis naval convoy rows: per column, the
/// percentage lost for each two-dice reading (41.66).
#[derive(Debug, Clone)]
pub struct ConvoyBombardment {
    columns: Vec<BombPoints>,
    /// (percent, the reading range per column).
    rows: Vec<(u32, Vec<Option<Range>>)>,
}

impl ConvoyBombardment {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        let file: BombardmentFile =
            serde_json::from_value(crate::errata::load_table(src, "air-bombardment")?)?;
        let group = file
            .groups
            .into_iter()
            .find(|g| g.id == "axis-naval-convoy")
            .context("air-bombardment.json has no axis-naval-convoy group")?;
        let rows = group
            .rows
            .into_iter()
            .map(|r| {
                let pct = r
                    .result
                    .as_u64()
                    .context("a convoy result is a percentage")?;
                Ok((u32::try_from(pct)?, r.dice))
            })
            .collect::<Result<_>>()?;
        Ok(ConvoyBombardment {
            columns: file.columns.into_iter().map(|c| c.bomb).collect(),
            rows,
        })
    }

    /// The percentage of the convoy lost to `bomb_points` on a two-dice
    /// `reading` (11–66, read in order); none off the table.
    pub fn percent(&self, bomb_points: u32, reading: u8) -> Option<u32> {
        let col = self.columns.iter().position(|c| c.contains(bomb_points))?;
        self.rows
            .iter()
            .find(|(_, dice)| dice[col].is_some_and(|r| r.contains(reading)))
            .map(|(pct, _)| *pct)
    }
}

/// Where a convoy lane ends: a port, in an off-map box or on a map hex.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Destination {
    pub port: String,
    #[serde(rename = "box")]
    pub off_map_box: Option<String>,
    pub hex: Option<String>,
}

/// One Axis convoy lane (56.1): a fixed origin and destination, its number
/// the Convoy Route column of the bombing chart (32.66).
#[derive(Debug, Clone, Deserialize)]
pub struct Lane {
    pub lane: u8,
    pub name: String,
    pub origin: String,
    pub destination: Destination,
    /// The first Game-Turn it may be used; none for from the start.
    pub first_game_turn: Option<u8>,
    /// Whether opening it also needs the Axis's activation roll (55.17).
    pub activation_roll: bool,
}

/// The Axis convoy lanes (`convoy-lanes.json`, 56.1).
#[derive(Debug, Clone, Deserialize)]
pub struct ConvoyLanes {
    pub lanes: Vec<Lane>,
}

impl ConvoyLanes {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table(
            src,
            "convoy-lanes",
        )?)?)
    }

    /// The lane that ends in off-map box `id` (e.g. "tripoli").
    pub fn to_box(&self, id: &str) -> Option<&Lane> {
        self.lanes
            .iter()
            .find(|l| l.destination.off_map_box.as_deref() == Some(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_division_s_column_picks_the_last_row_reached_on_the_lane_32_66() {
        let chart = ConvoyBombing::load().unwrap();
        // Lane 1 reads C1, B16, B1, A18 down its column.
        assert_eq!(chart.bomb_points(1, 'D', 5), None, "east of C1");
        let at = |s, c| chart.bomb_points(1, s, c).map(|b| b.from);
        assert_eq!(at('C', 1), Some(21), "in the threshold column");
        assert_eq!(at('B', 20), Some(21));
        assert_eq!(at('B', 16), Some(41));
        assert_eq!(at('A', 30), Some(81));
        // Below A18 lane 1 has only dashes: 121-160 is as hard as it gets.
        assert_eq!(at('A', 1), Some(121));
        assert_eq!(chart.bomb_points(7, 'A', 1), None, "no lane 7");
        // Lane 5's last row is 471 and more.
        let deepest = chart.bomb_points(5, 'A', 1).unwrap();
        assert_eq!((deepest.from, deepest.to), (471, None));
    }

    #[test]
    fn a_reading_gives_the_percentage_lost_41_66() {
        let t = ConvoyBombardment::load().unwrap();
        // 1-20 bomb points: nothing on any reading.
        assert_eq!(t.percent(10, 66), Some(0));
        // 21-40: 55-64 is 5 %, 65-66 is 10 %.
        assert_eq!(t.percent(30, 54), Some(0));
        assert_eq!(t.percent(30, 55), Some(5));
        assert_eq!(t.percent(30, 66), Some(10));
        // 81-120: 65-66 is 20 %.
        assert_eq!(t.percent(100, 65), Some(20));
        // Every column covers 11-66 once.
        for bomb in [10, 30, 60, 100, 140, 180, 230, 300, 350, 430, 500] {
            for tens in 1..=6u8 {
                for ones in 1..=6u8 {
                    assert!(
                        t.percent(bomb, tens * 10 + ones).is_some(),
                        "{bomb} {tens}{ones}"
                    );
                }
            }
        }
    }
    #[test]
    fn six_lanes_and_lane_2_runs_to_tripoli_from_the_start_56_1() {
        let lanes = ConvoyLanes::load().unwrap();
        assert_eq!(lanes.lanes.len(), 6);
        let tripoli = lanes.to_box("tripoli").unwrap();
        assert_eq!(tripoli.lane, 2);
        assert_eq!(tripoli.first_game_turn, None);
        assert!(!tripoli.activation_roll);
        let bizerta = lanes.to_box("tunisia").unwrap();
        assert_eq!((bizerta.lane, bizerta.first_game_turn), (1, Some(35)));
        assert!(bizerta.activation_roll);
    }
}
