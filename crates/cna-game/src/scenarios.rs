//! Every scenario cna ships, not only Graziani's Offensive: each sets up
//! and plays its first Game-Turn with both sides passing (Mission 5's
//! second half).

/// Scenarios that do not set up yet, waiting on cna (Requests for cna
/// in `PROGRESS.md`); the browser offers every other one.
pub const WAITING: &[&str] = &[
    // The four Italian divisions go "between" A2010 and A2703, two
    // hexes apart, but the data gives only those two hexes.
    "rommels-arrival",
    "desert-fox-campaign",
    "short-campaign-game",
];

#[cfg(test)]
mod tests {
    use super::WAITING;
    use crate::data::Data;
    use crate::state::test_support::{auto_place, pass_stage};
    use crate::state::{GameState, Modules};

    fn slugs() -> Vec<String> {
        let dir = cna_data::repo_root().join("vendor/cna/data/scenarios");
        let mut slugs: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| {
                let name = e.unwrap().file_name().into_string().unwrap();
                name.strip_suffix(".json").unwrap().to_owned()
            })
            .collect();
        slugs.sort();
        slugs
    }

    #[test]
    fn every_scenario_sets_up_and_plays_its_first_turn() {
        let slugs = slugs();
        assert!(slugs.len() >= 9, "{slugs:?}");
        let mut failed = Vec::new();
        for slug in slugs.iter().filter(|s| !WAITING.contains(&s.as_str())) {
            let run = std::panic::catch_unwind(|| {
                let data = Data::load(slug).unwrap_or_else(|e| panic!("{e:#}"));
                let mut state = GameState::with_modules(&data, 1, Modules::ALL)
                    .unwrap_or_else(|e| panic!("{e:#}"));
                auto_place(&data, &mut state);
                let start = state.clock.game_turn;
                while !state.is_over() && state.clock.game_turn == start {
                    pass_stage(&data, &mut state);
                }
            });
            if run.is_err() {
                failed.push(slug.clone());
            }
        }
        assert!(failed.is_empty(), "{failed:?}");
    }
}
