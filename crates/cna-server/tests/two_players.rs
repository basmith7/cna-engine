//! The state hash the two-player Playwright run (`web/tests/multiplayer/`)
//! must reach: the golden game's orders 0–76 replayed on a lobby game's
//! setting (its seed, every module on, the default ruleset).

use cna_game::log::{Log, replay};
use cna_rules::ruleset::Ruleset;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// How many of the golden game's orders the two players play.
const ORDERS: usize = 77;

fn pin() -> PathBuf {
    cna_data::repo_root().join("web/tests/fixtures/two-players-done.sha256")
}

fn hash() -> String {
    let root = cna_data::repo_root();
    let mut log: Log = serde_json::from_str(
        &std::fs::read_to_string(root.join("tests/games/graziani-m5.json")).unwrap(),
    )
    .unwrap();
    // A lobby game plays the default ruleset, as `insert_game` makes it.
    assert_eq!(log.ruleset, Ruleset::default());
    log.orders.truncate(ORDERS);
    let data = cna_embed::data(&log.scenario).unwrap();
    let json = replay(data, &log).unwrap().to_json().unwrap();
    format!("{:x}", Sha256::digest(json.as_bytes()))
}

#[test]
fn the_two_player_pin_is_replays_hash() {
    assert_eq!(
        hash(),
        std::fs::read_to_string(pin()).unwrap().trim(),
        "regenerate with `cargo test -p cna-server --test two_players -- --ignored regenerate_`"
    );
}

#[test]
#[ignore = "rewrites the pin"]
fn regenerate_two_player_pin() {
    std::fs::write(pin(), format!("{}\n", hash())).unwrap();
}
