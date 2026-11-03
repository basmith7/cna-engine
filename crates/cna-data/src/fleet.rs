//! The Commonwealth Fleet Reinforcement Schedule (`cw-fleet-schedule.json`,
//! SPI 30.59): each warship counter, its type and the OpStage it deploys
//! in, and its counter's gun and AA ratings (30.12).

use crate::errata::load_table;
use crate::time::GameTime;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Arrival {
    pub turn: u8,
    pub stage: u8,
}

/// One warship counter.
#[derive(Debug, Clone, Deserialize)]
pub struct Ship {
    /// The ship's name as cna spells it (`queen-elizabeth`).
    pub ship: String,
    /// BB, CA, CL, CLAA or DD.
    #[serde(rename = "type")]
    pub class: String,
    pub arrival: Arrival,
    /// Printed gun rating: barrage points needing no ammunition, and the
    /// ship's hit points (30.12–30.13).
    pub gun_rating: u32,
    /// Printed AA rating in AA points (30.12).
    pub aa_rating: u32,
}

impl Ship {
    pub fn arrives(&self) -> GameTime {
        GameTime::new(self.arrival.turn, self.arrival.stage)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FleetSchedule {
    pub rows: Vec<Ship>,
}

impl FleetSchedule {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(load_table(
            src,
            "cw-fleet-schedule",
        )?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schedule_has_its_nineteen_ships_eleven_at_the_start() {
        let s = FleetSchedule::load().unwrap();
        assert_eq!(s.rows.len(), 19);
        let first = s
            .rows
            .iter()
            .filter(|r| r.arrives() == GameTime::new(1, 1))
            .count();
        assert_eq!(first, 11);
    }

    #[test]
    fn every_ship_has_its_ratings() {
        let s = FleetSchedule::load().unwrap();
        let valiant = s.rows.iter().find(|r| r.ship == "valiant").unwrap();
        assert_eq!((valiant.gun_rating, valiant.aa_rating), (6, 12));
        assert!(s.rows.iter().all(|r| r.gun_rating >= 1 && r.aa_rating >= 1));
    }
}
