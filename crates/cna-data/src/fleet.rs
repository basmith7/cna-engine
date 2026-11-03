//! The Commonwealth Fleet Reinforcement Schedule (`cw-fleet-schedule.json`,
//! SPI 30.59): each warship counter, its type and the OpStage it deploys
//! in. Names, types and arrivals only: the counters' gun and AA ratings
//! are not in cna's data.

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
}
