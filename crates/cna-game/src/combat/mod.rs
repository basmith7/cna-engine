//! The combat segment of phase G (SPI 5.2 G.3, §11–§15; cna
//! `rules/60-combat.md`): strengths from the pieces, CP by role, and the
//! six steps wired to the rules core. Each step that both players answer
//! in secret is collected under `Pending::Sealed` and revealed only when
//! both answers are in.

mod assault;
mod barrage;
pub mod cp;
mod losses;
mod plots;
mod retreat;
pub(crate) mod segment;
pub mod strength;
pub mod terrain;

use crate::order::{Assignment, Fire, Order};
use crate::piece::PieceId;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A combat step answered by both players in secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SealedStep {
    GunPositions,
    Barrage,
    AssignForces,
}

impl SealedStep {
    /// Whether an order answers this step.
    pub fn matches(self, order: &Order) -> bool {
        matches!(
            (self, order),
            (SealedStep::GunPositions, Order::GunPositions { .. })
                | (SealedStep::Barrage, Order::Barrage { .. })
                | (SealedStep::AssignForces, Order::AssignForces { .. })
        )
    }
}

/// Why a stack must retreat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RetreatReason {
    /// The close assault table's retreat line (15.82): 10 % of the raw
    /// points lost per hex short.
    AssaultResult,
    /// A defender who committed nothing (15.29, R-012).
    Withheld,
    /// A unit that neither assaulted nor held off an enemy ZOC hex
    /// (10.36): all its CP and 3 DP; no stopping short.
    MustAttack,
}

/// One close assault as assigned: the phasing pieces and points against
/// a hex, resolved in the phasing player's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssaultRecord {
    /// A probe by the 50 % rule (15.2x, 15.9x).
    pub probe: bool,
    /// The basic differential (15.23), for the must-attack rule.
    pub basic: i32,
    /// The adjusted differential's column.
    pub column: String,
}

/// What one combat segment has recorded so far; cleared as it ends.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatSegment {
    /// Gun-class units standing forward (12.1x); every other is back.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub forward: BTreeSet<PieceId>,
    /// Barrage plots by side, sealed until both are in (12.2x).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub plots: BTreeMap<Side, Vec<Fire>>,
    /// Force assignments by side, sealed until both are in (14.1x).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assignments: BTreeMap<Side, Vec<Assignment>>,
    /// Units pinned by barrage for the rest of the segment (12.44).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub pinned: BTreeSet<PieceId>,
    /// Units that retreated before assault this segment (13.28).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub retreated: BTreeSet<PieceId>,
    /// Combat CP charged this segment, per piece (11.2x ceilings).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub charged: BTreeMap<PieceId, Cp>,
    /// Pieces that barraged or were barraged: no refund for them (6.3).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub barrage_involved: BTreeSet<PieceId>,
    /// Actual barrage points landed on each hex, for holding off (10.33).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub barraged: BTreeMap<HexId, u32>,
    /// Close assaults assigned, by target hex, once resolved.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assaults: BTreeMap<HexId, AssaultRecord>,
}
