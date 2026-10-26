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
use cna_game::combat::SealedStep;
use cna_game::data::Data;
use cna_game::event::Event;
use cna_game::order::Order;
use cna_game::piece::PieceId;
use cna_game::reach::Purpose;
use cna_game::state::{GameState, Modules, Pending};
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

    /// What the UI draws: the state minus `log` and `rng`, plus `to_act`
    /// and, per piece, `unit_type` from the characteristics chart (null
    /// for supply units and ID codes the chart lacks) and the OA unit's
    /// `name` and `abbreviation` (null for supply units), and `shell: true`
    /// on a formation that is a shell (9.26);
    /// `assaults_to_resolve`, the hexes the phasing player still has to
    /// assault in the close assault step. While a sealed step is
    /// pending, the answers already given to it by sides other than
    /// `to_act` are redacted (spec, *Secrecy*): their `combat.plots` or
    /// `combat.assignments` entry, and for gun positions their pieces in
    /// the shared `combat.forward` set.
    pub fn view(&self) -> Result<Value> {
        let mut v = serde_json::to_value(&self.state)?;
        let obj = v.as_object_mut().context("state is an object")?;
        obj.remove("log");
        obj.remove("rng");
        obj.insert("to_act".into(), json!(self.state.to_act()));
        obj.insert(
            "assaults_to_resolve".into(),
            json!(self.state.assaults_to_resolve()),
        );
        if let Some(Pending::Sealed { step, .. }) = &self.state.pending
            && let Some(acting) = self.state.to_act()
        {
            let combat = v["combat"].as_object_mut().context("combat")?;
            let others: Vec<String> = [Side::Axis, Side::Cw]
                .into_iter()
                .filter(|s| *s != acting)
                .map(|s| serde_json::to_value(s).map(|x| x.as_str().unwrap_or_default().to_owned()))
                .collect::<Result<_, _>>()?;
            let key = match step {
                SealedStep::GunPositions => None,
                SealedStep::Barrage => Some("plots"),
                SealedStep::AssignForces => Some("assignments"),
            };
            match key {
                Some(key) => {
                    if let Some(map) = combat.get_mut(key).and_then(Value::as_object_mut) {
                        for s in &others {
                            map.remove(s);
                        }
                    }
                }
                None => {
                    if let Some(forward) = combat.get_mut("forward").and_then(Value::as_array_mut) {
                        forward.retain(|id| {
                            self.state
                                .pieces
                                .values()
                                .find(|p| Some(p.id.as_str()) == id.as_str())
                                .is_some_and(|p| p.side == acting)
                        });
                    }
                }
            }
        }
        // 3.61: loads, motorisation points (pooled or attached),
        // repaired points waiting as replacements and what a project
        // builds are the owner's secret.
        if let Some(acting) = self.state.to_act() {
            let other = serde_json::to_value(acting.other())?
                .as_str()
                .unwrap_or_default()
                .to_owned();
            for key in ["motorisation", "repaired"] {
                if let Some(m) = v.get_mut(key).and_then(Value::as_object_mut) {
                    m.remove(&other);
                }
            }
            // Whether the other side's unrevealed minefields are real (26.1x).
            if let Some(m) = v.get_mut("minefields").and_then(Value::as_object_mut) {
                for field in m.values_mut().filter_map(Value::as_object_mut) {
                    let theirs = field.get("side").and_then(Value::as_str) == Some(other.as_str());
                    let revealed = field.get("revealed").and_then(Value::as_bool) == Some(true);
                    if theirs && !revealed {
                        field.remove("real");
                    }
                }
            }
            // What the other side builds (3.61); its projects still show.
            if let Some(m) = v.get_mut("projects").and_then(Value::as_object_mut) {
                for project in m.values_mut().filter_map(Value::as_object_mut) {
                    if project.get("side").and_then(Value::as_str) == Some(other.as_str()) {
                        project.remove("item");
                    }
                }
            }
            if let Some(m) = v
                .get_mut("pending_motorisation")
                .and_then(Value::as_object_mut)
            {
                m.retain(|id, _| {
                    self.state
                        .pieces
                        .values()
                        .any(|p| p.id.as_str() == id && p.side == acting)
                });
            }
            for p in self.state.pieces.values().filter(|p| p.side != acting) {
                if let Some(o) = v["pieces"][p.id.as_str()].as_object_mut() {
                    o.remove("load");
                    o.remove("motorised");
                    o.remove("escorted");
                }
            }
        }
        let labels: Vec<(String, Value)> = self
            .state
            .pieces
            .values()
            .map(|p| {
                let unit_type = p
                    .nation
                    .zip(p.id_code.as_deref())
                    .and_then(|(nation, code)| self.data.chars.row(nation, code))
                    .map(|row| row.unit_type.clone());
                let oa = p.id.unit().and_then(|id| self.data.oa.unit(&id));
                let labels = json!({
                    "unit_type": unit_type,
                    "name": oa.map(|u| u.name.clone()),
                    "abbreviation": oa.and_then(|u| u.abbreviation.clone()),
                });
                (p.id.as_str().to_owned(), labels)
            })
            .collect();
        // Shells (9.26) are worked out from what both sides see (attachments
        // and TOE are not redacted), so every piece carries the flag.
        let shells: Vec<String> = self
            .state
            .pieces
            .values()
            .filter(|p| self.state.is_shell(self.data, &p.id))
            .map(|p| p.id.as_str().to_owned())
            .collect();
        for id in shells {
            v["pieces"][&id]["shell"] = json!(true);
        }
        for (id, labels) in labels {
            for (k, val) in labels.as_object().context("labels")? {
                v["pieces"][&id][k] = val.clone();
            }
        }
        Ok(v)
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

    /// What `apply` would answer `order` from `side` with, on a copy:
    /// the same `{"ok": [Event]}` or `{"illegal": ...}`, the game
    /// untouched. Dice included, so the events are exactly the order's.
    pub fn preview(&self, side: &str, order: &str) -> Result<Value> {
        let side = Self::side(side)?;
        let order: Order = serde_json::from_str(order).context("parsing the order")?;
        Ok(Self::answer(
            self.state.clone().apply(self.data, side, order),
        ))
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
    use crate::embedded::{DEFAULT_SCENARIO as SCENARIO, RULES_COMMIT};

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

    /// Golden orders 66–72 are the sealed steps: after the Axis has
    /// answered (67, 69, 72 are the Commonwealth's turns) its answer is
    /// not in the Commonwealth's view; once both are in, both are.
    #[test]
    fn a_sealed_answer_is_redacted_from_the_other_side_s_view_until_revealed() {
        let axis_gun = json!("unit:it:1st-libyan-artillery-regt");
        // 67: the Axis gun stands forward; the Commonwealth sees no forward gun.
        let (game, _) = golden_before(66);
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
        // 68: revealed, both sides' positions are in the view.
        let (game, _) = golden_before(67);
        let view = game.view().unwrap();
        assert!(
            view["combat"]["forward"]
                .as_array()
                .unwrap()
                .contains(&axis_gun),
            "{}",
            view["combat"]
        );
        // 69: the Axis barrage is plotted; the Commonwealth sees no plot.
        let (game, _) = golden_before(68);
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
        // 70: revealed.
        let (game, _) = golden_before(69);
        let view = game.view().unwrap();
        assert_eq!(
            view["combat"]["plots"]["axis"][0]["target"],
            "unit:cw:1-krrc"
        );
        assert!(view["combat"]["plots"]["cw"].as_array().unwrap().is_empty());
        // 72: the Axis assignments are in; the Commonwealth sees none.
        let (game, _) = golden_before(71);
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
        // 73: revealed, and the close assault step waits on the Axis
        // with C3922 to assault.
        let (game, _) = golden_before(72);
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
        let (game, _) = golden_before(73);
        assert_eq!(game.view().unwrap()["assaults_to_resolve"], json!([]));
    }

    #[test]
    fn sealed_options_name_the_golden_barrage_and_nothing_at_other_moments() {
        let (game, _) = golden_before(67);
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
        let (game, _) = golden_before(69);
        assert_eq!(
            game.sealed_options("cw").unwrap()["illegal"]["case"],
            "11.0"
        );
        assert!(game.sealed_options("allies").is_err());
    }

    /// Golden order 70 is the Commonwealth's pass in retreat before
    /// assault: the query names where 4 RHA at C3923 could have gone
    /// instead, with `short` 0 throughout.
    #[test]
    fn reachable_for_a_retreat_before_assault_at_golden_order_70() {
        let (game, _) = golden_before(69);
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
            json!({ "retreat-bought-out": { "pieces": ["unit:cw:1-krrc", "unit:cw:3-cldgds", "unit:cw:7-med"], "short": 2 } }),
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
        for (slug, _) in crate::embedded::scenarios().unwrap() {
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
            use cna_game::combat::SealedStep;
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

        let combat_start = golden_prefix(65);
        assert_eq!(combat_start.to_act(), Some(Side::Axis));
        assert!(matches!(combat_start.pending, Some(Pending::Sealed { .. })));

        let retreat_before_assault = golden_prefix(69);
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
        let mut forced_retreat = golden_prefix(71);
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
                golden_log().orders[72].order.clone(),
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
        assert!(
            forced_retreat
                .reachable(graziani(), Side::Cw, pieces, Purpose::ForcedRetreat)
                .unwrap()
                .iter()
                .any(|r| r.short == 0),
            "the full retreat can be made"
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
        // Matruh may ride the railway east (golden order 86 passes).
        let rail = golden_prefix(86);
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
            }],
        );
        repair.apply(graziani(), Side::Cw, Order::Pass).unwrap();
        assert_eq!(repair.clock.phase, cna_game::clock::Phase::Repair);
        assert_eq!(repair.to_act(), Some(Side::Cw));
        assert_eq!(repair.field_repairs(graziani()).len(), 1);

        let game_over = replay(graziani(), &golden_log()).unwrap();
        assert!(game_over.is_over());
        assert_eq!(game_over.to_act(), None);

        vec![
            ("set-up-done", set_up_done),
            ("reserve", reserve),
            ("reserve-release", reserve_release),
            ("off-map", off_map),
            ("declare-zoc", declare_zoc),
            ("combat-start", combat_start),
            ("retreat-before-assault", retreat_before_assault),
            ("forced-retreat", forced_retreat),
            ("rail", rail),
            ("repair", repair),
            ("game-over", game_over),
        ]
    }

    /// The states the Playwright specs end at, as the SHA-256 of the
    /// state JSON after that many golden orders: set-up (slice 3), the
    /// movement stretch (slice 4), the combat stretch (slice 5) and the
    /// whole game (slice 6), which `cna_game::golden` pins too.
    const HASHED: [(&str, usize); 4] = [
        ("set-up-done", 56),
        ("movement-done", 65),
        ("combat-done", 79),
        ("golden-done", 459),
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
}
