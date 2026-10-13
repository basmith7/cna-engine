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
    /// A movement segment: these pieces, stacked together, move along
    /// `path` (adjacent hexes from where they stand) as one body (8.1x,
    /// 6.15).
    Move {
        pieces: Vec<PieceId>,
        path: Vec<HexId>,
    },
    /// The non-phasing player, asked which of its hexes exert a ZOC
    /// (10.16): these do; the rest asked about do not.
    DeclareZoc { exerting: Vec<HexId> },
    /// Phase G: the movement segment is over (stacking is checked, 9.14),
    /// or from reserve release, another cycle begins (8.2x).
    EndSegment,
    /// Nothing more this phase: ends F or G for the phasing player.
    Pass,
}

/// An accepted order and who gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggedOrder {
    pub player: Side,
    pub order: Order,
}
