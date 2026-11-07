//! Orders: everything a player can tell the engine. One closed enum,
//! externally tagged in JSON, so a UI or a server sends the same thing a
//! scripted game does. Later slices add the movement and combat orders.

use crate::piece::PieceId;
use cna_data::map::HexId;
use cna_data::oa::UnitId;
use cna_data::off_map::OffMapPlace;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Order {
    /// Set-up: put a pending piece on a hex (59.2).
    Place { piece: PieceId, hex: HexId },
    /// Set-up: take back a piece this side placed, to wait again (59.2).
    Unplace { piece: PieceId },
    /// Set-up: place a fortification level or a real or dummy minefield
    /// the scenario gives this side to place (59.12); `Pass` declines
    /// the optional rest.
    PlaceWork {
        item: crate::construction::Item,
        hex: HexId,
    },
    /// Set-up: `points` of the side's unattached motorisation points go
    /// to `hex`, one the scenario lists for them (61.72, R-160).
    PlaceMotorisation { hex: HexId, points: u32 },
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
    /// Combat step 6, before `target` is assaulted: `points` of the
    /// anti-armour points `piece` aimed at `target`, which held no
    /// armour, join the close assault on it (14.23).
    ReleaseAntiArmour {
        piece: PieceId,
        target: HexId,
        points: u32,
    },
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
    /// Phase C: the independent `piece`, attached to a parent with a
    /// free slot it fills, becomes assigned to that parent (19.2x).
    Reassign { piece: PieceId },
    /// Phase C: `piece` leaves its parent for `to` (none: independent),
    /// a change its O/A notes date to this month (19.24, R-176). It must
    /// be attached to `to`, which needs a slot it fills.
    ShiftParent { piece: PieceId, to: Option<PieceId> },
    /// Phase C, Commonwealth: two battalions standing alone trade the
    /// brigades they are assigned to, or `with` is independent and takes
    /// `piece`'s place while `piece` leaves it (19.2x).
    SwapBattalions { piece: PieceId, with: PieceId },
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
    /// Phase C, abstract supply, the Axis: the convoy lanes its supply
    /// sails by from the next convoy stage, filled in this order, three
    /// supply units a lane (32.63, 56.1). Empty: the Tripoli lane alone.
    ConvoyLanes { lanes: Vec<u8> },
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
    /// railway hex by hex along `path` (8.7x, 32.3x), changing the load at
    /// `stops` on the way (8.77, R-005).
    Rail {
        pieces: Vec<PieceId>,
        path: Vec<HexId>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        stops: Vec<RailStop>,
    },
    /// Phase K: one field-repair roll for the player's broken-down
    /// `weapon` in `hex` (22.2x); with abstract supply `fuel_from` pays a
    /// fuel a point attempted (none takes the first in range).
    Repair {
        hex: HexId,
        weapon: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fuel_from: Option<PieceId>,
        /// The unit whose broken-down `weapon` points in `hex` the
        /// repaired points go back to first (61.38's `all_tanks`); none
        /// takes the wrecks in the order they are held.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<UnitId>,
    },
    /// Phase K: one facility-repair roll for the player's broken-down
    /// `weapon` in `hex`, a repair facility of its own (22.3x); with
    /// abstract supply `fuel_from` pays a fuel a point attempted (none
    /// takes the first in range).
    FacilityRepair {
        hex: HexId,
        weapon: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fuel_from: Option<PieceId>,
        /// As for `Repair`: the unit whose wreck is repaired first.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<UnitId>,
    },
    /// Phase K: attempt `points` of the player's destroyed `weapon` tanks
    /// in `hex`, a repair facility of its own, a die each on the
    /// Destroyed Tank Repair Table (22.4x); with abstract supply
    /// `fuel_from` pre-pays two fuel a point (none takes the first in
    /// range).
    RepairDestroyedTanks {
        hex: HexId,
        weapon: String,
        points: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fuel_from: Option<PieceId>,
    },
    /// Phase K: tow `points` of the player's broken-down `weapon` from
    /// `hex` along `path`, as a medium truck on 10 CP, toward the nearest
    /// repair facility (21.6x).
    Tow {
        hex: HexId,
        weapon: String,
        points: u32,
        path: Vec<HexId>,
    },
    /// Phase E: the Commonwealth places `ship` at sea in `hex` for this
    /// OpStage (30.1x).
    Station { ship: PieceId, hex: HexId },
    /// Phase C, the Axis in a campaign game from July 1941, once a game:
    /// it notes secretly a chariot raid on Alexandria, made in the
    /// seventh OpStage after (30.4x).
    PlanChariotRaid,
    /// Phase C's construction segment: a dummy tank formation noted
    /// secretly in `hex`, with a friendly non-Italian formation (16.4x).
    PlaceDummyTank { hex: HexId },
    /// Phase L: a patrol of `sources`' points against `target`, the
    /// defender describing none of `exclude` (16.x).
    Patrol {
        sources: Vec<(PieceId, u32)>,
        target: HexId,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        exclude: Vec<PieceId>,
    },
    /// Phase H: `points` of the player's prisoners in `from` walk `path`
    /// (28.13).
    MarchPrisoners {
        from: HexId,
        path: Vec<HexId>,
        points: u32,
    },
    /// Phase C: `builders` in `hex` begin building `item` (24.1x).
    Build {
        item: crate::construction::Item,
        hex: HexId,
        builders: Vec<PieceId>,
    },
    /// Phase C or the player's phase H: `points` of `piece`'s infantry
    /// TOE points become guard points in its hex (28.21).
    DetailGuards { piece: PieceId, points: u32 },
    /// Phase C or the player's phase H: `points` of the guard points in
    /// `piece`'s hex rejoin it as infantry TOE points (28.22).
    ReleaseGuards { piece: PieceId, points: u32 },
    /// Phase C: `piece` absorbs `points` of the player's repaired
    /// `weapon` points waiting in its hex (19.61, 20.41).
    Absorb {
        piece: PieceId,
        weapon: String,
        points: u32,
    },
    /// The phasing Commonwealth player's movement segment: LRDG `piece`,
    /// next to Rommel's hex, spends 10 CP to raid him (27.6x).
    RaidRommel { piece: PieceId },
    /// The Commonwealth, while an OpStage's withdrawal row names a
    /// `pick` of `fills` units: `pieces` are the ones that go with it
    /// (20.82, R-154). A later pick replaces an earlier one.
    PickWithdrawal { fills: String, pieces: Vec<PieceId> },
    /// Phase C: raider counter `raider`, not in play, is formed in
    /// `from`'s hex from one of its recce or armoured-car TOE points
    /// (27.13, 27.16).
    FormRaider { raider: PieceId, from: PieceId },
    /// The raider's movement segment: raider `piece`, in or next to the
    /// hex of enemy supply unit `target` (its alias, to the raider's
    /// side), spends 5 CP to raid it as a supply dump (27.55, R-149).
    RaidDump { piece: PieceId, target: PieceId },
    /// The phasing player's movement segment: combat unit `piece`, told
    /// of an enemy raider in the hex it entered and still there, spends
    /// 1 CP to try to spot it (27.42–27.44).
    TrySpot { piece: PieceId },
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

/// A hex of a rail run's path, before its end, where pieces aboard get
/// `off` and pieces standing there get `on` (8.77, R-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RailStop {
    pub at: HexId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub off: Vec<PieceId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on: Vec<PieceId>,
}
