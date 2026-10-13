//! The game state: everything that changes during a game, and `apply`,
//! the only way it changes. An order is validated in full before anything
//! moves; a rejected order leaves the state as it was and never enters
//! the log.

use crate::clock::Clock;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::order::{LoggedOrder, Order};
use crate::piece::{Piece, PieceId};
use crate::rng::GameRng;
use crate::setup;
use anyhow::Result;
use cna_data::scenario::Side;
use cna_rules::ruleset::Ruleset;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The OpStage's weather (29.6): the result and, for a storm, the sheets
/// it covers (29.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Weather {
    Normal,
    Hot,
    Sandstorm { sheets: Vec<char> },
    Rainstorm { sheets: Vec<char> },
}

/// A decision one player owes before anything else may happen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pending {
    /// This side has pieces with a free placement to make (59.2).
    SetUp { side: Side },
}

impl Pending {
    pub fn side(&self) -> Side {
        match self {
            Pending::SetUp { side } => *side,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub scenario: String,
    /// The `vendor/cna` commit the game was made from.
    pub rules_commit: String,
    pub ruleset: Ruleset,
    pub clock: Clock,
    /// Who holds initiative this Game-Turn (7.1x).
    pub initiative: Option<Side>,
    /// Who acts first this OpStage.
    pub player_a: Option<Side>,
    pub weather: Weather,
    pub pieces: BTreeMap<PieceId, Piece>,
    pub pending: Option<Pending>,
    /// The scenario's last OpStage is over.
    pub over: bool,
    pub rng: GameRng,
    pub log: Vec<LoggedOrder>,
}

impl GameState {
    /// A new game: the set-up expanded to pieces, the Axis to place its
    /// free pieces first (our choice: 59 and 60 do not say who sets up
    /// first), then the Commonwealth; then GT1 OpStage 1 phase A.
    pub fn new(data: &Data, seed: u64) -> Result<GameState> {
        let setup = setup::expand(&data.scenario, &data.oa, &data.chars, &data.map)?;
        let pieces = setup
            .pieces
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();
        let mut state = GameState {
            scenario: data.slug().to_owned(),
            rules_commit: cna_data::rules_commit()?,
            ruleset: Ruleset::default(),
            clock: Clock::start(data.scenario.start.game_turn),
            initiative: None,
            player_a: None,
            weather: Weather::Normal,
            pieces,
            pending: None,
            over: false,
            rng: GameRng::new(seed),
            log: Vec::new(),
        };
        state.pending = state.next_set_up();
        Ok(state)
    }

    fn next_set_up(&self) -> Option<Pending> {
        [Side::Axis, Side::Cw]
            .into_iter()
            .find(|&side| {
                self.pieces
                    .values()
                    .any(|p| p.side == side && p.is_pending())
            })
            .map(|side| Pending::SetUp { side })
    }

    /// The player the engine is waiting on, or none when the game is over.
    pub fn to_act(&self) -> Option<Side> {
        if self.over {
            return None;
        }
        if let Some(p) = &self.pending {
            return Some(p.side());
        }
        match self.clock.half {
            None => self.initiative,
            Some(crate::clock::Half::A) => self.player_a,
            Some(crate::clock::Half::B) => self.player_a.map(Side::other),
        }
    }

    /// Applies one order from `player`. Fully validated first: on `Err`
    /// nothing has changed.
    pub fn apply(
        &mut self,
        data: &Data,
        player: Side,
        order: Order,
    ) -> Result<Vec<Event>, Illegal> {
        match self.to_act() {
            None => return Err(Illegal::new("60.8", "the game is over")),
            Some(s) if s != player => {
                return Err(Illegal::new(
                    "5.2",
                    format!("{player} may not act: waiting on {s} at {}", self.clock),
                ));
            }
            Some(_) => {}
        }
        let events = self.apply_checked(data, player, &order)?;
        self.log.push(LoggedOrder { player, order });
        Ok(events)
    }

    fn apply_checked(
        &mut self,
        _data: &Data,
        _player: Side,
        _order: &Order,
    ) -> Result<Vec<Event>, Illegal> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    #[test]
    fn an_order_from_the_wrong_player_is_illegal_and_changes_nothing() {
        let data = graziani();
        let mut state = GameState::new(&data, 7).unwrap();
        assert_eq!(state.to_act(), Some(Side::Axis));
        let before = serde_json::to_string(&state).unwrap();
        let err = state.apply(&data, Side::Cw, Order::Pass).unwrap_err();
        assert_eq!(err.case, "5.2");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        assert!(state.log.is_empty());
    }

    #[test]
    fn a_state_with_pieces_round_trips_through_json() {
        let data = graziani();
        let state = GameState::new(&data, 7).unwrap();
        assert_eq!(state.pieces.len(), 328);
        let json = serde_json::to_string(&state).unwrap();
        let back: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["scenario"], "grazianis-offensive");
        assert_eq!(v["rules_commit"].as_str().unwrap().len(), 40);
        assert_eq!(v["ruleset"]["R-001"], 1);
        assert_eq!(v["rng"]["seed"], 7);
    }
}
