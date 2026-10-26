//! The `Game` bindings as JavaScript sees them, run under Node by
//! `wasm-pack test --node crates/cna-web`: the real proof that the
//! embedded data loads and the engine runs on wasm32.

#![cfg(target_arch = "wasm32")]

use cna_web::Game;
use wasm_bindgen_test::*;

const PLACE: &str = r#"{"place":{"piece":"supply:axis:16","hex":"C0101"}}"#;

fn parse(json: &str) -> serde_json::Value {
    serde_json::from_str(json).unwrap()
}

#[wasm_bindgen_test]
fn a_new_game_waits_on_the_axis() {
    let game = Game::new_game("grazianis-offensive", 7).unwrap();
    let view = parse(&game.view().unwrap());
    assert_eq!(view["to_act"], "axis");
    assert_eq!(view["pending"]["set-up"]["side"], "axis");
    assert_eq!(view["rules_commit"], Game::rules_commit());
}

#[wasm_bindgen_test]
fn placements_of_a_pending_piece() {
    let game = Game::new_game("grazianis-offensive", 7).unwrap();
    let hexes = parse(&game.placements("supply:axis:16").unwrap());
    assert!(
        hexes
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("C0101"))
    );
    assert_eq!(
        parse(&game.placements("supply:cw:7").unwrap()),
        serde_json::json!([])
    );
}

#[wasm_bindgen_test]
fn legal_and_illegal_orders() {
    let mut game = Game::new_game("grazianis-offensive", 7).unwrap();
    let before = game.save().unwrap();
    let r = parse(
        &game
            .apply(
                "axis",
                r#"{"place":{"piece":"supply:axis:16","hex":"E0101"}}"#,
            )
            .unwrap(),
    );
    assert_eq!(r["illegal"]["case"], "59.2");
    assert_eq!(game.save().unwrap(), before);
    let r = parse(&game.apply("axis", PLACE).unwrap());
    assert!(r["ok"].is_array());
    assert!(game.apply("nobody", PLACE).is_err());
}

#[wasm_bindgen_test]
fn save_and_load_round_trip() {
    let mut game = Game::new_game("grazianis-offensive", 7).unwrap();
    game.apply("axis", PLACE).unwrap();
    let saved = game.save().unwrap();
    let loaded = Game::load(&saved).unwrap();
    assert_eq!(loaded.save().unwrap(), saved);
    assert_eq!(loaded.view().unwrap(), game.view().unwrap());
    assert!(Game::load("{}").is_err());
}

#[wasm_bindgen_test]
fn a_malformed_save_is_an_error_string_not_a_dead_page() {
    // A sealed step with nobody waiting: `to_act` would have panicked
    // on it before `from_json` checked invariants (slice 6).
    let game = Game::new_game("grazianis-offensive", 7).unwrap();
    let mut save: serde_json::Value = parse(&game.save().unwrap());
    save["pending"] = serde_json::json!({"sealed": {"step": "barrage", "waiting": []}});
    let err = Game::load(&save.to_string()).err().expect("refused");
    let shown = format!("{:?}", wasm_bindgen::JsValue::from(err));
    assert!(shown.contains("nobody waiting"), "{shown}");
    let mut save: serde_json::Value = parse(&game.save().unwrap());
    save["clock"]["stage"] = serde_json::json!(9);
    let err = Game::load(&save.to_string()).err().expect("refused");
    let shown = format!("{:?}", wasm_bindgen::JsValue::from(err));
    assert!(shown.contains("OpStage"), "{shown}");
}

#[wasm_bindgen_test]
fn reachable_answers_like_apply() {
    let game = Game::new_game("grazianis-offensive", 7).unwrap();
    // Set-up is pending: no move yet, with the case `apply` would give.
    let r = parse(
        &game
            .reachable("axis", r#"["unit:it:ixl-tank-bn"]"#, "move")
            .unwrap(),
    );
    assert_eq!(r["illegal"]["case"], "59.2");
    assert!(game.reachable("axis", "not json", "move").is_err());
    assert!(game.reachable("axis", "[]", "stroll").is_err());
}
