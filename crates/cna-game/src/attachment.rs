//! Attachment (SPI 19.1x, 4.25): a unit is assigned to one parent (its
//! `parent`) and may be attached, merged into a parent's counter, which
//! is `attached_to`. A unit set up or arriving with its parent starts
//! attached. Stacking (9.1x) and an HQ's CPA (6.15) read attachment.

use crate::piece::{Location, PieceId};
use crate::state::GameState;

impl GameState {
    /// Attaches every unit standing with its parent, on a hex or in a
    /// box (4.25), among `pieces` or, for none, every piece; and marks
    /// attachment as tracked in this state.
    pub(crate) fn attach_where_stacked(&mut self, pieces: Option<&[PieceId]>) {
        let ids: Vec<PieceId> = match pieces {
            Some(ids) => ids.to_vec(),
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
}

#[cfg(test)]
mod tests {
    use crate::data::Data;
    use crate::movement::test_support::{hex, moving, pid};
    use crate::piece::{Location, Piece};
    use crate::state::GameState;
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
}
