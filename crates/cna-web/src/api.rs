//! The `Game` API as plain Rust over JSON values: what the browser
//! calls through `wasm-bindgen`, and what a server will carry. Tested
//! natively; `lib.rs` only converts strings.
//!
//! Orders are `cna_game::order::Order` serde JSON verbatim. `apply`
//! answers `{"ok": [Event]}` or `{"illegal": {"case", "reason"}}`; a
//! refused order changes nothing. Malformed input (a side that is not
//! `axis` or `cw`, JSON that is not an `Order`) is an `Err`, not an
//! illegal order: it is a client bug, not a rules question.

use crate::embedded::data;
use anyhow::{Context, Result};
use cna_data::scenario::Side;
use cna_game::order::Order;
use cna_game::state::GameState;
use serde_json::{Value, json};

#[derive(Debug)]
pub struct Game {
    state: GameState,
}

impl Game {
    /// A new Graziani game, the Axis to place first.
    pub fn new(seed: u64) -> Result<Game> {
        Ok(Game {
            state: GameState::new(data(), seed)?,
        })
    }

    /// A saved game, checked against the embedded scenario and
    /// `rules_commit` only; invariants are checked from slice 6.
    pub fn load(json: &str) -> Result<Game> {
        Ok(Game {
            state: GameState::from_json(json, data())?,
        })
    }

    /// The full state as the save format.
    pub fn save(&self) -> Result<String> {
        self.state.to_json()
    }

    /// What the UI draws: the state minus `log` and `rng`, plus `to_act`
    /// and each piece's `unit_type` from the characteristics chart (null
    /// for supply units and ID codes the chart lacks).
    pub fn view(&self) -> Result<Value> {
        let mut v = serde_json::to_value(&self.state)?;
        let obj = v.as_object_mut().context("state is an object")?;
        obj.remove("log");
        obj.remove("rng");
        obj.insert("to_act".into(), json!(self.state.to_act()));
        let unit_types: Vec<(String, Option<String>)> = self
            .state
            .pieces
            .values()
            .map(|p| {
                let unit_type = p
                    .nation
                    .zip(p.id_code.as_deref())
                    .and_then(|(nation, code)| data().chars.row(nation, code))
                    .map(|row| row.unit_type.clone());
                (p.id.as_str().to_owned(), unit_type)
            })
            .collect();
        for (id, unit_type) in unit_types {
            v["pieces"][&id]["unit_type"] = json!(unit_type);
        }
        Ok(v)
    }

    /// One order from `side`.
    pub fn apply(&mut self, side: &str, order: &str) -> Result<Value> {
        let side: Side = serde_json::from_value(Value::String(side.to_owned()))
            .with_context(|| format!("side {side:?}"))?;
        let order: Order = serde_json::from_str(order).context("parsing the order")?;
        Ok(match self.state.apply(data(), side, order) {
            Ok(events) => json!({ "ok": events }),
            Err(illegal) => {
                json!({ "illegal": { "case": illegal.case, "reason": illegal.reason } })
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded::{RULES_COMMIT, SCENARIO};
    use cna_game::log::{Log, replay};

    fn setup_log() -> Log {
        let path = cna_data::repo_root().join("tests/games/setup.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn a_new_game_waits_on_the_axis_set_up() {
        let game = Game::new(7).unwrap();
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "axis");
        assert_eq!(view["pending"], json!({ "set-up": { "side": "axis" } }));
        assert_eq!(view["scenario"], SCENARIO);
        assert_eq!(view["rules_commit"], RULES_COMMIT);
        assert!(view["over"].is_null());
        assert!(view.get("log").is_none() && view.get("rng").is_none());
    }

    #[test]
    fn view_adds_unit_type_per_piece() {
        let view = Game::new(7).unwrap().view().unwrap();
        let pieces = view["pieces"].as_object().unwrap();
        assert!(!pieces.is_empty());
        let on_chart = |p: &&Value| p["kind"] == "combat" || p["kind"] == "hq";
        let typed = pieces
            .values()
            .filter(on_chart)
            .filter(|p| p["unit_type"].is_string())
            .count();
        assert_eq!(
            typed,
            pieces.values().filter(on_chart).count(),
            "every unit has a unit_type"
        );
        assert!(
            pieces
                .values()
                .filter(|p| !on_chart(p))
                .all(|p| p["unit_type"].is_null())
        );
    }

    #[test]
    fn an_illegal_order_is_a_value_and_changes_nothing() {
        let mut game = Game::new(7).unwrap();
        let before = game.save().unwrap();
        let r = game
            .apply(
                "axis",
                r#"{"place":{"piece":"supply:axis:16","hex":"E0101"}}"#,
            )
            .unwrap();
        assert_eq!(r["illegal"]["case"], "59.2", "{r}");
        assert!(r["illegal"]["reason"].is_string());
        assert_eq!(game.save().unwrap(), before);
    }

    #[test]
    fn a_legal_order_answers_with_its_events() {
        let mut game = Game::new(7).unwrap();
        let r = game
            .apply(
                "axis",
                r#"{"place":{"piece":"supply:axis:16","hex":"C0101"}}"#,
            )
            .unwrap();
        assert!(r["ok"].is_array(), "{r}");
        assert!(!r["ok"].as_array().unwrap().is_empty());
        assert_eq!(
            game.view().unwrap()["pieces"]["supply:axis:16"]["location"],
            json!({ "hex": "C0101" })
        );
    }

    #[test]
    fn malformed_input_is_an_error_not_an_illegal_order() {
        let mut game = Game::new(7).unwrap();
        assert!(game.apply("allies", r#""pass""#).is_err());
        assert!(game.apply("axis", r#"{"teleport":{}}"#).is_err());
        assert!(game.apply("axis", "not json").is_err());
    }

    #[test]
    fn load_of_a_save_round_trips_byte_for_byte() {
        let mut game = Game::new(7).unwrap();
        game.apply(
            "axis",
            r#"{"place":{"piece":"supply:axis:16","hex":"C0101"}}"#,
        )
        .unwrap();
        let saved = game.save().unwrap();
        let loaded = Game::load(&saved).unwrap();
        assert_eq!(loaded.save().unwrap(), saved);
        assert_eq!(loaded.view().unwrap(), game.view().unwrap());
    }

    #[test]
    fn a_save_from_another_rules_commit_is_refused() {
        let saved = Game::new(7).unwrap().save().unwrap();
        let other = saved.replace(RULES_COMMIT, "0000000000000000000000000000000000000000");
        assert_ne!(other, saved);
        let err = Game::load(&other).unwrap_err().to_string();
        assert!(err.contains("vendor/cna"), "{err}");
        assert!(Game::load("{}").is_err());
    }

    #[test]
    fn the_scripted_set_up_through_apply_matches_replay() {
        let log = setup_log();
        assert_eq!(log.orders.len(), 76);
        let mut game = Game::new(log.seed).unwrap();
        for (i, o) in log.orders.iter().enumerate() {
            let side = serde_json::to_value(o.player).unwrap();
            let r = game
                .apply(
                    side.as_str().unwrap(),
                    &serde_json::to_string(&o.order).unwrap(),
                )
                .unwrap();
            assert!(r["ok"].is_array(), "order {i}: {r}");
        }
        let replayed = replay(data(), &log).unwrap();
        assert_eq!(game.save().unwrap(), replayed.to_json().unwrap());
        let view = game.view().unwrap();
        assert!(view["pending"].is_null(), "{}", view["pending"]);
        assert_eq!(view["to_act"], "axis");
    }
}
