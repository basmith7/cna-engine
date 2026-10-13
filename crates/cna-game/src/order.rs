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
    /// Combat step 1: these gun-class units stand *forward* for the
    /// segment; every other gun unit is *back* (12.1x).
    GunPositions { forward: Vec<PieceId> },
    /// Combat step 2, sealed: some of a unit's TOE points fire at one
    /// enemy unit in an adjacent hex (12.2x).
    Barrage { fires: Vec<Fire> },
    /// Combat step 3: the non-phasing player pulls a stack back along
    /// `path`, paid as movement (13.2x); `Pass` ends the step.
    RetreatBeforeAssault {
        pieces: Vec<PieceId>,
        path: Vec<HexId>,
    },
    /// Combat step 4, sealed: each unit's TOE points split between
    /// anti-armour fire and close assault; an unlisted unit is withheld
    /// (14.1x, 15.2x).
    AssignForces { assignments: Vec<Assignment> },
    /// Combat step 6: resolve the close assault assigned against this
    /// hex (15.2x); `Pass` ends the step once every assault is resolved.
    Assault { target: HexId },
    /// A forced retreat (15.82, 10.36): the stack's path; `Pass` stands
    /// and pays per hex not retreated where the ruleset allows (R-012).
    Retreat { path: Vec<HexId> },
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

/// Some of one unit's TOE points barraging one enemy unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fire {
    pub piece: PieceId,
    pub points: u32,
    pub target: PieceId,
}

/// Some TOE points in one role: against `target` for the phasing
/// player (an adjacent enemy hex), at whoever assaults the unit's own
/// hex for the non-phasing player (`target` none, 14.12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<HexId>,
    pub points: u32,
}

/// One unit's roles for the segment; a point is never in both (14.0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    pub piece: PieceId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anti_armour: Vec<Allocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub close_assault: Vec<Allocation>,
}

/// An accepted order and who gave it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggedOrder {
    pub player: Side,
    pub order: Order,
}
