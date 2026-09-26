//! A ruleset: the printed rules plus one chosen option per ruling. Variant
//! numbers are the option numbers in `vendor/cna/rulings/R-nnn.md`; each
//! default is the option the printed text supports, per the ruling's
//! Rationale.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ruleset {
    pub r009: R009,
    pub r011: R011,
    pub r012: R012,
    pub r015: R015,
}

/// Rulings with a switch. Every other ruling is listed in `NOT_SIMULATED.md`.
pub const SWITCHED: &[&str] = &["R-009", "R-011", "R-012", "R-015"];

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
