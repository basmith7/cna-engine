//! The air lift under abstract supply (32.37, R-124): once a Game-Turn
//! each player may fly one supply unit in phase C, for 12 fuel from a
//! supply unit at the origin, between major cities, villages and the
//! Tripoli and Tunisia boxes, to a friendly division or brigade HQ no
//! farther forward than the farthest friendly division HQ.

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::{HexId, Settlement};
use cna_data::oa::Echelon;
use cna_data::scenario::{OffMapBox, Side};

/// The fuel one lift costs (32.37).
const LIFT_FUEL: u32 = 12;
/// No lift from within this many hexes of an enemy combat unit (32.37).
const ENEMY_CLEARANCE: u32 = 5;
const CASE: &str = "32.37";

/// West to east: the sheet, then the column within it.
fn east_west(hex: &HexId) -> (char, u8) {
    (hex.sheet(), hex.col())
}

impl GameState {
    fn echelon(&self, data: &Data, p: &Piece) -> Option<Echelon> {
        if p.kind != PieceKind::Hq {
            return None;
        }
        data.oa.unit(&p.id.unit()?)?.echelon
    }

    /// A major city or village hex, or the Tripoli or Tunisia box.
    fn airfield_place(data: &Data, at: &Location) -> bool {
        match at {
            Location::Hex(h) => data.map.hex(h).is_some_and(|x| {
                matches!(
                    x.settlement,
                    Some(Settlement::MajorCity | Settlement::Village)
                )
            }),
            Location::Box(b) => matches!(b, OffMapBox::Tripoli | OffMapBox::Tunisia),
            _ => false,
        }
    }

    /// `Order::AirLift` (32.37).
    pub(crate) fn air_lift(
        &mut self,
        data: &Data,
        player: Side,
        id: &PieceId,
        to: &Location,
        fuel_from: Option<&PieceId>,
    ) -> Result<Vec<Event>, Illegal> {
        let refuse = |why: String| Err(Illegal::new(CASE, why));
        if self.clock.phase != Phase::Organisation || !self.modules.abstract_supply {
            return refuse(format!("supply is flown in phase C, not at {}", self.clock));
        }
        let turn = self.clock.game_turn;
        if self.air_lifted.get(&player) == Some(&turn) {
            return refuse(format!("{player} has flown a supply unit this Game-Turn"));
        }
        let Some(p) = self.pieces.get(id) else {
            return refuse(format!("no piece {id}"));
        };
        if p.side != player || !p.kind.is_supply() {
            return refuse(format!("{id} is not one of {player}'s supply units"));
        }
        if p.motorised > 0 {
            return refuse(format!(
                "{id} carries motorisation points: detach them before it flies"
            ));
        }
        let from = p.location.clone();
        let name = to.key().unwrap_or_else(|| "nowhere".into());
        if !Self::airfield_place(data, &from) {
            return refuse(format!(
                "{id} is not in a major city, a village or the Tripoli or Tunisia box"
            ));
        }
        if !Self::airfield_place(data, to) {
            return refuse(format!(
                "{name} is not a major city, a village or the Tripoli or Tunisia box"
            ));
        }
        if *to == from {
            return refuse(format!("{id} is already there"));
        }
        if let Location::Hex(h) = &from {
            let near = data.map.within(h, ENEMY_CLEARANCE);
            if let Some(e) = self.pieces.values().find(|q| {
                q.side != player
                    && q.kind == PieceKind::Combat
                    && q.hex().is_some_and(|x| near.contains(x))
            }) {
                return refuse(format!(
                    "{} stands within {ENEMY_CLEARANCE} hexes of {h}: nothing flies from there",
                    e.id
                ));
            }
        }
        let hqs_at = |at: &Location, echelons: &[Echelon]| {
            self.pieces.values().any(|q| {
                q.side == player
                    && q.location == *at
                    && self.echelon(data, q).is_some_and(|e| echelons.contains(&e))
            })
        };
        if !hqs_at(to, &[Echelon::Division, Echelon::Brigade]) {
            return refuse(format!(
                "no {player} division or brigade HQ at {name} to receive it"
            ));
        }
        // The farthest division HQ forward: east for the Axis, west for
        // the Commonwealth. A box lies west of every hex.
        if let Location::Hex(dest) = to {
            let rows = self
                .pieces
                .values()
                .filter(|q| q.side == player && self.echelon(data, q) == Some(Echelon::Division))
                .filter_map(|q| q.hex().map(east_west));
            let limit = match player {
                Side::Axis => rows.max(),
                Side::Cw => rows.min(),
            };
            let beyond = match (player, limit) {
                (_, None) => true,
                (Side::Axis, Some(l)) => east_west(dest) > l,
                (Side::Cw, Some(l)) => east_west(dest) < l,
            };
            if beyond {
                return refuse(format!(
                    "{dest} is beyond the farthest {player} division HQ"
                ));
            }
        } else if player == Side::Cw {
            return refuse(format!("the Commonwealth flies nothing to {name}"));
        }
        let payer = self.lift_payer(id, &from, fuel_from)?;
        self.pieces
            .get_mut(&payer)
            .and_then(|q| q.load.as_mut())
            .expect("a payer has a load")
            .fuel -= LIFT_FUEL;
        let p = self.pieces.get_mut(id).expect("checked");
        p.location = to.clone();
        p.flown = true;
        self.air_lifted.insert(player, turn);
        Ok(vec![Event::AirLifted {
            piece: id.clone(),
            from,
            to: to.clone(),
            fuel_from: payer,
        }])
    }

    /// The real supply unit at `from` paying the lift's fuel: the one
    /// named, else the first with enough.
    fn lift_payer(
        &self,
        id: &PieceId,
        from: &Location,
        named: Option<&PieceId>,
    ) -> Result<PieceId, Illegal> {
        let side = self.pieces[id].side;
        let pays = |q: &Piece| {
            q.side == side
                && q.kind == PieceKind::SupplyUnit
                && q.location == *from
                && q.load.is_some_and(|l| l.fuel >= LIFT_FUEL)
        };
        match named {
            Some(n) => self
                .pieces
                .get(n)
                .filter(|q| pays(q))
                .map(|q| q.id.clone())
                .ok_or_else(|| {
                    Illegal::new(
                        CASE,
                        format!("{n} is not a supply unit beside {id} with {LIFT_FUEL} fuel"),
                    )
                }),
            None => self
                .pieces
                .values()
                .find(|q| pays(q))
                .map(|q| q.id.clone())
                .ok_or_else(|| {
                    Illegal::new(
                        CASE,
                        format!("no supply unit beside {id} has the {LIFT_FUEL} fuel to fly it"),
                    )
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;
    use crate::state::Modules;

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    fn at(s: &str) -> Location {
        Location::Hex(hex(s))
    }

    struct Lift {
        data: Data,
        state: GameState,
        supply: PieceId,
        division: PieceId,
    }

    /// Phase C, the Axis to act: the Commonwealth's combat units sent back
    /// to Cairo, an Axis supply unit at Tobruk with no motorisation, and an
    /// Axis division HQ at Bardia, the farthest east of them all.
    fn lift() -> Lift {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = GameState::with_modules(&data, 5, Modules::ALL).unwrap();
        crate::state::test_support::auto_place(&data, &mut state);
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        assert_eq!(state.clock.phase, Phase::Organisation);
        for p in state.pieces.values_mut() {
            if p.side == Side::Cw && p.hex().is_some() {
                p.location = at("E1730");
            }
        }
        let supply = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::SupplyUnit)
            .unwrap()
            .id
            .clone();
        let s = state.pieces.get_mut(&supply).unwrap();
        s.location = at("C4807");
        s.motorised = 0;
        let divisions: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && state.echelon(&data, p) == Some(Echelon::Division))
            .map(|p| p.id.clone())
            .collect();
        for d in &divisions {
            let p = state.pieces.get_mut(d).unwrap();
            if p.hex()
                .is_some_and(|h| east_west(h) > east_west(&hex("C4321")))
            {
                p.location = at("C4807");
            }
        }
        let division = divisions[0].clone();
        state.put(&division, at("C4321"));
        Lift {
            data,
            state,
            supply,
            division,
        }
    }

    fn fly(l: &mut Lift, to: &str) -> Result<Vec<Event>, Illegal> {
        l.state.apply(
            &l.data,
            Side::Axis,
            Order::AirLift {
                piece: l.supply.clone(),
                to: at(to),
                fuel_from: None,
            },
        )
    }

    #[test]
    fn a_supply_unit_flies_to_a_division_hq_for_12_fuel_and_stays_put_32_37() {
        let mut l = lift();
        let fuel = l.state.pieces[&l.supply].load.unwrap().fuel;
        let events = fly(&mut l, "C4321").unwrap();
        assert_eq!(
            events,
            vec![Event::AirLifted {
                piece: l.supply.clone(),
                from: at("C4807"),
                to: at("C4321"),
                fuel_from: l.supply.clone(),
            }]
        );
        let s = &l.state.pieces[&l.supply];
        assert_eq!(s.location, at("C4321"));
        assert_eq!(s.load.unwrap().fuel, fuel - 12);
        assert!(s.flown);
        // Even with motorisation points it does not move again this stage.
        let mut moved = s.clone();
        moved.motorised = 30;
        assert_eq!(l.state.cpa(&moved), None);
        // Once a Game-Turn.
        let s = l.state.pieces.get_mut(&l.supply).unwrap();
        s.location = at("C4807");
        let err = fly(&mut l, "C4321").unwrap_err();
        assert_eq!(err.case, "32.37");
        assert!(err.reason.contains("this Game-Turn"), "{}", err.reason);
    }

    #[test]
    fn nothing_flies_with_motorisation_or_from_open_desert_32_37() {
        let mut l = lift();
        l.state.pieces.get_mut(&l.supply).unwrap().motorised = 30;
        let err = fly(&mut l, "C4321").unwrap_err();
        assert!(err.reason.contains("motorisation"), "{}", err.reason);
        let s = l.state.pieces.get_mut(&l.supply).unwrap();
        s.motorised = 0;
        s.location = at("C4808");
        assert!(data_has_no_settlement(&l.data, "C4808"));
        let err = fly(&mut l, "C4321").unwrap_err();
        assert!(err.reason.contains("major city"), "{}", err.reason);
    }

    fn data_has_no_settlement(data: &Data, h: &str) -> bool {
        data.map.hex(&hex(h)).unwrap().settlement.is_none()
    }

    #[test]
    fn the_destination_needs_a_division_or_brigade_hq_32_37() {
        let mut l = lift();
        // Sidi Barrani, held by no Axis HQ.
        assert!(
            !l.state
                .pieces
                .values()
                .any(|p| p.side == Side::Axis && p.hex() == Some(&hex("C4131")))
        );
        let err = fly(&mut l, "C4131").unwrap_err();
        assert!(
            err.reason.contains("division or brigade HQ"),
            "{}",
            err.reason
        );
        // A combat unit there is no HQ either.
        let bn = l
            .state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .unwrap()
            .id
            .clone();
        l.state.pieces.get_mut(&bn).unwrap().location = at("C4021");
        assert!(fly(&mut l, "C4021").is_err());
    }

    #[test]
    fn no_farther_east_than_the_farthest_axis_division_hq_r_124() {
        let mut l = lift();
        // A brigade-equivalent HQ at Sidi Barrani, east of Bardia.
        let brigade = l
            .state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && l.state.echelon(&l.data, p) == Some(Echelon::Brigade))
            .unwrap()
            .id
            .clone();
        l.state.pieces.get_mut(&brigade).unwrap().location = at("C4131");
        let err = fly(&mut l, "C4131").unwrap_err();
        assert!(err.reason.contains("beyond"), "{}", err.reason);
        // Once the division HQ stands there too, it may.
        l.state.pieces.get_mut(&l.division).unwrap().location = at("C4131");
        fly(&mut l, "C4131").unwrap();
    }

    #[test]
    fn not_from_within_five_hexes_of_an_enemy_combat_unit_32_37() {
        let mut l = lift();
        let cw = l
            .state
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::Combat && p.hex().is_some())
            .unwrap()
            .id
            .clone();
        let near = l.data.map.within(&hex("C4807"), 5);
        let five_off = near
            .iter()
            .find(|h| !l.data.map.within(&hex("C4807"), 4).contains(*h))
            .unwrap()
            .clone();
        l.state.pieces.get_mut(&cw).unwrap().location = Location::Hex(five_off);
        let err = fly(&mut l, "C4321").unwrap_err();
        assert!(err.reason.contains("within 5 hexes"), "{}", err.reason);
    }

    #[test]
    fn the_lift_is_phase_c_only_and_needs_the_module() {
        let mut l = lift();
        l.state.clock.phase = Phase::MovementCombat;
        assert!(fly(&mut l, "C4321").is_err());
        let mut l = lift();
        l.state.modules.abstract_supply = false;
        assert!(fly(&mut l, "C4321").is_err());
    }
}
