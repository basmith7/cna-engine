//! The assault size shifts (`assault-size-shifts.json`, SPI 15.53,
//! E-006): columns the side with the larger unit equivalent gains.

use crate::errata::load_table;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Row {
    /// `division`, `brigade-3`, `brigade`, `battalion`.
    pub larger: String,
    /// `brigade-3`, `brigade-2`, `battalion`, `company`.
    pub smaller: String,
    pub shift: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssaultSizeShifts {
    pub rows: Vec<Row>,
}

impl AssaultSizeShifts {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::source::Fs)
    }

    /// The same, from any `Source`.
    pub fn load_from(src: &dyn crate::source::Source) -> Result<Self> {
        Ok(serde_json::from_value(load_table(src, "assault-size-shifts")?)?)
    }

    /// The shift for a pairing; none where the chart has no row (equal
    /// sizes, or a brigade against a brigade).
    pub fn shift(&self, larger: &str, smaller: &str) -> Option<i32> {
        self.rows
            .iter()
            .find(|r| {
                (r.larger == larger || (r.larger == "brigade" && larger.starts_with("brigade")))
                    && r.smaller == smaller
            })
            .map(|r| r.shift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairings() {
        let t = AssaultSizeShifts::load().unwrap();
        assert_eq!(t.shift("division", "battalion"), Some(4));
        assert_eq!(t.shift("division", "brigade-2"), Some(2));
        assert_eq!(t.shift("brigade-3", "brigade-2"), Some(0));
        assert_eq!(t.shift("brigade-2", "battalion"), Some(2));
        assert_eq!(t.shift("battalion", "company"), Some(2));
        assert_eq!(t.shift("battalion", "battalion"), None);
    }
}
