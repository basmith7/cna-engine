//! A soak of every scenario cna ships: each sets up with every module on
//! and plays two OpStages of random moves taken from `reachable`, so
//! the move list the browser shows and `apply` are held to agree
//! everywhere, not only in Graziani's Offensive. Along the way the state
//! must survive a save and a replay unchanged.

#[cfg(test)]
mod tests {
    use crate::clock::{Phase, Segment};
    use crate::data::Data;
    use crate::log::replay;
    use crate::order::Order;
    use crate::piece::{PieceId, PieceKind};
    use crate::reach::Purpose;
    use crate::state::test_support::{auto_place, drain_combat};
    use crate::state::{GameState, Modules, Pending};
    use cna_data::map::HexId;
    use rand_chacha::ChaCha8Rng;
    use rand_core::{RngCore, SeedableRng};
    use std::collections::BTreeMap;

    /// OpStages each scenario plays.
    const STAGES: usize = 2;
    /// Moves tried per movement segment.
    const TRIES: usize = 12;

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

    fn pick<T: Clone>(rng: &mut ChaCha8Rng, items: &[T]) -> Option<T> {
        if items.is_empty() {
            return None;
        }
        Some(items[(rng.next_u64() % items.len() as u64) as usize].clone())
    }

    /// The phasing player's stacks on the map: per hex, its combat units
    /// as one body, or one of them alone.
    fn bodies(state: &GameState, rng: &mut ChaCha8Rng) -> Vec<Vec<PieceId>> {
        let side = state.to_act().unwrap();
        let mut by_hex: BTreeMap<HexId, Vec<PieceId>> = BTreeMap::new();
        for p in state.pieces.values() {
            if p.side == side
                && p.kind == PieceKind::Combat
                && let Some(hex) = p.hex()
            {
                by_hex.entry(hex.clone()).or_default().push(p.id.clone());
            }
        }
        by_hex
            .into_values()
            .map(|ids| {
                if rng.next_u32() & 1 == 0 {
                    ids
                } else {
                    vec![pick(rng, &ids).unwrap()]
                }
            })
            .collect()
    }

    /// Answers what the non-phasing player owes mid-move: ZOC at random
    /// (10.16), reactions declined.
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

    /// One random move offered by `reachable`; a refusal by `apply` of a
    /// move it offered is a disagreement, returned as text.
    fn random_move(
        data: &Data,
        state: &mut GameState,
        rng: &mut ChaCha8Rng,
    ) -> Result<bool, String> {
        let side = state.to_act().unwrap();
        let bodies = bodies(state, rng);
        let Some(pieces) = pick(rng, &bodies) else {
            return Ok(false);
        };
        let Ok(reaches) = state.reachable(data, side, &pieces, Purpose::Move) else {
            return Ok(false);
        };
        let Some(reach) = pick(rng, &reaches) else {
            return Ok(false);
        };
        let order = Order::Move {
            pieces: pieces.clone(),
            path: reach.path.clone(),
            assault: false,
            fuel_from: None,
        };
        let mut trial = state.clone();
        if let Err(e) = trial.apply(data, side, order.clone()) {
            return Err(format!(
                "{} {side}: reachable offers {pieces:?} to {} by {:?}, apply says {e:?}",
                state.clock, reach.hex, reach.path
            ));
        }
        // A body over the ceiling (9.14) is legal mid-segment but would
        // hold up `EndSegment`; leave it out.
        if trial.over_ceiling_at(data, &reach.hex).is_some() {
            return Ok(false);
        }
        *state = trial;
        Ok(true)
    }

    /// Plays `STAGES` OpStages from set-up: random moves in each
    /// movement segment, every other answer as `pass_stage` gives it.
    fn soak(data: &Data, seed: u64) -> (GameState, Vec<String>) {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut state = GameState::with_modules(data, seed, Modules::ALL).unwrap();
        auto_place(data, &mut state);
        let mut faults = Vec::new();
        for _ in 0..STAGES {
            let (gt, stage) = (state.clock.game_turn, state.clock.stage);
            while !state.is_over() && (state.clock.game_turn, state.clock.stage) == (gt, stage) {
                let player = state.to_act().unwrap();
                if state.clock.phase == Phase::MovementCombat
                    && state.clock.segment == Some(Segment::Movement)
                    && state.pending.is_none()
                {
                    for _ in 0..TRIES {
                        if let Err(f) = random_move(data, &mut state, &mut rng) {
                            faults.push(f);
                        }
                        answer_pending(data, &mut state, &mut rng);
                    }
                    state.apply(data, player, Order::EndSegment).unwrap();
                    drain_combat(data, &mut state);
                    continue;
                }
                let order = match &state.pending {
                    Some(Pending::DeclareZoc { .. }) => Order::DeclareZoc { exerting: vec![] },
                    _ if state.clock.phase == Phase::InitiativeDeclaration => {
                        Order::DeclareInitiative { player_a: player }
                    }
                    _ => Order::Pass,
                };
                state
                    .apply(data, player, order.clone())
                    .unwrap_or_else(|e| panic!("{} {order:?}: {e:?}", state.clock));
            }
        }
        (state, faults)
    }

    #[test]
    fn every_scenario_takes_random_offered_moves_and_replays() {
        let mut failed = Vec::new();
        for slug in slugs() {
            let data = Data::load(&slug).unwrap();
            let run = std::panic::catch_unwind(|| {
                let (live, faults) = soak(&data, 7);
                let moves = live
                    .log
                    .iter()
                    .filter(|o| matches!(o.order, Order::Move { .. }))
                    .count();
                let saved = GameState::from_json(&live.to_json().unwrap(), &data).unwrap();
                assert_eq!(saved, live, "{slug}: the save does not round-trip");
                let replayed = replay(&data, &live.to_log()).unwrap();
                assert_eq!(replayed, live, "{slug}: the log does not replay");
                (moves, faults)
            });
            match run {
                Ok((moves, faults)) => {
                    eprintln!("{slug}: {moves} moves, {} faults", faults.len());
                    for f in faults.iter().take(5) {
                        eprintln!("  {f}");
                    }
                    if moves == 0 || !faults.is_empty() {
                        failed.push(slug);
                    }
                }
                Err(_) => failed.push(slug),
            }
        }
        assert!(failed.is_empty(), "{failed:?}");
    }
}
