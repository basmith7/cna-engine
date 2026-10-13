//! The log: what a game is made of, and nothing else. Replaying it on the
//! same `vendor/cna` commit rebuilds the state byte for byte.

use crate::data::Data;
use crate::order::LoggedOrder;
use crate::state::GameState;
use anyhow::{Context, Result, bail};
use cna_rules::ruleset::Ruleset;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Log {
    pub scenario: String,
    pub rules_commit: String,
    pub ruleset: Ruleset,
    pub seed: u64,
    pub orders: Vec<LoggedOrder>,
}

impl GameState {
    /// The game so far as a log.
    pub fn to_log(&self) -> Log {
        Log {
            scenario: self.scenario.clone(),
            rules_commit: self.rules_commit.clone(),
            ruleset: self.ruleset,
            seed: self.rng.seed(),
            orders: self.log.clone(),
        }
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)? + "\n")
    }

    /// Reads a saved state; fails if it was made on another `vendor/cna`
    /// commit than the one checked out.
    pub fn from_json(json: &str) -> Result<GameState> {
        let state: GameState = serde_json::from_str(json).context("parsing the state")?;
        check_commit(&state.rules_commit)?;
        Ok(state)
    }
}

fn check_commit(commit: &str) -> Result<()> {
    let current = cna_data::rules_commit()?;
    if commit != current {
        bail!("made on vendor/cna {commit}, but {current} is checked out");
    }
    Ok(())
}

/// Rebuilds the state a log describes by applying its orders afresh.
pub fn replay(data: &Data, log: &Log) -> Result<GameState> {
    if log.scenario != data.slug() {
        bail!(
            "the log is of {}, the data of {}",
            log.scenario,
            data.slug()
        );
    }
    check_commit(&log.rules_commit)?;
    let mut state = GameState::new(data, log.seed)?;
    state.ruleset = log.ruleset;
    for (i, o) in log.orders.iter().enumerate() {
        state
            .apply(data, o.player, o.order.clone())
            .with_context(|| format!("order {}: {:?}", i + 1, o))?;
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{Clock, Phase};
    use crate::order::Order;
    use crate::state::test_support;
    use cna_data::scenario::Side;
    use std::path::PathBuf;

    fn fixture() -> PathBuf {
        cna_data::repo_root().join("tests/games/setup.json")
    }

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// The scripted game `tests/games/setup.json` is: every set-up order,
    /// then GT1 OpStage 1 passed through by both players. Regenerate it
    /// with `cargo test -p cna-game -- --ignored regenerate_setup_fixture`
    /// after a change to the set-up or the orders.
    fn scripted_setup(data: &Data) -> GameState {
        let mut state = GameState::new(data, 7).unwrap();
        test_support::auto_place(data, &mut state);
        test_support::pass_stage(data, &mut state);
        state
    }

    #[test]
    #[ignore]
    fn regenerate_setup_fixture() {
        let data = graziani();
        let state = scripted_setup(&data);
        let json = serde_json::to_string_pretty(&state.to_log()).unwrap() + "\n";
        std::fs::write(fixture(), json).unwrap();
    }

    #[test]
    fn the_setup_game_replays_to_the_same_state() {
        let data = graziani();
        let text = std::fs::read_to_string(fixture()).unwrap();
        let log: Log = serde_json::from_str(&text).unwrap();
        assert_eq!(log.orders.len(), 59 + 1 + 4);
        let live = scripted_setup(&data);
        assert_eq!(live.to_log(), log, "the fixture is stale: regenerate it");
        let replayed = replay(&data, &log).unwrap();
        assert_eq!(replayed.to_json().unwrap(), live.to_json().unwrap());
        assert_eq!(
            replayed.clock,
            Clock {
                stage: 2,
                ..Clock::start(1)
            }
        );
        assert_eq!(replayed.clock.phase, Phase::InitiativeDeclaration);
        assert!(replayed.pieces.values().all(|p| !p.is_pending()));
        // Every replayed order is in the log; a rejected one never is.
        let mut copy = replayed.clone();
        assert!(copy.apply(&data, Side::Cw, Order::Pass).is_err());
        assert_eq!(copy.log, replayed.log);
    }

    #[test]
    fn a_saved_state_from_another_rules_commit_fails_to_load() {
        let data = graziani();
        let state = GameState::new(&data, 1).unwrap();
        let json = state.to_json().unwrap();
        assert_eq!(GameState::from_json(&json).unwrap(), state);
        let other = json.replace(&state.rules_commit, &"0".repeat(40));
        let err = GameState::from_json(&other).unwrap_err().to_string();
        assert!(err.contains("vendor/cna"), "{err}");
        let mut log = state.to_log();
        log.rules_commit = "0".repeat(40);
        assert!(replay(&data, &log).is_err());
        log.rules_commit = state.rules_commit.clone();
        log.scenario = "italian-campaign".into();
        assert!(replay(&data, &log).is_err());
    }

    #[test]
    fn the_log_round_trips_through_json() {
        let data = graziani();
        let mut state = GameState::new(&data, 9).unwrap();
        test_support::auto_place(&data, &mut state);
        let log = state.to_log();
        let json = serde_json::to_string(&log).unwrap();
        let back: Log = serde_json::from_str(&json).unwrap();
        assert_eq!(back, log);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["seed"], 9);
        assert_eq!(v["orders"][0]["player"], "axis");
        assert!(v["orders"][0]["order"]["place"].is_object());
    }
}
