//! The Initiative Ratings Chart (`initiative-ratings.json`, SPI 7.2).

use crate::errata::load_table;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Turns {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CwRow {
    pub turns: Turns,
    pub rating: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AxisRow {
    pub situation: String,
    pub rating: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitiativeRatings {
    pub commonwealth: Vec<CwRow>,
    pub axis: Vec<AxisRow>,
}

/// What the Axis has on the map (7.1x): the holding boxes do not count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisSituation {
    RommelOnMap,
    GermanLandCombatUnitsNoRommel,
    NoGermanCombatUnits,
}

impl AxisSituation {
    fn key(self) -> &'static str {
        match self {
            AxisSituation::RommelOnMap => "rommel-on-map",
            AxisSituation::GermanLandCombatUnitsNoRommel => "german-land-combat-units-no-rommel",
            AxisSituation::NoGermanCombatUnits => "no-german-combat-units",
        }
    }
}

impl InitiativeRatings {
    pub fn load() -> Result<InitiativeRatings> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<InitiativeRatings> {
        Ok(serde_json::from_value(load_table(
            src,
            "initiative-ratings",
        )?)?)
    }

    pub fn commonwealth(&self, game_turn: u8) -> Result<u8> {
        self.commonwealth
            .iter()
            .find(|r| r.turns.from <= game_turn && game_turn <= r.turns.to)
            .map(|r| r.rating)
            .with_context(|| format!("Game-Turn {game_turn}: no Commonwealth initiative rating"))
    }

    pub fn axis(&self, situation: AxisSituation) -> Result<u8> {
        self.axis
            .iter()
            .find(|r| r.situation == situation.key())
            .map(|r| r.rating)
            .with_context(|| format!("no Axis initiative rating for {situation:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_values() {
        let t = InitiativeRatings::load().unwrap();
        assert_eq!(t.commonwealth(1).unwrap(), 3);
        assert_eq!(t.commonwealth(42).unwrap(), 3);
        assert_eq!(t.commonwealth(43).unwrap(), 4);
        assert_eq!(t.commonwealth(111).unwrap(), 5);
        assert!(t.commonwealth(112).is_err());
        assert_eq!(t.axis(AxisSituation::NoGermanCombatUnits).unwrap(), 1);
        assert_eq!(
            t.axis(AxisSituation::GermanLandCombatUnitsNoRommel)
                .unwrap(),
            3
        );
        assert_eq!(t.axis(AxisSituation::RommelOnMap).unwrap(), 6);
    }
}
