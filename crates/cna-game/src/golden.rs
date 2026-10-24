//! The golden game (spec, *What "done" means* 2): a scripted Graziani's
//! Offensive played to its end and pinned. `tests/games/graziani-short.json`
//! is its log; the test checks the script still produces that log, that
//! the log replays to the same state, and that the final state's SHA-256
//! and victory report are what they were. Regenerate the fixture with
//! `cargo test -p cna-game -- --ignored regenerate_graziani_short_fixture`
//! after a deliberate change to the rules or the script, and pin the new
//! hash it prints.

#[cfg(test)]
mod tests {
    use crate::clock::Clock;
    use crate::data::Data;
    use crate::event::Event;
    use crate::log::{Log, replay};
    use crate::order::{Allocation, Assignment, Fire, Order};
    use crate::piece::PieceId;
    use crate::state::test_support::{auto_place, pass_stage};
    use crate::state::{GameState, Modules};
    use cna_data::map::HexId;
    use cna_data::scenario::{Side, VictoryLevel};
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;

    /// SHA-256 of the final state's JSON (`GameState::to_json`).
    const FINAL_STATE_SHA256: &str =
        "4f74be58169d973ad7e7b4de437975a9280faf0b8097859583c479dd0e4264e8";

    /// The same final state's hash with the abstract supply module on.
    const M5_FINAL_STATE_SHA256: &str =
        "3e4ab3fa635d7897c403b9691f4a1e3c072b3ea8d248268059166a01ac81391e";

    fn fixture() -> PathBuf {
        cna_data::repo_root().join("tests/games/graziani-short.json")
    }

    fn m5_fixture() -> PathBuf {
        cna_data::repo_root().join("tests/games/graziani-m5.json")
    }

    fn pid(s: &str) -> PieceId {
        PieceId::from(cna_data::oa::UnitId::parse(s).unwrap())
    }

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    fn stack(state: &GameState, h: &str) -> Vec<PieceId> {
        state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex(h)))
            .map(|p| p.id.clone())
            .collect()
    }

    fn attack(piece: &str, target: &str, points: u32) -> Assignment {
        Assignment {
            piece: pid(piece),
            anti_armour: vec![],
            close_assault: vec![Allocation {
                target: Some(hex(target)),
                points,
            }],
        }
    }

    fn defend(piece: &str, points: u32) -> Assignment {
        Assignment {
            piece: pid(piece),
            anti_armour: vec![],
            close_assault: vec![Allocation {
                target: None,
                points,
            }],
        }
    }

    /// Applies the orders in turn; every one must be legal.
    fn play(data: &Data, state: &mut GameState, orders: Vec<(Side, Order)>) -> Vec<Event> {
        let mut events = Vec::new();
        for (side, order) in orders {
            assert_eq!(state.to_act(), Some(side), "at {}: {order:?}", state.clock);
            let ev = state
                .apply(data, side, order.clone())
                .unwrap_or_else(|e| panic!("at {}: {order:?}: {e}", state.clock));
            events.extend(ev);
        }
        events
    }

    /// Phase C with the abstract supply module: both players pass.
    fn organise(data: &Data, state: &mut GameState) {
        while state.clock.phase == crate::clock::Phase::Organisation {
            let side = state.to_act().unwrap();
            state.apply(data, side, Order::Pass).unwrap();
        }
    }

    /// The script. GT1 OpStage 1: the 1st Libyan Division goes up the
    /// coast road from C4020 to Sollum (C4021), 4 RHA reacts out of
    /// Halfaya Pass (C3922) to C3923, the 2nd Libyan Division closes up to
    /// C4020; the Italian artillery barrages 1 KRRC and four Libyan
    /// battalions assault Halfaya Pass. In the Commonwealth half 4 RHA
    /// returns and 7 Medium barrages a Libyan battalion. GT1 OpStage 2:
    /// the 62nd Marmarica Division follows the road to C3920 and the
    /// artillery barrages the Coldstream Guards. Then both sides pass to
    /// the end of Game-Turn 6.
    fn script(data: &Data, modules: Modules) -> GameState {
        let mut state = GameState::with_modules(data, 7, modules).unwrap();
        auto_place(data, &mut state);
        let div1 = stack(&state, "C4020");
        let div2 = stack(&state, "C3920");
        let marmarica = stack(&state, "C3918");
        use Side::{Axis, Cw};
        let fire = |piece: &str, target: &str| Order::Barrage {
            fires: vec![Fire {
                piece: pid(piece),
                points: 6,
                target: pid(target),
            }],
        };
        let none = |pieces: Vec<PieceId>, path: &[&str], assault| Order::Move {
            pieces,
            path: path.iter().map(|h| hex(h)).collect(),
            assault,
            fuel_from: None,
        };
        // GT1 OpStage 1, Axis half.
        play(
            data,
            &mut state,
            vec![(Axis, Order::DeclareInitiative { player_a: Axis })],
        );
        organise(data, &mut state);
        play(
            data,
            &mut state,
            vec![
                (Axis, Order::Pass),
                (Axis, none(div1, &["C4021"], true)),
                (
                    Cw,
                    Order::React {
                        pieces: vec![pid("unit:cw:4-rha")],
                        path: vec![hex("C3923")],
                    },
                ),
                (Cw, Order::Pass),
                (Axis, none(div2, &["C4020"], false)),
                (Axis, Order::EndSegment),
                (
                    Axis,
                    Order::GunPositions {
                        forward: vec![pid("unit:it:1st-libyan-artillery-regt")],
                    },
                ),
                (Cw, Order::GunPositions { forward: vec![] }),
                (
                    Axis,
                    fire("unit:it:1st-libyan-artillery-regt", "unit:cw:1-krrc"),
                ),
                (Cw, Order::Barrage { fires: vec![] }),
                (Cw, Order::Pass),
                (
                    Axis,
                    Order::AssignForces {
                        assignments: vec![
                            attack("unit:it:viii-libyan-bn", "C3922", 5),
                            attack("unit:it:ix-libyan-bn", "C3922", 5),
                            attack("unit:it:x-libyan-bn", "C3922", 5),
                            attack("unit:it:xi-libyan-bn", "C3922", 5),
                        ],
                    },
                ),
                (
                    Cw,
                    Order::AssignForces {
                        assignments: vec![
                            defend("unit:cw:1-krrc", 6),
                            defend("unit:cw:3-cldgds", 6),
                        ],
                    },
                ),
                (
                    Axis,
                    Order::Assault {
                        target: hex("C3922"),
                    },
                ),
                (Axis, Order::Pass),
                (Axis, Order::Pass),
            ],
        );
        // GT1 OpStage 1, Commonwealth half. With the supply module on 4 RHA
        // (gun-class) owes fuel for the return and no supply unit is in
        // range (32.22), so the module-on script leaves it at C3923.
        // Likewise 7 Med's barrage owes 4 ammunition (32.21) and nothing
        // is in range, so the module-on script plots no fire.
        let shell = if modules.abstract_supply {
            Order::Barrage { fires: vec![] }
        } else {
            fire("unit:cw:7-med", "unit:it:ix-libyan-bn")
        };
        let mut back = vec![(Cw, Order::Pass)];
        if !modules.abstract_supply {
            back.push((Cw, none(vec![pid("unit:cw:4-rha")], &["C3922"], false)));
            back.push((Axis, Order::Pass));
        }
        play(
            data,
            &mut state,
            [
                back,
                vec![
                    (Cw, Order::EndSegment),
                    (
                        Cw,
                        Order::GunPositions {
                            forward: vec![pid("unit:cw:7-med")],
                        },
                    ),
                    (Axis, Order::GunPositions { forward: vec![] }),
                    (Cw, shell),
                    (Axis, Order::Barrage { fires: vec![] }),
                    (Axis, Order::Pass),
                    (
                        Cw,
                        Order::AssignForces {
                            assignments: vec![],
                        },
                    ),
                    (
                        Axis,
                        Order::AssignForces {
                            assignments: vec![],
                        },
                    ),
                    (Cw, Order::Pass),
                ],
            ]
            .concat(),
        );
        // GT1 OpStage 2, Axis half.
        play(
            data,
            &mut state,
            vec![(Axis, Order::DeclareInitiative { player_a: Axis })],
        );
        organise(data, &mut state);
        play(
            data,
            &mut state,
            vec![
                (Axis, Order::Pass),
                (Axis, none(marmarica, &["C3919", "C3920"], false)),
                (Axis, Order::EndSegment),
                (
                    Axis,
                    Order::GunPositions {
                        forward: vec![pid("unit:it:1st-libyan-artillery-regt")],
                    },
                ),
                (Cw, Order::GunPositions { forward: vec![] }),
                (
                    Axis,
                    fire("unit:it:1st-libyan-artillery-regt", "unit:cw:3-cldgds"),
                ),
                (Cw, Order::Barrage { fires: vec![] }),
                (Cw, Order::Pass),
                (
                    Axis,
                    Order::AssignForces {
                        assignments: vec![],
                    },
                ),
                (
                    Cw,
                    Order::AssignForces {
                        assignments: vec![],
                    },
                ),
                (Axis, Order::Pass),
            ],
        );
        // The Commonwealth half of OpStage 2 and everything after: passes.
        while !state.is_over() {
            pass_stage(data, &mut state);
        }
        state
    }

    fn sha256(text: &str) -> String {
        format!("{:x}", Sha256::digest(text.as_bytes()))
    }

    #[test]
    #[ignore]
    fn regenerate_graziani_short_fixture() {
        let data = Data::load("grazianis-offensive").unwrap();
        let state = script(&data, Modules::default());
        let json = serde_json::to_string_pretty(&state.to_log()).unwrap() + "\n";
        std::fs::write(fixture(), json).unwrap();
        println!("orders: {}", state.log.len());
        println!("FINAL_STATE_SHA256 = {}", sha256(&state.to_json().unwrap()));
        println!("{:#?}", state.over);
    }

    #[test]
    #[ignore]
    fn regenerate_graziani_m5_fixture() {
        let data = Data::load("grazianis-offensive").unwrap();
        let state = script(&data, Modules::ALL);
        let json = serde_json::to_string_pretty(&state.to_log()).unwrap() + "\n";
        std::fs::write(m5_fixture(), json).unwrap();
        println!(
            "M5_FINAL_STATE_SHA256 = {}",
            sha256(&state.to_json().unwrap())
        );
    }

    // The old orders with the module on (Mission 5): the same fight, the
    // same report, now with loads and motorisation in the state.
    #[test]
    fn the_golden_game_with_abstract_supply_replays_to_its_pinned_end() {
        let data = Data::load("grazianis-offensive").unwrap();
        let text = std::fs::read_to_string(m5_fixture()).unwrap();
        let log: Log = serde_json::from_str(&text).unwrap();
        assert!(log.modules.abstract_supply);
        let live = script(&data, Modules::ALL);
        assert_eq!(live.to_log(), log, "the fixture is stale: regenerate it");
        let replayed = replay(&data, &log).unwrap();
        let json = replayed.to_json().unwrap();
        assert_eq!(json, live.to_json().unwrap());
        assert_eq!(sha256(&json), M5_FINAL_STATE_SHA256);
        let old = script(&data, Modules::default());
        assert_eq!(replayed.over, old.over);
        assert!(replayed.pieces.values().any(|p| p.load.is_some()));
        assert!(!replayed.motorisation.is_empty());
    }

    #[test]
    fn the_golden_game_replays_to_its_pinned_end() {
        let data = Data::load("grazianis-offensive").unwrap();
        let text = std::fs::read_to_string(fixture()).unwrap();
        let log: Log = serde_json::from_str(&text).unwrap();
        let live = script(&data, Modules::default());
        assert_eq!(live.to_log(), log, "the fixture is stale: regenerate it");
        let replayed = replay(&data, &log).unwrap();
        let json = replayed.to_json().unwrap();
        assert_eq!(json, live.to_json().unwrap());
        assert_eq!(sha256(&json), FINAL_STATE_SHA256);
        assert_eq!(replayed.clock, Clock::start(7));
        assert_eq!(replayed.to_act(), None);
        // The scripted fight: 1 KRRC was barraged, Halfaya Pass assaulted
        // and held, the Libyans engaged below it.
        let moves = log
            .orders
            .iter()
            .filter(|o| matches!(o.order, Order::Move { .. } | Order::React { .. }))
            .count();
        assert_eq!(moves, 5);
        assert!(
            log.orders
                .iter()
                .any(|o| matches!(o.order, Order::Assault { .. }))
        );
        assert_eq!(
            replayed.pieces[&pid("unit:cw:1-krrc")].hex(),
            Some(&hex("C3922"))
        );
        assert_eq!(
            replayed.pieces[&pid("unit:it:ix-libyan-bn")].hex(),
            Some(&hex("C4021"))
        );
        // The report: the Italians hold Sollum, Fort Maddalena and Giarabub
        // but not Sidi Barrani; the Commonwealth holds Halfaya but not
        // Sollum. Neither reaches a level.
        let report = replayed.over.as_ref().unwrap();
        assert_eq!((report.axis, report.cw), (None, None));
        let met = |side, level, obj: &str| {
            report
                .checked
                .iter()
                .find(|c| c.side == side && c.level == level && c.objective == obj)
                .unwrap()
                .met
        };
        assert!(met(Side::Axis, VictoryLevel::Tactical, "sollum"));
        assert!(met(Side::Axis, VictoryLevel::Tactical, "fort-maddalena"));
        assert!(met(Side::Axis, VictoryLevel::Tactical, "giarabub"));
        assert!(!met(Side::Axis, VictoryLevel::Tactical, "sidi-barrani"));
        assert!(met(Side::Cw, VictoryLevel::Tactical, "C3922"));
        assert!(!met(Side::Cw, VictoryLevel::Tactical, "sollum"));
        assert!(report.unchecked.is_empty());
    }
}
