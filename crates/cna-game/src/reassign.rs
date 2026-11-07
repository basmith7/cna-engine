//! Assignment changes (19.2x), behind the `reassignment` module. A
//! unit's assignment is its piece's `parent`: an eliminated parent's
//! survivors become independent, an independent unit attached to a
//! parent with a free Formation Organisation Chart slot may be assigned
//! to it in phase C, and the Commonwealth may swap battalions between
//! brigades. With `armour_limits` on, a Commonwealth parent whose
//! chart row tightens at a new Game-Turn loses the units it no longer
//! has a slot for. Historical shifts are not built (`NOT_SIMULATED.md`).

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
    /// Organisation Chart row changed at a turn passed, the units in play
    /// that filled a slot of the old row and fill none of the new one
    /// become independent, highest id first (19.2x, the Commonwealth
    /// armour). A unit the old row had no slot for either stays.
    pub(crate) fn tighten_limits(&mut self, data: &Data, before: u8, events: &mut Vec<Event>) {
        if !self.modules.armour_limits || self.clock.game_turn <= before {
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
                let Some(key) = data.oa.unit(parent).and_then(|u| u.organisation.as_deref()) else {
                    continue;
                };
                let row = |t: u8| data.organisation.row(parent.nation(), key, u32::from(t));
                let (Some(old), Some(new)) = (row(turn - 1), row(turn)) else {
                    continue;
                };
                if std::ptr::eq(old, new) {
                    continue;
                }
                let mut holders = self.slot_holders(data, parent);
                let fill = |row, h: &[&Unit]| slots_filled_of(&data.organisation, row, h, false);
                let mut mine: Vec<PieceId> = self
                    .pieces
                    .values()
                    .filter(|p| {
                        p.location != Location::Eliminated && p.parent.as_ref() == Some(parent)
                    })
                    .map(|p| p.id.clone())
                    .collect();
                mine.sort();
                for id in mine.into_iter().rev() {
                    let sheet = data
                        .oa
                        .unit(&id.unit().expect("assigned"))
                        .expect("on a sheet");
                    let without: Vec<&Unit> = holders
                        .iter()
                        .copied()
                        .filter(|u| u.id != sheet.id)
                        .collect();
                    let needed_before = fill(old, &without) < fill(old, &holders);
                    let needed_now = fill(new, &without) < fill(new, &holders);
                    if needed_before && !needed_now {
                        holders = without;
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
                    || (p.substitute && p.attached_to.as_ref() == Some(parent))
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
}
