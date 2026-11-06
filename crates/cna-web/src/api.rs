//! The `Game` API as plain Rust over JSON values: what the browser
//! calls through `wasm-bindgen`, and what a server will carry. Tested
//! natively; `lib.rs` only converts strings.
//!
//! Orders are `cna_game::order::Order` serde JSON verbatim. `apply`
//! answers `{"ok": [Event]}` or `{"illegal": {"case", "reason"}}`; a
//! refused order changes nothing. Malformed input (a side that is not
//! `axis` or `cw`, JSON that is not an `Order`) is an `Err`, not an
//! illegal order: it is a client bug, not a rules question.

use anyhow::{Context, Result};
use cna_data::scenario::Side;
use cna_embed::data;
use cna_game::data::Data;
use cna_game::event::Event;
use cna_game::order::Order;
use cna_game::piece::PieceId;
use cna_game::reach::Purpose;
use cna_game::state::{GameState, Modules};
use cna_game::view::{Aliases, view_for};
use serde_json::{Value, json};

#[derive(Debug)]
pub struct Game {
    state: GameState,
    data: &'static Data,
}

impl Game {
    /// A new game of the scenario `slug` (an embedded one), waiting on
    /// its first set-up, with every rules module on (abstract supply
    /// included).
    pub fn new(slug: &str, seed: u64) -> Result<Game> {
        Game::with_modules(slug, seed, Modules::ALL)
    }

    /// `new` with the rules modules named.
    pub fn with_modules(slug: &str, seed: u64, modules: Modules) -> Result<Game> {
        let data = data(slug)?;
        Ok(Game {
            state: GameState::with_modules(data, seed, modules)?,
            data,
        })
    }

    /// A saved game, checked against its (embedded) scenario's data and
    /// `rules_commit`, and for the invariants `apply` relies on
    /// (`GameState::check_invariants`): a bad file is an `Err`.
    pub fn load(json: &str) -> Result<Game> {
        let head: Value = serde_json::from_str(json).context("parsing the state")?;
        let slug = head["scenario"]
            .as_str()
            .context("the state names no scenario")?;
        let data = data(slug)?;
        Ok(Game {
            state: GameState::from_json(json, data)?,
            data,
        })
    }

    /// The full state as the save format.
    pub fn save(&self) -> Result<String> {
        self.state.to_json()
    }

    /// What the UI draws (`cna_game::view::view_for`), seen by the side
    /// to act with open information: hot-seat players share a screen, so
    /// only the sealed answers and the owner's secrets (3.61) of the side
    /// not to act are hidden, and no ids are aliased.
    pub fn view(&self) -> Result<Value> {
        view_for(
            &self.state,
            self.data,
            &Aliases::new([0; 32]),
            self.state.to_act(),
            true,
        )
    }

    /// The hexes `place` would accept for a pending piece of the side
    /// setting up, as a JSON array of hex ids; empty for any other piece
    /// (`GameState::placements`). A piece id that is not a string is an
    /// `Err`.
    pub fn placements(&self, piece: &str) -> Result<Value> {
        let id: PieceId = serde_json::from_value(Value::String(piece.to_owned()))?;
        Ok(json!(self.state.placements(self.data, &id)))
    }

    /// The hexes a stack of `side`'s can end a move of `purpose`
    /// (`move` / `react`) in now, with the cheapest path and cost to
    /// each (`GameState::reachable`): `{"ok": [Reach]}`, or
    /// `{"illegal": {"case", "reason"}}` when the stack cannot move at
    /// all, as `apply` would say. `pieces` is a JSON array of piece ids.
    pub fn reachable(&self, side: &str, pieces: &str, purpose: &str) -> Result<Value> {
        let side = Self::side(side)?;
        let pieces: Vec<PieceId> = serde_json::from_str(pieces).context("parsing the pieces")?;
        let purpose: Purpose = serde_json::from_value(Value::String(purpose.to_owned()))
            .with_context(|| format!("purpose {purpose:?}"))?;
        Ok(
            match self.state.reachable(self.data, side, &pieces, purpose) {
                Ok(reaches) => json!({ "ok": reaches }),
                Err(illegal) => {
                    json!({ "illegal": { "case": illegal.case, "reason": illegal.reason } })
                }
            },
        )
    }

    /// What `side` may answer the sealed step now pending with
    /// (`GameState::sealed_options`): `{"ok": SealedOptions}`, or
    /// `{"illegal": {"case", "reason"}}` when no sealed step is pending
    /// or `side` is not the one to answer.
    pub fn sealed_options(&self, side: &str) -> Result<Value> {
        let side = Self::side(side)?;
        Ok(match self.state.sealed_options(self.data, side) {
            Ok(options) => json!({ "ok": options }),
            Err(illegal) => {
                json!({ "illegal": { "case": illegal.case, "reason": illegal.reason } })
            }
        })
    }

    /// The supply units that could pay the fuel of a move by `pieces`
    /// (`GameState::fuel_sources`), as a JSON array of piece ids: the
    /// choices for the order's `fuel_from`. Empty unless `side` is to
    /// act and owns every piece, so no one learns where the other
    /// side's supply stands (3.61).
    pub fn fuel_sources(&self, side: &str, pieces: &str) -> Result<Value> {
        let side = Self::side(side)?;
        let pieces: Vec<PieceId> = serde_json::from_str(pieces).context("parsing the pieces")?;
        let own = self.state.to_act() == Some(side)
            && pieces
                .iter()
                .all(|id| self.state.pieces.get(id).is_some_and(|p| p.side == side));
        Ok(if own {
            json!(self.state.fuel_sources(self.data, &pieces))
        } else {
            json!([])
        })
    }

    fn side(side: &str) -> Result<Side> {
        serde_json::from_value(Value::String(side.to_owned()))
            .with_context(|| format!("side {side:?}"))
    }

    /// One order from `side`.
    pub fn apply(&mut self, side: &str, order: &str) -> Result<Value> {
        let side = Self::side(side)?;
        let order: Order = serde_json::from_str(order).context("parsing the order")?;
        Ok(Self::answer(self.state.apply(self.data, side, order)))
    }

    /// What `apply` would answer `order` from `side` with, on a copy,
    /// the game untouched: the same `{"ok": [Event]}` or
    /// `{"illegal": ...}`, except that an order whose apply rolls dice
    /// answers `{"hidden": "dice"}` (the dice are the game's next rolls,
    /// so showing them would tell a player the future), and any order
    /// while a sealed step is pending answers `{"hidden": "sealed"}`.
    pub fn preview(&self, side: &str, order: &str) -> Result<Value> {
        let side = Self::side(side)?;
        let order: Order = serde_json::from_str(order).context("parsing the order")?;
        // A sealed answer's events reveal the other side's answer when
        // it completes the step: previewing one would show it early.
        if matches!(
            self.state.pending,
            Some(cna_game::state::Pending::Sealed { .. })
        ) {
            return Ok(json!({ "hidden": "sealed" }));
        }
        let mut copy = self.state.clone();
        let r = copy.apply(self.data, side, order);
        // A refusal is answered as one even if dice were drawn: it
        // carries no roll. (`apply` validates first, so none are.)
        if r.is_ok() && copy.rng.draws() != self.state.rng.draws() {
            return Ok(json!({ "hidden": "dice" }));
        }
        Ok(Self::answer(r))
    }

    fn answer(r: std::result::Result<Vec<Event>, cna_game::event::Illegal>) -> Value {
        match r {
            Ok(events) => json!({ "ok": events }),
            Err(illegal) => {
                json!({ "illegal": { "case": illegal.case, "reason": illegal.reason } })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cna_embed::{DEFAULT_SCENARIO as SCENARIO, RULES_COMMIT};
    use cna_game::combat::SealedStep;
    use cna_game::state::Pending;

    fn graziani() -> &'static Data {
        data(SCENARIO).unwrap()
    }
    use cna_game::log::{Log, replay};

    fn setup_log() -> Log {
        let path = cna_data::repo_root().join("tests/games/setup.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn a_new_game_waits_on_the_axis_set_up() {
        let game = Game::new(SCENARIO, 7).unwrap();
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "axis");
        assert_eq!(view["pending"], json!({ "set-up": { "side": "axis" } }));
        assert_eq!(view["scenario"], SCENARIO);
        assert_eq!(view["rules_commit"], RULES_COMMIT);
        assert!(view["over"].is_null());
        assert!(view.get("log").is_none() && view.get("rng").is_none());
    }

    #[test]
    fn view_marks_shell_formations_and_only_those() {
        let mut game = Game {
            data: graziani(),
            state: replay(graziani(), &setup_log()).unwrap(),
        };
        // Strip an HQ of its attached units: it becomes a shell (9.26).
        let hq = game
            .state
            .pieces
            .values()
            .find(|p| {
                p.kind == cna_game::piece::PieceKind::Hq
                    && !game.state.is_shell(graziani(), &p.id)
                    && game
                        .state
                        .pieces
                        .values()
                        .any(|q| q.attached_to == p.id.unit())
            })
            .expect("a whole formation")
            .id
            .clone();
        let unit = hq.unit();
        for q in game.state.pieces.values_mut() {
            if q.attached_to == unit {
                q.attached_to = None;
            }
        }
        assert!(game.state.is_shell(graziani(), &hq));
        let view = game.view().unwrap();
        for p in game.state.pieces.values() {
            let shell = game.state.is_shell(graziani(), &p.id);
            assert_eq!(
                view["pieces"][p.id.as_str()].get("shell").is_some(),
                shell,
                "{}",
                p.id.as_str()
            );
        }
        assert_eq!(view["pieces"][hq.as_str()]["shell"], true);
    }

    #[test]
    fn view_adds_unit_type_per_piece() {
        let view = Game::new(SCENARIO, 7).unwrap().view().unwrap();
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
    fn view_adds_the_oa_name_and_abbreviation_per_unit() {
        let view = Game::new(SCENARIO, 7).unwrap().view().unwrap();
        let pieces = view["pieces"].as_object().unwrap();
        let tank = &pieces["unit:it:ixl-tank-bn"];
        assert!(tank["name"].is_string(), "{tank}");
        assert!(tank["abbreviation"].is_string(), "{tank}");
        assert!(
            pieces
                .values()
                .filter(|p| p["kind"] == "combat" || p["kind"] == "hq")
                .all(|p| p["name"].is_string() && p["abbreviation"].is_string()),
            "every OA unit has a name and an abbreviation"
        );
        let supply = &pieces["supply:axis:1"];
        assert!(supply["name"].is_null() && supply["abbreviation"].is_null());
    }

    // 3.61: the other side's loads and motorisation points are not in the
    // view; the acting side's own are, and the save keeps both.
    // The replacement points waiting from repair (22.5) are a side's own
    // business too, like its motorisation.
    // 3.61: what a project builds is its owner's secret; that one is
    // under way, and by whom, is not.
    // 26.1x: a minefield is seen, but whether the other side's is real
    // only once an enemy has entered it.
    #[test]
    fn the_view_hides_whether_the_other_side_s_minefield_is_real() {
        use cna_game::construction::Minefield;
        let mut game = Game::with_modules(SCENARIO, 7, Modules::ALL).unwrap();
        for (side, hex, revealed) in [
            (Side::Axis, "C4021", false),
            (Side::Cw, "C3923", false),
            (Side::Cw, "C3924", true),
        ] {
            game.state.minefields.insert(
                cna_data::map::HexId::parse(hex).unwrap(),
                Minefield {
                    side,
                    real: true,
                    revealed,
                },
            );
        }
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "axis");
        assert_eq!(view["minefields"]["C4021"]["real"], true);
        assert!(view["minefields"]["C3923"].get("real").is_none());
        assert_eq!(view["minefields"]["C3923"]["side"], "cw");
        assert_eq!(view["minefields"]["C3924"]["real"], true);
    }

    #[test]
    fn the_view_hides_what_the_other_side_is_building() {
        let mut game = Game::with_modules(SCENARIO, 7, Modules::ALL).unwrap();
        for (side, hex) in [(Side::Axis, "C4021"), (Side::Cw, "C3923")] {
            game.state.projects.insert(
                cna_data::map::HexId::parse(hex).unwrap(),
                cna_game::construction::Project {
                    side,
                    item: cna_game::construction::Item::Fortification,
                    builders: vec![],
                    stages: 1,
                },
            );
        }
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "axis");
        assert_eq!(view["projects"]["C4021"]["item"], "fortification");
        assert!(view["projects"]["C3923"].get("item").is_none());
        assert_eq!(view["projects"]["C3923"]["side"], "cw");
    }

    #[test]
    fn the_view_hides_the_other_side_s_repaired_points() {
        use std::collections::BTreeMap;
        let mut game = Game::with_modules(SCENARIO, 7, Modules::ALL).unwrap();
        for side in [Side::Axis, Side::Cw] {
            game.state.repaired.insert(
                side,
                BTreeMap::from([(
                    cna_data::map::HexId::parse("C4021").unwrap(),
                    BTreeMap::from([("tank".to_string(), 1)]),
                )]),
            );
        }
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "axis");
        assert_eq!(view["repaired"]["axis"]["C4021"]["tank"], 1);
        assert!(view["repaired"].get("cw").is_none());
    }

    #[test]
    fn the_view_hides_the_other_side_s_loads_and_motorisation() {
        let game = Game::with_modules(SCENARIO, 7, Modules::ALL).unwrap();
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "axis");
        assert_eq!(
            view["pieces"]["supply:axis:1"]["load"],
            json!({"ammo": 15, "fuel": 50})
        );
        assert!(view["pieces"]["supply:cw:1"].get("load").is_none());
        assert!(view["motorisation"]["axis"].is_object());
        assert!(view["motorisation"].get("cw").is_none());
        // Only the Commonwealth's Alexandria deployment is pending: hidden.
        assert!(
            view["pending_motorisation"]
                .as_object()
                .is_none_or(|m| m.is_empty())
        );
        let save: Value = serde_json::from_str(&game.save().unwrap()).unwrap();
        assert!(save["pieces"]["supply:cw:1"]["load"].is_object());
        assert!(save["motorisation"]["cw"].is_object());
        assert!(save["pending_motorisation"].is_object());
    }

    #[test]
    fn the_view_hides_the_other_side_s_attached_motorisation() {
        let mut game = Game::with_modules(SCENARIO, 7, Modules::ALL).unwrap();
        for id in ["supply:axis:1", "supply:cw:1"] {
            game.state
                .pieces
                .get_mut(&serde_json::from_value::<PieceId>(json!(id)).unwrap())
                .unwrap()
                .motorised = 30;
        }
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "axis");
        assert_eq!(view["pieces"]["supply:axis:1"]["motorised"], 30);
        assert!(view["pieces"]["supply:cw:1"].get("motorised").is_none());
    }

    #[test]
    fn an_illegal_order_is_a_value_and_changes_nothing() {
        let mut game = Game::new(SCENARIO, 7).unwrap();
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
        let mut game = Game::new(SCENARIO, 7).unwrap();
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
    fn placements_lists_the_hexes_place_accepts_for_the_side_setting_up() {
        let mut game = Game::new(SCENARIO, 7).unwrap();
        let hexes = game.placements("supply:axis:16").unwrap();
        let hexes = hexes.as_array().unwrap();
        assert!(hexes.contains(&json!("C0101")), "{hexes:?}");
        assert!(hexes.iter().all(|h| h.as_str().unwrap().starts_with('C')));
        // The Commonwealth is not setting up yet; a placed piece and an
        // unknown one have nowhere to go either.
        assert_eq!(game.placements("supply:cw:7").unwrap(), json!([]));
        assert_eq!(game.placements("unit:it:no-such").unwrap(), json!([]));
        let r = game
            .apply(
                "axis",
                r#"{"place":{"piece":"supply:axis:16","hex":"C0101"}}"#,
            )
            .unwrap();
        assert!(r["ok"].is_array());
        assert_eq!(game.placements("supply:axis:16").unwrap(), json!([]));
    }

    #[test]
    fn malformed_input_is_an_error_not_an_illegal_order() {
        let mut game = Game::new(SCENARIO, 7).unwrap();
        assert!(game.apply("allies", r#""pass""#).is_err());
        assert!(game.apply("axis", r#"{"teleport":{}}"#).is_err());
        assert!(game.apply("axis", "not json").is_err());
        assert!(
            game.reachable("axis", r#"["unit:it:ixl-tank-bn"]"#, "stroll")
                .is_err()
        );
        assert!(
            game.reachable("axis", "unit:it:ixl-tank-bn", "move")
                .is_err()
        );
    }

    /// The golden game replayed to just before order `n`.
    fn golden_before(n: usize) -> (Game, Log) {
        let path = cna_data::repo_root().join("tests/games/graziani-m5.json");
        let log: Log = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let mut prefix = log.clone();
        prefix.orders.truncate(n);
        let state = replay(graziani(), &prefix).unwrap();
        (Game::load(&state.to_json().unwrap()).unwrap(), log)
    }

    #[test]
    fn reachable_names_golden_order_61_s_hex_with_its_path_and_cost() {
        let (game, log) = golden_before(60);
        let Order::Move { pieces, .. } = &log.orders[60].order else {
            panic!("order 61 is a move");
        };
        let r = game
            .reachable("axis", &serde_json::to_string(pieces).unwrap(), "move")
            .unwrap();
        let reaches = r["ok"].as_array().unwrap_or_else(|| panic!("{r}"));
        let c4021 = reaches
            .iter()
            .find(|x| x["hex"] == "C4021")
            .unwrap_or_else(|| panic!("{reaches:?}"));
        assert_eq!(
            c4021,
            &json!({
                "hex": "C4021",
                "cp": 1.0,
                "path": ["C4021"],
                "dp": 0,
                "ends_in_zoc": false,
                "may_be_interrupted": false,
                "short": 0,
                "fuel": 1
            })
        );
        // The fuel quoted is the fuel the order burns.
        let mut played = Game::load(&game.save().unwrap()).unwrap();
        let order = serde_json::to_string(&log.orders[60].order).unwrap();
        let events = played.apply("axis", &order).unwrap();
        let burnt: u64 = events["ok"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["fuel-spent"]["fuel"].as_u64())
            .sum();
        assert_eq!(burnt, 1, "{events}");
        // Not the Commonwealth's turn; a supply unit without motorisation points cannot move (32.31).
        let r = game
            .reachable("cw", r#"["unit:cw:4-rha"]"#, "move")
            .unwrap();
        assert_eq!(r["illegal"]["case"], "5.2", "{r}");
        let r = game
            .reachable("axis", r#"["supply:axis:1"]"#, "move")
            .unwrap();
        assert_eq!(r["illegal"]["case"], "32.31", "{r}");
    }

    #[test]
    fn fuel_sources_are_the_side_to_act_s_own_only() {
        let (game, _) = golden_before(60);
        let tank = game
            .state
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Axis
                    && !game
                        .state
                        .fuel_sources(graziani(), std::slice::from_ref(&p.id))
                        .is_empty()
            })
            .expect("an Axis fuel burner near supply")
            .id
            .clone();
        let pieces = serde_json::to_string(&[&tank]).unwrap();
        let mine = game.fuel_sources("axis", &pieces).unwrap();
        assert!(!mine.as_array().unwrap().is_empty(), "{mine}");
        // The Commonwealth may not ask after the Axis' supply (3.61).
        assert_eq!(game.fuel_sources("cw", &pieces).unwrap(), json!([]));
    }

    #[test]
    fn reachable_for_a_reaction_names_golden_order_62_s_hex() {
        let (game, _) = golden_before(61);
        assert_eq!(game.view().unwrap()["to_act"], "cw");
        let r = game
            .reachable("cw", r#"["unit:cw:4-rha"]"#, "react")
            .unwrap();
        let reaches = r["ok"].as_array().unwrap_or_else(|| panic!("{r}"));
        assert!(
            reaches
                .iter()
                .any(|x| x["hex"] == "C3923" && x["path"] == json!(["C3923"]))
        );
        let r = game
            .reachable("cw", r#"["unit:cw:4-rha"]"#, "move")
            .unwrap();
        assert_eq!(r["illegal"]["case"], "8.5x", "{r}");
    }

    /// Golden orders 65–71 are the sealed steps: after the Axis has
    /// answered (66, 68, 71 are the Commonwealth's turns) its answer is
    /// not in the Commonwealth's view; once both are in, both are.
    #[test]
    fn a_sealed_answer_is_redacted_from_the_other_side_s_view_until_revealed() {
        let axis_gun = json!("unit:it:1st-libyan-artillery-regt");
        // 66: the Axis gun stands forward; the Commonwealth sees no forward gun.
        let (game, _) = golden_before(65);
        let view = game.view().unwrap();
        assert_eq!(view["to_act"], "cw");
        assert_eq!(view["pending"]["sealed"]["step"], "gun-positions");
        assert!(
            view["combat"]
                .get("forward")
                .is_none_or(|f| f.as_array().unwrap().is_empty()),
            "{}",
            view["combat"]
        );
        // The save keeps everything.
        assert!(game.save().unwrap().contains("1st-libyan-artillery-regt\""));
        // 67: revealed, both sides' positions are in the view.
        let (game, _) = golden_before(66);
        let view = game.view().unwrap();
        assert!(
            view["combat"]["forward"]
                .as_array()
                .unwrap()
                .contains(&axis_gun),
            "{}",
            view["combat"]
        );
        // 68: the Axis barrage is plotted; the Commonwealth sees no plot.
        let (game, _) = golden_before(67);
        let view = game.view().unwrap();
        assert_eq!(view["pending"]["sealed"]["step"], "barrage");
        assert!(
            view["combat"]
                .get("plots")
                .is_none_or(|p| p.as_object().unwrap().is_empty()),
            "{}",
            view["combat"]
        );
        assert!(
            view["combat"]["forward"]
                .as_array()
                .unwrap()
                .contains(&axis_gun),
            "step 1 is revealed"
        );
        // 69: revealed.
        let (game, _) = golden_before(68);
        let view = game.view().unwrap();
        assert_eq!(
            view["combat"]["plots"]["axis"][0]["target"],
            "unit:cw:1-krrc"
        );
        assert!(view["combat"]["plots"]["cw"].as_array().unwrap().is_empty());
        // 71: the Axis assignments are in; the Commonwealth sees none.
        let (game, _) = golden_before(70);
        let view = game.view().unwrap();
        assert_eq!(view["pending"]["sealed"]["step"], "assign-forces");
        assert!(
            view["combat"]
                .get("assignments")
                .is_none_or(|a| a.as_object().unwrap().is_empty()),
            "{}",
            view["combat"]
        );
        assert_eq!(
            view["combat"]["plots"]["axis"][0]["points"], 6,
            "step 2 is revealed"
        );
        assert_eq!(view["assaults_to_resolve"], json!([]));
        // 72: revealed, and the close assault step waits on the Axis
        // with C3922 to assault.
        let (game, _) = golden_before(71);
        let view = game.view().unwrap();
        assert!(view["pending"].is_null());
        assert_eq!(view["clock"]["segment"], "close-assault");
        assert_eq!(
            view["combat"]["assignments"]["axis"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            view["combat"]["assignments"]["cw"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(view["assaults_to_resolve"], json!(["C3922"]));
        let (game, _) = golden_before(72);
        assert_eq!(game.view().unwrap()["assaults_to_resolve"], json!([]));
    }

    #[test]
    fn sealed_options_name_the_golden_barrage_and_nothing_at_other_moments() {
        let (game, _) = golden_before(66);
        let r = game.sealed_options("axis").unwrap();
        let firers = r["ok"]["barrage"]["firers"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"));
        let gun = firers
            .iter()
            .find(|f| f["piece"] == "unit:it:1st-libyan-artillery-regt")
            .unwrap();
        assert!(
            gun["targets"]
                .as_array()
                .unwrap()
                .contains(&json!("unit:cw:1-krrc"))
        );
        assert_eq!(gun["forward"], true);
        assert_eq!(game.sealed_options("cw").unwrap()["illegal"]["case"], "5.2");
        let (game, _) = golden_before(68);
        assert_eq!(
            game.sealed_options("cw").unwrap()["illegal"]["case"],
            "11.0"
        );
        assert!(game.sealed_options("allies").is_err());
    }

    /// Golden order 69 is the Commonwealth's pass in retreat before
    /// assault: the query names where 4 RHA at C3923 could have gone
    /// instead, with `short` 0 throughout.
    #[test]
    fn reachable_for_a_retreat_before_assault_at_golden_order_70() {
        let (game, _) = golden_before(68);
        assert_eq!(
            game.view().unwrap()["pending"],
            json!({ "retreat-before-assault": { "side": "cw" } })
        );
        let r = game
            .reachable("cw", r#"["unit:cw:4-rha"]"#, "retreat-before-assault")
            .unwrap();
        let reaches = r["ok"].as_array().unwrap_or_else(|| panic!("{r}"));
        assert!(!reaches.is_empty());
        assert!(
            reaches
                .iter()
                .all(|x| x["short"] == 0 && x["may_be_interrupted"] == false)
        );
        let r = game
            .reachable("cw", r#"["unit:cw:4-rha"]"#, "forced-retreat")
            .unwrap();
        assert_eq!(r["illegal"]["case"], "15.82", "{r}");
    }

    /// The `forced-retreat` fixture: standing still is two hexes short,
    /// and the preview says so with the loss `apply` then takes, the
    /// game unchanged in between.
    #[test]
    fn preview_of_pass_in_a_forced_retreat_is_what_apply_does() {
        let fixture = std::fs::read_to_string(fixtures_dir().join("forced-retreat.json")).unwrap();
        let mut game = Game::load(&fixture).unwrap();
        // A refusal previews as one, and bad input is an error.
        let refused = game.preview("axis", "\"pass\"").unwrap();
        assert!(refused["illegal"]["case"].is_string(), "{refused}");
        assert!(game.preview("cw", "{}").is_err());
        let previewed = game.preview("cw", "\"pass\"").unwrap();
        assert_eq!(game.save().unwrap(), fixture, "a preview changes nothing");
        assert_eq!(
            previewed["ok"][0],
            json!({ "retreat-bought-out": { "pieces": ["unit:cw:1-krrc", "unit:cw:3-cldgds", "unit:cw:4-rha", "unit:cw:7-med"], "short": 2 } }),
            "{previewed}"
        );
        assert!(
            previewed["ok"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e.get("losses").is_some())
        );
        assert_eq!(game.apply("cw", "\"pass\"").unwrap(), previewed);
    }

    #[test]
    fn load_of_a_save_round_trips_byte_for_byte() {
        let mut game = Game::new(SCENARIO, 7).unwrap();
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
    fn every_offered_scenario_starts_waiting_on_a_set_up() {
        for (slug, _) in cna_embed::scenarios().unwrap() {
            let view = Game::new(&slug, 7).unwrap().view().unwrap();
            assert_eq!(view["scenario"], slug.as_str());
            assert!(
                view["pending"]["set-up"].is_object(),
                "{slug}: {}",
                view["pending"]
            );
            assert!(!view["pieces"].as_object().unwrap().is_empty(), "{slug}");
        }
        assert!(Game::new("no-such-scenario", 7).is_err());
    }

    #[test]
    fn load_of_an_operation_crusader_save_is_that_scenario() {
        let game = Game::new("operation-crusader", 7).unwrap();
        let saved = game.save().unwrap();
        let loaded = Game::load(&saved).unwrap();
        assert_eq!(loaded.save().unwrap(), saved);
        assert_eq!(loaded.view().unwrap()["scenario"], "operation-crusader");
        let unknown = saved.replace("\"operation-crusader\"", "\"no-such-scenario\"");
        assert!(Game::load(&unknown).is_err());
    }

    #[test]
    fn a_save_from_another_rules_commit_is_refused() {
        let saved = Game::new(SCENARIO, 7).unwrap().save().unwrap();
        let other = saved.replace(RULES_COMMIT, "0000000000000000000000000000000000000000");
        assert_ne!(other, saved);
        let err = Game::load(&other).unwrap_err().to_string();
        assert!(err.contains("vendor/cna"), "{err}");
        assert!(Game::load("{}").is_err());
    }

    fn golden_log() -> Log {
        let path = cna_data::repo_root().join("tests/games/graziani-m5.json");
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// The golden game replayed through its first `n` orders.
    fn golden_prefix(n: usize) -> GameState {
        let mut log = golden_log();
        log.orders.truncate(n);
        replay(graziani(), &log).unwrap()
    }

    fn sha256(s: &str) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(s))
    }

    fn fixtures_dir() -> std::path::PathBuf {
        cna_data::repo_root().join("web/tests/fixtures")
    }

    /// The e2e fixtures (spec, *Testing*): states the Playwright specs
    /// start from, each built from a golden prefix through the public
    /// API and checked to be what its name says.
    ///
    /// - `set-up-done`: orders 0–58, GT1 phase A, the Axis to act.
    /// - `reserve`: orders 0–59, phase F, the Axis to designate.
    /// - `reserve-release`: the Libyans designated, the segment ended
    ///   and the combat steps answered with nothing, so the release
    ///   segment waits on the Axis.
    /// - `off-map`: orders 0–60, the Axis movement segment, the Tripoli
    ///   box full; a box stack's `move-off-map` is accepted.
    /// - `declare-zoc`: orders 0–60 and then the cheapest flagged move of
    ///   the 1st Libyan Division's stack that the engine stops for a ZOC
    ///   declaration, so the Commonwealth is asked.
    /// - `combat-start`: orders 0–65, gun positions sealed, the Axis to
    ///   answer.
    /// - `retreat-before-assault`: orders 0–69, the Commonwealth asked to
    ///   retreat before assault; 4 RHA at C3923 may.
    /// - `forced-retreat`: orders 0–71, the Commonwealth withholding
    ///   its Halfaya infantry in place of order 72, the assault (order
    ///   73) applied: the defenders must retreat three hexes or pay.
    /// - `game-over`: the whole golden game, the report in.
    /// - `raid-rommel`: Operation Crusader at the Commonwealth's movement
    ///   segment, a test-only LRDG hidden next to Rommel, who may be raided.
    /// - `spot-raider`: orders 0–60, the Axis movement segment, a
    ///   test-only Commonwealth raider hidden in an empty hex a lone Axis
    ///   combat unit may enter by a one-hex move: the move stops there and
    ///   the unit may try to spot it (27.42–27.43).
    /// - `patrol`: the golden game at the first phase L in which the
    ///   player to act has a unit that may patrol (16.x).
    fn e2e_fixtures() -> Vec<(&'static str, GameState)> {
        let set_up_done = golden_prefix(56);
        assert_eq!(set_up_done.to_act(), Some(Side::Axis));
        assert!(set_up_done.pending.is_none());
        assert_eq!(
            set_up_done.clock.phase,
            cna_game::clock::Phase::InitiativeDeclaration
        );

        let reserve = golden_prefix(59);
        assert_eq!(
            reserve.clock.phase,
            cna_game::clock::Phase::ReserveDesignation
        );
        assert_eq!(reserve.to_act(), Some(Side::Axis));
        let Order::Move {
            pieces: libyans, ..
        } = &golden_log().orders[60].order
        else {
            panic!("order 61 is a move");
        };
        let mut reserve_release = reserve.clone();
        reserve_release
            .apply(
                graziani(),
                Side::Axis,
                Order::DesignateReserve {
                    pieces: libyans.clone(),
                },
            )
            .unwrap();
        reserve_release
            .apply(graziani(), Side::Axis, Order::Pass)
            .unwrap();
        reserve_release
            .apply(graziani(), Side::Axis, Order::EndSegment)
            .unwrap();
        // The combat steps, answered with nothing (as the scripted games do).
        while reserve_release.clock.segment != Some(cna_game::clock::Segment::ReserveRelease) {
            let player = reserve_release.to_act().unwrap();
            let order = match &reserve_release.pending {
                Some(Pending::Sealed {
                    step: SealedStep::GunPositions,
                    ..
                }) => Order::GunPositions { forward: vec![] },
                Some(Pending::Sealed {
                    step: SealedStep::Barrage,
                    ..
                }) => Order::Barrage { fires: vec![] },
                Some(Pending::Sealed {
                    step: SealedStep::AssignForces,
                    ..
                }) => Order::AssignForces {
                    assignments: vec![],
                },
                _ => Order::Pass,
            };
            reserve_release.apply(graziani(), player, order).unwrap();
        }
        assert_eq!(reserve_release.to_act(), Some(Side::Axis));
        assert!(reserve_release.pending.is_none());

        let off_map = golden_prefix(60);
        assert_eq!(
            off_map.clock.segment,
            Some(cna_game::clock::Segment::Movement)
        );
        let in_tripoli: Vec<PieceId> = off_map
            .pieces
            .values()
            .filter(|p| {
                p.location == cna_game::piece::Location::Box(cna_data::scenario::OffMapBox::Tripoli)
            })
            .filter(|p| p.kind == cna_game::piece::PieceKind::Combat && p.attached_to.is_none())
            .map(|p| p.id.clone())
            .take(2)
            .collect();
        assert_eq!(in_tripoli.len(), 2);
        let mut probe = off_map.clone();
        probe
            .apply(
                graziani(),
                Side::Axis,
                Order::MoveOffMap {
                    pieces: in_tripoli,
                    to: cna_data::off_map::OffMapPlace::Tripolitania,
                },
            )
            .expect("a Tripoli stack may leave for Tripolitania");

        let Order::Move { pieces, .. } = &golden_log().orders[60].order else {
            panic!("order 61 is a move");
        };
        let mut reaches = off_map
            .reachable(graziani(), Side::Axis, pieces, Purpose::Move)
            .unwrap();
        reaches.sort_by_key(|r| (r.cp, r.path.len()));
        let declare_zoc = reaches
            .iter()
            .filter(|r| r.may_be_interrupted)
            .find_map(|r| {
                let mut probe = off_map.clone();
                probe
                    .apply(
                        graziani(),
                        Side::Axis,
                        Order::Move {
                            pieces: pieces.clone(),
                            path: r.path.clone(),
                            assault: false,
                            fuel_from: None,
                        },
                    )
                    .ok()?;
                matches!(probe.pending, Some(Pending::DeclareZoc { .. })).then_some(probe)
            })
            .expect("a flagged move of the Libyans is stopped for a declaration");
        assert_eq!(declare_zoc.to_act(), Some(Side::Cw));

        let combat_start = golden_prefix(64);
        assert_eq!(combat_start.to_act(), Some(Side::Axis));
        assert!(matches!(combat_start.pending, Some(Pending::Sealed { .. })));

        let retreat_before_assault = golden_prefix(68);
        assert_eq!(
            retreat_before_assault.pending,
            Some(Pending::RetreatBeforeAssault { side: Side::Cw })
        );
        let rha = vec![PieceId::from(
            cna_data::oa::UnitId::parse("unit:cw:4-rha").unwrap(),
        )];
        assert!(
            !retreat_before_assault
                .reachable(graziani(), Side::Cw, &rha, Purpose::RetreatBeforeAssault)
                .unwrap()
                .is_empty()
        );

        // Orders 0–71, the Commonwealth then listing its infantry at
        // Halfaya with no points (unlisted infantry would defend the hex
        // by default), so the assault (order 73) orders a three-hex
        // retreat without a die (15.29): the stack stands one hex from
        // its attackers, so Pass is two short and pays.
        let mut forced_retreat = golden_prefix(70);
        let withheld = ["unit:cw:1-krrc", "unit:cw:3-cldgds"]
            .into_iter()
            .map(|id| cna_game::order::Assignment {
                piece: PieceId::from(cna_data::oa::UnitId::parse(id).unwrap()),
                anti_armour: vec![],
                close_assault: vec![],
            })
            .collect();
        forced_retreat
            .apply(
                graziani(),
                Side::Cw,
                Order::AssignForces {
                    assignments: withheld,
                },
            )
            .unwrap();
        forced_retreat
            .apply(
                graziani(),
                Side::Axis,
                golden_log().orders[71].order.clone(),
            )
            .unwrap();
        let Some(Pending::ForcedRetreat {
            side: Side::Cw,
            reason: cna_game::combat::RetreatReason::Withheld,
            pieces,
            hexes: 3,
            ..
        }) = &forced_retreat.pending
        else {
            panic!(
                "the assault on an undefended Halfaya orders a three-hex retreat, not {:?}",
                forced_retreat.pending
            );
        };
        // 4 RHA and 7 Medium owe fuel nothing in range can pay, so the
        // stack makes no retreat at all (R-120).
        assert!(
            forced_retreat
                .reachable(graziani(), Side::Cw, pieces, Purpose::ForcedRetreat)
                .unwrap()
                .is_empty(),
            "no fuel, no retreat"
        );
        assert!(
            forced_retreat
                .clone()
                .apply(graziani(), Side::Cw, Order::Pass)
                .unwrap()
                .iter()
                .any(|e| matches!(e, Event::RetreatBoughtOut { short: 2, .. })),
            "standing still is two hexes short"
        );

        // Phase J of GT1 OpStage 1: the Commonwealth's supply at Mersa
        // Matruh may ride the railway east (golden order 85 passes).
        let rail = golden_prefix(85);
        assert_eq!(rail.clock.phase, cna_game::clock::Phase::Rail);
        assert_eq!(rail.to_act(), Some(Side::Cw));

        // Phase K straight after: four broken-down Commonwealth light tank
        // points put at Alexandria, where its supply stands (22.2x).
        let mut repair = rail.clone();
        let alexandria = cna_data::map::HexId::parse("E3613").unwrap();
        repair.broken_down.insert(
            alexandria.clone(),
            vec![cna_game::breakdown::Wreck {
                side: Side::Cw,
                nation: None,
                weapon: "Mark VI Light".into(),
                points: 4,
                this_stage: 0,
                towed: 0,
                unit: None,
            }],
        );
        repair.apply(graziani(), Side::Cw, Order::Pass).unwrap();
        assert_eq!(repair.clock.phase, cna_game::clock::Phase::Repair);
        assert_eq!(repair.to_act(), Some(Side::Cw));
        assert_eq!(repair.field_repairs(graziani()).len(), 1);

        // The same phase K with the light tanks a hex out of Alexandria
        // instead, where only a tow brings them to its facility (21.6x).
        let mut tow = rail.clone();
        let wrecks = repair.broken_down[&alexandria].clone();
        let out = graziani()
            .map
            .neighbours(&alexandria)
            .into_iter()
            .find(|h| {
                let mut s = tow.clone();
                s.broken_down.insert(h.clone(), wrecks.clone());
                s.apply(graziani(), Side::Cw, Order::Pass).unwrap();
                s.apply(
                    graziani(),
                    Side::Cw,
                    Order::Tow {
                        hex: h.clone(),
                        weapon: "Mark VI Light".into(),
                        points: 1,
                        path: vec![alexandria.clone()],
                    },
                )
                .is_ok()
            })
            .expect("a hex beside Alexandria to tow from");
        tow.broken_down.insert(out, wrecks);
        tow.apply(graziani(), Side::Cw, Order::Pass).unwrap();
        assert_eq!(tow.clock.phase, cna_game::clock::Phase::Repair);

        // The same phase K with three destroyed Commonwealth light tank
        // points at Alexandria's major facility instead (22.4x).
        let mut destroyed_repair = rail.clone();
        destroyed_repair.destroyed_tanks.insert(
            alexandria.clone(),
            vec![cna_game::breakdown::Wreck {
                side: Side::Cw,
                nation: None,
                weapon: "Mark VI Light".into(),
                points: 3,
                this_stage: 0,
                towed: 0,
                unit: None,
            }],
        );
        destroyed_repair.apply(graziani(), Side::Cw, Order::Pass).unwrap();
        assert_eq!(destroyed_repair.clock.phase, cna_game::clock::Phase::Repair);
        assert_eq!(destroyed_repair.destroyed_tank_repairs(graziani()).len(), 1);

        let game_over = replay(graziani(), &golden_log()).unwrap();
        assert!(game_over.is_over());
        assert_eq!(game_over.to_act(), None);

        // Operation Crusader set up hex by hex, each piece on the first
        // hex it is offered: Rommel with a German unit (R-135), lending
        // the units in his hex +5 CPA (R-137).
        let crusader = data("operation-crusader").unwrap();
        let mut crusader_set_up = GameState::with_modules(crusader, 7, Modules::ALL).unwrap();
        while let Some(Pending::SetUp { side }) = crusader_set_up.pending.clone() {
            let (piece, hex) = crusader_set_up
                .pieces
                .values()
                .filter(|p| p.side == side && p.is_pending())
                .find_map(|p| {
                    let hex = crusader_set_up
                        .placements(crusader, &p.id)
                        .into_iter()
                        .next()?;
                    Some((p.id.clone(), hex))
                })
                .expect("a pending piece with a hex to go to");
            crusader_set_up
                .apply(crusader, side, Order::Place { piece, hex })
                .unwrap();
        }
        assert!(crusader_set_up.pieces.values().any(|p| p.rommel_cpa == 5));

        // Operation Crusader played on to the Commonwealth's first
        // movement segment (every other step passed, the Commonwealth
        // Player A), with a test-only LRDG hidden next to Rommel: cna has
        // no raider counters yet (desert raiders plan, Task 3).
        let mut raid = crusader_set_up.clone();
        let cw_moving = |s: &GameState| {
            s.clock.phase == cna_game::clock::Phase::MovementCombat
                && s.clock.segment == Some(cna_game::clock::Segment::Movement)
                && s.to_act() == Some(Side::Cw)
                && s.pending.is_none()
        };
        while !cw_moving(&raid) {
            let player = raid.to_act().expect("the game goes on");
            let order = match &raid.pending {
                Some(Pending::Sealed {
                    step: SealedStep::GunPositions,
                    ..
                }) => Order::GunPositions { forward: vec![] },
                Some(Pending::Sealed {
                    step: SealedStep::Barrage,
                    ..
                }) => Order::Barrage { fires: vec![] },
                Some(Pending::Sealed {
                    step: SealedStep::AssignForces,
                    ..
                }) => Order::AssignForces {
                    assignments: vec![],
                },
                Some(Pending::DeclareZoc { .. }) => Order::DeclareZoc { exerting: vec![] },
                _ if raid.clock.phase == cna_game::clock::Phase::InitiativeDeclaration => {
                    Order::DeclareInitiative { player_a: Side::Cw }
                }
                _ => Order::Pass,
            };
            raid.apply(crusader, player, order.clone())
                .unwrap_or_else(|e| panic!("{order:?} at {:?}: {e:?}", raid.clock));
        }
        assert!(!raid.rommel_recalled, "Rommel commands this turn");
        let rommel = raid
            .pieces
            .values()
            .find(|p| p.kind == cna_game::piece::PieceKind::Commander)
            .and_then(|p| p.hex().cloned())
            .expect("Rommel is on the map");
        let beside = crusader
            .map
            .neighbours(&rommel)
            .into_iter()
            .find(|h| raid.pieces.values().all(|p| p.hex() != Some(h)))
            .expect("an empty hex next to Rommel");
        let mut lrdg = raid
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == cna_game::piece::PieceKind::Combat)
            .expect("a Commonwealth unit")
            .clone();
        lrdg.id = PieceId::from("unit:cw:lrdg-test".to_owned());
        lrdg.kind = cna_game::piece::PieceKind::Raider;
        lrdg.id_code = None;
        lrdg.toe = 0;
        lrdg.weapons.clear();
        lrdg.equivalent = None;
        lrdg.cpa = None;
        lrdg.parent = None;
        lrdg.attached_to = None;
        lrdg.reserve = None;
        lrdg.hidden = true;
        lrdg.location = cna_game::piece::Location::Hex(beside);
        raid.pieces.insert(lrdg.id.clone(), lrdg.clone());
        let mut probe = raid.clone();
        let events = probe
            .apply(
                crusader,
                Side::Cw,
                Order::RaidRommel {
                    piece: lrdg.id.clone(),
                },
            )
            .expect("the LRDG next to Rommel may raid him");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::RommelRaided { .. }))
        );

        // Orders 0–60 again, with a test-only Commonwealth raider hidden
        // in an empty hex one move from a lone Axis combat unit.
        let mut spot = off_map.clone();
        let hex_of = |s: &GameState, id: &PieceId| s.pieces[id].hex().cloned();
        let (mover, target) = spot
            .pieces
            .values()
            .filter(|p| {
                p.side == Side::Axis
                    && p.kind == cna_game::piece::PieceKind::Combat
                    && p.attached_to.is_none()
            })
            .filter_map(|p| Some((p.id.clone(), p.hex().cloned()?)))
            .filter(|(id, at)| {
                spot.pieces
                    .values()
                    .all(|q| q.id == *id || q.hex() != Some(at))
            })
            .find_map(|(id, _)| {
                let reaches = spot
                    .reachable(
                        graziani(),
                        Side::Axis,
                        std::slice::from_ref(&id),
                        Purpose::Move,
                    )
                    .ok()?;
                let to = reaches
                    .iter()
                    .filter(|r| r.path.len() == 1 && !r.may_be_interrupted && r.short == 0)
                    .map(|r| r.hex.clone())
                    .find(|h| spot.pieces.values().all(|p| p.hex() != Some(h)))?;
                Some((id, to))
            })
            .expect("a lone Axis unit with an empty hex one move away");
        let mut raider = spot
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == cna_game::piece::PieceKind::Combat)
            .expect("a Commonwealth unit")
            .clone();
        raider.id = PieceId::from("unit:cw:lrdg-test".to_owned());
        raider.kind = cna_game::piece::PieceKind::Raider;
        raider.id_code = None;
        raider.toe = 0;
        raider.weapons.clear();
        raider.equivalent = None;
        raider.cpa = None;
        raider.parent = None;
        raider.attached_to = None;
        raider.reserve = None;
        raider.hidden = true;
        raider.location = cna_game::piece::Location::Hex(target.clone());
        spot.pieces.insert(raider.id.clone(), raider);
        let mut probe = spot.clone();
        let events = probe
            .apply(
                graziani(),
                Side::Axis,
                Order::Move {
                    pieces: vec![mover.clone()],
                    path: vec![target.clone()],
                    assault: false,
                    fuel_from: None,
                },
            )
            .expect("the Axis unit may enter the raider's hex");
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::RaiderAnnounced { hex, pieces } if *hex == target && pieces == std::slice::from_ref(&mover)
            )),
            "the raider is announced to the mover: {events:?}"
        );
        assert_eq!(hex_of(&probe, &mover), Some(target.clone()));
        let events = probe
            .apply(graziani(), Side::Axis, Order::TrySpot { piece: mover })
            .expect("the mover may try to spot it");
        assert!(events.iter().any(|e| matches!(e, Event::SpotTried { .. })));

        let mut patrol = golden_prefix(0);
        for logged in &golden_log().orders {
            let side = patrol.to_act();
            if patrol.clock.phase == cna_game::clock::Phase::Patrol
                && side.is_some_and(|s| !patrol.patrol_options(graziani(), s).is_empty())
            {
                break;
            }
            patrol
                .apply(graziani(), logged.player, logged.order.clone())
                .unwrap();
        }
        let side = patrol.to_act().expect("someone patrols");
        let options = patrol.patrol_options(graziani(), side);
        assert!(
            !options.is_empty(),
            "the golden game reaches a phase L with a patrol to send"
        );
        let mut probe = patrol.clone();
        probe
            .apply(
                graziani(),
                side,
                Order::Patrol {
                    sources: vec![(options[0].piece.clone(), 1)],
                    target: options[0].targets[0].clone(),
                    exclude: vec![],
                },
            )
            .expect("an offered patrol goes out");

        // The golden set-up, then its orders up to the first phase E,
        // GT2/1 (Graziani's `first_sail`), with the ships in Alexandria
        // (30.1x, 30.59).
        let mut fleet = golden_prefix(56);
        for logged in &golden_log().orders[56..] {
            if fleet.clock.phase == cna_game::clock::Phase::CwFleet {
                break;
            }
            fleet
                .apply(graziani(), logged.player, logged.order.clone())
                .unwrap();
        }
        assert_eq!(fleet.clock.phase, cna_game::clock::Phase::CwFleet);
        assert_eq!(fleet.to_act(), Some(Side::Cw));
        assert!(fleet.station_options(graziani(), Side::Cw).is_some());

        // The golden barrage step at Sollum with Valiant stationed on an
        // Axis unit's coastal hex (30.2x): the fleet fixture's ship,
        // placed by hand, since the golden game sails none.
        let mut fleet_barrage = (0..golden_log().orders.len())
            .map(golden_prefix)
            .find(|s| {
                matches!(
                    s.pending,
                    Some(Pending::Sealed {
                        step: cna_game::combat::SealedStep::Barrage,
                        ..
                    })
                )
            })
            .expect("the golden game barrages");
        let valiant = cna_game::fleet::ship_id("valiant");
        let coast = fleet_barrage
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == cna_game::piece::PieceKind::Combat)
            .filter_map(|p| p.hex().cloned())
            .find(|h| graziani().map.hex(h).is_some_and(|x| x.coastal))
            .expect("an Axis unit on the coast");
        let mut ship = fleet.pieces[&valiant].clone();
        ship.location = cna_game::piece::Location::Hex(coast);
        fleet_barrage.modules.fleet = true;
        fleet_barrage.pieces.insert(valiant.clone(), ship);
        fleet_barrage
            .fleet
            .insert(valiant.clone(), cna_game::fleet::PortRecord::default());
        if fleet_barrage.to_act() == Some(Side::Axis) {
            fleet_barrage
                .apply(
                    graziani(),
                    Side::Axis,
                    cna_game::order::Order::Barrage { fires: vec![] },
                )
                .unwrap();
        }
        assert_eq!(fleet_barrage.to_act(), Some(Side::Cw));
        match fleet_barrage.sealed_options(graziani(), Side::Cw).unwrap() {
            cna_game::combat::options::SealedOptions::Barrage { firers } => {
                assert!(firers.iter().any(|f| f.piece == valiant), "{firers:?}")
            }
            o => panic!("{o:?}"),
        }

        vec![
            ("set-up-done", set_up_done),
            ("fleet", fleet),
            ("fleet-barrage", fleet_barrage),
            ("patrol", patrol),
            ("crusader-set-up", crusader_set_up),
            ("raid-rommel", raid),
            ("spot-raider", spot),
            ("reserve", reserve),
            ("reserve-release", reserve_release),
            ("off-map", off_map),
            ("declare-zoc", declare_zoc),
            ("combat-start", combat_start),
            ("retreat-before-assault", retreat_before_assault),
            ("forced-retreat", forced_retreat),
            ("rail", rail),
            ("repair", repair),
            ("tow", tow),
            ("destroyed-repair", destroyed_repair),
            ("game-over", game_over),
        ]
    }

    /// The states the Playwright specs end at, as the SHA-256 of the
    /// state JSON after that many golden orders: set-up (slice 3), the
    /// movement stretch (slice 4), the combat stretch (slice 5) and the
    /// whole game (slice 6), which `cna_game::golden` pins too.
    const HASHED: [(&str, usize); 4] = [
        ("set-up-done", 56),
        ("movement-done", 64),
        ("combat-done", 78),
        ("golden-done", 505),
    ];

    /// The Playwright specs start from `web/tests/fixtures/<name>.json`
    /// and end at states pinned by hash; regenerate all of them with
    /// `cargo test -p cna-web -- --ignored regenerate_e2e_fixtures` after
    /// a deliberate change to the rules or the golden game.
    #[test]
    fn the_e2e_fixtures_are_current() {
        for (name, state) in e2e_fixtures() {
            let pinned = std::fs::read_to_string(fixtures_dir().join(format!("{name}.json")))
                .unwrap_or_else(|e| panic!("{name}.json: {e}"));
            assert_eq!(pinned, state.to_json().unwrap(), "{name}.json");
        }
        assert_eq!(
            HASHED[3].1,
            golden_log().orders.len(),
            "golden-done is the whole game"
        );
        for (name, n) in HASHED {
            let pinned =
                std::fs::read_to_string(fixtures_dir().join(format!("{name}.sha256"))).unwrap();
            assert_eq!(
                pinned.trim(),
                sha256(&golden_prefix(n).to_json().unwrap()),
                "{name}.sha256"
            );
        }
    }

    #[test]
    #[ignore]
    fn regenerate_e2e_fixtures() {
        for (name, state) in e2e_fixtures() {
            std::fs::write(
                fixtures_dir().join(format!("{name}.json")),
                state.to_json().unwrap(),
            )
            .unwrap();
        }
        for (name, n) in HASHED {
            std::fs::write(
                fixtures_dir().join(format!("{name}.sha256")),
                sha256(&golden_prefix(n).to_json().unwrap()) + "\n",
            )
            .unwrap();
        }
    }

    #[test]
    fn the_scripted_set_up_through_apply_matches_replay() {
        let log = setup_log();
        assert_eq!(log.orders.len(), 76);
        // The scripted set-up predates the module (it stacks supply units
        // past 32.1x's cap), so it plays without it.
        let mut game = Game {
            data: graziani(),
            state: GameState::new(graziani(), log.seed).unwrap(),
        };
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
        let replayed = replay(graziani(), &log).unwrap();
        assert_eq!(game.save().unwrap(), replayed.to_json().unwrap());
        let view = game.view().unwrap();
        assert!(view["pending"].is_null(), "{}", view["pending"]);
        assert_eq!(view["to_act"], "axis");
    }

    /// Golden order 68 is the second side's sealed barrage plot, which
    /// resolves the barrages with dice: its preview must not show them,
    /// though `apply` still rolls them.
    #[test]
    fn preview_of_an_order_that_rolls_dice_hides_its_events() {
        // The golden game's first order outside a sealed step whose apply
        // rolls dice.
        let (_, log) = golden_before(0);
        let mut live = Game::with_modules(SCENARIO, log.seed, log.modules).unwrap();
        live.state.ruleset = log.ruleset;
        let n = log
            .orders
            .iter()
            .position(|o| {
                let sealed = matches!(live.state.pending, Some(Pending::Sealed { .. }));
                let before = live.state.rng.draws();
                live.state
                    .apply(live.data, o.player, o.order.clone())
                    .unwrap();
                !sealed && live.state.rng.draws() != before
            })
            .unwrap();
        let (game, _) = golden_before(n);
        let o = &log.orders[n];
        let side = serde_json::to_value(o.player).unwrap();
        let side = side.as_str().unwrap();
        let order = serde_json::to_string(&o.order).unwrap();
        let saved = game.save().unwrap();
        assert_eq!(
            game.preview(side, &order).unwrap(),
            json!({ "hidden": "dice" })
        );
        assert_eq!(game.save().unwrap(), saved, "a preview changes nothing");
    }

    #[test]
    fn preview_during_a_sealed_step_reveals_no_answer() {
        // Order 68: the Commonwealth's barrage plot, which completes the
        // step and would show the Axis plot's results.
        let (game, log) = golden_before(67);
        let o = &log.orders[67];
        assert!(matches!(game.state.pending, Some(Pending::Sealed { .. })));
        let side = serde_json::to_value(o.player).unwrap();
        let order = serde_json::to_string(&o.order).unwrap();
        assert_eq!(
            game.preview(side.as_str().unwrap(), &order).unwrap(),
            json!({ "hidden": "sealed" })
        );
    }
}
