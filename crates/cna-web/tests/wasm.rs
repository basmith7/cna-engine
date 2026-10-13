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
    let game = Game::new_game(7).unwrap();
    let view = parse(&game.view().unwrap());
    assert_eq!(view["to_act"], "axis");
    assert_eq!(view["pending"]["set-up"]["side"], "axis");
    assert_eq!(view["rules_commit"], Game::rules_commit());
}

#[wasm_bindgen_test]
fn legal_and_illegal_orders() {
    let mut game = Game::new_game(7).unwrap();
    let before = game.save().unwrap();
    let r = parse(
        &game
            .apply("axis", r#"{"place":{"piece":"supply:axis:16","hex":"E0101"}}"#)
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
    let mut game = Game::new_game(7).unwrap();
    game.apply("axis", PLACE).unwrap();
    let saved = game.save().unwrap();
    let loaded = Game::load(&saved).unwrap();
    assert_eq!(loaded.save().unwrap(), saved);
    assert_eq!(loaded.view().unwrap(), game.view().unwrap());
    assert!(Game::load("{}").is_err());
}
