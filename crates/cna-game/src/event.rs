//! Events: what an accepted order did, and why a rejected one was
//! rejected. Events are derived from the state and the order; the log
//! keeps orders, not events.

use crate::clock::Clock;
use crate::combat::{RetreatReason, SealedStep};
use crate::order::{Assignment, Fire};
use crate::piece::{Location, PieceId, Reserve};
use crate::state::Weather;
use crate::victory::VictoryReport;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::barrage::BarrageResult;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    /// The clock reached a phase that waits for a player.
    PhaseStarted {
        clock: Clock,
    },
    /// A phase this mission does not model, passed through.
    PhaseSkipped {
        clock: Clock,
    },
    DieRolled {
        purpose: String,
        value: u8,
    },
    /// The holder for the Game-Turn: fixed by the scenario, or the higher
    /// die plus rating (7.1x).
    InitiativeDetermined {
        holder: Side,
        rolled: bool,
    },
    /// Who is Player A this OpStage.
    InitiativeDeclared {
        player_a: Side,
    },
    WeatherSet {
        weather: Weather,
    },
    Placed {
        piece: PieceId,
        hex: HexId,
    },
    /// Every piece is on the map; play begins.
    SetUpComplete,
    /// Reinforcements landed in phase D (20.1x).
    Arrived {
        pieces: Vec<PieceId>,
        at: Location,
    },
    /// An OpStage of off-map movement (8.8x): where the piece now is.
    MovedOffMap {
        piece: PieceId,
        location: Location,
    },
    /// One step of a move: what it cost each piece (8.3x).
    Moved {
        pieces: Vec<PieceId>,
        from: HexId,
        to: HexId,
        cost: Cp,
    },
    /// CP over the CPA earned DP at once (6.21); the level after.
    Disorganised {
        piece: PieceId,
        dp: u32,
        level: i32,
    },
    /// A move stopped at `at` before entering `before`, until the
    /// non-phasing player says whether `declare` exert a ZOC (10.16).
    MoveInterrupted {
        pieces: Vec<PieceId>,
        at: HexId,
        before: HexId,
        declare: Vec<HexId>,
    },
    ZocDeclared {
        hex: HexId,
        exerts: bool,
    },
    /// A move stopped at `at`, next to non-phasing units that may react
    /// (8.5x); the mover may go on once they have answered.
    ReactionOffered {
        pieces: Vec<PieceId>,
        at: HexId,
    },
    /// A unit in contact or engaged paid to leave (8.6x).
    ContactBroken {
        piece: PieceId,
        cost: Cp,
    },
    ReserveDesignated {
        pieces: Vec<PieceId>,
    },
    /// Released from Reserve I or II (18.2x).
    ReserveReleased {
        pieces: Vec<PieceId>,
        from: Reserve,
    },
    /// Still in Reserve I as the first release segment closed: now
    /// Reserve II (18.2x).
    ReserveFlipped {
        pieces: Vec<PieceId>,
    },
    /// A sealed combat answer was accepted; its content waits for the
    /// other side's (11.0).
    Sealed {
        side: Side,
        step: SealedStep,
    },
    /// Step 1 revealed: this side's forward guns (12.1x).
    GunPositionsSet {
        side: Side,
        forward: Vec<PieceId>,
    },
    /// Step 2 revealed: this side's barrages (12.2x).
    BarragePlotted {
        side: Side,
        fires: Vec<Fire>,
    },
    /// One barrage rolled (12.4x): the firers' points pooled against one
    /// target, after the terrain shift.
    Barrage {
        side: Side,
        firers: Vec<PieceId>,
        target: PieceId,
        /// Actual barrage points, before the shift.
        points: u32,
        shift: i32,
        reading: u8,
        result: BarrageResult,
    },
    /// A non-phasing stack retreated before assault (13.2x).
    RetreatedBeforeAssault {
        pieces: Vec<PieceId>,
        to: HexId,
    },
    /// One anti-armour attack rolled (14.0): phasing fire at a hex, or
    /// non-phasing fire at the armour assaulting `target`.
    AntiArmour {
        side: Side,
        target: HexId,
        points: u32,
        shift: i32,
        reading: u8,
        damage: u32,
    },
    /// One close assault or probe resolved (15.x).
    Assault {
        target: HexId,
        attackers: Vec<PieceId>,
        probe: bool,
        attacker_raw: u32,
        defender_raw: u32,
        basic: i32,
        /// Column shifts applied: terrain, size, morale and the rest.
        shifts: i32,
        column: String,
        attacker: AssaultRoll,
        defender: AssaultRoll,
    },
    /// A side's losses were partly prisoners (15.89).
    Prisoners {
        side: Side,
        points: u32,
    },
    /// Pieces that surrendered when assaulted (15.88, E-009).
    Surrendered {
        pieces: Vec<PieceId>,
    },
    /// Units locked to the enemy by an assault (15.81).
    Engaged {
        pieces: Vec<PieceId>,
    },
    /// A stack must retreat (15.82, 15.29, 10.36).
    RetreatOrdered {
        pieces: Vec<PieceId>,
        hexes: u32,
        reason: RetreatReason,
    },
    /// A forced retreat stood (or stopped) `short` hexes short, at 10 %
    /// of its raw points per hex (15.82, R-012).
    RetreatBoughtOut {
        pieces: Vec<PieceId>,
        short: u32,
    },
    /// TOE points destroyed; `toe` is what is left.
    Losses {
        piece: PieceId,
        points: u32,
        toe: u32,
    },
    /// A supply unit paid fuel for a piece's land move (32.22).
    FuelSpent {
        piece: PieceId,
        from: PieceId,
        fuel: u32,
    },
    /// A supply unit paid ammunition for a piece's fire or assault
    /// (32.21).
    AmmunitionSpent {
        piece: PieceId,
        from: PieceId,
        ammo: u32,
    },
    /// A defender that no supply unit in range could pay still fought
    /// at full strength (32.21).
    AmmunitionShort {
        piece: PieceId,
    },
    /// A supply unit with no ammunition and no fuel left was removed
    /// (32.1x).
    SupplyUnitSpent {
        piece: PieceId,
    },
    /// A supply unit its own side destroyed (32.1x).
    SupplyDestroyed {
        piece: PieceId,
    },
    /// A breakdown check (21.3x): `points` of `weapon` broke down on a
    /// reading of `reading` at `percent` %.
    BrokeDown {
        piece: PieceId,
        weapon: String,
        reading: u8,
        percent: u32,
        points: u32,
    },
    /// Supply units ran along the railway (8.7x, 32.3x).
    Railed {
        pieces: Vec<PieceId>,
        from: HexId,
        to: HexId,
    },
    /// Motorisation points attached to a supply unit (32.31).
    MotorisationAttached {
        piece: PieceId,
        points: u32,
    },
    /// A supply unit's motorisation points back in its hex's pool.
    MotorisationDetached {
        piece: PieceId,
        points: u32,
    },
    /// Ammunition and fuel moved between supply units in one hex (32.1x).
    LoadTransferred {
        from: PieceId,
        to: PieceId,
        ammo: u32,
        fuel: u32,
    },
    /// A piece whose last TOE point went.
    Eliminated {
        piece: PieceId,
    },
    /// Step 4 revealed: this side's roles (14.1x).
    ForcesAssigned {
        side: Side,
        assignments: Vec<Assignment>,
    },
    /// Combat CP charged to a unit by its hex's role this segment (11.2x).
    CombatCp {
        piece: PieceId,
        cost: Cp,
    },
    /// CP given back to a defender of a bad attack (6.3's dagger), and
    /// the DP that CP had earned over the CPA, taken back with it.
    CombatCpRefunded {
        piece: PieceId,
        cp: Cp,
        dp: u32,
    },
    /// A non-combat unit alone in an enemy ZOC was taken (10.29).
    Captured {
        piece: PieceId,
        hex: HexId,
    },
    /// The scenario's last OpStage is over (60.22): each side's level
    /// and what was checked (60.8).
    GameOver {
        report: VictoryReport,
    },
}

/// One side's reading of the Close Assault CRT (15.7x).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssaultRoll {
    pub reading: u8,
    pub pct: u8,
    /// Raw points lost, before absorption.
    pub raw_lost: u32,
    /// TOE points removed.
    pub toe_lost: u32,
    pub engaged: bool,
    pub retreat: u32,
    pub captured: bool,
}

/// A rejected order: the case (or case group) it breaks and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Illegal {
    pub case: &'static str,
    pub reason: String,
}

impl Illegal {
    pub fn new(case: &'static str, reason: impl Into<String>) -> Illegal {
        Illegal {
            case,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for Illegal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "illegal ({}): {}", self.case, self.reason)
    }
}

impl std::error::Error for Illegal {}
