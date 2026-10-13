//! A moment of the game clock: a Game-Turn and one of its three Operations
//! Stages (cna's `gameTime`).

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GameTime {
    pub game_turn: u8,
    pub opstage: u8,
}

impl GameTime {
    pub const fn new(game_turn: u8, opstage: u8) -> GameTime {
        GameTime { game_turn, opstage }
    }
}

impl fmt::Display for GameTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.game_turn, self.opstage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_by_turn_then_stage() {
        assert!(GameTime::new(1, 3) < GameTime::new(2, 1));
        assert!(GameTime::new(2, 1) < GameTime::new(2, 2));
        assert_eq!(GameTime::new(6, 3).to_string(), "6/3");
    }
}
