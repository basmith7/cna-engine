//! The Weather Table (`weather.json`, SPI 29.6, with E-025's turn bands)
//! and the Foul Weather Location Table (`foul-weather-location.json`,
//! 29.7).

use crate::errata::load_table;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeatherKind {
    Normal,
    Hot,
    Sandstorm,
    Rainstorm,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Band {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Readings {
    pub normal: Option<Band>,
    pub hot: Option<Band>,
    pub sandstorm: Option<Band>,
    pub rainstorm: Option<Band>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonRow {
    pub season: String,
    /// Game-Turn bands, errata applied.
    pub turns: Vec<Band>,
    pub weather: Readings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeatherTable {
    pub rows: Vec<SeasonRow>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocationRow {
    pub die: u8,
    pub sections: Vec<char>,
    pub delta_sandstorm_is_normal: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FoulWeatherLocation {
    pub rows: Vec<LocationRow>,
}

#[derive(Debug, Clone)]
pub struct Weather {
    pub table: WeatherTable,
    pub location: FoulWeatherLocation,
}

impl Weather {
    pub fn load() -> Result<Weather> {
        Ok(Weather {
            table: serde_json::from_value(load_table("weather")?)?,
            location: serde_json::from_value(load_table("foul-weather-location")?)?,
        })
    }

    /// The season row whose turn band holds `game_turn`.
    pub fn season(&self, game_turn: u8) -> Result<&SeasonRow> {
        self.table
            .rows
            .iter()
            .find(|r| {
                r.turns
                    .iter()
                    .any(|b| b.from <= game_turn && game_turn <= b.to)
            })
            .with_context(|| format!("Game-Turn {game_turn}: no season band"))
    }

    /// The weather a sequential reading (11–66) gives in `game_turn`'s
    /// season.
    pub fn roll(&self, game_turn: u8, reading: u8) -> Result<WeatherKind> {
        let r = &self.season(game_turn)?.weather;
        let hit = |b: &Option<Band>| {
            b.as_ref()
                .is_some_and(|b| b.from <= reading && reading <= b.to)
        };
        if hit(&r.normal) {
            Ok(WeatherKind::Normal)
        } else if hit(&r.hot) {
            Ok(WeatherKind::Hot)
        } else if hit(&r.sandstorm) {
            Ok(WeatherKind::Sandstorm)
        } else if hit(&r.rainstorm) {
            Ok(WeatherKind::Rainstorm)
        } else {
            anyhow::bail!("Game-Turn {game_turn}: reading {reading} is in no weather band")
        }
    }

    /// The sheets a storm covers for one die (29.7).
    pub fn storm_sheets(&self, die: u8) -> Result<&[char]> {
        self.location
            .rows
            .iter()
            .find(|r| r.die == die)
            .map(|r| r.sections.as_slice())
            .with_context(|| format!("foul weather location: no row for a {die}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graziani_is_autumn_after_e025() {
        let w = Weather::load().unwrap();
        for gt in 1..=6 {
            assert_eq!(w.season(gt).unwrap().season, "autumn");
        }
        assert_eq!(w.season(13).unwrap().season, "winter");
        assert_eq!(w.season(25).unwrap().season, "spring");
        assert_eq!(w.season(37).unwrap().season, "summer");
        assert_eq!(w.season(110).unwrap().season, "winter");
    }

    #[test]
    fn autumn_readings() {
        let w = Weather::load().unwrap();
        assert_eq!(w.roll(1, 11).unwrap(), WeatherKind::Normal);
        assert_eq!(w.roll(1, 35).unwrap(), WeatherKind::Normal);
        assert_eq!(w.roll(1, 36).unwrap(), WeatherKind::Hot);
        assert_eq!(w.roll(1, 55).unwrap(), WeatherKind::Sandstorm);
        assert_eq!(w.roll(1, 62).unwrap(), WeatherKind::Rainstorm);
        assert_eq!(w.roll(1, 66).unwrap(), WeatherKind::Rainstorm);
        // Winter has no hot weather and no sandstorm.
        assert_eq!(w.roll(13, 52).unwrap(), WeatherKind::Normal);
        assert_eq!(w.roll(13, 53).unwrap(), WeatherKind::Rainstorm);
        assert!(w.roll(1, 10).is_err());
    }

    #[test]
    fn storm_location() {
        let w = Weather::load().unwrap();
        assert_eq!(w.storm_sheets(1).unwrap(), ['A', 'B']);
        assert_eq!(w.storm_sheets(6).unwrap(), ['B', 'C', 'D']);
        assert!(w.storm_sheets(7).is_err());
    }
}
