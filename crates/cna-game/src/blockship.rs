//! The *San Giorgio* (30.17, R-151): the half-sunk Italian cruiser in
//! Tobruk harbour, an immobile gun under the `fleet` module. It fires its
//! counter's 2 gun points, as actual points needing no ammunition, into an
//! adjacent hex once an OpStage, but only while an Axis land unit shares
//! its hex; nothing on land may attack it; it goes at the end of an
//! OpStage in which a Commonwealth engineer unit and no Axis unit holds
//! its hex. Its fire at ships (30.39) is not simulated.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::oa::{Blockship, Nation};
use cna_data::scenario::Side;

/// The OA unit that is a blockship, with its values.
fn blockship_row(data: &Data) -> Option<(PieceId, &Blockship)> {
    data.oa
        .units()
        .find_map(|u| Some((PieceId::from(&u.id), u.blockship.as_ref()?)))
}

impl GameState {
    /// Places the *San Giorgio* in its hex when the fleet is in play and
    /// the scenario starts on Game-Turn 1 (R-151): every later scenario
    /// starts with Tobruk in Commonwealth hands.
    pub(crate) fn place_blockship(&mut self, data: &Data) -> anyhow::Result<()> {
        if !self.modules.fleet || data.scenario.start.game_turn != 1 {
            return Ok(());
        }
        let Some((id, row)) = blockship_row(data) else {
            return Ok(());
        };
        let hex = HexId::parse(&row.hex)?;
        let mut p = crate::setup::supply_piece(
            Side::Axis,
            PieceKind::Blockship,
            id.clone(),
            Location::Hex(hex),
            None,
        );
        p.nation = Some(Nation::It);
        self.pieces.insert(id, p);
        Ok(())
    }

    /// Whether an Axis land unit (a combat unit or HQ) stands in `hex`.
    fn axis_unit_in(&self, hex: &HexId) -> bool {
        self.pieces_at(hex)
            .iter()
            .any(|p| p.side == Side::Axis && matches!(p.kind, PieceKind::Combat | PieceKind::Hq))
    }

    /// The actual barrage points the blockship fires at an enemy unit in
    /// `at` (R-151): its gun rating, into an adjacent hex, once an
    /// OpStage, while an Axis land unit shares its hex.
    pub(crate) fn blockship_points(
        &self,
        data: &Data,
        ship: &Piece,
        at: &HexId,
    ) -> Result<u32, Illegal> {
        let row = blockship_row(data).filter(|(id, _)| self.modules.fleet && *id == ship.id);
        let (Some((_, row)), Some(from)) = (row, ship.hex()) else {
            return Err(Illegal::new(
                "30.17",
                format!("{}: not a blockship in play", ship.id),
            ));
        };
        if self.blockship_fired {
            return Err(Illegal::new(
                "30.17",
                format!("{} has fired this OpStage: once a stage", ship.id),
            ));
        }
        if !self.axis_unit_in(from) {
            return Err(Illegal::new(
                "30.17",
                format!(
                    "{} fires only while an Axis land unit is in {from} (R-151)",
                    ship.id
                ),
            ));
        }
        if !data.map.adjacent(from, at) {
            return Err(Illegal::new(
                "30.17",
                format!("{at} is not adjacent to {} at {from}", ship.id),
            ));
        }
        Ok(row.gun_rating)
    }

    /// Whether the blockship may fire at an enemy unit now: opens a
    /// barrage step with no land fire in it, reading only what both sides
    /// see (3.61).
    pub(crate) fn blockship_may_fire(&self, data: &Data) -> bool {
        self.pieces
            .values()
            .filter(|p| p.kind == PieceKind::Blockship)
            .any(|ship| {
                self.pieces.values().any(|t| {
                    t.side != ship.side
                        && matches!(t.kind, PieceKind::Combat | PieceKind::Hq)
                        && t.hex()
                            .is_some_and(|at| self.blockship_points(data, ship, at).is_ok())
                })
            })
    }

    /// The end of an OpStage: a blockship whose hex holds a Commonwealth
    /// engineer unit (23.1: the Commonwealth's are its HQs marked E) and
    /// no Axis unit is removed (R-151).
    pub(crate) fn clear_blockship(&mut self, data: &Data, events: &mut Vec<Event>) {
        let cleared: Vec<PieceId> = self
            .pieces
            .values()
            .filter(|p| p.kind == PieceKind::Blockship)
            .filter(|p| {
                p.hex().is_some_and(|h| {
                    !self.axis_unit_in(h)
                        && self
                            .pieces_at(h)
                            .iter()
                            .any(|q| q.side == Side::Cw && self.engineer_capable(data, q))
                })
            })
            .map(|p| p.id.clone())
            .collect();
        for id in cleared {
            self.pieces.get_mut(&id).expect("a piece").location = Location::Eliminated;
            events.push(Event::Eliminated { piece: id });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Phase;
    use crate::combat::segment::test_support::at_sollum;
    use crate::movement::test_support::{hex, pid};
    use crate::order::{Fire, Order};
    use crate::raiders::tests::graziani;
    use crate::state::Modules;
    use crate::state::test_support::{auto_place, pass_stage};

    const WRECK: &str = "unit:it:the-san-giorgio";
    const TARGET: &str = "unit:cw:1-krrc";

    fn fleet_on() -> Modules {
        Modules {
            fleet: true,
            ..Modules::default()
        }
    }

    #[test]
    fn the_wreck_lies_in_tobruk_only_with_the_fleet_in_a_game_turn_1_scenario_r151() {
        let data = graziani();
        let state = GameState::with_modules(&data, 7, fleet_on()).unwrap();
        let p = &state.pieces[&pid(WRECK)];
        assert_eq!(p.kind, PieceKind::Blockship);
        assert_eq!(p.side, Side::Axis);
        assert_eq!(p.nation, Some(Nation::It));
        assert_eq!(p.location, Location::Hex(hex("C4807")));
        assert!(!p.kind.holds_ground(), "no ZOC, no stacking, never blocks");
        assert!(p.equivalent.is_none() && p.toe == 0);

        let off = GameState::with_modules(&data, 7, Modules::default()).unwrap();
        assert!(!off.pieces.contains_key(&pid(WRECK)), "fleet off: no wreck");

        // Rommel's Arrival starts after Tobruk fell (GT26 on): no wreck.
        let later = crate::data::Data::load("rommels-arrival").unwrap();
        assert!(later.scenario.start.game_turn > 1);
        let state = GameState::with_modules(&later, 7, fleet_on()).unwrap();
        assert!(!state.pieces.contains_key(&pid(WRECK)));
    }

    /// Sollum's combat with the wreck moved into C4020, the 1st Libyan
    /// Division's hex beside the Commonwealth stack at C4021, the gun
    /// positions answered and the barrage step open. The Commonwealth
    /// (phasing) has plotted nothing.
    fn at_barrage(data: &Data) -> GameState {
        let mut state = at_sollum(data);
        state.modules.fleet = true;
        state.place_blockship(data).unwrap();
        state.pieces.get_mut(&pid(WRECK)).unwrap().location = Location::Hex(hex("C4020"));
        for side in [Side::Cw, Side::Axis] {
            state
                .apply(data, side, Order::GunPositions { forward: vec![] })
                .unwrap();
        }
        assert_eq!(state.clock.segment, Some(crate::clock::Segment::Barrage));
        state
            .apply(data, Side::Cw, Order::Barrage { fires: vec![] })
            .unwrap();
        state
    }

    fn fire(points: u32) -> Order {
        Order::Barrage {
            fires: vec![Fire {
                piece: pid(WRECK),
                points,
                target: pid(TARGET),
            }],
        }
    }

    #[test]
    fn the_wreck_barrages_an_adjacent_hex_with_its_two_gun_points_r151() {
        let data = graziani();
        let mut state = at_barrage(&data);
        let options = state.sealed_options(&data, Side::Axis).unwrap();
        let crate::combat::options::SealedOptions::Barrage { firers } = options else {
            panic!("a barrage step");
        };
        let wreck = firers
            .iter()
            .find(|f| f.piece == pid(WRECK))
            .expect("offered");
        assert_eq!(wreck.ship_points.get(&pid(TARGET)), Some(&2));
        assert_eq!(wreck.ammo, 0);

        let err = state.clone().apply(&data, Side::Axis, fire(3)).unwrap_err();
        assert_eq!(err.case, "30.2x");
        let events = state.apply(&data, Side::Axis, fire(2)).unwrap();
        let points = events.iter().find_map(|e| match e {
            Event::Barrage { firers, points, .. } if firers == &vec![pid(WRECK)] => Some(*points),
            _ => None,
        });
        assert_eq!(points, Some(2), "{events:?}");
        assert!(state.blockship_fired, "once an OpStage");
        let at = hex("C4021");
        let wreck = state.pieces[&pid(WRECK)].clone();
        assert_eq!(
            state.blockship_points(&data, &wreck, &at).unwrap_err().case,
            "30.17"
        );
    }

    #[test]
    fn the_wreck_is_silent_without_an_axis_land_unit_in_its_hex_r151() {
        let data = graziani();
        let mut state = at_barrage(&data);
        for p in state.pieces.values_mut() {
            if p.side == Side::Axis
                && p.hex() == Some(&hex("C4020"))
                && p.kind != PieceKind::Blockship
            {
                p.location = Location::Eliminated;
            }
        }
        let err = state.apply(&data, Side::Axis, fire(2)).unwrap_err();
        assert_eq!(err.case, "30.17");
        assert!(!state.blockship_may_fire(&data));
    }

    #[test]
    fn the_wreck_fires_only_into_an_adjacent_hex_r151() {
        let data = graziani();
        let state = at_barrage(&data);
        let wreck = state.pieces[&pid(WRECK)].clone();
        let far = hex("C4807");
        assert!(!data.map.adjacent(&hex("C4020"), &far));
        assert_eq!(
            state
                .blockship_points(&data, &wreck, &far)
                .unwrap_err()
                .case,
            "30.17"
        );
        let own = hex("C4020");
        assert!(state.blockship_points(&data, &wreck, &own).is_err());
    }

    #[test]
    fn land_fire_may_not_target_the_wreck_nor_assault_it_alone_r151() {
        let data = graziani();
        let mut state = at_sollum(&data);
        state.modules.fleet = true;
        state.place_blockship(&data).unwrap();
        // The wreck alone in a hex beside the Commonwealth stack.
        let beside = data
            .map
            .neighbours(&hex("C4021"))
            .into_iter()
            .find(|n| *n != hex("C4020") && state.pieces_at(n).is_empty())
            .unwrap();
        state.pieces.get_mut(&pid(WRECK)).unwrap().location = Location::Hex(beside.clone());
        let gun = state.pieces[&pid("unit:cw:4-rha")].clone();
        let err = state
            .barrage_target(&data, Side::Cw, &gun, &pid(WRECK))
            .unwrap_err();
        assert_eq!(err.case, "12.2x");
        let err = state
            .assault_target(&data, &gun, &hex("C4021"), &beside)
            .unwrap_err();
        assert_eq!(
            err.case, "15.0",
            "a hex holding only the wreck is not enemy-held"
        );
    }

    /// Graziani's set-up with the fleet, placed, Tobruk's Axis units gone
    /// and `cw` put there; then played to the next OpStage.
    fn tobruk_held_by(cw: &[PieceId], axis_stays: bool) -> (GameState, Vec<Event>) {
        let data = graziani();
        let mut state = GameState::with_modules(&data, 7, fleet_on()).unwrap();
        auto_place(&data, &mut state);
        let tobruk = hex("C4807");
        let keep = state
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Axis && p.kind == PieceKind::Combat && p.hex() == Some(&tobruk)
            })
            .map(|p| p.id.clone());
        for p in state.pieces.values_mut() {
            if p.side == Side::Axis
                && p.hex() == Some(&tobruk)
                && p.kind != PieceKind::Blockship
                && !(axis_stays && Some(&p.id) == keep.as_ref())
            {
                p.location = Location::Eliminated;
            }
        }
        for id in cw {
            let p = state.pieces.get_mut(id).unwrap();
            p.side = Side::Cw;
            p.location = Location::Hex(tobruk.clone());
            assert!(state.engineer_capable(&data, &state.pieces[id]) == (id == &pid(ENGINEER)));
        }
        if state.clock.phase == Phase::InitiativeDeclaration {
            let p = state.to_act().unwrap();
            state
                .apply(&data, p, Order::DeclareInitiative { player_a: p })
                .unwrap();
        }
        let events = pass_stage(&data, &mut state);
        (state, events)
    }

    /// The engine knows no Commonwealth engineer yet (its E-HQs are read
    /// by a `^E` the OA no longer prints; see NOT_SIMULATED.md), so an
    /// Italian engineer battalion turned Commonwealth stands in.
    const ENGINEER: &str = "unit:it:201st-engineer-bn";

    fn cw_unit(engineer: bool) -> PieceId {
        if engineer {
            return pid(ENGINEER);
        }
        let data = graziani();
        let state = GameState::new(&data, 7).unwrap();
        state
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Cw
                    && p.kind == PieceKind::Combat
                    && !state.engineer_capable(&data, p)
            })
            .expect("a Commonwealth unit")
            .id
            .clone()
    }

    #[test]
    fn a_lone_commonwealth_engineer_removes_the_wreck_at_the_stage_end_r151() {
        let (state, events) = tobruk_held_by(&[cw_unit(true)], false);
        assert_eq!(state.pieces[&pid(WRECK)].location, Location::Eliminated);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::Eliminated { piece } if *piece == pid(WRECK)))
        );
    }

    #[test]
    fn the_wreck_stays_without_an_engineer_or_while_an_axis_unit_holds_tobruk_r151() {
        let (state, _) = tobruk_held_by(&[cw_unit(false)], false);
        assert_eq!(
            state.pieces[&pid(WRECK)].location,
            Location::Hex(hex("C4807"))
        );
        let (state, _) = tobruk_held_by(&[cw_unit(true)], true);
        assert_eq!(
            state.pieces[&pid(WRECK)].location,
            Location::Hex(hex("C4807"))
        );
    }

    #[test]
    fn with_the_fleet_off_the_state_serialises_as_before() {
        let data = graziani();
        let state = GameState::new(&data, 7).unwrap();
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("blockship") && !json.contains("san-giorgio"));
    }
}
