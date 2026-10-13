//! The combat segment of phase G (SPI 5.2 G.3, §11–§15; cna
//! `rules/60-combat.md`): strengths from the pieces, CP by role, and the
//! six steps wired to the rules core.

pub mod cp;
pub mod strength;

use crate::piece::PieceId;
use cna_data::cp::Cp;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// What one combat segment has recorded so far; cleared as it ends.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatSegment {
    /// Combat CP charged this segment, per piece (11.2x ceilings).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub charged: BTreeMap<PieceId, Cp>,
    /// Pieces that barraged or were barraged: no refund for them (6.3).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub barrage_involved: BTreeSet<PieceId>,
}
