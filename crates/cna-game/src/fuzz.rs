//! A fuzz of `apply` in every scenario: at each step of a quiet game the
//! player to act also sends random orders of every kind, built from the
//! pieces, hexes and weapons on the board. `apply` must refuse or accept
//! each without a panic; an accepted one is kept half the time, so the
//! game wanders into states no scripted test sets up; and the end state
//! must survive a save and a replay, with both sides' views drawn.
//!
//! `soak` holds the browser's overlays to `apply`; this holds `apply`
//! to its own contract, for the orders no overlay offers in Rust
//! (organisation, supply, repair, construction, raids, prisoners).

#[cfg(test)]
mod tests {
    use crate::clock::{Phase, Segment};
    use crate::construction::Item;
    use crate::data::Data;
    use crate::log::replay;
    use crate::order::{Fire, Order};
    use crate::piece::{Location, PieceId};
    use crate::state::test_support::auto_place;
    use crate::state::{GameState, Modules, Pending};
    use crate::view::{Aliases, view_for};
    use cna_data::map::HexId;
    use cna_data::scenario::Side;
    use rand_chacha::ChaCha8Rng;
    use rand_core::{RngCore, SeedableRng};
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// OpStages each scenario plays; `CNA_FUZZ_STAGES` for more.
    fn stages() -> u32 {
        std::env::var("CNA_FUZZ_STAGES").map_or(2, |s| s.parse().unwrap())
    }
    /// Wild orders sent before each step of the game.
    const WILD: usize = 24;

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

    fn small(rng: &mut ChaCha8Rng) -> u32 {
        // Mostly small, sometimes zero, now and then large.
        match rng.next_u32() % 8 {
            0 => 0,
            7 => 1 + rng.next_u32() % 100,
            _ => 1 + rng.next_u32() % 6,
        }
    }

    /// The pieces, hexes and weapons the orders are drawn from: `own` the
    /// actor's pieces, `any` everyone's, hexes those pieces stand in and
    /// their neighbours.
    struct Pool {
        own: Vec<PieceId>,
        any: Vec<PieceId>,
        hexes: Vec<HexId>,
        weapons: Vec<String>,
    }

    fn pool(data: &Data, state: &GameState, side: Side) -> Pool {
        let mut own = Vec::new();
        let mut any = Vec::new();
        let mut hexes = Vec::new();
        let mut weapons = Vec::new();
        for p in state.pieces.values() {
            any.push(p.id.clone());
            if p.side == side {
                own.push(p.id.clone());
            }
            if let Some(hex) = p.hex() {
                hexes.push(hex.clone());
                if p.side == side {
                    hexes.extend(data.map.neighbours(hex));
                }
            }
            weapons.extend(p.weapons.iter().map(|w| w.weapon.clone()));
        }
        hexes.sort();
        hexes.dedup();
        weapons.sort();
        weapons.dedup();
        weapons.push("no-such-weapon".into());
        Pool {
            own,
            any,
            hexes,
            weapons,
        }
    }

    /// A path of one to three steps from `from`, neighbour to neighbour;
    /// `from` itself left out, as orders give paths, but now and then in.
    fn path(data: &Data, rng: &mut ChaCha8Rng, from: &HexId) -> Vec<HexId> {
        let mut path = vec![from.clone()];
        for _ in 0..1 + rng.next_u32() % 3 {
            let last = path.last().unwrap().clone();
            match pick(rng, &data.map.neighbours(&last)) {
                Some(next) => path.push(next),
                None => break,
            }
        }
        if !rng.next_u32().is_multiple_of(8) {
            path.remove(0);
        }
        path
    }

    /// The kinds of order (as numbered in `wild`) most likely legal in
    /// the phase: three draws in four come from here, the rest from all.
    fn kinds_for(state: &GameState) -> &'static [u32] {
        match state.clock.phase {
            Phase::Organisation => &[0, 1, 2, 3, 4, 5, 6, 21, 22, 23, 24, 26, 27, 28, 31],
            Phase::CwFleet => &[8, 16, 17, 18],
            Phase::ReserveDesignation => &[32, 18],
            Phase::MovementCombat if state.pending.is_none() => &[
                36, 36, 36, 36, 36, 36, 36, 36, 33, 10, 29, 30, 25, 18, 20, 9, 7, 15,
            ],
            Phase::TruckConvoy => &[7, 9, 10],
            Phase::Rail => &[11, 11, 7],
            Phase::Repair => &[12, 13, 14, 15, 7],
            Phase::Patrol => &[19, 19, 20],
            _ => &[34, 35, 39, 31],
        }
    }

    fn wild(data: &Data, state: &GameState, side: Side, rng: &mut ChaCha8Rng) -> Option<Order> {
        let p = pool(data, state, side);
        let own = pick(rng, &p.own)?;
        let at = state.pieces[&own].hex().cloned();
        // Mostly a piece beside `own` and a hex at or next to it, where
        // orders between two pieces or on a hex can be legal at all.
        let beside: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|q| q.id != own && at.is_some() && q.hex() == at.as_ref())
            .map(|q| q.id.clone())
            .collect();
        let other = match rng.next_u32() % 5 {
            0..=2 => pick(rng, &beside).or_else(|| pick(rng, &p.any))?,
            3 => pick(rng, &p.own)?,
            _ => pick(rng, &p.any)?,
        };
        let hex = match (&at, rng.next_u32() % 5) {
            (Some(at), 0..=1) => at.clone(),
            (Some(at), 2..=3) => pick(rng, &data.map.neighbours(at)).unwrap_or_else(|| at.clone()),
            _ => pick(rng, &p.hexes)?,
        };
        let own_weapons: Vec<String> = state.pieces[&own]
            .weapons
            .iter()
            .map(|w| w.weapon.clone())
            .collect();
        let weapon = if rng.next_u32() & 1 == 0 {
            pick(rng, &own_weapons).or_else(|| pick(rng, &p.weapons))?
        } else {
            pick(rng, &p.weapons)?
        };
        let at = at.unwrap_or_else(|| hex.clone());
        let item = pick(
            rng,
            &[
                Item::Fortification,
                Item::RealMinefield,
                Item::DummyMinefield,
                Item::ClearMinefield,
            ],
        )?;
        let fuel_from = if rng.next_u32() & 1 == 0 {
            None
        } else {
            pick(rng, &p.own)
        };
        let unit = other.unit();
        let kind = match pick(rng, kinds_for(state)) {
            Some(k) if !rng.next_u32().is_multiple_of(4) => k,
            _ => rng.next_u32() % 40,
        };
        Some(match kind {
            0 => Order::Attach {
                piece: own,
                to: other,
            },
            1 => Order::Detach { piece: own },
            2 => Order::Reassign { piece: own },
            3 => Order::ShiftParent {
                piece: own,
                to: (rng.next_u32() & 1 == 0).then_some(other),
            },
            4 => Order::SwapBattalions {
                piece: own,
                with: other,
            },
            5 => Order::AttachMotorisation { piece: own },
            6 => Order::DetachMotorisation { piece: own },
            7 => Order::TransferLoad {
                from: own,
                to: other,
                ammo: small(rng),
                fuel: small(rng),
            },
            8 => Order::ConvoyLanes {
                lanes: (0..rng.next_u32() % 3)
                    .map(|_| (rng.next_u32() % 6) as u8)
                    .collect(),
            },
            9 => Order::AirLift {
                piece: own,
                to: Location::Hex(hex),
                fuel_from,
            },
            10 => Order::DestroySupply { hex },
            11 => Order::Rail {
                pieces: vec![own],
                path: path(data, rng, &at),
                stops: vec![],
            },
            12 => Order::Repair {
                hex,
                weapon,
                fuel_from,
                unit,
            },
            13 => Order::FacilityRepair {
                hex,
                weapon,
                fuel_from,
                unit,
            },
            14 => Order::RepairDestroyedTanks {
                hex,
                weapon,
                points: small(rng),
                fuel_from,
            },
            15 => Order::Tow {
                path: path(data, rng, &hex),
                hex,
                weapon,
                points: small(rng),
            },
            16 => Order::Station { ship: own, hex },
            17 => Order::PlanChariotRaid,
            18 => Order::PlaceDummyTank { hex },
            19 => Order::Patrol {
                sources: vec![(own, small(rng))],
                target: hex,
                exclude: vec![],
            },
            20 => Order::MarchPrisoners {
                path: path(data, rng, &hex),
                from: hex,
                points: small(rng),
            },
            21 => Order::Build {
                item,
                hex: at,
                builders: vec![own],
            },
            22 => Order::DetailGuards {
                piece: own,
                points: small(rng),
            },
            23 => Order::ReleaseGuards {
                piece: own,
                points: small(rng),
            },
            24 => Order::Absorb {
                piece: own,
                weapon,
                points: small(rng),
            },
            25 => Order::RaidRommel { piece: own },
            26 => Order::WithdrawVoluntarily { piece: own },
            27 => Order::ReturnWithdrawn { piece: own },
            28 => Order::FormRaider {
                raider: own,
                from: other,
            },
            29 => Order::RaidDump {
                piece: own,
                target: other,
            },
            30 => Order::TrySpot { piece: own },
            31 => Order::SetLossOrder {
                piece: own,
                weapons: vec![weapon],
            },
            32 => Order::DesignateReserve { pieces: vec![own] },
            33 => Order::ReleaseReserve { pieces: vec![own] },
            34 => Order::PlaceWork { item, hex },
            35 => Order::PlaceMotorisation {
                hex,
                points: small(rng),
            },
            36 => Order::Move {
                pieces: vec![own],
                path: path(data, rng, &at),
                assault: rng.next_u32() & 1 == 0,
                fuel_from,
            },
            37 => Order::Barrage {
                fires: vec![Fire {
                    piece: own,
                    points: small(rng),
                    target: other,
                }],
            },
            38 => Order::ReleaseAntiArmour {
                piece: own,
                target: hex,
                points: small(rng),
            },
            _ => Order::Unplace { piece: own },
        })
    }

    /// The quiet answer for the step: nothing fired, nothing assigned,
    /// the segment ended, the phase passed.
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

    /// Whether `after` has a hex over its stacking ceiling that `before`
    /// had not.
    fn newly_over(data: &Data, before: &GameState, after: &GameState) -> bool {
        let over = |state: &GameState| -> std::collections::BTreeSet<HexId> {
            state
                .pieces
                .values()
                .filter_map(|p| p.hex())
                .filter(|hex| state.over_ceiling_at(data, hex).is_some())
                .cloned()
                .collect()
        };
        !over(after).is_subset(&over(before))
    }

    /// Plays `stages()` OpStages; returns the state and its faults: panics
    /// in `apply` or a view, and a quiet answer refused after a kept
    /// order (the game stuck, which ends the run).
    fn fuzz(data: &Data, seed: u64) -> (GameState, Vec<String>, usize) {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut state = GameState::with_modules(data, seed, Modules::ALL).unwrap();
        auto_place(data, &mut state);
        let aliases = Aliases::new([3; 32]);
        let mut faults = Vec::new();
        let mut kept = 0;
        let mut last_kept: Option<Order> = None;
        let mut steps = 0;
        let mut left = stages();
        let mut at = (state.clock.game_turn, state.clock.stage);
        while !state.is_over() {
            if (state.clock.game_turn, state.clock.stage) != at {
                at = (state.clock.game_turn, state.clock.stage);
                left -= 1;
                if left == 0 {
                    break;
                }
            }
            steps += 1;
            if steps > 2_500 * stages() {
                faults.push(format!("{}: no end after {steps} steps", state.clock));
                break;
            }
            let side = state.to_act().unwrap();
            for _ in 0..WILD {
                let Some(order) = wild(data, &state, side, &mut rng) else {
                    continue;
                };
                let mut trial = state.clone();
                let run = catch_unwind(AssertUnwindSafe(|| {
                    trial.apply(data, side, order.clone()).is_ok()
                }));
                match run {
                    Err(_) => {
                        faults.push(format!("{} {side}: apply panics on {order:?}", state.clock))
                    }
                    // A stack over its ceiling (9.14) is legal mid-segment
                    // but holds up `EndSegment` until the player moves a
                    // unit out, which the quiet driver never does.
                    Ok(true) if rng.next_u32() & 1 == 0 && !newly_over(data, &state, &trial) => {
                        for viewer in [Some(Side::Axis), Some(Side::Cw), None] {
                            let v = catch_unwind(AssertUnwindSafe(|| {
                                view_for(&trial, data, &aliases, viewer, false).is_ok()
                            }));
                            if !matches!(v, Ok(true)) {
                                faults.push(format!(
                                    "{} {side}: view for {viewer:?} fails after {order:?}",
                                    state.clock
                                ));
                            }
                        }
                        state = trial;
                        kept += 1;
                        last_kept = Some(order);
                        if state.to_act() != Some(side) {
                            break;
                        }
                    }
                    Ok(_) => {}
                }
            }
            let Some(side) = state.to_act() else { break };
            let order = quiet(&state, side);
            let run = catch_unwind(AssertUnwindSafe(|| {
                let mut s = state.clone();
                s.apply(data, side, order.clone()).map(|_| s)
            }));
            match run {
                Ok(Ok(s)) => state = s,
                Ok(Err(e)) => {
                    faults.push(format!(
                        "{} {side}: quiet {order:?} refused ({e:?}) after {last_kept:?}",
                        state.clock
                    ));
                    break;
                }
                Err(_) => {
                    faults.push(format!(
                        "{} {side}: apply panics on quiet {order:?}",
                        state.clock
                    ));
                    break;
                }
            }
        }
        (state, faults, kept)
    }

    #[test]
    fn every_scenario_survives_random_orders_of_every_kind() {
        let seeds: Vec<u64> = std::env::var("CNA_FUZZ_SEEDS")
            .ok()
            .map(|s| s.split(',').map(|n| n.parse().unwrap()).collect())
            .unwrap_or_else(|| vec![5]);
        let mut failed = Vec::new();
        for slug in slugs() {
            let data = Data::load(&slug).unwrap();
            for &seed in &seeds {
                let (live, faults, kept) = fuzz(&data, seed);
                let saved = GameState::from_json(&live.to_json().unwrap(), &data).unwrap();
                let replayed = replay(&data, &live.to_log()).unwrap();
                let mut kinds = std::collections::BTreeMap::<String, usize>::new();
                for o in &live.log {
                    let name = format!("{:?}", o.order);
                    let name = name.split([' ', '(', '{']).next().unwrap().to_owned();
                    *kinds.entry(name).or_default() += 1;
                }
                eprintln!(
                    "{slug} seed {seed}: {kept} kept, {} faults; {kinds:?}",
                    faults.len()
                );
                for f in faults.iter().take(8) {
                    eprintln!("  {f}");
                }
                if saved != live {
                    failed.push(format!("{slug}/{seed}: the save does not round-trip"));
                }
                if replayed != live {
                    failed.push(format!("{slug}/{seed}: the log does not replay"));
                }
                if !faults.is_empty() {
                    failed.push(format!("{slug}/{seed}: {} faults", faults.len()));
                }
            }
        }
        assert!(failed.is_empty(), "{failed:#?}");
    }
}
