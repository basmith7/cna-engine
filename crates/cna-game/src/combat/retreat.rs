//! Retreats: before assault (SPI 13.x) and forced (15.82, 10.36).

use super::RetreatReason;
use super::losses::Share;
use super::strength::Kind;
use crate::clock::Segment;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use crate::state::Pending;
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::close_assault::may_buy_out;

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

    /// Hexes from `a` to `b`, up to `limit`; `limit + 1` beyond it.
    fn distance(data: &Data, a: &HexId, b: &HexId, limit: u32) -> u32 {
        (0..=limit)
            .find(|n| data.map.within(a, *n).contains(b))
            .unwrap_or(limit + 1)
    }

    /// `Order::Retreat` or `Pass` while a forced retreat is pending
    /// (15.82, 15.29, 10.36): the stack walks `path` as a move that may
    /// leave an enemy ZOC, ending at least the ordered hexes from every
    /// hex that forced it; each hex short costs 10 % of its raw points
    /// where the ruleset lets it stand (R-012). A must-attack retreat
    /// may not stop short; with no path the stack surrenders.
    pub(crate) fn forced_retreat(
        &mut self,
        data: &Data,
        player: Side,
        path: Option<&[HexId]>,
    ) -> Result<Vec<Event>, Illegal> {
        let Some(Pending::ForcedRetreat {
            side,
            pieces,
            hexes,
            from,
            reason,
            raw,
        }) = self.pending.clone()
        else {
            return Err(Illegal::new("15.82", "no retreat is pending"));
        };
        if player != side {
            return Err(Illegal::new(
                "15.82",
                format!("{side} retreats, not {player}"),
            ));
        }
        let pieces: Vec<PieceId> = pieces
            .into_iter()
            .filter(|id| self.pieces[id].hex().is_some())
            .collect();
        let mut events = Vec::new();
        let Some(start) = pieces.first().and_then(|id| self.pieces[id].hex().cloned()) else {
            self.pending = None;
            return Ok(events);
        };
        let phasing = self.phasing() == Some(side);
        let (end, plan) = match path {
            Some(path) => {
                let plan = self.plan_with(data, &pieces, &start, path, true)?;
                if let Some((before, _)) = &plan.interrupt {
                    return Err(match reason {
                        RetreatReason::MustAttack => Illegal::new(
                            "10.36",
                            format!(
                                "{before}: a must-attack retreat may not pass through enemy ZOC"
                            ),
                        ),
                        _ => Illegal::new(
                            "15.82",
                            format!(
                                "{before}: an undeclared enemy ZOC on a retreat path is not supported"
                            ),
                        ),
                    });
                }
                (
                    plan.steps
                        .last()
                        .map(|(h, _)| h.clone())
                        .unwrap_or(start.clone()),
                    Some(plan),
                )
            }
            None => (start.clone(), None),
        };
        let reached = from
            .iter()
            .map(|f| Self::distance(data, f, &end, hexes))
            .min()
            .unwrap_or(hexes);
        let short = hexes.saturating_sub(reached);
        if short > 0 {
            match reason {
                RetreatReason::MustAttack => {
                    if path.is_some() {
                        return Err(Illegal::new(
                            "10.36",
                            format!("a must-attack retreat goes the full {hexes} hexes"),
                        ));
                    }
                }
                _ => {
                    let path_exists = data.map.neighbours(&start).iter().any(|n| {
                        !self.pieces.values().any(|q| q.hex() == Some(n))
                            && self.zoc_at(data, n, side) == Zoc::Clear
                    });
                    if !may_buy_out(&self.ruleset, path_exists) {
                        return Err(Illegal::new(
                            "15.82",
                            format!("the stack must retreat the full {hexes} hexes (R-012)"),
                        ));
                    }
                }
            }
        }
        if let Some(plan) = plan {
            let break_off = |p: &Piece| {
                if p.engaged {
                    Cp::whole(4)
                } else if p.contact {
                    Cp::whole(2)
                } else {
                    Cp::ZERO
                }
            };
            if plan.break_off {
                for id in &pieces {
                    let cost = break_off(&self.pieces[id]);
                    if cost > Cp::ZERO {
                        events.push(Event::ContactBroken {
                            piece: id.clone(),
                            cost,
                        });
                        self.charge(id, cost, !phasing, &mut events);
                    }
                }
            }
            for id in &pieces {
                let p = self.pieces.get_mut(id).expect("checked");
                p.contact = false;
                p.engaged = false;
            }
            let mut at = start.clone();
            for (to, cost) in plan.steps {
                for id in &pieces {
                    self.pieces.get_mut(id).expect("checked").location = Location::Hex(to.clone());
                }
                events.push(Event::Moved {
                    pieces: pieces.clone(),
                    from: at,
                    to: to.clone(),
                    cost,
                });
                for id in &pieces {
                    self.charge(id, cost, !phasing, &mut events);
                }
                at = to;
            }
            if at != start {
                self.release_contact_around(data, &start, side);
            }
        }
        self.pending = None;
        if short > 0 {
            if reason == RetreatReason::MustAttack {
                let toe: Vec<(PieceId, u32)> = pieces
                    .iter()
                    .map(|id| (id.clone(), self.pieces[id].toe))
                    .collect();
                for (id, n) in toe {
                    self.take_loss(&id, n, &mut events);
                }
                events.push(Event::Surrendered {
                    pieces: pieces.clone(),
                });
            } else {
                events.push(Event::RetreatBoughtOut {
                    pieces: pieces.clone(),
                    short,
                });
                let lost = (raw * 10 * short).div_ceil(100);
                let paren_ok = !self.plain_in_hex(data, &start, Kind::Defence);
                let mut shares: Vec<Share> = pieces
                    .iter()
                    .map(|id| Share {
                        piece: id.clone(),
                        points: self.pieces[id].toe,
                    })
                    .collect();
                self.absorb(
                    data,
                    &mut shares,
                    |p| p.per_point(Kind::Defence, paren_ok),
                    lost,
                    &mut events,
                );
            }
        }
        if reason == RetreatReason::MustAttack && !self.end_close_assault(data, &mut events) {
            self.open_step(data, Segment::ReserveRelease, &mut events);
        }
        Ok(events)
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
