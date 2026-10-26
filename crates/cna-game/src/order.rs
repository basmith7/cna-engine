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
        /// The supply unit that pays the fuel (32.22); none takes the
        /// nearest in range with fuel.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fuel_from: Option<PieceId>,
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
    /// Phase C or the player's movement segment: `piece` attaches to `to`
    /// in its hex, a CP each (19.4x).
    Attach { piece: PieceId, to: PieceId },
    /// Phase C or the player's movement segment: `piece` detaches from
    /// the unit it is attached to, a CP each (19.4x).
    Detach { piece: PieceId },
    /// Phase C, abstract supply: the supply unit takes its motorisation
    /// points (30 real, 6 dummy) from its hex's pool (32.31).
    AttachMotorisation { piece: PieceId },
    /// Phase C: the supply unit's points go back to its hex's pool.
    DetachMotorisation { piece: PieceId },
    /// Phase C: ammunition and fuel move between two supply units in one
    /// hex (32.1x).
    TransferLoad {
        from: PieceId,
        to: PieceId,
        ammo: u32,
        fuel: u32,
    },
    /// Phase C, abstract supply: the player's once-a-Game-Turn air lift
    /// of one supply unit to `to`, 12 fuel from `fuel_from` (none takes
    /// the first at the origin with enough) (32.37).
    AirLift {
        piece: PieceId,
        to: crate::piece::Location,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fuel_from: Option<PieceId>,
    },
    /// A movement segment, before anything moves: the player's supply
    /// units in `hex` are put to the torch, one die each (32.1x).
    DestroySupply { hex: HexId },
    /// Phase J, abstract supply: the Commonwealth's supply units ride the
    /// railway hex by hex along `path` (8.7x, 32.3x).
    Rail {
        pieces: Vec<PieceId>,
        path: Vec<HexId>,
    },
    /// Phase K: one field-repair roll for the player's broken-down
    /// `weapon` in `hex` (22.2x); with abstract supply `fuel_from` pays a
    /// fuel a point attempted (none takes the first in range).
    Repair {
        hex: HexId,
        weapon: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fuel_from: Option<PieceId>,
    },
    /// Nothing more this phase: ends F or G for the phasing player, or
    /// C for one player.
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
