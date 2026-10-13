//! A piece: one counter of the scenario as the engine tracks it. OA units
//! become pieces at set-up; supply units are pieces too (inert in Mission
//! 2).

use cna_data::characteristics::{Equivalent, Rating};
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::oa::{Nation, UnitId};
use cna_data::scenario::{OffMapBox, Placement, Side, Status};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A piece id: the OA unit id for a counter from the sheets
/// (`unit:it:maletti-div-hq`), or `supply:<side>:<n>` and
/// `supply:<side>:dummy-<n>` for the supply units a scenario creates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PieceId(String);

impl PieceId {
    pub fn supply(side: Side, n: u32) -> PieceId {
        PieceId(format!("supply:{side}:{n}"))
    }

    pub fn dummy_supply(side: Side, n: u32) -> PieceId {
        PieceId(format!("supply:{side}:dummy-{n}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The OA unit this piece is, if it came from a sheet.
    pub fn unit(&self) -> Option<UnitId> {
        UnitId::parse(&self.0).ok()
    }
}

impl From<UnitId> for PieceId {
    fn from(id: UnitId) -> PieceId {
        PieceId(id.as_str().to_owned())
    }
}

impl From<&UnitId> for PieceId {
    fn from(id: &UnitId) -> PieceId {
        PieceId(id.as_str().to_owned())
    }
}

impl fmt::Display for PieceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PieceKind {
    Combat,
    Hq,
    SupplyUnit,
    DummySupplyUnit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Location {
    Hex(HexId),
    Box(OffMapBox),
    /// A free set-up the owner has yet to make (59.2).
    Pending(Placement),
    Eliminated,
}

/// The two reserve states (18.1x): I may shift a hex and be released at
/// the first release segment; II waits for a later one and moves not at
/// all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reserve {
    I,
    II,
}

impl fmt::Display for Reserve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Reserve::I => "I",
            Reserve::II => "II",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub id: PieceId,
    pub side: Side,
    pub kind: PieceKind,
    /// None for a supply unit.
    pub nation: Option<Nation>,
    /// The ID code on the sheet; none where the sheet prints none.
    pub id_code: Option<String>,
    /// The unit this one is under: the OA parent, or the unit the set-up
    /// attached or assigned it to.
    pub parent: Option<UnitId>,
    /// TOE strength points at set-up: the printed points, the row's
    /// `max_toe` for `N`, or the `U@x`/`O@x` figure.
    pub toe: u32,
    /// The characteristics row's CPA, with its mark.
    pub cpa: Option<Rating>,
    /// Basic morale: the unit's own, else its sheet's.
    pub morale: Option<i32>,
    pub status: Option<Status>,
    /// Stacking equivalent (9.4): the row's, or the unit's own for a
    /// `by-unit` row.
    pub equivalent: Option<Equivalent>,
    /// The hex this garrison unit takes no stacking space in (9.16).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub garrison_of: Option<HexId>,
    pub location: Location,
    /// CP spent this OpStage, both halves (6.14).
    #[serde(default)]
    pub cp_spent: Cp,
    /// The part of `cp_spent` spent in the enemy's half (reaction,
    /// retreat before assault): it does not count toward the 150 % cap
    /// (8.1x).
    #[serde(default)]
    pub cp_enemy_half: Cp,
    /// The cohesion level and DP tally (6.2x).
    #[serde(default)]
    pub cohesion: cna_rules::cohesion::Unit,
    /// In an enemy ZOC when a movement segment began (8.6x, R-003):
    /// leaving costs 2 CP.
    #[serde(default)]
    pub contact: bool,
    /// Left engaged by a close assault (8.6x): leaving costs 4 CP. Set
    /// by combat (slice 5).
    #[serde(default)]
    pub engaged: bool,
    /// In reserve this OpStage (18.1x).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserve: Option<Reserve>,
    /// Released from reserve this OpStage (18.2x): its CP cap for the
    /// rest of the stage, and the combat limits slice 5 reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released: Option<Reserve>,
}

impl Piece {
    pub fn hex(&self) -> Option<&HexId> {
        match &self.location {
            Location::Hex(h) => Some(h),
            _ => None,
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.location, Location::Pending(_))
    }

    /// CP spent voluntarily, in the unit's own half.
    pub fn cp_own_half(&self) -> Cp {
        self.cp_spent.saturating_sub(self.cp_enemy_half)
    }
}
