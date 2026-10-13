//! Determinism (spec, *Testing*): the same seed and orders give the same
//! state on every run. A seeded generator plays one OpStage of random
//! legal moves by both players; the log then replays to the same state.

#[cfg(test)]
mod tests {
    use crate::clock::{Phase, Segment};
    use crate::data::Data;
    use crate::log::replay;
    use crate::order::Order;
    use crate::piece::PieceId;
    use crate::state::test_support::{drain_combat, set_up};
    use crate::state::{GameState, Pending};
    use cna_data::map::HexId;
    use cna_data::scenario::Side;
    use cna_rules::movement::{Step, step_cost};
    use rand_chacha::ChaCha8Rng;
    use rand_core::{RngCore, SeedableRng};

    fn pick<T>(rng: &mut ChaCha8Rng, items: &[T]) -> Option<T>
    where
        T: Clone,
    {
        if items.is_empty() {
            return None;
        }
        let i = (rng.next_u64() % items.len() as u64) as usize;
        Some(items[i].clone())
    }

    /// The hexes one piece could step into from where it stands, on the
    /// chart alone (8.3x); the engine decides the rest.
    fn steps(data: &Data, state: &GameState, id: &PieceId) -> Vec<HexId> {
        let piece = &state.pieces[id];
        let Some(from) = piece.hex() else {
            return Vec::new();
        };
        let map = &data.map;
        let here = map.hex(from).unwrap();
        let mobility = state.mobility(data, piece);
        map.neighbours(from)
            .into_iter()
            .filter(|n| {
                let Some(to) = map.hex(n) else { return false };
                let hexside = map.hexside(from, n);
                let step = Step {
                    from: here,
                    to,
                    hexside,
                    climbs: hexside.is_some_and(|hs| hs.climbs(from, n)),
                    weather: state.weather_at(n),
                };
                step_cost(&data.terrain, mobility, &step).is_ok()
            })
            .collect()
    }

    /// Tries a random single-piece step for the phasing player; true if
    /// one was accepted. A step is applied to a copy first so that a
    /// stack over the ceiling (9.14) is never left for `EndSegment`.
    fn random_step(data: &Data, state: &mut GameState, rng: &mut ChaCha8Rng, side: Side) -> bool {
        let ids: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == side && p.hex().is_some())
            .map(|p| p.id.clone())
            .collect();
        let Some(id) = pick(rng, &ids) else {
            return false;
        };
        let Some(to) = pick(rng, &steps(data, state, &id)) else {
            return false;
        };
        let order = Order::Move {
            pieces: vec![id],
            path: vec![to.clone()],
            assault: false,
        };
        let mut trial = state.clone();
        if trial.apply(data, side, order.clone()).is_err()
            || trial.over_ceiling_at(data, &to).is_some()
        {
            return false;
        }
        state.apply(data, side, order).unwrap();
        true
    }

    /// Answers whatever the non-phasing player owes mid-move: ZOC
    /// declarations at random (10.16), reactions declined.
    fn answer_pending(data: &Data, state: &mut GameState, rng: &mut ChaCha8Rng) {
        while let Some(p) = state.pending.clone() {
            let side = p.side();
            let order = match p {
                Pending::DeclareZoc { hexes, .. } => Order::DeclareZoc {
                    exerting: hexes
                        .into_iter()
                        .filter(|_| rng.next_u32() & 1 == 0)
                        .collect(),
                },
                Pending::Reaction { .. } => Order::Pass,
                _ => return,
            };
            state.apply(data, side, order).unwrap();
        }
    }

    /// One OpStage: both halves of phase G filled with random moves, the
    /// combat steps answered with nothing.
    fn random_stage(data: &Data, seed: u64, moves: usize) -> GameState {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut state = set_up(data, seed);
        let (gt, stage) = (state.clock.game_turn, state.clock.stage);
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        while (state.clock.game_turn, state.clock.stage) == (gt, stage) {
            let side = state.to_act().unwrap();
            match (state.clock.phase, state.clock.segment) {
                (Phase::ReserveDesignation, _) => {
                    state.apply(data, side, Order::Pass).unwrap();
                }
                (Phase::MovementCombat, Some(Segment::Movement)) => {
                    let mut accepted = 0;
                    let mut tries = 0;
                    while accepted < moves && tries < moves * 20 {
                        tries += 1;
                        if random_step(data, &mut state, &mut rng, side) {
                            accepted += 1;
                        }
                        answer_pending(data, &mut state, &mut rng);
                    }
                    assert!(accepted > 0, "seed {seed}: no legal step found");
                    state.apply(data, side, Order::EndSegment).unwrap();
                    drain_combat(data, &mut state);
                }
                (Phase::MovementCombat, Some(Segment::ReserveRelease)) => {
                    state.apply(data, side, Order::Pass).unwrap();
                }
                other => panic!("unexpected {other:?} at {}", state.clock),
            }
        }
        state
    }

    #[test]
    fn random_legal_moves_replay_to_the_same_state() {
        let data = Data::load("grazianis-offensive").unwrap();
        for seed in [1, 2, 3] {
            let live = random_stage(&data, seed, 12);
            let moves = live
                .log
                .iter()
                .filter(|o| matches!(o.order, Order::Move { .. }))
                .count();
            assert!(moves >= 12, "seed {seed}: {moves} moves");
            let again = random_stage(&data, seed, 12);
            assert_eq!(again.to_json().unwrap(), live.to_json().unwrap());
            let replayed = replay(&data, &live.to_log()).unwrap();
            assert_eq!(replayed.to_json().unwrap(), live.to_json().unwrap());
            assert_eq!(replayed, live);
        }
    }
}
