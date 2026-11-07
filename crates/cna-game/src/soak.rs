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
    use crate::state::test_support::auto_place;
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
    /// (10.16); a reaction (8.5x), half the time, by one eligible unit
    /// to a hex `reachable` offers it, else declined. A reaction `apply`
    /// refuses is a disagreement, kept in `faults`, but for fuel: the
    /// reaction overlay quotes fuel nothing in range can pay, by design
    /// (`reach`'s `a_reaction_quotes_the_fuel_it_owes`).
    fn answer_pending(
        data: &Data,
        state: &mut GameState,
        rng: &mut ChaCha8Rng,
        faults: &mut Vec<String>,
    ) {
        while let Some(p) = state.pending.clone() {
            let side = p.side();
            let order = match p {
                Pending::DeclareZoc { hexes, .. } => Order::DeclareZoc {
                    exerting: hexes
                        .into_iter()
                        .filter(|_| rng.next_u32() & 1 == 0)
                        .collect(),
                },
                Pending::Reaction { eligible, .. } => {
                    let reactor = pick(rng, &eligible).filter(|_| rng.next_u32() & 1 == 0);
                    let reach = reactor.as_ref().and_then(|id| {
                        let reaches = state
                            .reachable(data, side, std::slice::from_ref(id), Purpose::React)
                            .ok()?;
                        pick(rng, &reaches)
                    });
                    match (reactor, reach) {
                        (Some(id), Some(r)) => {
                            let order = Order::React {
                                pieces: vec![id.clone()],
                                path: r.path.clone(),
                            };
                            let mut trial = state.clone();
                            match trial.apply(data, side, order) {
                                Ok(_) => {
                                    *state = trial;
                                    continue;
                                }
                                Err(e) if e.case == "32.22" => Order::Pass,
                                Err(e) => {
                                    faults.push(format!(
                                        "{} {side}: reachable offers {id} a reaction to {} by {:?}, apply says {e:?}",
                                        state.clock, r.hex, r.path
                                    ));
                                    Order::Pass
                                }
                            }
                        }
                        _ => Order::Pass,
                    }
                }
                _ => return,
            };
            state.apply(data, side, order).unwrap();
        }
    }

    /// Answers every combat step with nothing until reserve release
    /// opens, as `drain_combat` does, but a forced retreat goes to a hex
    /// `reachable` offers it (15.82, 10.36), checked against `apply`.
    fn through_combat(
        data: &Data,
        state: &mut GameState,
        rng: &mut ChaCha8Rng,
        faults: &mut Vec<String>,
    ) {
        loop {
            if let Some(Pending::ForcedRetreat { side, pieces, .. }) = state.pending.clone() {
                let reaches = state
                    .reachable(data, side, &pieces, Purpose::ForcedRetreat)
                    .unwrap_or_default();
                if let Some(r) = pick(rng, &reaches) {
                    let order = Order::Retreat {
                        path: r.path.clone(),
                    };
                    let mut trial = state.clone();
                    match trial.apply(data, side, order) {
                        Ok(_) => {
                            *state = trial;
                            continue;
                        }
                        Err(e) => faults.push(format!(
                            "{} {side}: reachable offers a retreat to {} by {:?}, apply says {e:?}",
                            state.clock, r.hex, r.path
                        )),
                    }
                }
            }
            let before = state.log.len();
            drain_one(data, state);
            if state.log.len() == before {
                return;
            }
        }
    }

    /// One answer of `drain_combat`'s, if a combat step is pending.
    fn drain_one(data: &Data, state: &mut GameState) {
        use crate::combat::SealedStep;
        let player = state.to_act().unwrap();
        let order = match &state.pending {
            Some(Pending::Sealed { step, .. }) => match step {
                SealedStep::GunPositions => Order::GunPositions { forward: vec![] },
                SealedStep::Barrage => Order::Barrage { fires: vec![] },
                SealedStep::AssignForces => Order::AssignForces {
                    assignments: vec![],
                },
            },
            Some(Pending::RetreatBeforeAssault { .. }) | Some(Pending::ForcedRetreat { .. }) => {
                Order::Pass
            }
            None if state.clock.segment == Some(Segment::CloseAssault) => Order::Pass,
            _ => return,
        };
        state
            .apply(data, player, order.clone())
            .unwrap_or_else(|e| panic!("{} {order:?}: {e:?}", state.clock));
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
        // Half the time toward the enemy: a move that may be stopped by
        // a ZOC declaration or a reaction, where the rules are busiest.
        let contested: Vec<_> = reaches
            .iter()
            .filter(|r| r.may_be_interrupted || r.ends_in_zoc)
            .cloned()
            .collect();
        let pool = if !contested.is_empty() && rng.next_u32() & 1 == 0 {
            &contested
        } else {
            &reaches
        };
        let Some(reach) = pick(rng, pool) else {
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
                        answer_pending(data, &mut state, &mut rng, &mut faults);
                    }
                    state.apply(data, player, Order::EndSegment).unwrap();
                    through_combat(data, &mut state, &mut rng, &mut faults);
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
                let reactions = live
                    .log
                    .iter()
                    .filter(|o| matches!(o.order, Order::React { .. }))
                    .count();
                let saved = GameState::from_json(&live.to_json().unwrap(), &data).unwrap();
                assert_eq!(saved, live, "{slug}: the save does not round-trip");
                let replayed = replay(&data, &live.to_log()).unwrap();
                assert_eq!(replayed, live, "{slug}: the log does not replay");
                (moves, reactions, faults)
            });
            match run {
                Ok((moves, reactions, faults)) => {
                    eprintln!(
                        "{slug}: {moves} moves, {reactions} reactions, {} faults",
                        faults.len()
                    );
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
