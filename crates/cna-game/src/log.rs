//! The log: what a game is made of, and nothing else. Replaying it on the
//! same `vendor/cna` commit rebuilds the state byte for byte. The commit
//! a state or log names is checked against `Data::rules_commit`, never
//! against git: the engine stays free of the file system.

use crate::clock::Phase;
use crate::data::Data;
use crate::order::LoggedOrder;
use crate::piece::PieceId;
use crate::state::{GameState, Pending};
use anyhow::{Context, Result, bail};
use cna_data::map::HexId;
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

    /// Reads a saved state; fails if it is of another scenario, was
    /// made on another `vendor/cna` commit than `data`, or breaks an
    /// invariant `apply` relies on (`check_invariants`): a hostile or
    /// hand-edited save is an error, never a panic.
    pub fn from_json(json: &str, data: &Data) -> Result<GameState> {
        let state: GameState = serde_json::from_str(json).context("parsing the state")?;
        state.check(data)?;
        state.check_invariants(data)?;
        Ok(state)
    }

    /// What serde cannot check: the clock's shape (an OpStage of 1–3,
    /// `half` exactly in phases F–L, `segment` exactly in phase G) and
    /// a pending that names someone (a sealed step with nobody waiting
    /// panicked in `Pending::side`) and only pieces and hexes that
    /// exist.
    pub fn check_invariants(&self, data: &Data) -> Result<()> {
        let c = &self.clock;
        if !(1..=3).contains(&c.stage) {
            bail!("the clock's OpStage is {}, not 1–3", c.stage);
        }
        if c.half.is_some() == c.phase.is_joint() {
            bail!(
                "the clock's half is {} in phase {}",
                if c.half.is_some() { "set" } else { "unset" },
                c.phase.letter()
            );
        }
        if c.segment.is_some() != (c.phase == Phase::MovementCombat) {
            bail!(
                "the clock's segment is {} in phase {}",
                if c.segment.is_some() { "set" } else { "unset" },
                c.phase.letter()
            );
        }
        let piece = |id: &PieceId| -> Result<()> {
            if self.pieces.contains_key(id) {
                Ok(())
            } else {
                bail!("the pending names a piece that does not exist: {id}")
            }
        };
        let hex = |h: &HexId| -> Result<()> {
            if data.map.hex(h).is_some() {
                Ok(())
            } else {
                bail!("the pending names a hex that is not on the map: {h}")
            }
        };
        match &self.pending {
            None | Some(Pending::SetUp { .. }) | Some(Pending::RetreatBeforeAssault { .. }) => {}
            Some(Pending::Sealed { waiting, .. }) => {
                if waiting.is_empty() {
                    bail!("a sealed step is pending with nobody waiting to answer it");
                }
                if waiting.len() > 1 && waiting[0] == waiting[1] {
                    bail!("a sealed step names {} twice", waiting[0]);
                }
            }
            Some(Pending::DeclareZoc { hexes, .. }) => hexes.iter().try_for_each(hex)?,
            Some(Pending::Reaction { eligible, .. }) => eligible.iter().try_for_each(piece)?,
            Some(Pending::ForcedRetreat { pieces, from, .. }) => {
                pieces.iter().try_for_each(piece)?;
                from.iter().try_for_each(hex)?;
            }
        }
        Ok(())
    }

    /// Whether `data` is this game's: the same scenario and the same
    /// `vendor/cna` commit.
    pub fn check(&self, data: &Data) -> Result<()> {
        check(&self.scenario, &self.rules_commit, data)
    }
}

fn check(scenario: &str, commit: &str, data: &Data) -> Result<()> {
    if scenario != data.slug() {
        bail!("the game is of {scenario}, the data of {}", data.slug());
    }
    if commit != data.rules_commit {
        bail!(
            "made on vendor/cna {commit}, but the data is from {}",
            data.rules_commit
        );
    }
    Ok(())
}

/// Rebuilds the state a log describes by applying its orders afresh.
pub fn replay(data: &Data, log: &Log) -> Result<GameState> {
    check(&log.scenario, &log.rules_commit, data)?;
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
    use crate::clock::{Clock, Half, Phase, Segment};
    use crate::combat::SealedStep;
    use crate::order::Order;
    use crate::piece::PieceId;
    use crate::state::{Pending, test_support};
    use cna_data::map::HexId;
    use cna_data::oa::UnitId;
    use cna_data::scenario::Side;
    use std::path::PathBuf;

    fn fixture() -> PathBuf {
        cna_data::repo_root().join("tests/games/setup.json")
    }

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// The scripted game `tests/games/setup.json` is: every set-up order,
    /// then GT1 OpStage 1 passed through by both players, the combat
    /// steps answered with nothing (six orders a half). Regenerate it
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
        assert_eq!(log.orders.len(), 59 + 1 + 2 * 8);
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
        assert_eq!(GameState::from_json(&json, &data).unwrap(), state);
        let mut other = graziani();
        other.rules_commit = "0".repeat(40);
        let err = GameState::from_json(&json, &other).unwrap_err().to_string();
        assert!(err.contains("vendor/cna"), "{err}");
        let log = state.to_log();
        assert!(replay(&other, &log).is_err());
        let mut log = state.to_log();
        log.scenario = "italian-campaign".into();
        assert!(replay(&data, &log).is_err());
    }

    /// A save whose pending is `sealed` with nobody waiting: the one that
    /// used to panic in `Pending::side` (so in `to_act`).
    fn malformed(data: &Data, edit: impl FnOnce(&mut GameState)) -> String {
        let mut state = GameState::new(data, 1).unwrap();
        test_support::auto_place(data, &mut state);
        edit(&mut state);
        state.to_json().unwrap()
    }

    #[test]
    fn a_sealed_pending_with_nobody_waiting_is_refused_not_a_panic() {
        let data = graziani();
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::Sealed {
                step: SealedStep::Barrage,
                waiting: vec![],
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("waiting"), "{err}");
    }

    #[test]
    fn a_sealed_pending_naming_a_side_twice_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::Sealed {
                step: SealedStep::Barrage,
                waiting: vec![Side::Axis, Side::Axis],
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("twice"), "{err}");
    }

    #[test]
    fn a_clock_with_an_opstage_outside_one_to_three_is_refused() {
        let data = graziani();
        for stage in [0, 4] {
            let json = malformed(&data, |s| s.clock.stage = stage);
            let err = GameState::from_json(&json, &data).unwrap_err().to_string();
            assert!(err.contains("OpStage"), "{err}");
        }
    }

    #[test]
    fn a_half_outside_phases_f_to_l_or_missing_inside_them_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| s.clock.half = Some(Half::A));
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("half"), "{err}");
        let json = malformed(&data, |s| {
            s.clock.phase = Phase::ReserveDesignation;
            s.clock.half = None;
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("half"), "{err}");
    }

    #[test]
    fn a_segment_outside_phase_g_or_missing_in_it_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| s.clock.segment = Some(Segment::Barrage));
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("segment"), "{err}");
        let json = malformed(&data, |s| {
            s.clock.phase = Phase::MovementCombat;
            s.clock.half = Some(Half::A);
            s.clock.segment = None;
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("segment"), "{err}");
    }

    #[test]
    fn a_pending_naming_a_piece_or_hex_that_does_not_exist_is_refused() {
        let data = graziani();
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::Reaction {
                side: Side::Cw,
                eligible: vec![PieceId::from(UnitId::parse("unit:cw:nobody").unwrap())],
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("unit:cw:nobody"), "{err}");
        let json = malformed(&data, |s| {
            s.pending = Some(Pending::DeclareZoc {
                side: Side::Cw,
                hexes: vec![HexId::parse("C9999").unwrap()],
                at_start: false,
            });
        });
        let err = GameState::from_json(&json, &data).unwrap_err().to_string();
        assert!(err.contains("C9999"), "{err}");
    }

    #[test]
    fn a_well_formed_save_in_every_phase_loads() {
        let data = graziani();
        let mut state = GameState::new(&data, 7).unwrap();
        test_support::auto_place(&data, &mut state);
        test_support::pass_stage(&data, &mut state);
        let json = state.to_json().unwrap();
        assert_eq!(GameState::from_json(&json, &data).unwrap(), state);
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
