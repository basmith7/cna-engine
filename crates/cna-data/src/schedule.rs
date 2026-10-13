//! The Land Unit Reinforcement and Withdrawal Schedules
//! (`reinforcement-schedule.json`, SPI 4.43): rows by side and OpStage.
//! A unit named stands for its OA sheet, as in a set-up (59.2).

use crate::errata::load_table;
use crate::oa::UnitId;
use crate::scenario::{Side, Trucks};
use crate::time::GameTime;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Arrives,
    Withdraws,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Row {
    pub side: Side,
    #[serde(flatten)]
    pub at: GameTime,
    pub kind: Kind,
    pub units: Vec<UnitId>,
    pub trucks: Option<Trucks>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Deserialize)]
struct Table {
    through: u8,
    rows: Vec<Row>,
}

#[derive(Debug)]
pub struct Schedule {
    /// The last Game-Turn the schedule covers.
    pub through: u8,
    rows: Vec<Row>,
}

impl Schedule {
    /// Loads the schedule with its errata applied. Rows that bring no
    /// counter (`unplaced`) are not read.
    pub fn load() -> Result<Schedule> {
        let t: Table = serde_json::from_value(load_table("reinforcement-schedule")?)
            .context("parsing reinforcement-schedule")?;
        Ok(Schedule {
            through: t.through,
            rows: t.rows,
        })
    }

    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter()
    }

    /// The `arrives` rows for one side at one OpStage.
    pub fn arrivals(&self, side: Side, at: GameTime) -> impl Iterator<Item = &Row> {
        self.rows
            .iter()
            .filter(move |r| r.kind == Kind::Arrives && r.side == side && r.at == at)
    }

    /// The `withdraws` rows for one side at one OpStage.
    pub fn withdrawals(&self, side: Side, at: GameTime) -> impl Iterator<Item = &Row> {
        self.rows
            .iter()
            .filter(move |r| r.kind == Kind::Withdraws && r.side == side && r.at == at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_turns_one_to_six_are_six_commonwealth_rows() {
        let s = Schedule::load().unwrap();
        assert_eq!(s.rows().count(), 278);
        let early: Vec<&Row> = s.rows().filter(|r| r.at.game_turn <= 6).collect();
        assert_eq!(early.len(), 6);
        let expect = [
            (1, 3, vec!["unit:cw:polish-hq"]),
            (
                2,
                3,
                vec!["unit:cw:2-rtr", "unit:cw:7-rtr", "unit:cw:6-nz-fld"],
            ),
            (3, 3, vec!["unit:cw:1-fld"]),
            (4, 3, vec!["unit:cw:3-hus"]),
            (5, 1, vec!["unit:cw:7-in-hq"]),
            (6, 3, vec!["unit:cw:6-nz-hq"]),
        ];
        for (row, (gt, st, units)) in early.iter().zip(expect) {
            assert_eq!(row.side, Side::Cw);
            assert_eq!(row.kind, Kind::Arrives);
            assert_eq!(row.at, GameTime::new(gt, st));
            let ids: Vec<&str> = row.units.iter().map(UnitId::as_str).collect();
            assert_eq!(ids, units);
        }
    }

    #[test]
    fn arrivals_are_by_side_and_time() {
        let s = Schedule::load().unwrap();
        let rows: Vec<&Row> = s.arrivals(Side::Cw, GameTime::new(2, 3)).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].units.len(), 3);
        assert_eq!(s.arrivals(Side::Axis, GameTime::new(2, 3)).count(), 0);
        assert_eq!(s.arrivals(Side::Cw, GameTime::new(2, 2)).count(), 0);
        let trucks = s
            .arrivals(Side::Cw, GameTime::new(5, 1))
            .next()
            .unwrap()
            .trucks
            .as_ref()
            .unwrap();
        assert_eq!((trucks.light, trucks.medium, trucks.heavy), (5, 20, 0));
        assert!(s.rows().any(|r| r.kind == Kind::Withdraws));
    }
}
