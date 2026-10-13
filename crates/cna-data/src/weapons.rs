//! The Tank and Gun Characteristics Charts (`weapon-systems.json`, SPI
//! 4.47–4.49): the ratings of one TOE point of each weapon system, and
//! the map from the names the OA sheets print to a row.

use crate::characteristics::Rating;
use crate::errata::load_table;
use crate::oa::Nation;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::BTreeMap;

/// The chart's section a row is printed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WeaponClass {
    Tank,
    Artillery,
    AntiTank,
    AntiAir,
}

/// Values that apply until a Game-Turn (the 75/27's anti-armour 0).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Before {
    pub game_turn: u8,
    pub anti_armour: Option<Rating>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WeaponRow {
    pub id: String,
    pub nation: Nation,
    pub weapon: String,
    pub class: WeaponClass,
    pub cpa: Option<Rating>,
    pub anti_air: Option<Rating>,
    pub barrage: Option<Rating>,
    pub anti_armour: Option<Rating>,
    pub vulnerability: Option<Rating>,
    pub armour_protection: Option<Rating>,
    pub close_assault_offence: Option<Rating>,
    pub close_assault_defence: Option<Rating>,
    pub before: Option<Before>,
}

impl WeaponRow {
    /// The anti-armour rating in force on a Game-Turn: `before` until
    /// its turn, the printed value from then on.
    pub fn anti_armour_at(&self, game_turn: u8) -> Option<&Rating> {
        match &self.before {
            Some(b) if game_turn < b.game_turn => b.anti_armour.as_ref(),
            _ => self.anti_armour.as_ref(),
        }
    }
}

#[derive(Deserialize)]
struct Table {
    rows: Vec<WeaponRow>,
    oa_names: BTreeMap<Nation, BTreeMap<String, String>>,
}

#[derive(Debug)]
pub struct WeaponSystems {
    rows: BTreeMap<String, WeaponRow>,
    oa_names: BTreeMap<Nation, BTreeMap<String, String>>,
}

impl WeaponSystems {
    /// Loads the chart, checking every OA name maps to a row.
    pub fn load() -> Result<WeaponSystems> {
        let table: Table = serde_json::from_value(load_table("weapon-systems")?)
            .context("parsing weapon-systems")?;
        let mut rows = BTreeMap::new();
        for r in table.rows {
            if rows.insert(r.id.clone(), r).is_some() {
                bail!("weapon-systems: duplicate row");
            }
        }
        for names in table.oa_names.values() {
            for (name, id) in names {
                if !rows.contains_key(id) {
                    bail!("weapon-systems: {name:?} maps to unknown row {id}");
                }
            }
        }
        Ok(WeaponSystems {
            rows,
            oa_names: table.oa_names,
        })
    }

    pub fn row(&self, id: &str) -> Option<&WeaponRow> {
        self.rows.get(id)
    }

    /// The row an OA sheet's `weapon` name stands for, for a nation.
    pub fn resolve(&self, nation: Nation, name: &str) -> Option<&WeaponRow> {
        let id = self.oa_names.get(&nation)?.get(name)?;
        self.rows.get(id)
    }

    pub fn rows(&self) -> impl Iterator<Item = &WeaponRow> {
        self.rows.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oa_names_resolve_to_rows() {
        let w = WeaponSystems::load().unwrap();
        let a9 = w.resolve(Nation::Cw, "A9").unwrap();
        assert_eq!(a9.id, "weapon:cw:a9-cruiser");
        assert_eq!(a9.class, WeaponClass::Tank);
        assert_eq!(a9.armour_protection.as_ref().unwrap().value, Some(1));
        assert_eq!(a9.close_assault_offence.as_ref().unwrap().value, Some(3));
        let gun = w.resolve(Nation::Cw, "18/25-pounders").unwrap();
        assert_eq!(gun.class, WeaponClass::Artillery);
        assert_eq!(gun.barrage.as_ref().unwrap().value, Some(8));
        assert_eq!(gun.vulnerability.as_ref().unwrap().value, Some(5));
        assert!(gun.armour_protection.is_none());
        // The Italian `vv` is an ID code, not a weapon.
        assert!(w.resolve(Nation::It, "vv").is_none());
        assert!(w.resolve(Nation::De, "A9").is_none());
    }

    #[test]
    fn before_values_apply_until_their_turn() {
        let w = WeaponSystems::load().unwrap();
        let gun = w.resolve(Nation::It, "75/27 Gun").unwrap();
        assert_eq!(gun.anti_armour_at(1).unwrap().value, Some(0));
        assert_eq!(gun.anti_armour_at(63).unwrap().value, Some(2));
        let cv = w.resolve(Nation::It, "CV33").unwrap();
        assert_eq!(cv.anti_armour_at(1).unwrap().value, Some(0));
    }
}
