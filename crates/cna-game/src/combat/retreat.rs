//! Retreats: before assault (SPI 13.x) and forced (15.82, 10.36).

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;

impl GameState {
    /// `Order::RetreatBeforeAssault` (13.2x): ordinary voluntary movement
    /// by an unpinned, uncollapsed non-phasing stack, paid in the enemy
    /// half; a stack not adjacent to an enemy combat unit may spend at
    /// most 4 CP or move one hex (13.22). Any direction (R-010).
    pub(crate) fn retreat_before_assault(
        &mut self,
        data: &Data,
        player: Side,
        pieces: &[PieceId],
        path: &[HexId],
    ) -> Result<Vec<Event>, Illegal> {
        let from = self.moving_stack(player, pieces, false)?;
        for id in pieces {
            let p = &self.pieces[id];
            if self.combat.pinned.contains(id) {
                return Err(Illegal::new("12.44", format!("{id} is pinned")));
            }
            if self.combat.retreated.contains(id) {
                return Err(Illegal::new(
                    "13.28",
                    format!("{id} has retreated before assault this segment"),
                ));
            }
            if p.cohesion.collapsed(&self.ruleset) {
                return Err(Illegal::new(
                    "13.1",
                    format!("{id} has collapsed: it cannot move"),
                ));
            }
        }
        let plan = self.plan(data, pieces, &from, path)?;
        if let Some((before, _)) = &plan.interrupt {
            return Err(Illegal::new(
                "13.2x",
                format!("{before}: an undeclared enemy ZOC on a retreat path is not supported"),
            ));
        }
        let break_off = |p: &Piece| {
            if p.engaged {
                Cp::whole(4)
            } else if p.contact {
                Cp::whole(2)
            } else {
                Cp::ZERO
            }
        };
        let mut total: Cp = plan.steps.iter().map(|(_, c)| *c).sum();
        if plan.break_off {
            total += pieces
                .iter()
                .map(|id| break_off(&self.pieces[id]))
                .max()
                .unwrap_or(Cp::ZERO);
        }
        let adjacent = data.map.neighbours(&from).iter().any(|n| {
            self.pieces
                .values()
                .any(|q| q.side != player && q.kind == PieceKind::Combat && q.hex() == Some(n))
        });
        if !adjacent && plan.steps.len() > 1 && total > Cp::whole(4) {
            return Err(Illegal::new(
                "13.22",
                format!(
                    "{} is not adjacent to an enemy combat unit: it may spend 4 CP or move one hex, not {total} over {} hexes",
                    pieces[0],
                    plan.steps.len()
                ),
            ));
        }
        let mut events = Vec::new();
        if plan.break_off {
            for id in pieces {
                let cost = break_off(&self.pieces[id]);
                if cost > Cp::ZERO {
                    events.push(Event::ContactBroken {
                        piece: id.clone(),
                        cost,
                    });
                    self.charge(id, cost, true, &mut events);
                }
                let p = self.pieces.get_mut(id).expect("checked");
                p.contact = false;
                p.engaged = false;
            }
        }
        let mut at = from.clone();
        for (to, cost) in plan.steps {
            for id in pieces {
                self.pieces.get_mut(id).expect("checked").location = Location::Hex(to.clone());
            }
            events.push(Event::Moved {
                pieces: pieces.to_vec(),
                from: at,
                to: to.clone(),
                cost,
            });
            for id in pieces {
                self.charge(id, cost, true, &mut events);
            }
            at = to;
        }
        if at != from {
            self.release_contact_around(data, &from, player);
        }
        self.combat.retreated.extend(pieces.iter().cloned());
        events.push(Event::RetreatedBeforeAssault {
            pieces: pieces.to_vec(),
            to: at,
        });
        Ok(events)
    }

    /// `Order::Retreat` or `Pass` while a forced retreat is pending.
    pub(crate) fn forced_retreat(
        &mut self,
        _data: &Data,
        _player: Side,
        _path: Option<&[HexId]>,
    ) -> Result<Vec<Event>, Illegal> {
        Err(Illegal::new("15.82", "forced retreats are not built yet"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::segment::test_support::at_sollum;
    use crate::data::Data;
    use crate::event::Event;
    use crate::movement::test_support::{hex, pid};
    use crate::order::Order;
    use crate::state::{GameState, Pending};
    use cna_data::cp::Cp;
    use cna_data::scenario::Side;

    /// The step open, the Axis to answer.
    fn retreating(data: &Data) -> GameState {
        let mut state = at_sollum(data);
        for (side, order) in [
            (Side::Cw, Order::GunPositions { forward: vec![] }),
            (Side::Axis, Order::GunPositions { forward: vec![] }),
            (Side::Cw, Order::Barrage { fires: vec![] }),
            (Side::Axis, Order::Barrage { fires: vec![] }),
        ] {
            state.apply(data, side, order).unwrap();
        }
        assert_eq!(
            state.pending,
            Some(Pending::RetreatBeforeAssault { side: Side::Axis })
        );
        state
    }

    fn retreat(pieces: &[&str], path: &[&str]) -> Order {
        Order::RetreatBeforeAssault {
            pieces: pieces.iter().map(|p| pid(p)).collect(),
            path: path.iter().map(|h| hex(h)).collect(),
        }
    }

    #[test]
    fn an_adjacent_unit_pulls_back_in_the_enemy_half_unless_pinned() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = retreating(&data);
        let bn = pid("unit:it:viii-libyan-bn");
        // A neighbour of C4020 out of reach of C4021, and empty.
        let away = data
            .map
            .neighbours(&hex("C4020"))
            .into_iter()
            .find(|n| {
                !data.map.adjacent(n, &hex("C4021"))
                    && !state.pieces.values().any(|p| p.hex() == Some(n))
            })
            .unwrap();
        state.combat.pinned.insert(bn.clone());
        let err = state
            .apply(
                &data,
                Side::Axis,
                retreat(&["unit:it:viii-libyan-bn"], &[away.as_str()]),
            )
            .unwrap_err();
        assert_eq!(err.case, "12.44");
        state.combat.pinned.clear();
        let events = state
            .apply(
                &data,
                Side::Axis,
                retreat(&["unit:it:viii-libyan-bn"], &[away.as_str()]),
            )
            .unwrap();
        assert_eq!(state.pieces[&bn].hex(), Some(&away));
        assert!(state.pieces[&bn].cp_enemy_half > Cp::ZERO);
        assert_eq!(state.pieces[&bn].cp_enemy_half, state.pieces[&bn].cp_spent);
        assert!(state.combat.retreated.contains(&bn));
        assert_eq!(
            events.last(),
            Some(&Event::RetreatedBeforeAssault {
                pieces: vec![bn.clone()],
                to: away.clone(),
            })
        );
        // Once a segment; the step stays open for others.
        let err = state
            .apply(
                &data,
                Side::Axis,
                retreat(&["unit:it:viii-libyan-bn"], &[away.as_str()]),
            )
            .unwrap_err();
        assert_eq!(err.case, "13.28");
        assert_eq!(
            state.pending,
            Some(Pending::RetreatBeforeAssault { side: Side::Axis })
        );
        // Not the Commonwealth's turn to move.
        let err = state
            .apply(&data, Side::Cw, retreat(&["unit:cw:1-krrc"], &["C4022"]))
            .unwrap_err();
        assert_ne!(err.case, "");
    }

    #[test]
    fn a_unit_away_from_the_enemy_has_four_cp_or_one_hex() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = retreating(&data);
        let bn = pid("unit:it:xvii-libyan-bn");
        assert_eq!(state.pieces[&bn].hex(), Some(&hex("C3617")));
        // Walk away from the enemy: three hexes west along row 36, 2 CP each.
        let path3 = ["C3616", "C3615", "C3614"];
        let err = state
            .apply(
                &data,
                Side::Axis,
                retreat(&["unit:it:xvii-libyan-bn"], &path3),
            )
            .unwrap_err();
        assert_eq!(err.case, "13.22");
        state
            .apply(
                &data,
                Side::Axis,
                retreat(&["unit:it:xvii-libyan-bn"], &path3[..1]),
            )
            .unwrap();
        assert_eq!(state.pieces[&bn].hex(), Some(&hex("C3616")));
    }
}
