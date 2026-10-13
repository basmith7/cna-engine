//! The Prisoners Captured Results Table (`prisoners-captured.json`, SPI
//! 15.89): the share of the losses just taken that are prisoners.

use crate::errata::load_table;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Die {
    pub from: u8,
    pub to: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    pub die: Die,
    pub percent: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Prisoners {
    pub rows: Vec<Row>,
}

impl Prisoners {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(load_table(
            src,
            "prisoners-captured",
        )?)?)
    }

    /// The percentage for one die.
    pub fn percent(&self, die: u8) -> Option<u32> {
        self.rows
            .iter()
            .find(|r| (r.die.from..=r.die.to).contains(&die))
            .map(|r| r.percent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_die() {
        let t = Prisoners::load().unwrap();
        assert_eq!(t.percent(1), Some(10));
        assert_eq!(t.percent(4), Some(50));
        assert_eq!(t.percent(5), Some(50));
        assert_eq!(t.percent(6), Some(75));
        assert_eq!(t.percent(7), None);
    }
}
