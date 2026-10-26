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
