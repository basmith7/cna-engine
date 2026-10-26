//! Reaction (SPI 8.5x, R-006, R-007, R-008): when a phasing combat unit
//! moves next to motorised non-phasing combat units (or Rommel, 31.0)
//! that are not in an enemy ZOC, in contact, engaged or pinned, the
//! non-phasing player may move them, never into an enemy ZOC and never
//! paying to break off.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind, Reserve};
use crate::state::{GameState, Pending};
use crate::zoc::Zoc;
use cna_data::map::HexId;
use cna_data::scenario::{Side, Status};

/// A unit may not react to an assaulting enemy whose CPA is this much
/// higher (8.5x).
const PIN_MARGIN: u32 = 6;

impl GameState {
    /// Whether `pinner` (a stack of the given SP and lowest CPA, that has
    /// declared an assault) pins `target` by speed and size (8.5x): a
    /// battalion never pins a division, a company nothing of brigade size.
    fn pins(&self, data: &Data, pinner_sp: u32, pinner_cpa: u32, target: &Piece) -> bool {
        let Some(cpa) = self.cpa(target) else {
            return true;
        };
        if pinner_cpa < cpa + PIN_MARGIN {
            return false;
        }
        let target_sp = cna_rules::stacking::points(&data.stacking, target.equivalent);
        match pinner_sp {
            0 => target_sp < 2,
            1 => target_sp < 5,
            _ => true,
        }
    }

    /// The non-phasing combat units next to `at` that may react to the
    /// stack that just moved there (8.5x, R-006, R-008).
    pub(crate) fn reactors_next_to(
        &self,
        data: &Data,
        at: &HexId,
        mover: &[PieceId],
        assault: bool,
    ) -> Vec<PieceId> {
        let phasing = self.pieces[&mover[0]].side;
        let pinner_sp: u32 = mover
            .iter()
            .map(|id| cna_rules::stacking::points(&data.stacking, self.pieces[id].equivalent))
            .sum();
        // Supply units riding along do not slow the pinner (32.5x's CPA 15
        // is the trucks', not the stack's).
        let pinner_cpa = mover
            .iter()
            .filter(|id| {
                !matches!(
                    self.pieces[*id].kind,
                    PieceKind::SupplyUnit | PieceKind::DummySupplyUnit
                )
            })
            .filter_map(|id| self.cpa(&self.pieces[id]))
            .min()
            .unwrap_or(0);
        let mut out = Vec::new();
        for n in data.map.neighbours(at) {
            let pinned_by_zoc = || self.zoc_at(data, &n, phasing.other()) == Zoc::Exerts;
            let mut any = false;
            for p in self
                .pieces
                .values()
                .filter(|p| p.side != phasing && p.hex() == Some(&n))
            {
                // Rommel reacts like a motorised combat unit (31.0).
                let commanding = p.kind == PieceKind::Commander && !self.rommel_recalled;
                if (p.kind != PieceKind::Combat && !commanding)
                    || !self.mobility(data, p).motorised()
                    || p.cohesion.collapsed(&self.ruleset)
                    || p.contact
                    || p.engaged
                    || p.status == Some(Status::InTraining)
                    || p.reserve == Some(Reserve::II)
                    || (assault && self.pins(data, pinner_sp, pinner_cpa, p))
                {
                    continue;
                }
                if !any && pinned_by_zoc() {
                    break;
                }
                any = true;
                out.push(p.id.clone());
            }
        }
        out
    }

    // TODO(32.22): fuel is not charged here until cna answers "What a unit may
    // do that cannot pay under abstract supply" in docs/autopilot/PROGRESS.md.
    /// `Order::React`: some of the units offered a reaction move along
    /// `path` (8.5x, R-007).
    pub(crate) fn react(
        &mut self,
        data: &Data,
        player: Side,
        pieces: &[PieceId],
        path: &[HexId],
    ) -> Result<Vec<Event>, Illegal> {
        let Some(Pending::Reaction { side, eligible }) = self.pending.clone() else {
            return Err(Illegal::new("8.5x", "no reaction is pending"));
        };
        if side != player {
            return Err(Illegal::new("8.5x", format!("{side} reacts, not {player}")));
        }
        let pieces = &self.carried(pieces)?;
        for id in pieces {
            if !eligible.contains(id) {
                return Err(Illegal::new(
                    "8.5x",
                    format!("{id} was not offered a reaction (eligible: {eligible:?})"),
                ));
            }
        }
        let from = self.moving_stack(player, pieces, false)?;
        let plan = self.plan_with(data, pieces, &from, path, crate::reach::Purpose::React)?;
        if let Some((before, _)) = &plan.interrupt {
            return Err(Illegal::new(
                "8.5x",
                format!("{before}: a reacting unit never enters an enemy ZOC"),
            ));
        }
        if plan.ends_in_zoc {
            let (hex, _) = plan.steps.last().expect("a step into the ZOC");
            return Err(Illegal::new(
                "8.5x",
                format!("{hex} is in an enemy ZOC: a reacting unit never enters one"),
            ));
        }
        // The ceiling binds where the reaction ends (9.14): the only
        // moment the engine can refuse it, so it is settled before any
        // CP is charged. The stack stands on its end hex for the count.
        let (end, _) = plan.steps.last().expect("a priced path has a step");
        for id in pieces {
            self.pieces.get_mut(id).expect("checked").location = Location::Hex(end.clone());
        }
        if let Some(e) = self.over_ceiling_at(data, end) {
            for id in pieces {
                self.pieces.get_mut(id).expect("checked").location = Location::Hex(from.clone());
            }
            return Err(e);
        }
        let mut events = Vec::new();
        let mut at = from.clone();
        let storm =
            crate::breakdown::mostly_in_sandstorm(self, plan.steps.iter().map(|(h, c)| (h, *c)));
        for (to, cost) in &plan.steps {
            self.collect_breakdown(data, pieces, &at, to);
            self.capture_wrecks(data, pieces, to, &mut events);
            self.mine_rolls(data, pieces, to, &mut events);
            if let Some(side) = pieces.first().map(|id| self.pieces[id].side) {
                self.reveal_minefield(side, to, &mut events);
            }
            self.surrender_collapsed(data, pieces, to, &mut events);
            self.free_prisoners(data, pieces, to, &mut events);
            events.push(Event::Moved {
                pieces: pieces.to_vec(),
                from: at.clone(),
                to: to.clone(),
                cost: *cost,
            });
            for id in pieces {
                self.charge(id, *cost, true, &mut events);
            }
            at = to.clone();
        }
        // It stops: a breakdown check (21.2x).
        self.check_breakdown(data, pieces, storm, &from, &mut events);
        // Its new hex is a possible ZOC source the phasing player has not
        // been told about.
        self.zoc.remove(&at);
        let left: Vec<PieceId> = eligible
            .into_iter()
            .filter(|id| !pieces.contains(id))
            .collect();
        self.pending = (!left.is_empty()).then_some(Pending::Reaction {
            side,
            eligible: left,
        });
        Ok(events)
    }

    /// `Pass` while a reaction is offered: no (more) reaction.
    pub(crate) fn decline_reaction(&mut self) -> Vec<Event> {
        self.pending = None;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;
    use cna_data::cp::Cp;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn snapshot(state: &GameState) -> String {
        serde_json::to_string(state).unwrap()
    }

    fn react(pieces: &[&str], path: &[&str]) -> Order {
        Order::React {
            pieces: pieces.iter().map(|s| pid(s)).collect(),
            path: path.iter().map(|s| hex(s)).collect(),
        }
    }

    /// The Axis LXII(L) tank battalion (CPA 30) put at C3924, two hexes
    /// from the four Commonwealth units at C3922, in the Axis movement
    /// segment; two Libyan battalions put at C3820, whose ZOC covers
    /// C3821. The Commonwealth has declared that C3922 exerts no ZOC.
    fn tanks_near_halfaya(data: &Data) -> GameState {
        tanks_near_halfaya_after(data, |_| {})
    }

    /// The same, with `prepare` run on the state before the tanks move.
    fn tanks_near_halfaya_after(data: &Data, prepare: impl FnOnce(&mut GameState)) -> GameState {
        let mut state = moving(data, 3, Side::Axis);
        let tanks = pid("unit:it:lxiil-tank-bn");
        state.put(&tanks, Location::Hex(hex("C3924")));
        for id in ["unit:it:xvii-libyan-bn", "unit:it:xix-libyan-bn"] {
            state.put(&pid(id), Location::Hex(hex("C3820")));
        }
        prepare(&mut state);
        assert!(state.could_exert(data, &hex("C3820")));
        let events = state
            .apply(data, Side::Axis, mv(&["unit:it:lxiil-tank-bn"], &["C3923"]))
            .unwrap();
        assert!(matches!(events[0], Event::MoveInterrupted { .. }));
        state
            .apply(data, Side::Cw, Order::DeclareZoc { exerting: vec![] })
            .unwrap();
        state
    }

    #[test]
    fn motorised_units_next_to_the_mover_are_offered_a_reaction() {
        let data = graziani();
        let mut state = tanks_near_halfaya(&data);
        let events = state
            .apply(
                &data,
                Side::Axis,
                mv(&["unit:it:lxiil-tank-bn"], &["C3923"]),
            )
            .unwrap();
        // 4 RHA and 7 Med (CPA 20, vehicles) may react; the two foot
        // battalions may not (R-006).
        let eligible = vec![pid("unit:cw:4-rha"), pid("unit:cw:7-med")];
        assert_eq!(
            events.last(),
            Some(&Event::ReactionOffered {
                pieces: eligible.clone(),
                at: hex("C3923"),
            })
        );
        assert_eq!(
            state.pending,
            Some(Pending::Reaction {
                side: Side::Cw,
                eligible: eligible.clone(),
            })
        );
        assert_eq!(state.to_act(), Some(Side::Cw));
        let before = snapshot(&state);
        // Not the Axis's turn; a foot unit may not react; nor may a unit
        // react into the Axis ZOC over C3821 (from the Libyans at C3820).
        let err = state
            .apply(&data, Side::Axis, Order::EndSegment)
            .unwrap_err();
        assert_eq!(err.case, "5.2");
        let err = state
            .apply(&data, Side::Cw, react(&["unit:cw:1-krrc"], &["C3822"]))
            .unwrap_err();
        assert_eq!(err.case, "8.5x");
        let err = state
            .apply(&data, Side::Cw, react(&["unit:cw:7-med"], &["C3821"]))
            .unwrap_err();
        assert_eq!(err.case, "8.5x");
        assert!(err.reason.contains("ZOC"), "{}", err.reason);
        assert_eq!(snapshot(&state), before);
        // 7 Med pulls back along the unfinished road (a track) to C3822:
        // 1 CP, charged in the enemy half.
        let events = state
            .apply(&data, Side::Cw, react(&["unit:cw:7-med"], &["C3822"]))
            .unwrap();
        assert_eq!(
            events[0],
            Event::Moved {
                pieces: vec![pid("unit:cw:7-med")],
                from: hex("C3922"),
                to: hex("C3822"),
                cost: Cp::whole(1)
            }
        );
        let med = &state.pieces[&pid("unit:cw:7-med")];
        assert_eq!(med.hex(), Some(&hex("C3822")));
        assert_eq!(med.cp_spent, Cp::whole(1));
        assert_eq!(med.cp_enemy_half, Cp::whole(1));
        assert_eq!(
            state.pending,
            Some(Pending::Reaction {
                side: Side::Cw,
                eligible: vec![pid("unit:cw:4-rha")],
            })
        );
        // Enough: the Axis goes on, and a reaction is not logged as a
        // phasing move.
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.pending, None);
        assert_eq!(state.to_act(), Some(Side::Axis));
        assert_eq!(state.clock.segment, Some(crate::clock::Segment::Movement));
        let err = state
            .apply(&data, Side::Cw, react(&["unit:cw:4-rha"], &["C3822"]))
            .unwrap_err();
        assert_eq!(err.case, "5.2");
    }

    #[test]
    fn units_in_contact_or_in_reserve_ii_are_not_offered_a_reaction() {
        let data = graziani();
        // 7 Med is in contact (R-008) and 4 RHA sits in Reserve II
        // (18.2x): neither may react, so no reaction is offered at all.
        let mut state = tanks_near_halfaya_after(&data, |state| {
            state.pieces.get_mut(&pid("unit:cw:7-med")).unwrap().contact = true;
            state.pieces.get_mut(&pid("unit:cw:4-rha")).unwrap().reserve = Some(Reserve::II);
        });
        let events = state
            .apply(
                &data,
                Side::Axis,
                mv(&["unit:it:lxiil-tank-bn"], &["C3923"]),
            )
            .unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::ReactionOffered { .. })),
            "{events:?}"
        );
        assert_eq!(state.pending, None);
        assert_eq!(state.to_act(), Some(Side::Axis));
    }

    #[test]
    fn a_reaction_refused_by_the_ceiling_changes_nothing() {
        let data = graziani();
        // Six foot battalions fill C3822 (clear, ceiling 6) before the
        // tanks come up.
        let mut state = tanks_near_halfaya_after(&data, |state| {
            let foot: Vec<PieceId> = state
                .pieces
                .values()
                .filter(|p| {
                    p.side == Side::Cw
                        && p.kind == PieceKind::Combat
                        && p.hex().is_some_and(|h| h.sheet() == 'E')
                        && state.cpa(p).is_some_and(|c| c <= 10)
                        && cna_rules::stacking::points(&data.stacking, p.equivalent) == 1
                })
                .map(|p| p.id.clone())
                .take(6)
                .collect();
            assert_eq!(foot.len(), 6);
            for id in foot {
                state.put(&id, Location::Hex(hex("C3822")));
            }
        });
        assert_eq!(state.stacked(&data, &hex("C3822")), 6);
        state
            .apply(
                &data,
                Side::Axis,
                mv(&["unit:it:lxiil-tank-bn"], &["C3923"]),
            )
            .unwrap();
        assert!(matches!(state.pending, Some(Pending::Reaction { .. })));
        let before = snapshot(&state);
        let err = state
            .apply(&data, Side::Cw, react(&["unit:cw:7-med"], &["C3822"]))
            .unwrap_err();
        assert_eq!(err.case, "9.14");
        assert!(err.reason.contains("C3822"), "{}", err.reason);
        // Nothing moved, nothing was charged, nothing was logged.
        assert_eq!(snapshot(&state), before);
        let med = &state.pieces[&pid("unit:cw:7-med")];
        assert_eq!(med.hex(), Some(&hex("C3922")));
        assert_eq!(med.cp_spent, Cp::ZERO);
        assert_eq!(med.cp_enemy_half, Cp::ZERO);
        // The offer still stands.
        assert_eq!(
            state.pending,
            Some(Pending::Reaction {
                side: Side::Cw,
                eligible: vec![pid("unit:cw:4-rha"), pid("unit:cw:7-med")],
            })
        );
    }

    #[test]
    fn a_declared_assault_by_a_much_faster_unit_pins() {
        let data = graziani();
        let mut state = tanks_near_halfaya(&data);
        // CPA 30 against 20: 10 over, so with an assault declared neither
        // gun unit may react, and no offer is made.
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::Move {
                    pieces: vec![pid("unit:it:lxiil-tank-bn")],
                    path: vec![hex("C3923")],
                    assault: true,
                    fuel_from: None,
                },
            )
            .unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::ReactionOffered { .. }))
        );
        assert_eq!(state.pending, None);
        assert_eq!(state.to_act(), Some(Side::Axis));
    }

    #[test]
    fn size_limits_on_pinning() {
        let data = graziani();
        let state = tanks_near_halfaya(&data);
        let rha = &state.pieces[&pid("unit:cw:4-rha")];
        // 1 SP at CPA 30 pins a battalion at CPA 20 ...
        assert!(state.pins(&data, 1, 30, rha));
        // ... but not at CPA 25 (5 short of the margin, 8.5x) ...
        assert!(!state.pins(&data, 1, 25, rha));
        // ... and a company (0 SP) pins a battalion but nothing of
        // brigade size, whatever its speed.
        assert!(state.pins(&data, 0, 45, rha));
        let mut brigade = rha.clone();
        brigade.equivalent = Some(cna_data::characteristics::Equivalent::Brigade);
        assert!(!state.pins(&data, 0, 45, &brigade));
        assert!(state.pins(&data, 1, 45, &brigade));
    }

    #[test]
    fn a_unit_under_the_mover_s_zoc_cannot_react() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        // Two Libyan battalions (2 SP, 10 defence points) next to 11
        // Hussars at C3020 exert a ZOC over it: no reaction. (The
        // battalions come from C3617 by the tracks, well within CPA 10.)
        for id in ["unit:it:xvii-libyan-bn", "unit:it:xix-libyan-bn"] {
            state.put(&pid(id), Location::Hex(hex("C3121")));
        }
        assert!(data.map.adjacent(&hex("C3121"), &hex("C3020")));
        let events = state
            .apply(
                &data,
                Side::Axis,
                mv(
                    &["unit:it:xvii-libyan-bn", "unit:it:xix-libyan-bn"],
                    &["C3021"],
                ),
            )
            .unwrap();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::ReactionOffered { .. })),
            "{events:?}"
        );
        assert_eq!(state.pending, None);
    }
}
