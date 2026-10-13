//! Events: what an accepted order did, and why a rejected one was
//! rejected. Events are derived from the state and the order; the log
//! keeps orders, not events.

use crate::clock::Clock;
use crate::piece::PieceId;
use crate::state::Weather;
use cna_data::map::HexId;
use cna_data::scenario::Side;
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
    /// The scenario's last OpStage is over.
    GameOver,
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
