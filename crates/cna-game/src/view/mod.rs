//! What each side may know (3.61, limited intelligence): the engine, not a
//! host, decides it, so the browser and the server show the same thing.

mod aliases;
mod events;
mod leaks;
mod state;

pub use aliases::Aliases;
pub use events::events_for;
pub use leaks::{leaks, refusal_for};
pub use state::{ENEMY_PIECE_KEEPS, ENEMY_PIECE_REMOVES, as_known_to, view_for};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;
    use crate::log::Log;
    use crate::state::GameState;
    use cna_data::scenario::Side;
    use serde_json::Value;

    /// Replays a scripted game order by order; after each, neither side's
    /// view nor its events say what 3.61 hides about the other, and each order tried by
    /// the wrong side is refused in words that say nothing either.
    fn check(name: &str) {
        let data = Data::load("grazianis-offensive").unwrap();
        let path = cna_data::repo_root().join("tests/games").join(name);
        let log: Log = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let aliases = Aliases::new([5; 32]);
        let mut s = GameState::with_modules(&data, log.seed, log.modules).unwrap();
        s.ruleset = log.ruleset;
        for (i, o) in log.orders.iter().enumerate() {
            let wrong = o.player.other();
            let refused = s.clone().apply(&data, wrong, o.order.clone());
            if let Err(illegal) = refused {
                let r = refusal_for(&aliases, Some(wrong), &illegal, false);
                assert_eq!(
                    leaks(&s, &r, o.player),
                    Vec::<String>::new(),
                    "{name} order {i}"
                );
            }
            let events = s.apply(&data, o.player, o.order.clone()).unwrap();
            for viewer in [Side::Axis, Side::Cw] {
                // A finished game's view hides nothing; its events still do.
                if !s.is_over() {
                    let v = view_for(&s, &data, &aliases, Some(viewer), false).unwrap();
                    let found = leaks(&s, &v, viewer.other());
                    assert!(
                        found.is_empty(),
                        "{name} after order {i}, {viewer} sees {found:?}"
                    );
                }
                let e = Value::from(events_for(&s, &aliases, &events, Some(viewer), false));
                let found = leaks(&s, &e, viewer.other());
                assert!(
                    found.is_empty(),
                    "{name} order {i}'s events, {viewer} sees {found:?}"
                );
            }
        }
    }

    /// The module-on golden game: the short one's orders with supply,
    /// dummies and motorisation in the state, so it covers more.
    #[test]
    fn no_side_learns_what_3_61_hides() {
        check("graziani-m5.json");
    }

    #[test]
    fn no_side_learns_what_3_61_hides_during_set_up() {
        check("setup.json");
    }
}
