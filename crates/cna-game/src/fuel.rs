//! Fuel for land moves under abstract supply (SPI 32.22): who owes it,
//! which supply unit pays, and the debit.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Load, Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::characteristics::Equivalent;
use cna_data::cp::Cp;
use cna_rules::supply::{Carrying, FuelClass, components, fuel_owed};
use std::collections::BTreeMap;

/// One payment: `fuel` points from the supply unit `from` for `piece`.
pub(crate) struct Payment {
    pub piece: PieceId,
    pub from: PieceId,
    pub fuel: u32,
}

impl GameState {
    /// What burns fuel, read from the characteristics row's unit type:
    /// tanks; gun-class (artillery, anti-tank, anti-air) and
    /// reconnaissance; camel reconnaissance. Everything else, HQs
    /// included, is `Other` (32.22).
    pub(crate) fn fuel_class(&self, data: &Data, piece: &Piece) -> FuelClass {
        if piece.kind != PieceKind::Combat {
            return FuelClass::Other;
        }
        let Some(row) = piece
            .nation
            .zip(piece.id_code.as_deref())
            .and_then(|(n, code)| data.chars.row(n, code))
        else {
            return FuelClass::Other;
        };
        let t = row.unit_type.as_str();
        if t.contains("Camel") {
            FuelClass::CamelRecce
        } else if t.starts_with("Tank") {
            FuelClass::Tank
        } else if t.contains("Recce")
            || t.contains("Reconnaissance")
            || t.starts_with("Artillery")
            || t.starts_with("Anti-tank")
            || t.contains("Anti-air")
        {
            FuelClass::GunOrRecce
        } else {
            FuelClass::Other
        }
    }

    /// Fuel a piece owes for the move it is about to make, which costs
    /// `total` CP: once for the stage's first move, and once more for the
    /// first spending past its CPA (32.22). Returns the sum and whether
    /// each kind falls due now.
    fn fuel_due(&self, data: &Data, piece: &Piece, total: Cp) -> (u32, bool, bool) {
        // A brigade counter pays by its battalion components.
        let (eq, parts) = components(piece.equivalent.unwrap_or(Equivalent::Hq));
        let each = parts * fuel_owed(self.fuel_class(data, piece), eq, Carrying::Nothing);
        if each == 0 {
            return (0, false, false);
        }
        let first = !piece.fuelled;
        let cpa = Cp::whole(self.cpa(piece).unwrap_or(0));
        let over = !piece.fuelled_over && piece.cp_spent + total > cpa;
        let n = u32::from(first) + u32::from(over);
        (each * n, first, over)
    }

    /// Chooses the supply unit that pays each fuel-using mover, or
    /// refuses with 32.22 when something owed cannot be paid. Nothing is
    /// changed. `source` is the player's choice for every payer, else the
    /// nearest in range with the fuel, ties by id.
    pub(crate) fn plan_fuel(
        &self,
        data: &Data,
        pieces: &[PieceId],
        total: Cp,
        source: Option<&PieceId>,
    ) -> Result<Vec<Payment>, Illegal> {
        if !self.modules.abstract_supply {
            return Ok(Vec::new());
        }
        let mut left: BTreeMap<PieceId, u32> = BTreeMap::new();
        let mut payments = Vec::new();
        for id in pieces {
            let p = &self.pieces[id];
            let (owed, _, _) = self.fuel_due(data, p, total);
            if owed == 0 {
                continue;
            }
            let here = p.hex().expect("a mover stands on the map");
            let have = |q: &PieceId, left: &BTreeMap<PieceId, u32>| {
                left.get(q)
                    .copied()
                    .or_else(|| self.pieces[q].load.map(|l| l.fuel))
                    .unwrap_or(0)
            };
            let in_range = self.supply_in_range(data, id);
            let from = match source {
                Some(s) if in_range.contains(s) && have(s, &left) >= owed => s.clone(),
                Some(s) => {
                    return Err(Illegal::new(
                        "32.22",
                        format!("{id} owes {owed} fuel and {s} cannot pay it from within range"),
                    ));
                }
                None => in_range
                    .iter()
                    .filter(|q| have(q, &left) >= owed)
                    .min_by_key(|q| {
                        let at = self.pieces[*q].hex().expect("in range means on the map");
                        (Self::distance(data, here, at, 64), (*q).clone())
                    })
                    .cloned()
                    .ok_or_else(|| {
                        Illegal::new(
                            "32.22",
                            format!("{id} owes {owed} fuel and no supply unit in range can pay it"),
                        )
                    })?,
            };
            *left
                .entry(from.clone())
                .or_insert_with(|| have(&from, &BTreeMap::new())) -= owed;
            payments.push(Payment {
                piece: id.clone(),
                from,
                fuel: owed,
            });
        }
        Ok(payments)
    }

    /// Debits what `plan_fuel` chose and marks the movers fuelled.
    pub(crate) fn spend_fuel(
        &mut self,
        data: &Data,
        total: Cp,
        payments: Vec<Payment>,
        events: &mut Vec<Event>,
    ) {
        for pay in payments {
            let (_, first, over) = self.fuel_due(data, &self.pieces[&pay.piece], total);
            let p = self.pieces.get_mut(&pay.piece).expect("checked");
            p.fuelled |= first;
            p.fuelled_over |= over;
            let supply = self.pieces.get_mut(&pay.from).expect("checked");
            let load = supply.load.as_mut().expect("a supply unit with a load");
            load.fuel -= pay.fuel;
            events.push(Event::FuelSpent {
                piece: pay.piece,
                from: pay.from.clone(),
                fuel: pay.fuel,
            });
            self.retire_if_empty(&pay.from, events);
        }
    }

    /// A supply unit with neither ammunition nor fuel left is removed
    /// (32.1x).
    pub(crate) fn retire_if_empty(&mut self, id: &PieceId, events: &mut Vec<Event>) {
        let p = self.pieces.get_mut(id).expect("checked");
        if p.load == Some(Load { ammo: 0, fuel: 0 }) {
            p.location = Location::Eliminated;
            events.push(Event::SupplyUnitSpent { piece: id.clone() });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Data;
    use crate::event::Event;
    use crate::movement::test_support::moving;
    use crate::order::Order;
    use crate::piece::{Load, Location, PieceId, PieceKind};
    use crate::state::{GameState, Modules};
    use cna_data::characteristics::Equivalent;
    use cna_data::cp::Cp;
    use cna_data::scenario::Side;
    use cna_rules::supply::FuelClass;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// A module-on game in the first movement segment, `Axis` phasing.
    fn game(data: &Data) -> GameState {
        let mut state = GameState::with_modules(data, 5, Modules::ALL).unwrap();
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
        state.apply(data, Side::Axis, Order::Pass).unwrap();
        state
    }

    /// Axis supply units stand on `at` with `fuel` each; the rest are
    /// spent (0/0 and out of range).
    fn supply_at(state: &mut GameState, at: &cna_data::map::HexId, fuel: u32) -> Vec<PieceId> {
        let ids: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::SupplyUnit)
            .map(|p| p.id.clone())
            .collect();
        for (i, id) in ids.iter().enumerate() {
            let p = state.pieces.get_mut(id).unwrap();
            if i == 0 {
                p.location = Location::Hex(at.clone());
                p.load = Some(Load { ammo: 10, fuel });
            } else {
                p.load = Some(Load { ammo: 0, fuel: 0 });
            }
        }
        ids
    }

    /// An Axis tank battalion and a legal one-hex move for it.
    fn tank_and_move(data: &Data, state: &GameState) -> (PieceId, Order) {
        for p in state.pieces.values() {
            if p.side != Side::Axis || p.kind != PieceKind::Combat || p.hex().is_none() {
                continue;
            }
            if state.fuel_class(data, p) != FuelClass::Tank || p.reserve.is_some() {
                continue;
            }
            for n in data.map.neighbours(p.hex().unwrap()) {
                let order = Order::Move {
                    pieces: vec![p.id.clone()],
                    path: vec![n],
                    assault: false,
                    fuel_from: None,
                };
                // Probe on a copy with the module off: only the move.
                let mut probe = state.clone();
                probe.modules = Modules::default();
                if probe.apply(data, Side::Axis, order.clone()).is_ok() {
                    return (p.id.clone(), order);
                }
            }
        }
        panic!("no tank can move");
    }

    #[test]
    fn a_tank_with_no_supply_in_range_is_refused_32_22() {
        let data = graziani();
        let mut state = game(&data);
        let (tank, order) = tank_and_move(&data, &state);
        let far = data.map.within(state.pieces[&tank].hex().unwrap(), 40);
        let far = far.iter().next_back().unwrap().clone();
        supply_at(&mut state, &far, 60);
        let before = state.clone();
        let err = state.apply(&data, Side::Axis, order).unwrap_err();
        assert!(format!("{err:?}").contains("32.22"), "{err:?}");
        assert_eq!(state, before);
    }

    #[test]
    fn the_first_move_pays_two_fuel_and_a_second_pays_nothing() {
        let data = graziani();
        let mut state = game(&data);
        let (tank, order) = tank_and_move(&data, &state);
        let here = state.pieces[&tank].hex().unwrap().clone();
        let ids = supply_at(&mut state, &here, 60);
        let events = state.apply(&data, Side::Axis, order.clone()).unwrap();
        assert!(events.contains(&Event::FuelSpent {
            piece: tank.clone(),
            from: ids[0].clone(),
            fuel: 2
        }));
        assert_eq!(state.pieces[&ids[0]].load.unwrap().fuel, 58);
        // Back where it started, in the same stage: free.
        let back = Order::Move {
            pieces: vec![tank.clone()],
            path: vec![here],
            assault: false,
            fuel_from: None,
        };
        let events = state.apply(&data, Side::Axis, back).unwrap();
        assert!(!events.iter().any(|e| matches!(e, Event::FuelSpent { .. })));
        assert_eq!(state.pieces[&ids[0]].load.unwrap().fuel, 58);
    }

    #[test]
    fn a_brigade_equivalent_tank_counter_pays_two_per_battalion_32_22() {
        let data = graziani();
        let mut state = game(&data);
        let (tank, order) = tank_and_move(&data, &state);
        state.pieces.get_mut(&tank).unwrap().equivalent = Some(Equivalent::Brigade);
        let here = state.pieces[&tank].hex().unwrap().clone();
        let ids = supply_at(&mut state, &here, 60);
        let events = state.apply(&data, Side::Axis, order).unwrap();
        assert!(events.contains(&Event::FuelSpent {
            piece: tank,
            from: ids[0].clone(),
            fuel: 6
        }));
    }

    #[test]
    fn a_foot_unit_and_a_module_off_game_pay_nothing() {
        let data = graziani();
        let mut off = moving(&data, 5, Side::Axis);
        let (tank, order) = tank_and_move(&data, &game(&data));
        let events = off.apply(&data, Side::Axis, order).unwrap();
        assert!(!events.iter().any(|e| matches!(e, Event::FuelSpent { .. })));
        assert!(off.pieces[&tank].load.is_none());
    }

    #[test]
    fn an_empty_supply_unit_is_removed() {
        let data = graziani();
        let mut state = game(&data);
        let (tank, order) = tank_and_move(&data, &state);
        let here = state.pieces[&tank].hex().unwrap().clone();
        let ids = supply_at(&mut state, &here, 2);
        state.pieces.get_mut(&ids[0]).unwrap().load = Some(Load { ammo: 0, fuel: 2 });
        let events = state.apply(&data, Side::Axis, order).unwrap();
        assert!(events.contains(&Event::SupplyUnitSpent {
            piece: ids[0].clone()
        }));
        assert_eq!(state.pieces[&ids[0]].location, Location::Eliminated);
    }

    #[test]
    fn spending_past_the_cpa_pays_again_once_32_22() {
        let data = graziani();
        let mut state = game(&data);
        let (tank, order) = tank_and_move(&data, &state);
        let here = state.pieces[&tank].hex().unwrap().clone();
        let ids = supply_at(&mut state, &here, 60);
        let cpa = state.cpa(&state.pieces[&tank]).unwrap();
        let p = state.pieces.get_mut(&tank).unwrap();
        p.fuelled = true;
        p.cp_spent = Cp::whole(cpa - 1);
        state.apply(&data, Side::Axis, order).unwrap();
        assert_eq!(state.pieces[&ids[0]].load.unwrap().fuel, 58);
        assert!(state.pieces[&tank].fuelled_over);
        let back = Order::Move {
            pieces: vec![tank],
            path: vec![here],
            assault: false,
            fuel_from: None,
        };
        state.apply(&data, Side::Axis, back).unwrap();
        assert_eq!(state.pieces[&ids[0]].load.unwrap().fuel, 58);
    }

    #[test]
    fn a_named_source_that_cannot_pay_is_refused_and_one_that_can_is_used() {
        let data = graziani();
        let mut state = game(&data);
        let (tank, order) = tank_and_move(&data, &state);
        let here = state.pieces[&tank].hex().unwrap().clone();
        let ids = supply_at(&mut state, &here, 60);
        let named = |from: &PieceId| match order.clone() {
            Order::Move {
                pieces,
                path,
                assault,
                ..
            } => Order::Move {
                pieces,
                path,
                assault,
                fuel_from: Some(from.clone()),
            },
            _ => unreachable!(),
        };
        // ids[1] is spent: it is not in range for anything.
        let before = state.clone();
        let err = state.apply(&data, Side::Axis, named(&ids[1])).unwrap_err();
        assert!(format!("{err:?}").contains("32.22"));
        assert_eq!(state, before);
        let events = state.apply(&data, Side::Axis, named(&ids[0])).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::FuelSpent { from, .. } if *from == ids[0]))
        );
    }

    #[test]
    fn an_old_move_order_reads_and_writes_without_fuel_from() {
        let json =
            r#"{"move":{"pieces":["unit:it:iim-tank-bn"],"path":["C3517"],"assault":false}}"#;
        let order: Order = serde_json::from_str(json).unwrap();
        assert!(matches!(
            &order,
            Order::Move {
                fuel_from: None,
                ..
            }
        ));
        assert_eq!(serde_json::to_string(&order).unwrap(), json);
    }
}
