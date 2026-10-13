//! Orders: everything a player can tell the engine. One closed enum,
//! externally tagged in JSON, so a UI or a server sends the same thing a
//! scripted game does. Later slices add the movement and combat orders.

use crate::piece::PieceId;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Order {
    /// Set-up: put a pending piece on a hex (59.2).
    Place { piece: PieceId, hex: HexId },
    /// Phase A: the initiative holder names who acts first this OpStage
    /// (7.1x).
    DeclareInitiative { player_a: Side },
    /// Nothing more this phase: ends F or G for the phasing player.
    Pass,
}

/// An accepted order and who gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggedOrder {
    pub player: Side,
    pub order: Order,
}
