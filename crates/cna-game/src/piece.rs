//! A piece: one counter of the scenario as the engine tracks it. OA units
//! become pieces at set-up; supply units are pieces too (inert in Mission
//! 2).

use cna_data::characteristics::{Equivalent, Rating};
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
    pub location: Location,
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
}
