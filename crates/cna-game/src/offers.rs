//! The engine's offer lists, what the browser shows a player as choices,
//! held to `apply` in every scenario: at each step of a quiet game the
//! player to act takes each offer (or an even sample of many) on a copy
//! of the state, as the order the browser would send for it, and `apply`
//! must accept it. Covered: set-up works (`work_options`), patrols
//! (`patrol_options`, one source at a time), voluntary withdrawals and
//! returns (`voluntary_options`), withdrawal picks
//! (`withdrawal_pick_options`), ship stations (`station_options`),
//! historical shifts of parent (`parent_shifts`), rail runs
//! (`reachable` for `Purpose::Rail`) and a move's choice of fuel
//! (`fuel_sources`).
//!
//! `soak` holds the move and combat overlays to `apply`; `fuzz` holds
//! `apply` to its own contract for orders no overlay offers.

#[cfg(test)]
mod tests {
    use crate::clock::{Phase, Segment};
    use crate::construction::Item;
    use crate::data::Data;
    use crate::order::Order;
    use crate::piece::{PieceId, PieceKind};
    use crate::reach::Purpose;
    use crate::state::test_support::place_pieces;
    use crate::state::{GameState, Modules, Pending};
    use cna_data::map::HexId;
    use cna_data::scenario::Side;
    use std::collections::{BTreeMap, BTreeSet};

    /// OpStages each scenario plays; `CNA_OFFERS_STAGES` for more.
    fn stages() -> u32 {
        let n = std::env::var("CNA_OFFERS_STAGES").map_or(2, |s| s.parse().unwrap());
        assert!(n > 0, "CNA_OFFERS_STAGES must be at least 1");
        n
    }

    fn seed() -> u64 {
        std::env::var("CNA_OFFERS_SEED").map_or(7, |s| s.parse().unwrap())
    }

    /// The kinds of offer, in the order they are reported.
    const KINDS: [&str; 9] = [
        "work", "patrol", "withdraw", "return", "pick", "station", "shift", "rail", "fuel",
    ];

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

    /// At most `cap` of `items`, spread evenly from first to last.
    fn sample<T: Clone>(items: &[T], cap: usize) -> Vec<T> {
        if items.len() <= cap {
            return items.to_vec();
        }
        (0..cap)
            .map(|i| items[i * (items.len() - 1) / (cap - 1).max(1)].clone())
            .collect()
    }

    /// Offers tried per kind, the ones `apply` refused, and those
    /// already tried in the phase: a quiet game offers much the same at
    /// each step, and each try clones the state.
    #[derive(Default)]
    struct Tally {
        tried: BTreeMap<&'static str, usize>,
        faults: Vec<String>,
        seen: BTreeSet<String>,
    }

    impl Tally {
        /// Sends `order` for the offer `what` of `kind` on a copy of
        /// `state`, unless already tried in this phase; a refusal is a
        /// fault. The copy, if `apply` took it.
        fn take(
            &mut self,
            data: &Data,
            state: &GameState,
            side: Side,
            kind: &'static str,
            what: &str,
            order: Order,
        ) -> Option<GameState> {
            let key = format!(
                "{:?} {:?} {order:?}",
                state.clock.game_turn, state.clock.phase
            );
            if !self.seen.insert(key) {
                return None;
            }
            *self.tried.entry(kind).or_default() += 1;
            let mut trial = state.clone();
            match trial.apply(data, side, order.clone()) {
                Ok(_) => Some(trial),
                Err(e) => {
                    self.faults.push(format!(
                        "{} {side} [{kind}] {}: offered {what} as {order:?}, apply says {e:?}",
                        state.clock, data.scenario.name
                    ));
                    None
                }
            }
        }
    }

    /// Set-up works placed as `auto_place` places them, one at a time
    /// in the first hex open, until both sides have set up; at every
    /// `WORK_EVERY`th placement (`work_options` is slow where a side has
    /// hundreds to place) the works offered are tried, and `may_pass`
    /// must let `Pass` through.
    fn set_up(data: &Data, state: &mut GameState, tally: &mut Tally) {
        const WORK_EVERY: usize = 16;
        let mut step: usize = 0;
        while let Some(Pending::SetUp { side }) = state.pending.clone() {
            place_pieces(data, state, side);
            if state.pending != Some(Pending::SetUp { side }) {
                continue;
            }
            step += 1;
            if !(step - 1).is_multiple_of(WORK_EVERY) {
                let order = [
                    Item::Fortification,
                    Item::RealMinefield,
                    Item::DummyMinefield,
                ]
                .into_iter()
                .find_map(|item| {
                    let hex = state.work_placements(data, side, item).into_iter().next()?;
                    Some(Order::PlaceWork { item, hex })
                })
                .unwrap_or(Order::Pass);
                state
                    .apply(data, side, order.clone())
                    .unwrap_or_else(|e| panic!("{order:?}: {e:?}"));
                continue;
            }
            let mut first = None;
            if let Some(o) = state.work_options(data, side) {
                for item in [
                    Item::Fortification,
                    Item::RealMinefield,
                    Item::DummyMinefield,
                ] {
                    let key = serde_json::to_value(item).unwrap();
                    let hexes: Vec<HexId> = o
                        .get(key.as_str().unwrap())
                        .map(|v| serde_json::from_value(v.clone()).unwrap())
                        .unwrap_or_default();
                    for hex in sample(&hexes, 3) {
                        let order = Order::PlaceWork {
                            item,
                            hex: hex.clone(),
                        };
                        tally.take(
                            data,
                            state,
                            side,
                            "work",
                            &format!("{item:?} in {hex}"),
                            order,
                        );
                    }
                    if first.is_none() && !hexes.is_empty() {
                        first = Some(Order::PlaceWork {
                            item,
                            hex: hexes[0].clone(),
                        });
                    }
                }
                if o["may_pass"] == serde_json::json!(true) {
                    tally.take(data, state, side, "work", "may_pass", Order::Pass);
                }
            }
            let order = first.unwrap_or(Order::Pass);
            state
                .apply(data, side, order.clone())
                .unwrap_or_else(|e| panic!("{order:?}: {e:?}"));
        }
    }

    /// The hexes holding `side`'s pieces on the map, each with its
    /// pieces of `kinds`.
    fn stacks(state: &GameState, side: Side, kinds: &[PieceKind]) -> Vec<Vec<PieceId>> {
        let mut by_hex: BTreeMap<HexId, Vec<PieceId>> = BTreeMap::new();
        for p in state.pieces.values() {
            if p.side == side
                && kinds.contains(&p.kind)
                && let Some(hex) = p.hex()
            {
                by_hex.entry(hex.clone()).or_default().push(p.id.clone());
            }
        }
        by_hex.into_values().collect()
    }

    /// Takes every offer `side`, to act, has now.
    fn offers(data: &Data, state: &GameState, side: Side, tally: &mut Tally) {
        for o in sample(&state.patrol_options(data, side), 6) {
            for target in sample(&o.targets, 3) {
                let order = Order::Patrol {
                    sources: vec![(o.piece.clone(), o.points)],
                    target: target.clone(),
                    exclude: vec![],
                };
                let what = format!("{} patrolling {target} with {}", o.piece, o.points);
                tally.take(data, state, side, "patrol", &what, order);
            }
        }

        let v = state.voluntary_options(data, side);
        for piece in sample(&v.withdrawable, 4) {
            let order = Order::WithdrawVoluntarily {
                piece: piece.clone(),
            };
            // Brought back at once, where the rules allow it.
            let Some(away) = tally.take(data, state, side, "withdraw", &piece.to_string(), order)
            else {
                continue;
            };
            if away
                .voluntary_options(data, side)
                .returnable
                .contains(&piece)
            {
                let order = Order::ReturnWithdrawn {
                    piece: piece.clone(),
                };
                tally.take(data, &away, side, "return", &piece.to_string(), order);
            }
        }
        for piece in sample(&v.returnable, 4) {
            let order = Order::ReturnWithdrawn {
                piece: piece.clone(),
            };
            tally.take(data, state, side, "return", &piece.to_string(), order);
        }

        for o in state.withdrawal_pick_options(data, side) {
            let n = o.count as usize;
            let first = o.candidates[..n].to_vec();
            let last = o.candidates[o.candidates.len() - n..].to_vec();
            for pieces in [first, last] {
                let what = format!("{} of {}: {pieces:?}", o.count, o.fills);
                let order = Order::PickWithdrawal {
                    fills: o.fills.clone(),
                    pieces,
                };
                tally.take(data, state, side, "pick", &what, order);
            }
        }

        if let Some(o) = state.station_options(data, side) {
            let hexes = sample(&o.hexes, 4);
            for ship in sample(&o.ships, 4) {
                for hex in &hexes {
                    let order = Order::Station {
                        ship: ship.clone(),
                        hex: hex.clone(),
                    };
                    tally.take(
                        data,
                        state,
                        side,
                        "station",
                        &format!("{ship} to {hex}"),
                        order,
                    );
                }
            }
        }

        for (piece, to) in state.parent_shifts(data, side) {
            let what = format!("{piece} to {to:?}");
            let order = Order::ShiftParent { piece, to };
            tally.take(data, state, side, "shift", &what, order);
        }

        if state.clock.phase == Phase::Rail && state.pending.is_none() {
            for stack in sample(&stacks(state, side, &[PieceKind::SupplyUnit]), 4) {
                let Ok(reaches) = state.reachable(data, side, &stack, Purpose::Rail) else {
                    continue;
                };
                for r in sample(&reaches, 3) {
                    let order = Order::Rail {
                        pieces: stack.clone(),
                        path: r.path.clone(),
                        stops: vec![],
                    };
                    let what = format!("{stack:?} by rail to {}", r.hex);
                    tally.take(data, state, side, "rail", &what, order);
                }
            }
        }

        if state.clock.phase == Phase::MovementCombat
            && state.clock.segment == Some(Segment::Movement)
            && state.pending.is_none()
        {
            fuel(data, state, side, tally);
        }
    }

    /// A move `reachable` offers, with each supply unit `fuel_sources`
    /// offers to pay for it. A source short of the fuel the move owes is
    /// not tried: the list is per stack, the fuel per path, and the
    /// browser shows the reach's fuel beside it. A move that may be
    /// interrupted is held only to the stage's first fuel (see `soak`).
    fn fuel(data: &Data, state: &GameState, side: Side, tally: &mut Tally) {
        let kinds = [PieceKind::Combat, PieceKind::Hq];
        let fuelled: Vec<_> = stacks(state, side, &kinds)
            .into_iter()
            .map(|s| (state.fuel_sources(data, &s), s))
            .filter(|(sources, _)| !sources.is_empty())
            .collect();
        for (sources, stack) in sample(&fuelled, 3) {
            let Ok(reaches) = state.reachable(data, side, &stack, Purpose::Move) else {
                continue;
            };
            let reaches: Vec<_> = reaches
                .into_iter()
                .filter(|r| r.fuel > 0 && !r.may_be_interrupted)
                .collect();
            for r in sample(&reaches, 2) {
                for src in sample(&sources, 1) {
                    let has = state.pieces[&src].load.map_or(0, |l| l.fuel);
                    if has < r.fuel {
                        continue;
                    }
                    let order = Order::Move {
                        pieces: stack.clone(),
                        path: r.path.clone(),
                        assault: false,
                        fuel_from: Some(src.clone()),
                    };
                    let what = format!("{stack:?} to {} on {src}'s fuel", r.hex);
                    tally.take(data, state, side, "fuel", &what, order);
                }
            }
        }
    }

    /// The quiet answer for the step, as `fuzz` gives it.
    fn quiet(state: &GameState, side: Side) -> Order {
        use crate::combat::SealedStep;
        match &state.pending {
            Some(Pending::Sealed { step, .. }) => match step {
                SealedStep::GunPositions => Order::GunPositions { forward: vec![] },
                SealedStep::Barrage => Order::Barrage { fires: vec![] },
                SealedStep::AssignForces => Order::AssignForces {
                    assignments: vec![],
                },
            },
            Some(Pending::DeclareZoc { .. }) => Order::DeclareZoc { exerting: vec![] },
            Some(_) => Order::Pass,
            None if state.clock.phase == Phase::InitiativeDeclaration => {
                Order::DeclareInitiative { player_a: side }
            }
            None if state.clock.phase == Phase::MovementCombat
                && state.clock.segment == Some(Segment::Movement) =>
            {
                Order::EndSegment
            }
            None => Order::Pass,
        }
    }

    /// Sets up and plays `stages()` quiet OpStages, taking the offers at
    /// every step.
    fn play(data: &Data, seed: u64) -> Tally {
        let mut state = GameState::with_modules(data, seed, Modules::ALL).unwrap();
        let mut tally = Tally::default();
        set_up(data, &mut state, &mut tally);
        let mut left = stages();
        let mut at = (state.clock.game_turn, state.clock.stage);
        while let Some(side) = state.to_act() {
            if (state.clock.game_turn, state.clock.stage) != at {
                at = (state.clock.game_turn, state.clock.stage);
                left -= 1;
                if left == 0 {
                    break;
                }
            }
            offers(data, &state, side, &mut tally);
            let order = quiet(&state, side);
            state
                .apply(data, side, order.clone())
                .unwrap_or_else(|e| panic!("{} {order:?}: {e:?}", state.clock));
        }
        tally
    }

    /// `CNA_OFFERS_STAGES` and `CNA_OFFERS_SEED` go further.
    #[test]
    fn every_offer_in_every_scenario_is_an_order_apply_takes() {
        let seed = seed();
        let mut total: BTreeMap<&str, usize> = BTreeMap::new();
        let mut faults = Vec::new();
        for slug in slugs() {
            let data = Data::load(&slug).unwrap();
            let start = std::time::Instant::now();
            let tally = play(&data, seed);
            let counts: Vec<String> = KINDS
                .iter()
                .map(|k| format!("{k} {}", tally.tried.get(k).copied().unwrap_or(0)))
                .collect();
            eprintln!(
                "{slug}: {}; {} faults, {:.1?}",
                counts.join(", "),
                tally.faults.len(),
                start.elapsed()
            );
            for f in tally.faults.iter().take(5) {
                eprintln!("  {f}");
            }
            for (k, n) in tally.tried {
                *total.entry(k).or_default() += n;
            }
            faults.extend(tally.faults);
        }
        eprintln!("all: {total:?}");
        assert!(faults.is_empty(), "{} faults: {faults:#?}", faults.len());
        // Not `pick` or `shift`, which a short run never offers: the
        // schedule's one pick row is at Game-Turn 97, and of the shifts
        // the O/A notes date, those in a scenario's first stages (April
        // 1941) leave a parent no unit starts with. `withdraw` and
        // `reassign` test them on states made for them.
        for kind in KINDS.iter().filter(|k| !["pick", "shift"].contains(k)) {
            assert!(
                total.get(kind).is_some_and(|&n| n > 0),
                "no {kind} offer tried"
            );
        }
    }
}
