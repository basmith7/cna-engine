//! Attachment (SPI 19.1x, 4.25): a unit is assigned to one parent (its
//! `parent`) and may be attached, merged into a parent's counter, which
//! is `attached_to`. A unit set up or arriving with its parent starts
//! attached. Stacking (9.1x) and an HQ's CPA (6.15) read attachment.
//! A move, reaction or retreat of a parent carries what is attached to
//! it; an attached unit never moves without its parent (19.4x).

use crate::clock::{Phase, Segment};
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::scenario::Side;
use std::collections::BTreeSet;

impl GameState {
    /// Attaches every unit standing with its parent, on a hex or in a
    /// box (4.25), among `pieces` and the units assigned to them or, for
    /// none, every piece; and marks attachment as tracked in this state.
    pub(crate) fn attach_where_stacked(&mut self, pieces: Option<&[PieceId]>) {
        let ids: Vec<PieceId> = match pieces {
            Some(ids) => {
                let units: Vec<_> = ids.iter().filter_map(|id| id.unit()).collect();
                let under = self
                    .pieces
                    .values()
                    .filter(|q| q.parent.as_ref().is_some_and(|u| units.contains(u)))
                    .map(|q| q.id.clone());
                ids.iter().cloned().chain(under).collect()
            }
            None => self.pieces.keys().cloned().collect(),
        };
        for id in ids {
            let p = &self.pieces[&id];
            let Some(parent) = p.parent.clone() else {
                continue;
            };
            let together = matches!(p.location, Location::Hex(_) | Location::Box(_))
                && self
                    .pieces
                    .get(&PieceId::from(&parent))
                    .is_some_and(|q| q.location == p.location);
            if together {
                self.pieces.get_mut(&id).expect("listed").attached_to = Some(parent);
            }
        }
        self.attachment_tracked = true;
    }

    /// The pieces a move of `listed` takes: those listed, then everything
    /// attached to them, nested attachments included, in id order. A
    /// listed unit attached to a parent left behind is refused, and so is
    /// a unit detached this movement segment moving without its parent,
    /// or the parent without it (19.4x).
    pub(crate) fn carried(&self, listed: &[PieceId]) -> Result<Vec<PieceId>, Illegal> {
        let mut taken: BTreeSet<PieceId> = listed.iter().cloned().collect();
        let mut heads: Vec<PieceId> = listed.to_vec();
        let mut added = BTreeSet::new();
        while let Some(head) = heads.pop() {
            let Some(unit) = head.unit() else {
                continue;
            };
            for q in self.pieces.values() {
                if q.attached_to.as_ref() == Some(&unit) && taken.insert(q.id.clone()) {
                    added.insert(q.id.clone());
                    heads.push(q.id.clone());
                }
            }
        }
        for (unit, parent) in &self.split {
            if taken.contains(unit) != taken.contains(parent) {
                return Err(Illegal::new(
                    "19.4x",
                    format!(
                        "{unit} detached from {parent} this movement segment: \
                         both stop unless they go on together"
                    ),
                ));
            }
        }
        for id in listed {
            let Some(parent) = self.pieces.get(id).and_then(|p| p.attached_to.as_ref()) else {
                continue;
            };
            if !taken.contains(&PieceId::from(parent)) {
                return Err(Illegal::new(
                    "19.4x",
                    format!("{id} is attached to {parent}: detach it first"),
                ));
            }
        }
        Ok(listed.iter().cloned().chain(added).collect())
    }

    /// A piece gone from the map takes no attachments with it: what was
    /// attached to it stands alone where it is, and it is attached to
    /// nothing.
    pub(crate) fn release_attached(&mut self, id: &PieceId) {
        let unit = id.unit();
        for q in self.pieces.values_mut() {
            if unit.is_some() && q.attached_to == unit {
                q.attached_to = None;
            }
        }
        if let Some(p) = self.pieces.get_mut(id) {
            p.attached_to = None;
        }
    }

    /// Attachment changes in the player's phase C or its own movement
    /// segment (19.4x).
    fn reorganising(&self, player: Side) -> Result<(), Illegal> {
        let own_segment = self.clock.phase == Phase::MovementCombat
            && self.clock.segment == Some(Segment::Movement)
            && self.phasing() == Some(player);
        if self.clock.phase == Phase::Organisation || own_segment {
            Ok(())
        } else {
            Err(Illegal::new(
                "19.4x",
                format!(
                    "units attach and detach in phase C or their own movement segment, not at {}",
                    self.clock
                ),
            ))
        }
    }

    /// One of the player's combat units or HQs, on the map.
    fn own_unit(&self, player: Side, id: &PieceId) -> Result<&crate::piece::Piece, Illegal> {
        self.pieces
            .get(id)
            .filter(|p| p.side == player && matches!(p.kind, PieceKind::Combat | PieceKind::Hq))
            .filter(|p| p.hex().is_some())
            .ok_or_else(|| {
                Illegal::new(
                    "19.4x",
                    format!("{id} is not one of {player}'s units on the map"),
                )
            })
    }

    /// A CP each from the unit and its parent (19.4x).
    fn charge_attachment(&mut self, a: &PieceId, b: &PieceId) {
        for id in [a, b] {
            self.pieces.get_mut(id).expect("checked").cp_spent += Cp::whole(1);
        }
    }

    /// `Order::Attach`: `piece` merges into `to`'s counter in their hex
    /// (19.4x).
    pub(crate) fn attach(
        &mut self,
        player: Side,
        piece: &PieceId,
        to: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        self.reorganising(player)?;
        let p = self.own_unit(player, piece)?;
        let parent = self.own_unit(player, to)?;
        if let Some(on) = &p.attached_to {
            return Err(Illegal::new(
                "19.4x",
                format!("{piece} is attached to {on}: detach it first"),
            ));
        }
        if p.hex() != parent.hex() {
            return Err(Illegal::new(
                "19.4x",
                format!("{piece} and {to} are not in one hex"),
            ));
        }
        let unit = to.unit().expect("a unit has a sheet id");
        if p.parent.as_ref() != Some(&unit) {
            return Err(Illegal::new(
                "19.5",
                format!(
                    "{piece} is not assigned to {to}: attaching it waits on the attachment limits"
                ),
            ));
        }
        self.charge_attachment(piece, to);
        self.pieces.get_mut(piece).expect("checked").attached_to = Some(unit);
        Ok(vec![Event::Attached {
            piece: piece.clone(),
            to: to.clone(),
        }])
    }

    /// `Order::Detach`: `piece` stands on its own again (19.4x).
    pub(crate) fn detach(&mut self, player: Side, piece: &PieceId) -> Result<Vec<Event>, Illegal> {
        self.reorganising(player)?;
        let Some(on) = self.own_unit(player, piece)?.attached_to.clone() else {
            return Err(Illegal::new("19.4x", format!("{piece} is not attached")));
        };
        let from = PieceId::from(&on);
        self.charge_attachment(piece, &from);
        self.pieces.get_mut(piece).expect("checked").attached_to = None;
        if self.clock.phase == Phase::MovementCombat {
            self.split.insert(piece.clone(), from.clone());
        }
        Ok(vec![Event::Detached {
            piece: piece.clone(),
            from,
        }])
    }
}

#[cfg(test)]
impl GameState {
    /// Puts a piece somewhere by hand, as a test fixture does: it stands
    /// alone there, detached from its parent and with nothing attached.
    pub(crate) fn put(&mut self, id: &PieceId, at: Location) {
        self.release_attached(id);
        self.pieces
            .get_mut(id)
            .expect("a piece of the test")
            .location = at;
    }

    /// Splits a piece from its formation where it stands (a fixture).
    pub(crate) fn stand_alone(&mut self, id: &PieceId) {
        let at = self.pieces[id].location.clone();
        self.put(id, at);
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Data;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;
    use crate::piece::{Location, Piece};
    use crate::state::GameState;
    use cna_data::cp::Cp;
    use cna_data::scenario::Side;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn with_parent(state: &GameState, p: &Piece) -> bool {
        let Some(parent) = &p.parent else {
            return false;
        };
        state
            .pieces
            .get(&crate::piece::PieceId::from(parent))
            .is_some_and(|q| {
                matches!(q.location, Location::Hex(_) | Location::Box(_))
                    && q.location == p.location
            })
    }

    #[test]
    fn set_up_attaches_exactly_the_units_standing_with_their_parent() {
        let data = graziani();
        let state = moving(&data, 3, Side::Axis);
        assert!(state.attachment_tracked);
        let mut attached = 0;
        let mut apart = 0;
        for p in state.pieces.values() {
            if with_parent(&state, p) {
                assert_eq!(p.attached_to, p.parent, "{}", p.id);
                attached += 1;
            } else {
                assert_eq!(p.attached_to, None, "{}", p.id);
                apart += usize::from(p.parent.is_some());
            }
        }
        assert!(attached > 0 && apart > 0, "{attached} {apart}");
        // Gruppo Maletti's tank battalion, attached in the set-up (4.25).
        let tanks = &state.pieces[&pid("unit:it:iim-tank-bn")];
        assert_eq!(tanks.hex(), Some(&hex("C3617")));
        assert!(tanks.attached_to.is_some());
    }

    #[test]
    fn a_detached_unit_stacks_on_its_own() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        assert_eq!(state.stacked(&data, &hex("C3617")), 5);
        let tanks = pid("unit:it:iim-tank-bn");
        state.pieces.get_mut(&tanks).unwrap().attached_to = None;
        assert_eq!(state.stacked(&data, &hex("C3617")), 6);
    }

    #[test]
    fn an_hq_moves_at_the_pace_of_its_attached_units_only() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        // The 1st Libyan Division HQ moves at its slowest attached
        // unit's 10 (6.15); with nothing attached, at its own.
        let hq = pid("unit:it:1st-libyan-infantry-hq");
        assert_eq!(state.cpa(&state.pieces[&hq]), Some(10));
        let ids: Vec<_> = state.pieces.keys().cloned().collect();
        for id in ids {
            let p = state.pieces.get_mut(&id).unwrap();
            if id != hq {
                p.attached_to = None;
            }
        }
        let own = state.pieces[&hq]
            .cpa
            .as_ref()
            .and_then(|r| r.value)
            .unwrap();
        assert!(own > 10);
        assert_eq!(state.cpa(&state.pieces[&hq]), Some(own as u32));
    }

    #[test]
    fn an_older_save_reads_its_stacked_units_as_attached() {
        let data = graziani();
        let state = moving(&data, 3, Side::Axis);
        let mut json: serde_json::Value = serde_json::from_str(&state.to_json().unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("attachment_tracked");
        for p in json["pieces"].as_object_mut().unwrap().values_mut() {
            p.as_object_mut().unwrap().remove("attached_to");
        }
        let old = GameState::from_json(&json.to_string(), &data).unwrap();
        assert_eq!(old.to_json().unwrap(), state.to_json().unwrap());
    }

    /// Gruppo Maletti at C3617, its HQ and the II M tank battalion
    /// attached to it in the set-up.
    const MALETTI: &str = "unit:it:maletti-div-hq";
    const TANKS: &str = "unit:it:iim-tank-bn";

    #[test]
    fn a_move_of_a_parent_carries_what_is_attached_to_it() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let attached: Vec<_> = state
            .pieces
            .values()
            .filter(|p| p.attached_to.is_some() && p.hex() == Some(&hex("C3617")))
            .map(|p| p.id.clone())
            .collect();
        assert!(attached.contains(&pid(TANKS)));
        let near = data.map.neighbours(&hex("C3617"))[0].clone();
        state
            .apply(&data, Side::Axis, mv(&[MALETTI], &[&near.to_string()]))
            .unwrap();
        for id in &attached {
            assert_eq!(state.pieces[id].hex(), Some(&near), "{id}");
        }
    }

    #[test]
    fn an_attached_unit_does_not_move_without_its_parent() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let near = data.map.neighbours(&hex("C3617"))[0].to_string();
        let err = state
            .apply(&data, Side::Axis, mv(&[TANKS], &[&near]))
            .unwrap_err();
        assert_eq!(err.case, "19.4x");
        assert!(err.reason.contains("detach"), "{}", err.reason);
    }

    #[test]
    fn detaching_and_attaching_an_assigned_unit_cost_a_cp_each() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let (tanks, hq) = (pid(TANKS), pid(MALETTI));
        let before = (state.pieces[&tanks].cp_spent, state.pieces[&hq].cp_spent);
        state
            .apply(
                &data,
                Side::Axis,
                Order::Detach {
                    piece: tanks.clone(),
                },
            )
            .unwrap();
        assert_eq!(state.pieces[&tanks].attached_to, None);
        assert_eq!(state.pieces[&tanks].cp_spent, before.0 + Cp::whole(1));
        assert_eq!(state.pieces[&hq].cp_spent, before.1 + Cp::whole(1));
        // Detached, it stacks on its own and may move alone.
        assert_eq!(state.stacked(&data, &hex("C3617")), 6);
        state
            .apply(
                &data,
                Side::Axis,
                Order::Attach {
                    piece: tanks.clone(),
                    to: hq.clone(),
                },
            )
            .unwrap();
        assert!(state.pieces[&tanks].attached_to.is_some());
        assert_eq!(state.pieces[&tanks].cp_spent, before.0 + Cp::whole(2));
        assert_eq!(state.pieces[&hq].cp_spent, before.1 + Cp::whole(2));
    }

    #[test]
    fn attaching_needs_the_same_hex_an_own_parent_and_the_side_s_segment() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let (tanks, hq) = (pid(TANKS), pid(MALETTI));
        state
            .apply(
                &data,
                Side::Axis,
                Order::Detach {
                    piece: tanks.clone(),
                },
            )
            .unwrap();
        let attach = |to: &str| Order::Attach {
            piece: tanks.clone(),
            to: pid(to),
        };
        // Already detached: nothing to detach.
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Detach {
                    piece: tanks.clone(),
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "19.4x");
        // Another hex.
        let near = data.map.neighbours(&hex("C3617"))[0].clone();
        state.put(&tanks, Location::Hex(near));
        assert_eq!(
            state
                .apply(&data, Side::Axis, attach(MALETTI))
                .unwrap_err()
                .case,
            "19.4x"
        );
        // A parent it is not assigned to waits on 19.5's limits.
        let other = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == crate::piece::PieceKind::Hq && p.id != hq)
            .unwrap()
            .id
            .clone();
        let at = state.pieces[&other].location.clone();
        state.pieces.get_mut(&tanks).unwrap().location = at;
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Attach {
                    piece: tanks.clone(),
                    to: other,
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "19.5");
        // The other side's movement segment: not the Axis's to change.
        let mut cw = moving(&data, 3, Side::Cw);
        let err = cw
            .apply(
                &data,
                Side::Axis,
                Order::Detach {
                    piece: tanks.clone(),
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "5.2");
    }

    #[test]
    fn a_unit_detached_while_moving_stops_with_its_parent_unless_they_go_on_together() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let tanks = pid(TANKS);
        state
            .apply(
                &data,
                Side::Axis,
                Order::Detach {
                    piece: tanks.clone(),
                },
            )
            .unwrap();
        let near = data.map.neighbours(&hex("C3617"))[0].to_string();
        for alone in [TANKS, MALETTI] {
            let err = state
                .clone()
                .apply(&data, Side::Axis, mv(&[alone], &[&near]))
                .unwrap_err();
            assert_eq!(err.case, "19.4x", "{alone}");
            assert!(err.reason.contains("together"), "{}", err.reason);
        }
        state
            .apply(&data, Side::Axis, mv(&[MALETTI, TANKS], &[&near]))
            .unwrap();
        assert_eq!(state.pieces[&tanks].hex(), Some(&hex(&near)));
    }

    #[test]
    fn what_was_attached_to_an_eliminated_unit_stands_alone() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Axis);
        let (tanks, hq) = (pid(TANKS), pid(MALETTI));
        let mut events = Vec::new();
        state.pieces.get_mut(&hq).unwrap().toe = 0;
        state.note_losses(&hq, 1, &mut events);
        assert_eq!(state.pieces[&hq].location, Location::Eliminated);
        assert_eq!(state.pieces[&tanks].attached_to, None);
        let near = data.map.neighbours(&hex("C3617"))[0].to_string();
        state
            .apply(&data, Side::Axis, mv(&[TANKS], &[&near]))
            .unwrap();
    }
}
