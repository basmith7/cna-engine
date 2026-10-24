//! Ammunition under abstract supply (SPI 32.21, R-013): who owes it,
//! which supply unit pays, and the debit.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{PieceId, PieceKind};
use crate::state::GameState;
use cna_data::characteristics::Equivalent;
use cna_rules::supply::{Role, ammo_owed, components};
use std::collections::BTreeMap;

/// One payment: `ammo` points from the supply unit `from` for `piece`.
pub(crate) struct AmmoPayment {
    pub piece: PieceId,
    pub from: PieceId,
    pub ammo: u32,
}

/// What `plan_ammo` found: the payments that can be made and the
/// payers (with what they owe) that no supply unit in range can cover.
#[derive(Default)]
pub(crate) struct AmmoPlan {
    pub payments: Vec<AmmoPayment>,
    pub short: Vec<(PieceId, u32)>,
}

impl AmmoPlan {
    /// The refusal for the first payer left short (32.21).
    pub(crate) fn refuse(&self) -> Result<(), Illegal> {
        match self.short.first() {
            Some((id, owed)) => Err(Illegal::new(
                "32.21",
                format!("{id} owes {owed} ammunition and no supply unit in range can pay it"),
            )),
            None => Ok(()),
        }
    }
}

impl GameState {
    /// Ammunition one counter owes in `role`: per battalion-equivalent,
    /// a brigade counter counting as three; companies pay none.
    fn ammo_due(&self, id: &PieceId, role: Role, phasing: bool) -> u32 {
        let Some(eq) = self.pieces[id].equivalent else {
            return 0;
        };
        let (each, n) = components(eq);
        if each != Equivalent::Battalion {
            return 0;
        }
        ammo_owed(each, role, phasing) * n
    }

    /// Plans each payer's ammunition: the nearest supply unit in range
    /// that can pay, ties by id. Nothing is changed. Module off: no one
    /// owes anything.
    pub(crate) fn plan_ammo(
        &self,
        data: &Data,
        payers: &[PieceId],
        role: Role,
        phasing: bool,
    ) -> AmmoPlan {
        let mut plan = AmmoPlan::default();
        if !self.modules.abstract_supply {
            return plan;
        }
        let mut left: BTreeMap<PieceId, u32> = BTreeMap::new();
        for id in payers {
            let p = &self.pieces[id];
            if p.kind != PieceKind::Combat {
                continue;
            }
            let owed = self.ammo_due(id, role, phasing);
            if owed == 0 {
                continue;
            }
            let Some(here) = p.hex() else { continue };
            let have = |q: &PieceId, left: &BTreeMap<PieceId, u32>| {
                left.get(q)
                    .copied()
                    .or_else(|| self.pieces[q].load.map(|l| l.ammo))
                    .unwrap_or(0)
            };
            let from = self
                .supply_in_range(data, id)
                .into_iter()
                .filter(|q| have(q, &left) >= owed)
                .min_by_key(|q| {
                    let at = self.pieces[q].hex().expect("in range means on the map");
                    (Self::distance(data, here, at, 64), q.clone())
                });
            match from {
                Some(from) => {
                    let h = have(&from, &left);
                    left.insert(from.clone(), h - owed);
                    plan.payments.push(AmmoPayment {
                        piece: id.clone(),
                        from,
                        ammo: owed,
                    });
                }
                None => plan.short.push((id.clone(), owed)),
            }
        }
        plan
    }

    /// What one counter would owe in `role` and whether nothing in range
    /// can pay it, for the sealed options (32.21); (0, false) with the
    /// module off.
    pub(crate) fn ammo_quote(
        &self,
        data: &Data,
        id: &PieceId,
        role: Role,
        phasing: bool,
    ) -> (u32, bool) {
        let plan = self.plan_ammo(data, std::slice::from_ref(id), role, phasing);
        let paid: u32 = plan.payments.iter().map(|p| p.ammo).sum();
        let short: u32 = plan.short.iter().map(|(_, owed)| owed).sum();
        (paid + short, short > 0)
    }

    /// Debits what `plan_ammo` chose.
    pub(crate) fn spend_ammo(&mut self, payments: Vec<AmmoPayment>, events: &mut Vec<Event>) {
        for pay in payments {
            let supply = self.pieces.get_mut(&pay.from).expect("checked");
            let load = supply.load.as_mut().expect("a supply unit with a load");
            load.ammo -= pay.ammo;
            events.push(Event::AmmunitionSpent {
                piece: pay.piece,
                from: pay.from.clone(),
                ammo: pay.ammo,
            });
            self.retire_if_empty(&pay.from, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::combat::options::SealedOptions;
    use crate::combat::segment::test_support::at_sollum;
    use crate::data::Data;
    use crate::event::Event;
    use crate::movement::test_support::{hex, pid};
    use crate::order::{Allocation, Assignment, Fire, Order};
    use crate::piece::{Load, Location, PieceId, PieceKind};
    use crate::state::{GameState, Modules};
    use cna_data::scenario::Side;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// Sollum with the module on, guns set, and every Commonwealth
    /// supply unit spent but the first, which stands at `at` with `ammo`.
    fn sollum(data: &Data, at: &str, ammo: u32) -> (GameState, PieceId) {
        let mut state = at_sollum(data);
        state.modules = Modules::ALL;
        let ids: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::SupplyUnit)
            .map(|p| p.id.clone())
            .collect();
        for (i, id) in ids.iter().enumerate() {
            let p = state.pieces.get_mut(id).unwrap();
            if i == 0 {
                p.location = Location::Hex(hex(at));
                p.load = Some(Load { ammo, fuel: 5 });
            } else {
                p.load = Some(Load { ammo: 0, fuel: 0 });
            }
        }
        for (side, forward) in [(Side::Cw, vec![pid("unit:cw:4-rha")]), (Side::Axis, vec![])] {
            state
                .apply(data, side, Order::GunPositions { forward })
                .unwrap();
        }
        (state, ids[0].clone())
    }

    fn barrage() -> Order {
        Order::Barrage {
            fires: vec![Fire {
                piece: pid("unit:cw:4-rha"),
                points: 4,
                target: pid("unit:it:viii-libyan-bn"),
            }],
        }
    }

    /// The sealed options' ammunition quote for `piece`: (owed, short).
    fn quoted(state: &GameState, data: &Data, side: Side, piece: &str) -> (u32, bool) {
        match state.sealed_options(data, side).unwrap() {
            SealedOptions::Barrage { firers } => {
                let f = firers.iter().find(|f| f.piece == pid(piece)).unwrap();
                (f.ammo, f.ammo_short)
            }
            SealedOptions::AssignForces { units, .. } => {
                let u = units.iter().find(|u| u.piece == pid(piece)).unwrap();
                (u.ammo, u.ammo_short)
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_barrage_options_quote_each_firer_s_ammunition() {
        let data = graziani();
        let (state, _) = sollum(&data, "C4021", 10);
        assert_eq!(quoted(&state, &data, Side::Cw, "unit:cw:4-rha"), (4, false));
        let (state, _) = sollum(&data, "C4021", 3);
        assert_eq!(quoted(&state, &data, Side::Cw, "unit:cw:4-rha"), (4, true));
        let (mut state, _) = sollum(&data, "C4021", 3);
        state.modules = Modules::default();
        assert_eq!(quoted(&state, &data, Side::Cw, "unit:cw:4-rha"), (0, false));
    }

    #[test]
    fn the_assign_options_quote_what_an_assault_would_cost() {
        let data = graziani();
        let (state, _) = assigning(&data, "C4021", 1);
        let (owed, short) = quoted(&state, &data, Side::Cw, "unit:cw:1-krrc");
        assert!(owed > 1 && short, "{owed} {short}");
        let (state, _) = assigning(&data, "C4021", 40);
        assert_eq!(
            quoted(&state, &data, Side::Cw, "unit:cw:1-krrc"),
            (owed, false)
        );
    }

    #[test]
    fn a_barrage_nobody_in_range_can_pay_is_refused_32_21() {
        let data = graziani();
        let (mut state, _) = sollum(&data, "C4021", 3);
        let before = state.clone();
        let err = state.apply(&data, Side::Cw, barrage()).unwrap_err();
        assert!(format!("{err:?}").contains("32.21"), "{err:?}");
        assert_eq!(state, before);
    }

    #[test]
    fn a_barrage_pays_at_the_roll_from_the_supply_unit_in_range() {
        let data = graziani();
        let (mut state, supply) = sollum(&data, "C4021", 10);
        state.apply(&data, Side::Cw, barrage()).unwrap();
        assert_eq!(state.pieces[&supply].load.unwrap().ammo, 10, "not yet");
        let events = state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        let paid: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::AmmunitionSpent { piece, from, ammo } => Some((piece, from, *ammo)),
                _ => None,
            })
            .collect();
        assert_eq!(paid, vec![(&pid("unit:cw:4-rha"), &supply, 4)]);
        assert_eq!(state.pieces[&supply].load.unwrap().ammo, 6);
    }

    #[test]
    fn a_supply_unit_drained_by_ammunition_and_fuel_is_removed() {
        let data = graziani();
        let (mut state, supply) = sollum(&data, "C4021", 4);
        state.pieces.get_mut(&supply).unwrap().load = Some(Load { ammo: 4, fuel: 0 });
        state.apply(&data, Side::Cw, barrage()).unwrap();
        let events = state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        assert!(events.contains(&Event::SupplyUnitSpent {
            piece: supply.clone()
        }));
        assert_eq!(state.pieces[&supply].location, Location::Eliminated);
    }

    #[test]
    fn a_module_off_barrage_pays_nothing() {
        let data = graziani();
        let (mut state, supply) = sollum(&data, "C4021", 10);
        state.modules = Modules::default();
        state.apply(&data, Side::Cw, barrage()).unwrap();
        state
            .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
            .unwrap();
        assert_eq!(state.pieces[&supply].load.unwrap().ammo, 10);
    }

    /// Steps 1-3 passed with nothing done, the module on, and every
    /// supply unit but one Commonwealth unit (at `at`, with `ammo`) spent.
    fn assigning(data: &Data, at: &str, ammo: u32) -> (GameState, PieceId) {
        let (mut state, supply) = sollum(data, at, ammo);
        for id in state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::SupplyUnit)
            .map(|p| p.id.clone())
            .collect::<Vec<_>>()
        {
            state.pieces.get_mut(&id).unwrap().load = Some(Load { ammo: 0, fuel: 0 });
        }
        for (side, order) in [
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Pass),
        ] {
            state.apply(data, side, order).unwrap();
        }
        (state, supply)
    }

    fn assault_krrc() -> Order {
        Order::AssignForces {
            assignments: vec![Assignment {
                piece: pid("unit:cw:1-krrc"),
                anti_armour: vec![],
                close_assault: vec![Allocation {
                    target: Some(hex("C4020")),
                    points: 6,
                }],
            }],
        }
    }

    #[test]
    fn an_assault_the_attacker_cannot_pay_for_is_refused_32_21() {
        let data = graziani();
        let (mut state, _) = assigning(&data, "C4021", 1);
        let before = state.clone();
        let err = state.apply(&data, Side::Cw, assault_krrc()).unwrap_err();
        assert!(format!("{err:?}").contains("32.21"), "{err:?}");
        assert_eq!(state, before);
    }

    #[test]
    fn attackers_pay_double_and_a_defender_who_cannot_pay_still_fights() {
        let data = graziani();
        let (mut state, supply) = assigning(&data, "C4021", 10);
        state.apply(&data, Side::Cw, assault_krrc()).unwrap();
        state
            .apply(
                &data,
                Side::Axis,
                Order::AssignForces {
                    assignments: vec![],
                },
            )
            .unwrap();
        let events = state
            .apply(
                &data,
                Side::Cw,
                Order::Assault {
                    target: hex("C4020"),
                },
            )
            .unwrap();
        let paid: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                Event::AmmunitionSpent { piece, ammo, .. } => Some((piece.clone(), *ammo)),
                _ => None,
            })
            .collect();
        assert_eq!(paid, vec![(pid("unit:cw:1-krrc"), 2)]);
        assert_eq!(state.pieces[&supply].load.unwrap().ammo, 8);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AmmunitionShort { .. })),
            "the Libyan defenders have no supply unit in range"
        );
        assert!(events.iter().any(|e| matches!(e, Event::Assault { .. })));
    }
}
