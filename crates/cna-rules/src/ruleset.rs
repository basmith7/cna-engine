//! A ruleset: the printed rules plus one chosen option per ruling. Variant
//! numbers are the option numbers in `vendor/cna/rulings/R-nnn.md`; each
//! default is the option cna's ruling accepted (its `## Decision`), so a
//! game plays the decided rules unless a group switches one. JSON is
//! `{"R-001": 1, ...}`.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// R-001: what the −26 collapse threshold counts (SPI 6.26, 17.5).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum R001 {
    /// The cohesion level.
    #[default]
    CohesionLevel = 1,
    /// Disorganisation points accumulated.
    DpTally = 2,
}

/// R-009: whose barrages the target's terrain shifts (SPI 12.33, 14.0).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum R009 {
    /// Every barrage, whichever side fires.
    #[default]
    EitherSide = 1,
    /// Only barrages fired by the phasing player.
    PhasingFireOnly = 2,
}

/// R-011: whose raw points form the percentage-loss base (SPI 15.83b).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum R011 {
    /// Each side's own raw points (the printed example).
    #[default]
    OwnRaw = 1,
    /// Both sides' raw points combined.
    CombinedRaw = 2,
}

/// R-012: a defender who commits nothing (SPI 15.29, 15.82).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum R012 {
    /// Only with a three-hex path, retreated in full; no buy-out.
    MustRetreat = 1,
    /// 15.82 applies: stop short and pay 10 % per hex not retreated.
    #[default]
    BuyOut = 2,
    /// As 2 when a path exists; withholding forbidden when none does.
    NoPathNoWithhold = 3,
}

/// R-015: which side's anti-armour fire terrain weakens (SPI 14.0, 14.32,
/// 14.33). Phasing fire is always shifted by the defended hex and the
/// hexside crossed; the options differ on non-phasing fire at the
/// assaulting armour.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum R015 {
    /// Shifted by the assaulting armour's own hex, not the hexside.
    OwnHexBoth = 1,
    /// Not shifted (the literal 14.32).
    #[default]
    PhasingOnly = 2,
    /// Shifted by the assaulting armour's own hex and the hexside.
    HexAndHexsideBoth = 3,
}

/// R-018: what a supply dump costs (SPI 24.9 text against the 24.17 and
/// 6.3 charts). cna decided for the charts.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum R018 {
    /// The text: real 3 CP + 20 stores, dummy 3 CP.
    Text = 1,
    /// The charts: real 3 CP + 10 stores, dummy 2 CP.
    #[default]
    Charts = 2,
    /// CP from the charts, stores from the text.
    CpChartsStoresText = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ruleset {
    pub r001: R001,
    pub r009: R009,
    pub r011: R011,
    pub r012: R012,
    pub r015: R015,
    pub r018: R018,
}

/// The switches as (ruling id, read, write), so the JSON form and the
/// list of switched rulings cannot drift apart.
macro_rules! switches {
    ($($id:literal => $field:ident: $ty:ident [$($variant:ident),+]),+ $(,)?) => {
        /// Rulings with a switch. Every other ruling is listed in `NOT_SIMULATED.md`.
        pub const SWITCHED: &[&str] = &[$($id),+];

        $(impl TryFrom<u8> for $ty {
            type Error = String;
            fn try_from(n: u8) -> Result<$ty, String> {
                $(if n == $ty::$variant as u8 { return Ok($ty::$variant); })+
                Err(format!("{}: no option {n}", $id))
            }
        })+

        impl Ruleset {
            /// Each switch as (ruling id, option number).
            pub fn options(&self) -> Vec<(&'static str, u8)> {
                vec![$(($id, self.$field as u8)),+]
            }

            /// Sets one switch by ruling id and option number.
            pub fn set(&mut self, id: &str, option: u8) -> Result<(), String> {
                match id {
                    $($id => self.$field = $ty::try_from(option)?,)+
                    _ => return Err(format!("{id}: no such switch")),
                }
                Ok(())
            }
        }
    };
}

switches! {
    "R-001" => r001: R001 [CohesionLevel, DpTally],
    "R-009" => r009: R009 [EitherSide, PhasingFireOnly],
    "R-011" => r011: R011 [OwnRaw, CombinedRaw],
    "R-012" => r012: R012 [MustRetreat, BuyOut, NoPathNoWithhold],
    "R-015" => r015: R015 [OwnHexBoth, PhasingOnly, HexAndHexsideBoth],
    "R-018" => r018: R018 [Text, Charts, CpChartsStoresText],
}

impl Serialize for Ruleset {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let map: BTreeMap<&str, u8> = self.options().into_iter().collect();
        map.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Ruleset {
    /// Every switch must be given and no other key: a saved ruleset pins
    /// the game's rules completely.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Ruleset, D::Error> {
        let map = BTreeMap::<String, u8>::deserialize(d)?;
        let mut rules = Ruleset::default();
        for (id, option) in &map {
            rules.set(id, *option).map_err(D::Error::custom)?;
        }
        for id in SWITCHED {
            if !map.contains_key(*id) {
                return Err(D::Error::custom(format!("{id}: missing")));
            }
        }
        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ruling_is_switched_or_listed() {
        let dir = cna_data::cna_root().join("rulings");
        let listed =
            std::fs::read_to_string(cna_data::repo_root().join("NOT_SIMULATED.md")).unwrap();
        let mut missing = vec![];
        for e in std::fs::read_dir(dir).unwrap() {
            let name = e.unwrap().file_name().into_string().unwrap();
            let Some(id) = name.strip_suffix(".md").filter(|s| s.starts_with("R-")) else {
                continue;
            };
            if !SWITCHED.contains(&id) && !listed.contains(&format!("| {id} |")) {
                missing.push(id.to_string());
            }
        }
        missing.sort();
        assert!(
            missing.is_empty(),
            "no switch and not in NOT_SIMULATED.md: {missing:?}"
        );
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;

    /// The option number a ruling's `## Decision` section opens with.
    fn decided_option(id: &str) -> u8 {
        let path = cna_data::cna_root().join(format!("rulings/{id}.md"));
        let text = std::fs::read_to_string(&path).unwrap();
        let decision = text
            .split("\n## Decision")
            .nth(1)
            .unwrap_or_else(|| panic!("{id}: no `## Decision` section"));
        let body = decision.split("\n## ").next().unwrap().trim();
        let rest = body
            .strip_prefix("Option ")
            .unwrap_or_else(|| panic!("{id}: Decision does not open with `Option n.`: {body:?}"));
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        digits.parse().unwrap()
    }

    #[test]
    fn every_default_is_the_option_cna_decided() {
        for (id, option) in Ruleset::default().options() {
            assert_eq!(
                option,
                decided_option(id),
                "{id}: default option {option} is not cna's Decision"
            );
        }
    }

    #[test]
    fn ruleset_json_is_a_map_of_ruling_to_option() {
        let rules = Ruleset {
            r012: R012::MustRetreat,
            r018: R018::Charts,
            ..Ruleset::default()
        };
        let json = serde_json::to_string(&rules).unwrap();
        assert_eq!(
            json,
            r#"{"R-001":1,"R-009":1,"R-011":1,"R-012":1,"R-015":2,"R-018":2}"#
        );
        let back: Ruleset = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rules);
        assert!(serde_json::from_str::<Ruleset>(r#"{"R-001":9}"#).is_err());
        assert!(serde_json::from_str::<Ruleset>(r#"{"R-999":1}"#).is_err());
    }
}
