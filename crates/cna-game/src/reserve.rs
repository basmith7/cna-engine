//! Reserves (SPI 18.1x, 18.2x): the phasing player designates units in
//! phase F; a Reserve I unit may shift one hex a movement segment, free,
//! never into an enemy ZOC; at the first reserve release segment of the
//! phase each is released or flipped to Reserve II, which cannot move; a
//! released unit may move in the next segment whatever the two-hex rule,
//! under a CP cap for the rest of the stage (its CPA from I, half from
//! II). The combat limits of a released unit are slice 5's, from
//! `Piece::released`.

use crate::clock::{Phase, Segment};
use crate::event::{Event, Illegal};
use crate::piece::{PieceId, PieceKind, Reserve};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::scenario::Side;

impl GameState {
    /// `Order::DesignateReserve`: phase F, the phasing player's own units
    /// on the map (18.1x).
    pub(crate) fn designate_reserve(
        &mut self,
        player: Side,
        pieces: &[PieceId],
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::ReserveDesignation {
            return Err(Illegal::new(
                "18.1x",
                format!("reserves are designated in phase F, not at {}", self.clock),
            ));
        }
        if pieces.is_empty() {
            return Err(Illegal::new("18.1x", "no pieces to designate"));
        }
        for (i, id) in pieces.iter().enumerate() {
            if pieces[..i].contains(id) {
                return Err(Illegal::new("18.1x", format!("{id} is listed twice")));
            }
            let p = self
                .pieces
                .get(id)
                .ok_or_else(|| Illegal::new("18.1x", format!("{id}: no such piece")))?;
            if p.side != player {
                return Err(Illegal::new("18.1x", format!("{id} is not {player}'s")));
            }
            if p.hex().is_none() {
                return Err(Illegal::new("18.1x", format!("{id} is not on the map")));
            }
            if !matches!(p.kind, PieceKind::Combat | PieceKind::Hq) {
                return Err(Illegal::new(
                    "18.1x",
                    format!("{id}: only combat units and HQs go into reserve"),
                ));
            }
            if p.reserve.is_some() {
                return Err(Illegal::new("18.1x", format!("{id} is already in reserve")));
            }
        }
        for id in pieces {
            self.pieces.get_mut(id).expect("checked").reserve = Some(Reserve::I);
        }
        Ok(vec![Event::ReserveDesignated {
            pieces: pieces.to_vec(),
        }])
    }

    /// `Order::ReleaseReserve`: in a reserve release segment, the phasing
    /// player's units in reserve come out, free to move next segment
    /// (18.2x).
    pub(crate) fn release_reserve(
        &mut self,
        player: Side,
        pieces: &[PieceId],
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::MovementCombat
            || self.clock.segment != Some(Segment::ReserveRelease)
        {
            return Err(Illegal::new(
                "18.2x",
                format!(
                    "reserves are released in a reserve release segment, not at {}",
                    self.clock
                ),
            ));
        }
        if pieces.is_empty() {
            return Err(Illegal::new("18.2x", "no pieces to release"));
        }
        let mut from = None;
        for (i, id) in pieces.iter().enumerate() {
            if pieces[..i].contains(id) {
                return Err(Illegal::new("18.2x", format!("{id} is listed twice")));
            }
            let p = self
                .pieces
                .get(id)
                .ok_or_else(|| Illegal::new("18.2x", format!("{id}: no such piece")))?;
            if p.side != player {
                return Err(Illegal::new("18.2x", format!("{id} is not {player}'s")));
            }
            let Some(r) = p.reserve else {
                return Err(Illegal::new("18.2x", format!("{id} is not in reserve")));
            };
            match from {
                None => from = Some(r),
                Some(f) if f != r => {
                    return Err(Illegal::new(
                        "18.2x",
                        format!(
                            "{id} is in Reserve {r}, not {f} like the rest: release them apart"
                        ),
                    ));
                }
                Some(_) => {}
            }
        }
        let from = from.expect("at least one piece");
        for id in pieces {
            let p = self.pieces.get_mut(id).expect("checked");
            p.reserve = None;
            p.released = Some(from);
            self.continuing.insert(id.clone());
        }
        Ok(vec![Event::ReserveReleased {
            pieces: pieces.to_vec(),
            from,
        }])
    }

    /// Leaving the first reserve release segment of the phase: every unit
    /// still in Reserve I flips to Reserve II (18.2x).
    pub(crate) fn close_release_segment(&mut self, events: &mut Vec<Event>) {
        if self.cycle != 1 {
            return;
        }
        let phasing = self.phasing();
        let flipped: Vec<PieceId> = self
            .pieces
            .values_mut()
            .filter(|p| Some(p.side) == phasing && p.reserve == Some(Reserve::I))
            .map(|p| {
                p.reserve = Some(Reserve::II);
                p.id.clone()
            })
            .collect();
        if !flipped.is_empty() {
            events.push(Event::ReserveFlipped { pieces: flipped });
        }
    }

    /// The most a piece may spend voluntarily in its own half this stage
    /// (8.1x, 18.2x), with the case that says so; none for a motorised
    /// unit under no reserve limit.
    pub(crate) fn voluntary_cap(&self, id: &PieceId) -> Option<(Cp, &'static str, &'static str)> {
        let p = &self.pieces[id];
        let cpa = self.cpa(p)?;
        match p.released {
            Some(Reserve::I) => Some((
                Cp::whole(cpa),
                "18.2x",
                "released from Reserve I: it may not exceed its CPA this stage",
            )),
            // Half the CPA rounded down; leg infantry (CPA 10 or less,
            // 8.1x) gets 5 whatever its CPA (18.24).
            Some(Reserve::II) => Some((
                Cp::whole(if cpa <= 10 { 5 } else { cpa / 2 }),
                "18.2x",
                "released from Reserve II: it may not exceed half its CPA this stage",
            )),
            None if cpa <= 10 => Some((
                Cp::halves(cpa * 3),
                "8.1x",
                "a non-motorised unit may not go past 150 % of its CPA voluntarily",
            )),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;
    use crate::movement::test_support::{hex, mv, pid};
    use crate::order::Order;
    use crate::state::Pending;
    use crate::state::test_support::set_up;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn snapshot(state: &GameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    fn designate(pieces: &[&str]) -> Order {
        Order::DesignateReserve {
            pieces: pieces.iter().map(|s| pid(s)).collect(),
        }
    }

    fn release(pieces: &[&str]) -> Order {
        Order::ReleaseReserve {
            pieces: pieces.iter().map(|s| pid(s)).collect(),
        }
    }

    /// GT1 OpStage 1 with the Commonwealth as Player A, in its phase F.
    fn in_phase_f(data: &Data) -> GameState {
        let mut state = set_up(data, 3);
        state
            .apply(
                data,
                Side::Axis,
                Order::DeclareInitiative { player_a: Side::Cw },
            )
            .unwrap();
        assert_eq!(state.clock.phase, Phase::ReserveDesignation);
        assert_eq!(state.to_act(), Some(Side::Cw));
        state
    }

    #[test]
    fn reserves_are_designated_in_phase_f_by_the_phasing_player() {
        let data = graziani();
        let mut state = in_phase_f(&data);
        let before = snapshot(&state);
        for (order, player) in [
            (designate(&["unit:it:lxiil-tank-bn"]), Side::Cw),
            (designate(&["unit:cw:nobody"]), Side::Cw),
            (designate(&[]), Side::Cw),
            (designate(&["unit:cw:1-krrc", "unit:cw:1-krrc"]), Side::Cw),
            (
                Order::DesignateReserve {
                    pieces: vec![PieceId::supply(Side::Cw, 1)],
                },
                Side::Cw,
            ),
        ] {
            let err = state.apply(&data, player, order).unwrap_err();
            assert_eq!(err.case, "18.1x");
        }
        assert_eq!(snapshot(&state), before);
        let events = state
            .apply(
                &data,
                Side::Cw,
                designate(&["unit:cw:1-krrc", "unit:cw:7-med"]),
            )
            .unwrap();
        assert_eq!(
            events,
            [Event::ReserveDesignated {
                pieces: vec![pid("unit:cw:1-krrc"), pid("unit:cw:7-med")]
            }]
        );
        assert_eq!(
            state.pieces[&pid("unit:cw:1-krrc")].reserve,
            Some(Reserve::I)
        );
        let err = state
            .apply(&data, Side::Cw, designate(&["unit:cw:1-krrc"]))
            .unwrap_err();
        assert_eq!(err.case, "18.1x");
        // Not in G.
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        let err = state
            .apply(&data, Side::Cw, designate(&["unit:cw:4-rha"]))
            .unwrap_err();
        assert_eq!(err.case, "18.1x");
        // Nor released there.
        let err = state
            .apply(&data, Side::Cw, release(&["unit:cw:1-krrc"]))
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
    }

    #[test]
    fn reserve_one_shifts_a_hex_free_then_is_released_or_flipped() {
        let data = graziani();
        let mut state = in_phase_f(&data);
        state
            .apply(
                &data,
                Side::Cw,
                designate(&["unit:cw:1-krrc", "unit:cw:7-med"]),
            )
            .unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.segment, Some(Segment::Movement));
        let krrc = pid("unit:cw:1-krrc");
        // Two hexes, or a hex in an enemy ZOC (C4021, once the Axis says
        // C4020 exerts), is no shift.
        let before = snapshot(&state);
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc"], &["C3923", "C3924"]),
            )
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&["unit:cw:1-krrc", "unit:cw:4-rha"], &["C3923"]),
            )
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        assert_eq!(snapshot(&state), before);
        state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4021"]))
            .unwrap();
        assert!(matches!(state.pending, Some(Pending::DeclareZoc { .. })));
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareZoc {
                    exerting: vec![hex("C4020")],
                },
            )
            .unwrap();
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C4021"]))
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        // One hex east, for nothing; not a second time this segment.
        let events = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3923"]))
            .unwrap();
        assert_eq!(
            events[0],
            Event::Moved {
                pieces: vec![krrc.clone()],
                from: hex("C3922"),
                to: hex("C3923"),
                cost: Cp::ZERO
            }
        );
        assert_eq!(state.pieces[&krrc].cp_spent, Cp::ZERO);
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:1-krrc"], &["C3924"]))
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        // The first release segment: 1 KRRC comes out of Reserve I; 7 Med
        // stays and flips to Reserve II as the segment closes.
        crate::state::test_support::through_combat(&data, &mut state);
        assert_eq!(state.clock.segment, Some(Segment::ReserveRelease));
        let err = state
            .apply(&data, Side::Cw, release(&["unit:cw:4-rha"]))
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        let events = state
            .apply(&data, Side::Cw, release(&["unit:cw:1-krrc"]))
            .unwrap();
        assert_eq!(
            events,
            [Event::ReserveReleased {
                pieces: vec![krrc.clone()],
                from: Reserve::I
            }]
        );
        assert_eq!(state.pieces[&krrc].reserve, None);
        assert_eq!(state.pieces[&krrc].released, Some(Reserve::I));
        let events = state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        assert!(events.contains(&Event::ReserveFlipped {
            pieces: vec![pid("unit:cw:7-med")]
        }));
        assert_eq!(
            state.pieces[&pid("unit:cw:7-med")].reserve,
            Some(Reserve::II)
        );
        // Reserve II does not move; released 1 KRRC, more than two hexes
        // from any Axis unit at C3923, moves (18.25) but may not go past
        // its CPA of 10 this stage (18.23): five hexes east (9 CP, the
        // C3926-C3927 track halving one), not six (11).
        let err = state
            .apply(&data, Side::Cw, mv(&["unit:cw:7-med"], &["C3923"]))
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        assert!(!state.continuing.is_empty());
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(
                    &["unit:cw:1-krrc"],
                    &["C3924", "C3925", "C3926", "C3927", "C3928", "C3929"],
                ),
            )
            .unwrap_err();
        assert_eq!(err.case, "18.2x");
        state
            .apply(
                &data,
                Side::Cw,
                mv(
                    &["unit:cw:1-krrc"],
                    &["C3924", "C3925", "C3926", "C3927", "C3928"],
                ),
            )
            .unwrap();
        assert_eq!(state.pieces[&krrc].cp_spent, Cp::whole(9));
        // A later release segment lets Reserve II out, at half its CPA.
        crate::state::test_support::through_combat(&data, &mut state);
        let events = state
            .apply(&data, Side::Cw, release(&["unit:cw:7-med"]))
            .unwrap();
        assert_eq!(
            events,
            [Event::ReserveReleased {
                pieces: vec![pid("unit:cw:7-med")],
                from: Reserve::II
            }]
        );
        assert_eq!(
            state.voluntary_cap(&pid("unit:cw:7-med")),
            Some((
                Cp::whole(10),
                "18.2x",
                "released from Reserve II: it may not exceed half its CPA this stage"
            ))
        );
        // Everything is forgotten when the OpStage ends.
        crate::state::test_support::pass_stage(&data, &mut state);
        assert_eq!(state.clock.stage, 2);
        assert_eq!(state.pieces[&krrc].released, None);
        assert_eq!(state.pieces[&pid("unit:cw:7-med")].released, None);
    }

    #[test]
    fn leg_infantry_out_of_reserve_two_has_five_cp_not_half_its_cpa() {
        let data = graziani();
        let mut state = in_phase_f(&data);
        let rnf = pid("unit:cw:1-rnf");
        assert_eq!(state.cpa(&state.pieces[&rnf]), Some(8));
        state
            .apply(&data, Side::Cw, designate(&["unit:cw:1-rnf"]))
            .unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        crate::state::test_support::through_combat(&data, &mut state);
        state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
        assert_eq!(state.pieces[&rnf].reserve, Some(Reserve::II));
        crate::state::test_support::through_combat(&data, &mut state);
        state
            .apply(&data, Side::Cw, release(&["unit:cw:1-rnf"]))
            .unwrap();
        // 18.24: half the CPA rounded down, but 5 for leg infantry.
        assert_eq!(
            state.voluntary_cap(&rnf),
            Some((
                Cp::whole(5),
                "18.2x",
                "released from Reserve II: it may not exceed half its CPA this stage"
            ))
        );
    }

    #[test]
    fn pass_from_the_first_release_segment_flips_too() {
        let data = graziani();
        let mut state = in_phase_f(&data);
        state
            .apply(&data, Side::Cw, designate(&["unit:cw:7-med"]))
            .unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        crate::state::test_support::through_combat(&data, &mut state);
        let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert!(events.contains(&Event::ReserveFlipped {
            pieces: vec![pid("unit:cw:7-med")]
        }));
        // Reserve binds only until the stage ends; the Axis half leaves
        // it as it is.
        assert_eq!(
            state.pieces[&pid("unit:cw:7-med")].reserve,
            Some(Reserve::II)
        );
    }
}
