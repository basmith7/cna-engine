//! Phase C under abstract supply (SPI 32.1x, 32.31): Player A, then
//! Player B, attaches or detaches motorisation points on supply units
//! and moves loads between supply units in one hex, then passes.

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Load, Location, MAX_AMMO, MAX_FUEL, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;

/// The points that move a real supply unit and a dummy (32.31).
const REAL_POINTS: u32 = 30;
const DUMMY_POINTS: u32 = 6;

impl GameState {
    /// The supply unit `player` may organise now, and its hex: phase C,
    /// the module on, its own, on the map.
    fn organisable(
        &self,
        player: Side,
        id: &PieceId,
        case: &'static str,
    ) -> Result<String, Illegal> {
        if self.clock.phase != Phase::Organisation || !self.modules.abstract_supply {
            return Err(Illegal::new(
                case,
                format!("supply is organised in phase C, not at {}", self.clock),
            ));
        }
        let p = self
            .pieces
            .get(id)
            .ok_or_else(|| Illegal::new(case, format!("no piece {id}")))?;
        if p.side != player || !matches!(p.kind, PieceKind::SupplyUnit | PieceKind::DummySupplyUnit)
        {
            return Err(Illegal::new(
                case,
                format!("{id} is not one of {player}'s supply units"),
            ));
        }
        p.hex()
            .map(|h| h.to_string())
            .ok_or_else(|| Illegal::new(case, format!("{id} is not on the map")))
    }

    /// Phase H waits on the phasing player only when it has a supply
    /// unit on the map with its motorisation points (32.31); otherwise
    /// there is nothing to do and it is skipped.
    pub(crate) fn trucks_to_move(&self) -> bool {
        let Some(side) = self.phasing() else {
            return false;
        };
        self.modules.abstract_supply
            && self
                .pieces
                .values()
                .any(|p| p.side == side && p.motorised > 0 && !p.escorted && p.hex().is_some())
    }

    /// `Order::DestroySupply` (32.1x): before any movement in the
    /// segment, every unit of the player's in the hex spends half its
    /// CPA (rounded up), and each of its supply units there not yet tried
    /// this segment goes on a die of 1–3, +1 when the hex holds 1 SP or
    /// less.
    pub(crate) fn destroy_supply(
        &mut self,
        data: &Data,
        player: Side,
        hex: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        if !self.modules.abstract_supply
            || self.clock.phase != Phase::MovementCombat
            || self.clock.segment != Some(crate::clock::Segment::Movement)
            || self.phasing() != Some(player)
        {
            return Err(Illegal::new(
                "32.1x",
                format!(
                    "supply is destroyed in one's own movement segment, not at {}",
                    self.clock
                ),
            ));
        }
        if !self.moved.is_empty() {
            return Err(Illegal::new(
                "32.1x",
                "supply is destroyed before anything moves in the segment",
            ));
        }
        let here = |p: &&crate::piece::Piece| p.side == player && p.hex() == Some(hex);
        let targets: Vec<PieceId> = self
            .pieces
            .values()
            .filter(here)
            .filter(|p| matches!(p.kind, PieceKind::SupplyUnit | PieceKind::DummySupplyUnit))
            .filter(|p| !self.scuttled.contains(&p.id))
            .map(|p| p.id.clone())
            .collect();
        if targets.is_empty() {
            return Err(Illegal::new(
                "32.1x",
                format!("{player} has no supply unit at {hex} left to destroy this segment"),
            ));
        }
        let payers: Vec<(PieceId, u32)> = self
            .pieces
            .values()
            .filter(here)
            .filter(|p| matches!(p.kind, PieceKind::Combat | PieceKind::Hq))
            .filter_map(|p| Some((p.id.clone(), self.cpa(p)?)))
            .collect();
        let bonus = u8::from(self.stacked(data, hex) <= 1);
        let mut events = Vec::new();
        for (id, cpa) in payers {
            self.charge(&id, Cp::whole(cpa.div_ceil(2)), false, &mut events);
        }
        for id in targets {
            let value = self.rng.d6();
            events.push(Event::DieRolled {
                purpose: "destroy supply".into(),
                value,
            });
            self.scuttled.insert(id.clone());
            if value + bonus <= 3 {
                self.pieces.get_mut(&id).expect("listed").location = Location::Eliminated;
                events.push(Event::SupplyDestroyed { piece: id });
            }
        }
        Ok(events)
    }

    /// `Order::AttachMotorisation` (32.31).
    pub(crate) fn attach_motorisation(
        &mut self,
        player: Side,
        id: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        let hex = self.organisable(player, id, "32.31")?;
        let p = &self.pieces[id];
        if p.motorised > 0 {
            return Err(Illegal::new(
                "32.31",
                format!("{id} already has its motorisation points"),
            ));
        }
        let need = if p.kind == PieceKind::SupplyUnit {
            REAL_POINTS
        } else {
            DUMMY_POINTS
        };
        let have = self
            .motorisation
            .get(&player)
            .and_then(|m| m.get(&hex))
            .copied()
            .unwrap_or(0);
        if have < need {
            return Err(Illegal::new(
                "32.31",
                format!("{id} needs {need} motorisation points and {hex} has {have}"),
            ));
        }
        let pool = self.motorisation.entry(player).or_default();
        if have == need {
            pool.remove(&hex);
        } else {
            pool.insert(hex, have - need);
        }
        self.pieces.get_mut(id).expect("checked").motorised = need;
        Ok(vec![Event::MotorisationAttached {
            piece: id.clone(),
            points: need,
        }])
    }

    /// `Order::DetachMotorisation` (32.31).
    pub(crate) fn detach_motorisation(
        &mut self,
        player: Side,
        id: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        let hex = self.organisable(player, id, "32.31")?;
        let points = self.pieces[id].motorised;
        if points == 0 {
            return Err(Illegal::new(
                "32.31",
                format!("{id} has no motorisation points"),
            ));
        }
        *self
            .motorisation
            .entry(player)
            .or_default()
            .entry(hex)
            .or_default() += points;
        self.pieces.get_mut(id).expect("checked").motorised = 0;
        Ok(vec![Event::MotorisationDetached {
            piece: id.clone(),
            points,
        }])
    }

    /// `Order::TransferLoad` (32.1x): within one hex and the caps; a unit
    /// left with nothing is removed.
    pub(crate) fn transfer_load(
        &mut self,
        player: Side,
        from: &PieceId,
        to: &PieceId,
        ammo: u32,
        fuel: u32,
    ) -> Result<Vec<Event>, Illegal> {
        let at = self.organisable(player, from, "32.1x")?;
        let there = self.organisable(player, to, "32.1x")?;
        let (Some(a), Some(b)) = (self.pieces[from].load, self.pieces[to].load) else {
            return Err(Illegal::new("32.1x", "a dummy carries no supply"));
        };
        if from == to || at != there {
            return Err(Illegal::new(
                "32.1x",
                format!("{from} and {to} must be two supply units in one hex"),
            ));
        }
        if ammo > a.ammo || fuel > a.fuel {
            return Err(Illegal::new(
                "32.1x",
                format!("{from} carries {} ammunition and {} fuel", a.ammo, a.fuel),
            ));
        }
        if b.ammo + ammo > MAX_AMMO || b.fuel + fuel > MAX_FUEL {
            return Err(Illegal::new(
                "32.1x",
                format!("{to} would carry more than {MAX_AMMO} ammunition or {MAX_FUEL} fuel"),
            ));
        }
        self.pieces.get_mut(from).expect("checked").load = Some(Load {
            ammo: a.ammo - ammo,
            fuel: a.fuel - fuel,
        });
        self.pieces.get_mut(to).expect("checked").load = Some(Load {
            ammo: b.ammo + ammo,
            fuel: b.fuel + fuel,
        });
        let mut events = vec![Event::LoadTransferred {
            from: from.clone(),
            to: to.clone(),
            ammo,
            fuel,
        }];
        self.retire_if_empty(from, &mut events);
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use crate::clock::Phase;
    use crate::data::Data;
    use crate::event::Event;
    use crate::order::Order;
    use crate::piece::{Load, PieceId, PieceKind};
    use crate::state::{GameState, Modules};
    use cna_data::scenario::Side;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// A game with `modules`, set up, the Axis declared Player A.
    fn declared(data: &Data, modules: Modules) -> GameState {
        let mut state = GameState::with_modules(data, 5, modules).unwrap();
        crate::state::test_support::auto_place(data, &mut state);
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        state
    }

    /// An Axis supply unit on the map, its hex given `points` motorisation
    /// points.
    fn supply_with_points(state: &mut GameState, points: u32) -> PieceId {
        let p = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::SupplyUnit && p.hex().is_some())
            .unwrap();
        let (id, hex) = (p.id.clone(), p.hex().unwrap().to_string());
        state
            .motorisation
            .entry(Side::Axis)
            .or_default()
            .insert(hex, points);
        id
    }

    fn pool(state: &GameState, id: &PieceId) -> u32 {
        let hex = state.pieces[id].hex().unwrap().to_string();
        state.motorisation[&Side::Axis]
            .get(&hex)
            .copied()
            .unwrap_or(0)
    }

    #[test]
    fn phase_c_waits_on_player_a_then_player_b_with_the_module_on() {
        let data = graziani();
        let mut state = declared(&data, Modules::ALL);
        assert_eq!(state.clock.phase, Phase::Organisation);
        assert_eq!(state.to_act(), Some(Side::Axis));
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(state.clock.phase, Phase::Organisation);
        assert_eq!(state.to_act(), Some(Side::Cw));
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.phase, Phase::ReserveDesignation);
        assert_eq!(state.to_act(), Some(Side::Axis));
    }

    #[test]
    fn phase_c_is_skipped_with_the_module_off() {
        let data = graziani();
        let state = declared(&data, Modules::default());
        assert_eq!(state.clock.phase, Phase::ReserveDesignation);
    }

    #[test]
    fn a_real_supply_unit_takes_thirty_points_from_its_hex_and_gives_them_back_32_31() {
        let data = graziani();
        let mut state = declared(&data, Modules::ALL);
        let id = supply_with_points(&mut state, 40);
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::AttachMotorisation { piece: id.clone() },
            )
            .unwrap();
        assert_eq!(
            events,
            vec![Event::MotorisationAttached {
                piece: id.clone(),
                points: 30
            }]
        );
        assert_eq!(state.pieces[&id].motorised, 30);
        assert_eq!(pool(&state, &id), 10);
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::AttachMotorisation { piece: id.clone() },
            )
            .unwrap_err();
        assert_eq!(err.case, "32.31");
        state
            .apply(
                &data,
                Side::Axis,
                Order::DetachMotorisation { piece: id.clone() },
            )
            .unwrap();
        assert_eq!(state.pieces[&id].motorised, 0);
        assert_eq!(pool(&state, &id), 40);
    }

    #[test]
    fn attaching_is_refused_short_of_points_outside_c_or_for_another_side_s_unit() {
        let data = graziani();
        let mut state = declared(&data, Modules::ALL);
        let id = supply_with_points(&mut state, 29);
        let before = state.clone();
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::AttachMotorisation { piece: id.clone() },
            )
            .unwrap_err();
        assert_eq!(err.case, "32.31");
        assert_eq!(state, before);
        let cw = state
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::SupplyUnit)
            .unwrap()
            .id
            .clone();
        let err = state
            .apply(&data, Side::Axis, Order::AttachMotorisation { piece: cw })
            .unwrap_err();
        assert_eq!(err.case, "32.31");
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        state
            .motorisation
            .get_mut(&Side::Axis)
            .unwrap()
            .insert(state.pieces[&id].hex().unwrap().to_string(), 40);
        let err = state
            .apply(&data, Side::Axis, Order::AttachMotorisation { piece: id })
            .unwrap_err();
        assert_eq!(err.case, "32.31");
    }

    #[test]
    fn loads_move_between_supply_units_in_one_hex_within_their_caps_32_1x() {
        let data = graziani();
        let mut state = declared(&data, Modules::ALL);
        let ids: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| {
                p.side == Side::Axis && p.kind == PieceKind::SupplyUnit && p.hex().is_some()
            })
            .take(2)
            .map(|p| p.id.clone())
            .collect();
        let here = state.pieces[&ids[0]].location.clone();
        state.pieces.get_mut(&ids[1]).unwrap().location = here;
        state.pieces.get_mut(&ids[0]).unwrap().load = Some(Load { ammo: 20, fuel: 30 });
        state.pieces.get_mut(&ids[1]).unwrap().load = Some(Load { ammo: 30, fuel: 50 });
        let order = |ammo, fuel| Order::TransferLoad {
            from: ids[0].clone(),
            to: ids[1].clone(),
            ammo,
            fuel,
        };
        // 50 + 20 fuel is past the 60 cap.
        let err = state.apply(&data, Side::Axis, order(0, 20)).unwrap_err();
        assert_eq!(err.case, "32.1x");
        state.apply(&data, Side::Axis, order(10, 10)).unwrap();
        assert_eq!(
            state.pieces[&ids[0]].load,
            Some(Load { ammo: 10, fuel: 20 })
        );
        assert_eq!(
            state.pieces[&ids[1]].load,
            Some(Load { ammo: 40, fuel: 60 })
        );
    }

    /// A module-on game in the Axis' first movement segment, with an
    /// Axis supply unit (motorised by `points`, loaded) stacked with a
    /// combat unit, and a one-hex move the pair may make.
    fn supply_and_escort(data: &Data, points: u32) -> (GameState, PieceId, PieceId, Order) {
        let mut state = declared(data, Modules::ALL);
        for side in [Side::Axis, Side::Cw, Side::Axis] {
            state.apply(data, side, Order::Pass).unwrap();
        }
        let supply = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::SupplyUnit && p.hex().is_some())
            .unwrap()
            .id
            .clone();
        let escorts: Vec<(PieceId, cna_data::map::HexId)> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::Combat && p.reserve.is_none())
            .filter_map(|p| Some((p.id.clone(), p.hex()?.clone())))
            .collect();
        for (escort, at) in escorts {
            let mut s = state.clone();
            let q = s.pieces.get_mut(&supply).unwrap();
            q.location = crate::piece::Location::Hex(at.clone());
            q.motorised = 30;
            q.load = Some(Load { ammo: 10, fuel: 50 });
            for n in data.map.neighbours(&at) {
                let order = Order::Move {
                    pieces: vec![escort.clone(), supply.clone()],
                    path: vec![n],
                    assault: false,
                    fuel_from: None,
                };
                if s.clone().apply(data, Side::Axis, order.clone()).is_ok() {
                    s.pieces.get_mut(&supply).unwrap().motorised = points;
                    return (s, supply, escort, order);
                }
            }
        }
        panic!("no escort can take a supply unit anywhere");
    }

    #[test]
    fn a_motorised_supply_unit_moves_with_a_combat_unit_in_g_32_31() {
        let data = graziani();
        let (mut state, supply, escort, order) = supply_and_escort(&data, 30);
        state.apply(&data, Side::Axis, order).unwrap();
        assert_eq!(
            state.pieces[&supply].location,
            state.pieces[&escort].location
        );
        assert_eq!(state.cpa(&state.pieces[&supply]), Some(15));
    }

    #[test]
    fn a_supply_unit_without_points_or_without_an_escort_may_not_move_32_31() {
        let data = graziani();
        let (mut state, _, _, order) = supply_and_escort(&data, 0);
        let err = state.apply(&data, Side::Axis, order).unwrap_err();
        assert_eq!(err.case, "32.31");
        let (mut state, supply, _, order) = supply_and_escort(&data, 30);
        let Order::Move { path, .. } = order else {
            unreachable!()
        };
        let alone = Order::Move {
            pieces: vec![supply],
            path,
            assault: false,
            fuel_from: None,
        };
        let err = state.apply(&data, Side::Axis, alone).unwrap_err();
        assert_eq!(err.case, "32.31");
    }

    /// Plays the default answers (as `pass_stage` does) until phase H or
    /// the end of the Axis half.
    fn to_truck_convoy(data: &Data, state: &mut GameState) {
        use crate::state::Pending;
        use crate::state::test_support::through_combat;
        while state.clock.phase != Phase::TruckConvoy
            && state.clock.half != Some(crate::clock::Half::B)
        {
            let player = state.to_act().unwrap();
            if state.clock.phase == Phase::MovementCombat
                && state.clock.segment == Some(crate::clock::Segment::Movement)
                && state.pending.is_none()
                && state.sides_adjacent(data)
            {
                through_combat(data, state);
                continue;
            }
            let order = match &state.pending {
                Some(Pending::DeclareZoc { .. }) => Order::DeclareZoc { exerting: vec![] },
                _ => Order::Pass,
            };
            state.apply(data, player, order).unwrap();
        }
    }

    #[test]
    fn phase_h_waits_when_the_phasing_player_has_a_motorised_supply_unit_32_31() {
        let data = graziani();
        let (mut state, supply, _, order) = supply_and_escort(&data, 30);
        let Order::Move { path, .. } = order else {
            unreachable!()
        };
        to_truck_convoy(&data, &mut state);
        assert_eq!(state.clock.phase, Phase::TruckConvoy);
        assert_eq!(state.to_act(), Some(Side::Axis));
        // A combat unit may not move in H; the supply unit moves alone.
        let combat = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat && p.hex().is_some())
            .unwrap()
            .id
            .clone();
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Move {
                    pieces: vec![combat],
                    path: path.clone(),
                    assault: false,
                    fuel_from: None,
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "32.31");
        state
            .apply(
                &data,
                Side::Axis,
                Order::Move {
                    pieces: vec![supply.clone()],
                    path: path.clone(),
                    assault: false,
                    fuel_from: None,
                },
            )
            .unwrap();
        assert_eq!(state.pieces[&supply].hex(), path.last());
        state.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_ne!(state.clock.phase, Phase::TruckConvoy);
    }

    #[test]
    fn phase_h_is_skipped_without_a_motorised_supply_unit() {
        let data = graziani();
        let (mut state, _, _, _) = supply_and_escort(&data, 0);
        to_truck_convoy(&data, &mut state);
        assert_ne!(state.clock.phase, Phase::TruckConvoy);
    }

    #[test]
    fn a_supply_unit_that_moved_in_g_may_not_move_in_h_32_31() {
        let data = graziani();
        let (mut state, supply, escort, order) = supply_and_escort(&data, 30);
        state.apply(&data, Side::Axis, order).unwrap();
        let at = state.pieces[&escort].hex().unwrap().clone();
        // H opens for another motorised supply unit; the escorted one has
        // nothing to do there.
        let other = state
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Axis
                    && p.kind == PieceKind::SupplyUnit
                    && p.hex().is_some()
                    && p.id != supply
            })
            .unwrap()
            .id
            .clone();
        state.pieces.get_mut(&other).unwrap().motorised = 30;
        to_truck_convoy(&data, &mut state);
        assert_eq!(state.clock.phase, Phase::TruckConvoy);
        let next = data.map.neighbours(&at)[0].clone();
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Move {
                    pieces: vec![supply],
                    path: vec![next],
                    assault: false,
                    fuel_from: None,
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "32.31");
    }

    #[test]
    fn destroying_one_s_own_supply_costs_half_the_cpa_and_a_die_32_1x() {
        let data = graziani();
        let (mut state, supply, escort, order) = supply_and_escort(&data, 30);
        let hex = state.pieces[&supply].hex().unwrap().clone();
        let cpa = state.cpa(&state.pieces[&escort]).unwrap();
        let sp = state.stacked(&data, &hex);
        let events = state
            .apply(&data, Side::Axis, Order::DestroySupply { hex: hex.clone() })
            .unwrap();
        let roll = events
            .iter()
            .find_map(|e| match e {
                Event::DieRolled { purpose, value } if purpose == "destroy supply" => Some(*value),
                _ => None,
            })
            .expect("one die per supply unit");
        let destroyed = events.contains(&Event::SupplyDestroyed {
            piece: supply.clone(),
        });
        assert_eq!(destroyed, roll + u8::from(sp <= 1) <= 3, "{events:?}");
        assert_eq!(
            state.pieces[&escort].cp_spent,
            cna_data::cp::Cp::whole(cpa.div_ceil(2))
        );
        // One attempt per unit per segment.
        if !destroyed {
            let err = state
                .apply(&data, Side::Axis, Order::DestroySupply { hex: hex.clone() })
                .unwrap_err();
            assert_eq!(err.case, "32.1x");
        }
        // Not once anything has moved this segment.
        let (mut state, _, _, order2) = supply_and_escort(&data, 30);
        let _ = order;
        state.apply(&data, Side::Axis, order2).unwrap();
        let err = state
            .apply(&data, Side::Axis, Order::DestroySupply { hex })
            .unwrap_err();
        assert_eq!(err.case, "32.1x");
    }
}
