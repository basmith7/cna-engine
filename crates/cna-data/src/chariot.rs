//! The Chariot Raid Table (`chariot-raid.json`, SPI 30.46): one die when
//! the Italian 10th Light Flotilla raids Alexandria (30.4x), giving how
//! many dice of damage to roll.

use crate::errata::load_table;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Die {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Row {
    pub die: Die,
    pub damage_dice: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChariotRaid {
    pub rows: Vec<Row>,
}

impl ChariotRaid {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(load_table(src, "chariot-raid")?)?)
    }

    /// The dice of damage a die roll gives (none off the table).
    pub fn damage_dice(&self, die: u8) -> u8 {
        self.rows
            .iter()
            .find(|r| (r.die.from..=r.die.to).contains(&die))
            .map_or(0, |r| r.damage_dice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_1_rolls_three_dice_and_a_6_none_30_46() {
        let t = ChariotRaid::load().unwrap();
        let dice: Vec<u8> = (1..=6).map(|d| t.damage_dice(d)).collect();
        assert_eq!(dice, [3, 2, 2, 1, 1, 0]);
    }
}
