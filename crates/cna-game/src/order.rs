//! Orders: everything a player can tell the engine. One closed enum,
//! externally tagged in JSON, so a UI or a server sends the same thing a
//! scripted game does. Later slices add the movement and combat orders.

use crate::piece::PieceId;
use cna_data::map::HexId;
use cna_data::off_map::OffMapPlace;
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
        /// The mover means to assault whoever it ends next to: with a
        /// CPA 6 or more higher it pins them (8.5x). Combat itself is
        /// slice 5's.
        #[serde(default)]
        assault: bool,
    },
    /// The non-phasing player, offered a reaction (8.5x): some of the
    /// units offered move along `path`, never into an enemy ZOC. `Pass`
    /// declines for the rest.
    React {
        pieces: Vec<PieceId>,
        path: Vec<HexId>,
    },
    /// The non-phasing player, asked which of its hexes exert a ZOC
    /// (10.16): these do; the rest asked about do not.
    DeclareZoc { exerting: Vec<HexId> },
    /// A movement segment: an Axis stack in a box, in transit, or at
    /// Nofilia sets out for `to`, spending the whole OpStage (8.8x).
    MoveOffMap {
        pieces: Vec<PieceId>,
        to: OffMapPlace,
    },
    /// Phase F: these units go into Reserve I (18.1x).
    DesignateReserve { pieces: Vec<PieceId> },
    /// A reserve release segment: these units, all in the same reserve
    /// state, come out (18.2x).
    ReleaseReserve { pieces: Vec<PieceId> },
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
