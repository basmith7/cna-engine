//! Assignment changes (19.2x), behind the `reassignment` module. A
//! unit's assignment is its piece's `parent`: an eliminated parent's
//! survivors become independent, an independent unit attached to a
//! parent with a free Formation Organisation Chart slot may be assigned
//! to it in phase C, and the Commonwealth may swap battalions between
//! brigades. With `armour_limits` on, a Commonwealth parent whose
//! chart row tightens at a new Game-Turn loses the units it no longer
//! has a slot for. With `parent_shifts` on, a unit makes the change of
//! parent its O/A notes date, in that month or never (19.24, R-176).

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::limits::slots_filled_of;
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::characteristics::Equivalent;
use cna_data::oa::{Arrival, Echelon, Unit, UnitId};
use cna_data::scenario::Side;
use cna_data::time::GameTime;
use std::collections::BTreeSet;

impl GameState {
    /// After every order: a unit whose parent has been eliminated is
    /// independent from then on (19.2x).
    pub(crate) fn unassign_orphans(&mut self, events: &mut Vec<Event>) {
        if !self.modules.reassignment {
            return;
        }
        let gone: BTreeSet<UnitId> = self
            .pieces
            .values()
            .filter(|p| p.location == Location::Eliminated)
            .filter_map(|p| p.id.unit())
            .collect();
        let orphans: Vec<PieceId> = self
            .pieces
            .values()
            .filter(|p| p.location != Location::Eliminated)
            .filter(|p| p.parent.as_ref().is_some_and(|u| gone.contains(u)))
            .map(|p| p.id.clone())
            .collect();
        for id in orphans {
            let p = self.pieces.get_mut(&id).expect("listed");
            let from = PieceId::from(&p.parent.take().expect("listed"));
            events.push(Event::Unassigned { piece: id, from });
        }
    }

    fn reassigning(&self, player: Side, piece: &PieceId) -> Result<&Piece, Illegal> {
        if !self.modules.reassignment {
            return Err(Illegal::new(
                "19.2x",
                "this game plays without reassignment",
            ));
        }
        if self.clock.phase != Phase::Organisation {
            return Err(Illegal::new(
                "19.2x",
                format!("assignments change in phase C, not at {}", self.clock),
            ));
        }
        self.pieces
            .get(piece)
            .filter(|p| p.side == player && matches!(p.kind, PieceKind::Combat | PieceKind::Hq))
            .filter(|p| p.location != Location::Eliminated)
            .ok_or_else(|| Illegal::new("19.2x", format!("{piece} is not one of {player}'s units")))
    }

    /// After an order that moved the clock from Game-Turn `before` on:
    /// for every Commonwealth parent in play whose Formation
    /// Organisation Chart row changed at a turn passed, as many of its
    /// assigned units in play as the new row has fewer slots filled than
    /// the old become independent (19.2x, the Commonwealth armour). Only
    /// units the new row can spare go, those the old row needed first,
    /// each pass highest id first, so a unit beyond the old row as well
    /// stays where it was. Substitutes, withdrawn units and units yet to
    /// arrive neither count nor go.
    pub(crate) fn tighten_limits(&mut self, data: &Data, before: u8, events: &mut Vec<Event>) {
        if !self.modules.armour_limits
            || !self.modules.reassignment
            || self.clock.game_turn <= before
        {
            return;
        }
        let parents: Vec<UnitId> = self
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::Hq)
            .filter(|p| p.location != Location::Eliminated)
            .filter_map(|p| p.id.unit())
            .collect();
        for turn in before + 1..=self.clock.game_turn {
            for parent in &parents {
                for id in self.spared(data, parent, turn) {
                    let p = self.pieces.get_mut(&id).expect("listed");
                    let from = PieceId::from(&p.parent.take().expect("listed"));
                    events.push(Event::Reassigned {
                        piece: id,
                        to: None,
                        from: Some(from),
                    });
                }
            }
        }
    }

    /// The units `tighten_limits` frees from `parent` as `turn` opens.
    fn spared(&self, data: &Data, parent: &UnitId, turn: u8) -> Vec<PieceId> {
        let Some(key) = data.oa.unit(parent).and_then(|u| u.organisation.as_deref()) else {
            return Vec::new();
        };
        let row = |t: u8| data.organisation.row(parent.nation(), key, u32::from(t));
        let (Some(old), Some(new)) = (row(turn - 1), row(turn)) else {
            return Vec::new();
        };
        if std::ptr::eq(old, new) {
            return Vec::new();
        }
        let fill = |row, h: &[&Unit]| slots_filled_of(&data.organisation, row, h, false);
        let mut held: Vec<(PieceId, &Unit)> = self
            .pieces
            .values()
            .filter(|p| p.location != Location::Eliminated && p.parent.as_ref() == Some(parent))
            .filter_map(|p| Some((p.id.clone(), data.oa.unit(&p.id.unit()?)?)))
            .collect();
        held.sort_by(|a, b| b.0.cmp(&a.0));
        fn sheets<'d>(h: &[(PieceId, &'d Unit)]) -> Vec<&'d Unit> {
            h.iter().map(|(_, u)| *u).collect()
        }
        let mut owed = fill(old, &sheets(&held)).saturating_sub(fill(new, &sheets(&held)));
        let mut freed = Vec::new();
        for old_needs in [true, false] {
            let mut k = 0;
            while owed > 0 && k < held.len() {
                let all = sheets(&held);
                let mut rest = held.clone();
                rest.remove(k);
                let without = sheets(&rest);
                let spare = fill(new, &without) == fill(new, &all);
                let needed = fill(old, &without) < fill(old, &all);
                if spare && needed == old_needs {
                    freed.push(held.remove(k).0);
                    owed -= 1;
                } else {
                    k += 1;
                }
            }
        }
        freed
    }

    /// The sheets of the units holding a slot of `parent`: its assigned
    /// pieces still in play, wherever they are, or withdrawn to return;
    /// the substitutes attached to it (19.4x); and the units its OA sheet
    /// assigns it that have yet to arrive, this OpStage's included (phase
    /// D comes after phase C).
    fn slot_holders<'d>(&self, data: &'d Data, parent: &UnitId) -> Vec<&'d Unit> {
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        let mut out: Vec<&Unit> = self
            .pieces
            .values()
            .filter(|p| p.location != Location::Eliminated)
            .filter(|p| {
                p.parent.as_ref() == Some(parent)
                    || (p.attached_to.as_ref() == Some(parent) && self.stands_in(data, p))
            })
            .chain(
                self.withdrawn
                    .values()
                    .filter(|p| p.parent.as_ref() == Some(parent)),
            )
            .filter_map(|p| data.oa.unit(&p.id.unit()?))
            .collect();
        out.extend(data.oa.units().filter(|u| {
            u.parent.as_ref() == Some(parent)
                && matches!(u.arrives, Some(Arrival::At(t)) if t >= now)
                && !self.pieces.contains_key(&PieceId::from(&u.id))
                && !self.withdrawn.contains_key(&PieceId::from(&u.id))
        }));
        out
    }

    /// The slots of `parent`'s chart row that `holders` fill; none when
    /// the chart has no row for it (the O/A notes govern).
    fn filled(&self, data: &Data, parent: &UnitId, holders: &[&Unit]) -> Option<u32> {
        let key = data.oa.unit(parent)?.organisation.as_deref()?;
        let row = data
            .organisation
            .row(parent.nation(), key, u32::from(self.clock.game_turn))?;
        Some(slots_filled_of(&data.organisation, row, holders, false))
    }

    /// `Order::Reassign`: the independent `piece` takes a free slot of
    /// the parent it is attached to (19.2x).
    pub(crate) fn reassign(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        let p = self.reassigning(player, piece)?;
        if let Some(parent) = &p.parent {
            return Err(Illegal::new(
                "19.2x",
                format!("{piece} is assigned to {parent}: only an independent unit is reassigned"),
            ));
        }
        let Some(to) = p.attached_to.clone() else {
            return Err(Illegal::new(
                "19.2x",
                format!("{piece} must be attached to the parent it joins"),
            ));
        };
        if self
            .pieces
            .get(&PieceId::from(&to))
            .is_none_or(|q| q.location == Location::Eliminated)
        {
            return Err(Illegal::new(
                "19.2x",
                format!("{to} is not in play: no slot to fill"),
            ));
        }
        let unit = piece.unit().expect("a combat unit or HQ has a sheet id");
        let sheet = data.oa.unit(&unit).expect("a piece's unit is on a sheet");
        if data.oa.formation_of(&unit).reassignment_restricted() {
            return Err(Illegal::new(
                "19.27",
                format!("{piece}'s parent never reached Africa: it may not be reassigned"),
            ));
        }
        let mut holders = self.slot_holders(data, &to);
        let Some(before) = self.filled(data, &to, &holders) else {
            return Err(Illegal::new(
                "19.2x",
                format!("{to} has no Formation Organisation Chart row: no slot to fill"),
            ));
        };
        holders.push(sheet);
        if self.filled(data, &to, &holders) <= Some(before) {
            return Err(Illegal::new(
                "19.2x",
                format!("{to} has no free slot that {piece} fills"),
            ));
        }
        let p = self.pieces.get_mut(piece).expect("checked");
        p.parent = Some(to.clone());
        p.substitute = false;
        Ok(vec![Event::Reassigned {
            piece: piece.clone(),
            to: Some(PieceId::from(&to)),
            from: None,
        }])
    }

    /// The shifts of parent `player` may ask for now, for the browser's
    /// phase C control: each unit of its in play whose O/A notes date a
    /// shift from its parent to this month, and the parent it joins
    /// (none: independent). `shift_parent` judges the rest (19.24).
    pub fn parent_shifts(&self, data: &Data, player: Side) -> Vec<(PieceId, Option<PieceId>)> {
        if !self.modules.parent_shifts
            || !self.modules.reassignment
            || self.clock.phase != Phase::Organisation
        {
            return Vec::new();
        }
        let Some(now) = data.calendar.week(u32::from(self.clock.game_turn)) else {
            return Vec::new();
        };
        let now = now.month_key();
        let mut out = Vec::new();
        for p in self.pieces.values() {
            if p.side != player || p.location == Location::Eliminated || p.parent.is_none() {
                continue;
            }
            let Some(sheet) = p.id.unit().and_then(|u| data.oa.unit(&u)) else {
                continue;
            };
            for s in &sheet.shifts {
                if s.from == p.parent && s.month.as_deref() == Some(now.as_str()) {
                    out.push((p.id.clone(), s.to.as_ref().map(PieceId::from)));
                }
            }
        }
        out
    }

    /// `Order::ShiftParent`: `piece` leaves its parent for `to` (none:
    /// independent), a change its O/A notes date to this month (19.24,
    /// R-176). The shift is made within 19.21: the unit must be attached
    /// to `to`, which needs a slot it fills, as `reassign` judges it.
    pub(crate) fn shift_parent(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        to: Option<&PieceId>,
    ) -> Result<Vec<Event>, Illegal> {
        if !self.modules.parent_shifts {
            return Err(Illegal::new(
                "19.24",
                "this game plays without historical shifts of parent",
            ));
        }
        let p = self.reassigning(player, piece)?;
        let to = match to {
            Some(t) => Some(t.unit().ok_or_else(|| {
                Illegal::new("19.24", format!("{t} is not a unit that takes others"))
            })?),
            None => None,
        };
        let unit = piece.unit().expect("a combat unit or HQ has a sheet id");
        let sheet = data.oa.unit(&unit).expect("a piece's unit is on a sheet");
        let Some(from) = p.parent.clone() else {
            return Err(Illegal::new(
                "19.24",
                format!("{piece} is independent: it is assigned by Reassign, in any month"),
            ));
        };
        let named = |t: &Option<UnitId>| t.as_ref().map_or("no parent".into(), |u| u.to_string());
        let Some(shift) = sheet
            .shifts
            .iter()
            .find(|s| s.from.as_ref() == Some(&from) && s.to == to)
        else {
            return Err(Illegal::new(
                "19.24",
                format!(
                    "{piece}'s O/A notes list no shift from {from} to {}",
                    named(&to)
                ),
            ));
        };
        let Some(month) = &shift.month else {
            return Err(Illegal::new(
                "19.24",
                format!(
                    "{piece}'s O/A note gives no month for its shift: it may not be made (R-176)"
                ),
            ));
        };
        let now = data
            .calendar
            .week(u32::from(self.clock.game_turn))
            .map(|w| w.month_key());
        if now.as_deref() != Some(month.as_str()) {
            return Err(Illegal::new(
                "19.24",
                format!(
                    "{piece}'s shift to {} is made in {month} or never",
                    named(&to)
                ),
            ));
        }
        if let Some(to) = &to {
            if p.attached_to.as_ref() != Some(to) {
                return Err(Illegal::new(
                    "19.24",
                    format!("{piece} must be attached to the parent it joins, {to}"),
                ));
            }
            if self
                .pieces
                .get(&PieceId::from(to))
                .is_none_or(|q| q.location == Location::Eliminated)
            {
                return Err(Illegal::new(
                    "19.24",
                    format!("{to} is not in play: no slot to fill"),
                ));
            }
            // Where the chart has no row for `to`, its O/A notes govern,
            // and the note that dates this shift is one of them.
            let mut holders = self.slot_holders(data, to);
            if let Some(before) = self.filled(data, to, &holders) {
                holders.push(sheet);
                if self.filled(data, to, &holders) <= Some(before) {
                    return Err(Illegal::new(
                        "19.24",
                        format!("{to} has no free slot that {piece} fills"),
                    ));
                }
            }
        }
        // Leaving for no parent, a unit attached to the parent it leaves
        // stays attached as a non-assigned unit only where 19.5's caps
        // allow; otherwise it detaches, at no CP.
        let old = PieceId::from(&from);
        let detach = to.is_none()
            && p.attached_to.as_ref() == Some(&from)
            && (!self.pieces.contains_key(&old)
                || self.attachment_cap_refusal(data, piece, &old).is_some());
        let p = self.pieces.get_mut(piece).expect("checked");
        p.parent = to.clone();
        p.substitute = false;
        let mut events = vec![Event::Reassigned {
            piece: piece.clone(),
            to: to.as_ref().map(PieceId::from),
            from: Some(old.clone()),
        }];
        if detach {
            p.attached_to = None;
            events.push(Event::Detached {
                piece: piece.clone(),
                from: old,
            });
        }
        Ok(events)
    }

    /// `Order::SwapBattalions`: two Commonwealth battalions standing
    /// alone trade brigades, or an independent one takes an assigned
    /// one's place (19.2x).
    pub(crate) fn swap_battalions(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        with: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        if player != Side::Cw {
            return Err(Illegal::new(
                "19.2x",
                "only the Commonwealth swaps battalions between brigades",
            ));
        }
        let mut parents = Vec::new();
        for id in [piece, with] {
            let p = self.reassigning(player, id)?;
            if p.kind != PieceKind::Combat || p.equivalent != Some(Equivalent::Battalion) {
                return Err(Illegal::new("19.2x", format!("{id} is not a battalion")));
            }
            if p.attached_to.is_some() || self.pieces.values().any(|q| q.attached_to == id.unit()) {
                return Err(Illegal::new(
                    "19.2x",
                    format!("{id} must stand alone to change brigades: detach it first"),
                ));
            }
            let brigade = p
                .parent
                .clone()
                .filter(|u| data.oa.unit(u).and_then(|s| s.echelon) == Some(Echelon::Brigade));
            if p.parent.is_some() && brigade.is_none() {
                return Err(Illegal::new(
                    "19.2x",
                    format!("{id} is not assigned to a brigade"),
                ));
            }
            parents.push(brigade);
        }
        let (a, b) = (parents[0].clone(), parents[1].clone());
        let Some(from) = a.clone() else {
            return Err(Illegal::new(
                "19.2x",
                format!("{piece} is not assigned to a brigade: it may not be cut loose"),
            ));
        };
        if a == b {
            return Err(Illegal::new(
                "19.2x",
                format!("{piece} and {with} are assigned to one brigade"),
            ));
        }
        // Each brigade keeps every slot it held: the incoming battalion
        // fills what the outgoing one leaves.
        let sheet = |id: &PieceId| {
            data.oa
                .unit(&id.unit().expect("a battalion"))
                .expect("on a sheet")
        };
        for (brigade, out, inn) in [(Some(&from), piece, with), (b.as_ref(), with, piece)] {
            let Some(brigade) = brigade else { continue };
            let holders = self.slot_holders(data, brigade);
            let Some(before) = self.filled(data, brigade, &holders) else {
                continue;
            };
            let gone = sheet(out);
            let mut after: Vec<&Unit> = holders.into_iter().filter(|u| u.id != gone.id).collect();
            after.push(sheet(inn));
            if self.filled(data, brigade, &after) < Some(before) {
                return Err(Illegal::new(
                    "19.2x",
                    format!("{inn} does not fill the slot {out} leaves in {brigade}"),
                ));
            }
        }
        self.pieces.get_mut(piece).expect("checked").parent = b.clone();
        self.pieces.get_mut(with).expect("checked").parent = Some(from.clone());
        for p in [piece, with] {
            self.pieces.get_mut(p).expect("checked").substitute = false;
        }
        Ok(vec![
            Event::Reassigned {
                piece: piece.clone(),
                to: b.as_ref().map(PieceId::from),
                from: Some(PieceId::from(&from)),
            },
            Event::Reassigned {
                piece: with.clone(),
                to: Some(PieceId::from(&from)),
                from: b.as_ref().map(PieceId::from),
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use crate::clock::Phase;
    use crate::data::Data;
    use crate::event::Event;
    use crate::movement::test_support::{moving, pid};
    use crate::order::Order;
    use crate::piece::{Location, PieceId};
    use crate::state::GameState;
    use cna_data::cp::Cp;
    use cna_data::scenario::Side;

    const MALETTI: &str = "unit:it:maletti-div-hq";
    const REGT: &str = "unit:it:5th-libyan-regt-hq";
    /// The 1st Libyan Regiment: the VIII, IX and X Battalions.
    const FIRST: &str = "unit:it:1st-libyan-regt-hq";

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// Graziani in the Axis's phase C, with `ids` standing with `hq`.
    fn organising(data: &Data, side: Side, hq: &str, ids: &[&str]) -> GameState {
        let mut state = moving(data, 3, side);
        state.modules.reassignment = true;
        state.clock.phase = Phase::Organisation;
        state.clock.half = None;
        state.clock.segment = None;
        let at = state.pieces[&pid(hq)].location.clone();
        for id in ids {
            state.put(&pid(id), at.clone());
        }
        state
    }

    fn attach(state: &mut GameState, data: &Data, side: Side, to: &str, piece: &str) {
        state
            .apply(
                data,
                side,
                Order::Attach {
                    piece: pid(piece),
                    to: pid(to),
                },
            )
            .unwrap();
    }

    fn reassign(piece: &str) -> Order {
        Order::Reassign { piece: pid(piece) }
    }

    /// Eliminates `id` as an order would, sweeping after it.
    fn eliminate(state: &mut GameState, id: &str) {
        state.put(&pid(id), Location::Eliminated);
        state.unassign_orphans(&mut Vec::new());
    }

    #[test]
    fn an_eliminated_parent_s_units_become_independent() {
        // Combat eliminates the 1st Libyan Regiment; its VIII Battalion,
        // elsewhere, is independent once the order is done.
        let data = graziani();
        let bn = "unit:it:viii-libyan-bn";
        let mut state = organising(&data, Side::Axis, MALETTI, &[bn]);
        assert_eq!(state.pieces[&pid(bn)].parent, pid(FIRST).unit());
        state.put(&pid(FIRST), Location::Eliminated);
        let mut events = Vec::new();
        state.unassign_orphans(&mut events);
        assert_eq!(events.len(), 3, "the VIII, IX and X: {events:?}");
        assert!(events.contains(&Event::Unassigned {
            piece: pid(bn),
            from: pid(FIRST),
        }));
        assert_eq!(state.pieces[&pid(bn)].parent, None);
        // Every order sweeps when it is done.
        let vii = "unit:it:x-libyan-bn";
        state.pieces.get_mut(&pid(vii)).unwrap().parent = pid(FIRST).unit();
        let events = state
            .apply(&data, Side::Axis, Order::Reassign { piece: pid(bn) })
            .unwrap_err();
        assert_eq!(events.case, "19.2x");
        assert!(
            state.pieces[&pid(vii)].parent.is_some(),
            "a refused order sweeps nothing"
        );
    }

    #[test]
    fn without_the_module_an_orphan_stays_assigned() {
        let data = graziani();
        let bn = "unit:it:viii-libyan-bn";
        let mut state = organising(&data, Side::Axis, MALETTI, &[bn]);
        state.modules.reassignment = false;
        eliminate(&mut state, FIRST);
        attach(&mut state, &data, Side::Axis, MALETTI, bn);
        assert_eq!(state.pieces[&pid(bn)].parent, pid(FIRST).unit());
        let err = state.apply(&data, Side::Axis, reassign(bn)).unwrap_err();
        assert!(
            err.reason.contains("without reassignment"),
            "{}",
            err.reason
        );
    }

    #[test]
    fn an_independent_unit_takes_a_free_slot_of_the_parent_it_is_attached_to() {
        // The 5th Libyan Regiment (three battalion slots) has the XIX
        // and has lost the XVII. The 1st's battalions, independent once
        // the 1st is gone, attach in the XVII's place and take the two
        // free slots; the third finds the regiment full.
        let data = graziani();
        let (viii, ix, x) = (
            "unit:it:viii-libyan-bn",
            "unit:it:ix-libyan-bn",
            "unit:it:x-libyan-bn",
        );
        let mut state = organising(&data, Side::Axis, REGT, &[viii, ix, x]);
        eliminate(&mut state, "unit:it:xvii-libyan-bn");
        eliminate(&mut state, FIRST);
        let err = state.apply(&data, Side::Axis, reassign(ix)).unwrap_err();
        assert!(err.reason.contains("must be attached"), "{}", err.reason);
        attach(&mut state, &data, Side::Axis, REGT, ix);
        let cp = state.pieces[&pid(ix)].cp_spent;
        let events = state.apply(&data, Side::Axis, reassign(ix)).unwrap();
        assert_eq!(
            events,
            vec![Event::Reassigned {
                piece: pid(ix),
                to: Some(pid(REGT)),
                from: None,
            }]
        );
        let p = &state.pieces[&pid(ix)];
        assert_eq!(p.parent, pid(REGT).unit());
        assert!(!p.substitute);
        assert_eq!(p.cp_spent, cp, "reassignment costs no CP");
        // Assigned now, it detaches and attaches for a CP each.
        state
            .apply(&data, Side::Axis, Order::Detach { piece: pid(ix) })
            .unwrap();
        attach(&mut state, &data, Side::Axis, REGT, ix);
        assert_eq!(state.pieces[&pid(ix)].cp_spent, cp + Cp::whole(2));
        // The IX holds the XVII's slot now: the X attaches as an extra,
        // no substitute.
        attach(&mut state, &data, Side::Axis, REGT, x);
        assert!(!state.pieces[&pid(x)].substitute);
    }

    #[test]
    fn a_substitute_holds_its_slot() {
        // The XVII is gone and the IX stands in for it; the X takes the
        // third slot; the VIII then finds the regiment full.
        let data = graziani();
        let (viii, ix, x) = (
            "unit:it:viii-libyan-bn",
            "unit:it:ix-libyan-bn",
            "unit:it:x-libyan-bn",
        );
        let mut state = organising(&data, Side::Axis, REGT, &[viii, ix, x]);
        eliminate(&mut state, "unit:it:xvii-libyan-bn");
        eliminate(&mut state, FIRST);
        attach(&mut state, &data, Side::Axis, REGT, ix);
        assert!(state.pieces[&pid(ix)].substitute);
        state.pieces.get_mut(&pid(x)).unwrap().attached_to = pid(REGT).unit();
        state.apply(&data, Side::Axis, reassign(x)).unwrap();
        state.pieces.get_mut(&pid(viii)).unwrap().attached_to = pid(REGT).unit();
        let err = state.apply(&data, Side::Axis, reassign(viii)).unwrap_err();
        assert!(err.reason.contains("no free slot"), "{}", err.reason);
    }

    #[test]
    fn an_assigned_unit_and_a_unit_whose_parent_never_came_stay_put() {
        let data = graziani();
        let (bn, xviii) = ("unit:it:viii-libyan-bn", "unit:it:xviii-libyan-bn");
        let mut state = organising(&data, Side::Axis, REGT, &[bn, xviii]);
        eliminate(&mut state, "unit:it:xvii-libyan-bn");
        attach(&mut state, &data, Side::Axis, REGT, xviii);
        let err = state.apply(&data, Side::Axis, reassign(xviii)).unwrap_err();
        assert_eq!(err.case, "19.27");
        let err = state.apply(&data, Side::Axis, reassign(bn)).unwrap_err();
        assert!(err.reason.contains("only an independent"), "{}", err.reason);
    }

    #[test]
    fn reassignment_waits_for_phase_c() {
        let data = graziani();
        let ix = "unit:it:ix-libyan-bn";
        let mut state = moving(&data, 3, Side::Axis);
        state.modules.reassignment = true;
        let err = state.apply(&data, Side::Axis, reassign(ix)).unwrap_err();
        assert!(err.reason.contains("phase C"), "{}", err.reason);
    }

    fn swap(piece: &str, with: &str) -> Order {
        Order::SwapBattalions {
            piece: pid(piece),
            with: pid(with),
        }
    }

    /// Graziani in the Commonwealth's phase C, `ids` standing alone.
    fn cw_organising(data: &Data, ids: &[&str]) -> GameState {
        let mut state = moving(data, 3, Side::Cw);
        state.modules.reassignment = true;
        state.clock.phase = Phase::Organisation;
        state.clock.half = None;
        state.clock.segment = None;
        for id in ids {
            state.stand_alone(&pid(id));
        }
        state
    }

    const FIFTH_IN: &str = "unit:cw:5-in-hq";
    const ELEVENTH_IN: &str = "unit:cw:11-in-hq";
    const RFSLR: &str = "unit:cw:1-rfslr";
    const CMRN: &str = "unit:cw:2-cmrn";

    #[test]
    fn commonwealth_battalions_trade_brigades() {
        let data = graziani();
        let mut state = cw_organising(&data, &[RFSLR, CMRN]);
        let events = state.apply(&data, Side::Cw, swap(RFSLR, CMRN)).unwrap();
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(state.pieces[&pid(RFSLR)].parent, pid(ELEVENTH_IN).unit());
        assert_eq!(state.pieces[&pid(CMRN)].parent, pid(FIFTH_IN).unit());
    }

    #[test]
    fn an_independent_battalion_takes_an_assigned_one_s_place() {
        let data = graziani();
        let qns = "unit:cw:2-qns";
        let mut state = cw_organising(&data, &[RFSLR, qns]);
        eliminate(&mut state, "unit:cw:16th-infantry-bde-hq");
        assert_eq!(state.pieces[&pid(qns)].parent, None);
        // Never cut loose: the independent one is not the one leaving.
        let err = state.apply(&data, Side::Cw, swap(qns, RFSLR)).unwrap_err();
        assert!(err.reason.contains("cut loose"), "{}", err.reason);
        state.apply(&data, Side::Cw, swap(RFSLR, qns)).unwrap();
        assert_eq!(state.pieces[&pid(qns)].parent, pid(FIFTH_IN).unit());
        assert_eq!(state.pieces[&pid(RFSLR)].parent, None);
    }

    #[test]
    fn a_swap_keeps_each_brigade_s_slots_and_wants_them_standing_alone() {
        let data = graziani();
        let rtr = "unit:cw:6-rtr";
        let pjb = "unit:cw:3-1-pjb";
        let mut state = cw_organising(&data, &[RFSLR, rtr, pjb]);
        let err = state.apply(&data, Side::Cw, swap(RFSLR, rtr)).unwrap_err();
        assert!(
            err.reason.contains("does not fill the slot"),
            "{}",
            err.reason
        );
        // The Cameron Highlanders still stand with their brigade.
        let err = state.apply(&data, Side::Cw, swap(RFSLR, CMRN)).unwrap_err();
        assert!(err.reason.contains("stand alone"), "{}", err.reason);
        let err = state.apply(&data, Side::Cw, swap(RFSLR, pjb)).unwrap_err();
        assert!(err.reason.contains("one brigade"), "{}", err.reason);
    }

    #[test]
    fn only_the_commonwealth_swaps() {
        let data = graziani();
        let mut state = organising(&data, Side::Axis, MALETTI, &[]);
        let err = state
            .apply(
                &data,
                Side::Axis,
                swap("unit:it:viii-libyan-bn", "unit:it:xix-libyan-bn"),
            )
            .unwrap_err();
        assert!(
            err.reason.contains("only the Commonwealth"),
            "{}",
            err.reason
        );
    }

    /// Graziani's 7th Support Group, its parent row read at Game-Turn 18
    /// and then 19, when the chart cuts it to one infantry battalion and
    /// no armoured car (19.2x).
    fn support_group_at_19(on: bool) -> (GameState, Vec<Event>) {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state.modules.reassignment = true;
        state.modules.armour_limits = on;
        state.clock.game_turn = 19;
        let mut events = Vec::new();
        state.tighten_limits(&data, 18, &mut events);
        (state, events)
    }

    #[test]
    fn a_tighter_support_group_row_frees_what_it_no_longer_holds() {
        let (state, events) = support_group_at_19(true);
        let spt = pid("unit:cw:7-spt-hq");
        let freed: Vec<&PieceId> = events
            .iter()
            .filter_map(|e| match e {
                Event::Reassigned {
                    piece,
                    to: None,
                    from: Some(f),
                } if *f == spt => Some(piece),
                _ => None,
            })
            .collect();
        // Of the 1 KRRC and 2 Rifle Brigade one stays; the 11th Hussars'
        // armoured-car slot is gone. The 3rd and 4th RHA, one more than
        // the old row's one artillery slot, both stay.
        assert_eq!(
            freed,
            [&pid("unit:cw:2-rflbde"), &pid("unit:cw:11-hus")],
            "{events:?}"
        );
        assert_eq!(events.len(), 2);
        for id in ["unit:cw:1-krrc", "unit:cw:3-rha", "unit:cw:4-rha"] {
            assert_eq!(
                state.pieces[&pid(id)].parent.as_ref().map(PieceId::from),
                Some(spt.clone()),
                "{id}"
            );
        }
        assert!(state.pieces[&pid("unit:cw:11-hus")].parent.is_none());
    }

    #[test]
    fn without_the_module_or_a_new_turn_nothing_changes() {
        let (_, events) = support_group_at_19(false);
        assert!(events.is_empty());
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state.modules.armour_limits = true;
        let mut events = Vec::new();
        let turn = state.clock.game_turn;
        state.tighten_limits(&data, turn, &mut events);
        assert!(events.is_empty());
    }

    /// Three battalions under a support group whose row goes from two
    /// infantry slots to one: the slot that goes takes one battalion with
    /// it, and the third, beyond the old row already, stays (19.2x).
    #[test]
    fn units_over_the_old_row_still_give_up_the_slots_that_go() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state.modules.reassignment = true;
        state.modules.armour_limits = true;
        let spt = pid("unit:cw:7-spt-hq");
        // A third infantry battalion assigned to the 7th Support Group.
        let third = pid("unit:cw:2-cmrn");
        state.pieces.get_mut(&third).unwrap().parent = spt.unit();
        state.clock.game_turn = 19;
        let mut events = Vec::new();
        state.tighten_limits(&data, 18, &mut events);
        let infantry_left = ["unit:cw:1-krrc", "unit:cw:2-rflbde", "unit:cw:2-cmrn"]
            .iter()
            .filter(|id| state.pieces[&pid(id)].parent == spt.unit())
            .count();
        assert_eq!(infantry_left, 2, "{events:?}");
        assert_eq!(events.len(), 2, "one battalion and the armoured car");
        assert!(state.pieces[&pid("unit:cw:11-hus")].parent.is_none());
    }

    #[test]
    fn without_reassignment_no_slot_is_taken_away() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state.modules.armour_limits = true;
        state.clock.game_turn = 19;
        let mut events = Vec::new();
        state.tighten_limits(&data, 18, &mut events);
        assert!(events.is_empty());
    }

    /// Operation Crusader read at Game-Turn 70 and then 71: an armoured
    /// division's row drops to one armoured brigade, so one of the 7th
    /// Armoured's two goes; its support group's new row fills more than
    /// the old, so nothing leaves it (19.2x).
    #[test]
    fn at_turn_71_an_armoured_division_keeps_one_brigade() {
        let data = Data::load("operation-crusader").unwrap();
        let mut state = crate::state::test_support::set_up(&data, 3);
        state.modules.reassignment = true;
        state.modules.armour_limits = true;
        let div = pid("unit:cw:7th-armored-div-hq").unit();
        let brigades = |state: &GameState| {
            ["unit:cw:4th-armored-bde-hq", "unit:cw:7th-armored-bde-hq"]
                .iter()
                .filter(|id| state.pieces[&pid(id)].parent == div)
                .count()
        };
        assert_eq!(brigades(&state), 2, "Crusader sets up both");
        state.clock.game_turn = 71;
        let mut events = Vec::new();
        state.tighten_limits(&data, 70, &mut events);
        assert_eq!(brigades(&state), 1, "{events:?}");
        let spt = pid("unit:cw:7-spt-hq");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::Reassigned { from: Some(f), .. } if *f == spt)),
            "{events:?}"
        );
    }

    /// The 3/1 Punjab, of the 5th Indian Brigade, joins the 25th Indian
    /// Brigade when the 25th arrives, in May 1942 (R-176's example).
    const PUNJAB: &str = "unit:cw:3-1-pjb";
    const FIFTH: &str = "unit:cw:5-in-hq";
    const TWENTY_FIFTH: &str = "unit:cw:25-in-hq";

    /// Graziani in the Commonwealth's phase C at Game-Turn `turn`, the
    /// 25th Indian Brigade (none of its battalions) beside the 3/1
    /// Punjab, which stands alone.
    fn shifting(data: &Data, turn: u8) -> GameState {
        let mut state = cw_organising(data, &[PUNJAB]);
        state.modules.parent_shifts = true;
        state.clock.game_turn = turn;
        let at = state.pieces[&pid(PUNJAB)].location.clone();
        let head = cna_data::oa::UnitId::parse(TWENTY_FIFTH).unwrap();
        let hq = crate::setup::reinforcement(
            Side::Cw,
            &head,
            &data.oa,
            &data.chars,
            cna_data::time::GameTime::new(turn, 1),
            at,
        )
        .unwrap()
        .into_iter()
        .find(|p| p.id == pid(TWENTY_FIFTH))
        .unwrap();
        state.pieces.insert(hq.id.clone(), hq);
        state
    }

    fn shift(piece: &str, to: Option<&str>) -> Order {
        Order::ShiftParent {
            piece: pid(piece),
            to: to.map(pid),
        }
    }

    /// Applies `order` as the Commonwealth, which must be refused and
    /// change nothing; the reason.
    fn refused(state: &mut GameState, data: &Data, order: Order) -> String {
        let before = serde_json::to_string(&*state).unwrap();
        let err = state.apply(data, Side::Cw, order).unwrap_err();
        assert_eq!(serde_json::to_string(&*state).unwrap(), before);
        err.reason
    }

    #[test]
    fn a_unit_shifts_parent_in_the_month_its_note_dates() {
        let data = graziani();
        // Game-Turn 82, the fourth week of May 1942.
        let mut state = shifting(&data, 82);
        assert_eq!(state.pieces[&pid(PUNJAB)].parent, pid(FIFTH).unit());
        assert_eq!(
            state.parent_shifts(&data, Side::Cw),
            vec![(pid(PUNJAB), Some(pid(TWENTY_FIFTH)))]
        );
        let why = refused(&mut state, &data, shift(PUNJAB, Some(TWENTY_FIFTH)));
        assert!(why.contains("must be attached"), "{why}");
        state.pieces.get_mut(&pid(PUNJAB)).unwrap().attached_to = pid(TWENTY_FIFTH).unit();
        let cp = state.pieces[&pid(PUNJAB)].cp_spent;
        let events = state
            .apply(&data, Side::Cw, shift(PUNJAB, Some(TWENTY_FIFTH)))
            .unwrap();
        assert_eq!(
            events,
            vec![Event::Reassigned {
                piece: pid(PUNJAB),
                to: Some(pid(TWENTY_FIFTH)),
                from: Some(pid(FIFTH)),
            }]
        );
        let p = &state.pieces[&pid(PUNJAB)];
        assert_eq!(p.parent, pid(TWENTY_FIFTH).unit());
        assert_eq!(p.cp_spent, cp, "a shift costs no CP");
        // Made once: the shift from the 5th no longer applies.
        assert!(state.parent_shifts(&data, Side::Cw).is_empty());
        let why = refused(&mut state, &data, shift(PUNJAB, Some(TWENTY_FIFTH)));
        assert!(why.contains("list no shift"), "{why}");
    }

    #[test]
    fn a_shift_is_refused_outside_its_month() {
        let data = graziani();
        for turn in [78, 83] {
            // April and June 1942.
            let mut state = shifting(&data, turn);
            let offered = state.parent_shifts(&data, Side::Cw);
            assert!(
                !offered.iter().any(|(p, _)| *p == pid(PUNJAB)),
                "{offered:?}"
            );
            state.pieces.get_mut(&pid(PUNJAB)).unwrap().attached_to = pid(TWENTY_FIFTH).unit();
            let why = refused(&mut state, &data, shift(PUNJAB, Some(TWENTY_FIFTH)));
            assert!(why.contains("1942-05 or never"), "{turn}: {why}");
        }
    }

    #[test]
    fn a_shift_whose_note_gives_no_month_is_never_made() {
        let data = graziani();
        let mut state = shifting(&data, 82);
        // The 1st RTR leaves the 7th Armoured Brigade for the 22nd
        // "later": no month.
        let rtr = "unit:cw:1-rtr";
        state.pieces.get_mut(&pid(rtr)).unwrap().parent = pid("unit:cw:7th-armored-bde-hq").unit();
        let why = refused(
            &mut state,
            &data,
            shift(rtr, Some("unit:cw:22-armd-bde-hq")),
        );
        assert!(why.contains("gives no month"), "{why}");
    }

    #[test]
    fn only_a_listed_shift_is_made() {
        let data = graziani();
        let mut state = shifting(&data, 82);
        // To no parent: not the shift the note gives.
        let why = refused(&mut state, &data, shift(PUNJAB, None));
        assert!(why.contains("list no shift"), "{why}");
        // Another brigade: not listed either.
        let why = refused(&mut state, &data, shift(PUNJAB, Some("unit:cw:11-in-hq")));
        assert!(why.contains("list no shift"), "{why}");
    }

    #[test]
    fn a_shift_waits_for_phase_c_and_the_module() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state.modules.reassignment = true;
        state.modules.parent_shifts = true;
        let why = refused(&mut state, &data, shift(PUNJAB, Some(TWENTY_FIFTH)));
        assert!(why.contains("phase C"), "{why}");
        let mut state = shifting(&data, 82);
        state.pieces.get_mut(&pid(PUNJAB)).unwrap().attached_to = pid(TWENTY_FIFTH).unit();
        state.modules.parent_shifts = false;
        let why = refused(&mut state, &data, shift(PUNJAB, Some(TWENTY_FIFTH)));
        assert!(why.contains("without historical shifts"), "{why}");
    }

    const GUARDS: &str = "unit:cw:22-gds";
    const SEVENTIETH: &str = "unit:cw:70th-infantry-div-hq";

    /// Game-Turn 28, April 1941: the 22nd Guards Brigade assigned and
    /// attached to the 70th Division beside it, as after its February
    /// shift, ready to leave it (R-176).
    fn guards_leaving(data: &Data) -> GameState {
        let mut state = shifting(data, 28);
        let at = state.pieces[&pid(PUNJAB)].location.clone();
        for head in [GUARDS, SEVENTIETH] {
            let head = cna_data::oa::UnitId::parse(head).unwrap();
            let p = crate::setup::reinforcement(
                Side::Cw,
                &head,
                &data.oa,
                &data.chars,
                cna_data::time::GameTime::new(28, 1),
                at.clone(),
            )
            .unwrap()
            .into_iter()
            .find(|p| p.id == PieceId::from(&head))
            .unwrap();
            state.pieces.insert(p.id.clone(), p);
        }
        let g = state.pieces.get_mut(&pid(GUARDS)).unwrap();
        g.parent = pid(SEVENTIETH).unit();
        g.attached_to = pid(SEVENTIETH).unit();
        state
    }

    #[test]
    fn a_shift_to_no_parent_keeps_an_attachment_the_caps_allow() {
        // The Commonwealth infantry division's 19.5 row takes no brigade
        // beyond its structure, so the chart is widened here to one that
        // does: under it the Guards stay attached.
        let mut data = graziani();
        for r in &mut data.max_attachments.rows {
            if r.parent == "infantry-division" && r.covers(28) {
                r.alternatives = vec![cna_data::max_attachments::Alternative {
                    brigades: 1,
                    units: r.max_units,
                }];
            }
        }
        let mut state = guards_leaving(&data);
        assert_eq!(
            state.attachment_cap_refusal(&data, &pid(GUARDS), &pid(SEVENTIETH)),
            None,
            "the 70th's 19.5 row takes the Guards as a non-assigned unit"
        );
        let events = state.apply(&data, Side::Cw, shift(GUARDS, None)).unwrap();
        assert_eq!(
            events,
            vec![Event::Reassigned {
                piece: pid(GUARDS),
                to: None,
                from: Some(pid(SEVENTIETH)),
            }]
        );
        let g = &state.pieces[&pid(GUARDS)];
        assert_eq!(g.parent, None);
        assert_eq!(g.attached_to, pid(SEVENTIETH).unit());
    }

    #[test]
    fn a_shift_to_no_parent_detaches_a_unit_the_caps_no_longer_allow() {
        // The Commonwealth infantry division's 19.5 row takes no brigade
        // beyond its structure: the Guards, independent now, may not stay
        // attached to the 70th.
        let data = graziani();
        let mut state = guards_leaving(&data);
        let why = state
            .attachment_cap_refusal(&data, &pid(GUARDS), &pid(SEVENTIETH))
            .expect("the row refuses a brigade");
        assert!(why.contains("1 brigade(s)"), "{why}");
        let cp = state.pieces[&pid(GUARDS)].cp_spent;
        let events = state.apply(&data, Side::Cw, shift(GUARDS, None)).unwrap();
        assert_eq!(
            events,
            vec![
                Event::Reassigned {
                    piece: pid(GUARDS),
                    to: None,
                    from: Some(pid(SEVENTIETH)),
                },
                Event::Detached {
                    piece: pid(GUARDS),
                    from: pid(SEVENTIETH),
                },
            ]
        );
        let g = &state.pieces[&pid(GUARDS)];
        assert_eq!(g.parent, None);
        assert_eq!(g.attached_to, None);
        assert_eq!(g.cp_spent, cp, "the detachment costs no CP");
    }
}
