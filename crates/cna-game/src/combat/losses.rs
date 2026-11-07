//! TOE losses (SPI 12.45, 14.4x, 15.8x). A count of points lost goes by
//! the owner's standing loss order (`Order::SetLossOrder`), then, past
//! it, the engine chooses for the owner: the largest entry first. Damage
//! absorbed by rate (`absorb`) takes the loss order's entries first too
//! (with `absorb_by_loss_order` on), then the best-rated points, so the
//! fewest points go. A piece at zero leaves the map.

use super::strength::Part;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::map::HexId;
use cna_data::scenario::Side;

/// A piece and how many of its TOE points may be taken.
#[derive(Debug, Clone)]
pub(crate) struct Share {
    pub piece: PieceId,
    pub points: u32,
}

impl GameState {
    /// `Order::SetLossOrder`: `player`'s `piece` loses its points from
    /// `weapons`, in that order, before the engine picks (12.45, 14.4x,
    /// 15.83); an empty list clears it.
    pub(crate) fn set_loss_order(
        &mut self,
        player: Side,
        piece: &PieceId,
        weapons: &[String],
    ) -> Result<Vec<Event>, Illegal> {
        let bad = |why: String| Illegal::new("12.45", why);
        let p = self
            .pieces
            .get_mut(piece)
            .filter(|p| p.side == player)
            .ok_or_else(|| bad(format!("{piece} is not one of {player}'s pieces")))?;
        for (i, w) in weapons.iter().enumerate() {
            if !p.weapons.iter().any(|e| e.weapon == *w) {
                return Err(bad(format!("{piece} has no {w} entry")));
            }
            if weapons[..i].contains(w) {
                return Err(bad(format!("{w} is named twice")));
            }
        }
        p.loss_order = weapons.to_vec();
        Ok(vec![Event::LossOrderSet {
            piece: piece.clone(),
            weapons: weapons.to_vec(),
        }])
    }

    /// Destroys up to `points` TOE points of `id`: from the first entry
    /// of its loss order with points left, else the largest entry;
    /// returns what went.
    pub(crate) fn take_loss(&mut self, id: &PieceId, points: u32, events: &mut Vec<Event>) -> u32 {
        let mut taken = 0;
        while taken < points {
            let piece = &self.pieces[id];
            let has = |name: &str| {
                piece
                    .weapons
                    .iter()
                    .any(|w| w.weapon == name && w.points > 0)
            };
            let listed: u32 = piece.weapons.iter().map(|w| w.points).sum();
            let unlisted = piece.toe.saturating_sub(listed);
            let chosen = piece
                .loss_order
                .iter()
                .find(|w| has(w))
                .cloned()
                .or_else(|| {
                    piece
                        .weapons
                        .iter()
                        .max_by_key(|w| w.points)
                        .filter(|w| w.points > unlisted)
                        .map(|w| w.weapon.clone())
                });
            if !self.take_loss_entry(id, chosen.as_deref()) {
                break;
            }
            taken += 1;
        }
        self.note_losses(id, taken, events);
        taken
    }

    /// Takes one TOE point off the named weapon entry, or off the ID
    /// code's unlisted points; false when there is none to take.
    pub(crate) fn take_loss_entry(&mut self, id: &PieceId, weapon: Option<&str>) -> bool {
        let piece = self.pieces.get_mut(id).expect("a piece");
        if piece.toe == 0 {
            return false;
        }
        match weapon {
            Some(name) => {
                let Some(w) = piece
                    .weapons
                    .iter_mut()
                    .find(|w| w.weapon == name && w.points > 0)
                else {
                    return false;
                };
                w.points -= 1;
            }
            None => {
                let listed: u32 = piece.weapons.iter().map(|w| w.points).sum();
                if piece.toe <= listed {
                    return false;
                }
            }
        }
        piece.toe -= 1;
        true
    }

    /// Records `points` lost by `id`, and its elimination at zero.
    pub(crate) fn note_losses(&mut self, id: &PieceId, points: u32, events: &mut Vec<Event>) {
        self.note_losses_with(id, points, 0, events);
    }

    /// `note_losses`, with the motorisation points that went with them.
    fn note_losses_with(
        &mut self,
        id: &PieceId,
        points: u32,
        motorisation: u32,
        events: &mut Vec<Event>,
    ) {
        if points == 0 {
            return;
        }
        let piece = self.pieces.get_mut(id).expect("a piece");
        events.push(Event::Losses {
            piece: id.clone(),
            points,
            toe: piece.toe,
            motorisation,
        });
        if piece.toe == 0 {
            piece.location = Location::Eliminated;
            events.push(Event::Eliminated { piece: id.clone() });
            self.release_attached(id);
        }
    }

    /// With `tanks_with_units` on, `side`'s combat units in `hex` with no
    /// TOE left (their tanks broken down for them, 61.38's `all_tanks`)
    /// are eliminated once no unit of theirs there with TOE remains: an
    /// assault that finds nothing to fight, or that kills every point
    /// beside them, clears the hex. A counter with no points has only its
    /// CPA (3.5), so it has nothing to defend with (15.55) and no point
    /// for a loss to take; zero by combat is gone (19.62). Their tanks
    /// stay broken down where they are.
    pub(crate) fn eliminate_bare(&mut self, hex: &HexId, side: Side, events: &mut Vec<Event>) {
        if !self.modules.tanks_with_units {
            return;
        }
        let ours = || {
            self.pieces.values().filter(|p| {
                p.side == side
                    && p.hex() == Some(hex)
                    && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
            })
        };
        if ours().any(|p| p.toe > 0) {
            return;
        }
        let bare: Vec<PieceId> = ours()
            .filter(|p| p.kind == PieceKind::Combat)
            .map(|p| p.id.clone())
            .collect();
        for id in bare {
            self.pieces.get_mut(&id).expect("listed").location = Location::Eliminated;
            events.push(Event::Eliminated { piece: id.clone() });
            self.release_attached(&id);
        }
    }

    /// Removes TOE points from `shares` until their `rate` (raw rating,
    /// armour protection, vulnerability) absorbs at least `needed`: with
    /// `absorb_by_loss_order` on, the entries of each piece's loss order
    /// first, in its order; then the highest rate first. Points rated
    /// zero absorb nothing and stay.
    /// Returns (absorbed, TOE points removed).
    pub(crate) fn absorb(
        &mut self,
        data: &Data,
        shares: &mut [Share],
        rate: impl Fn(&Part) -> u32,
        needed: u32,
        events: &mut Vec<Event>,
    ) -> (u32, u32) {
        self.absorb_by(data, shares, |_, p| rate(p), needed, events)
    }

    /// `absorb` with a rate that also reads the piece.
    pub(crate) fn absorb_by(
        &mut self,
        data: &Data,
        shares: &mut [Share],
        rate: impl Fn(&PieceId, &Part) -> u32,
        needed: u32,
        events: &mut Vec<Event>,
    ) -> (u32, u32) {
        self.absorb_capped(data, shares, rate, needed, None, events)
    }

    /// `absorb_by`, taking no more than `halftracks` halftrack points
    /// when given (E-004) and a motorisation point with each (14.48,
    /// R-177); the count left is written back.
    pub(crate) fn absorb_capped(
        &mut self,
        data: &Data,
        shares: &mut [Share],
        rate: impl Fn(&PieceId, &Part) -> u32,
        needed: u32,
        mut halftracks: Option<&mut u32>,
        events: &mut Vec<Event>,
    ) -> (u32, u32) {
        let mut absorbed = 0;
        let mut removed = 0;
        // Per piece: TOE points and motorisation points lost.
        let mut per_piece: Vec<(PieceId, u32, u32)> = Vec::new();
        let mut destroyed = Vec::new();
        while absorbed < needed {
            // (share, weapon, rate, tank, capped, rank in the loss order)
            let mut best: Option<(usize, Option<String>, u32, bool, bool, usize)> = None;
            for (i, share) in shares.iter().enumerate() {
                if share.points == 0 {
                    continue;
                }
                let piece = &self.pieces[&share.piece];
                for part in self.parts(data, piece) {
                    let r = rate(&share.piece, &part);
                    let capped = part.halftrack && halftracks.is_some();
                    if r == 0 || part.points == 0 || (capped && halftracks.as_deref() == Some(&0)) {
                        continue;
                    }
                    // The owner's standing order first (14.4x: any points
                    // that absorb at least the damage will do), then the
                    // best rate, so the fewest points go.
                    let rank = self
                        .modules
                        .absorb_by_loss_order
                        .then(|| {
                            piece
                                .loss_order
                                .iter()
                                .position(|w| part.weapon.as_deref() == Some(w.as_str()))
                        })
                        .flatten()
                        .unwrap_or(usize::MAX);
                    if best
                        .as_ref()
                        .is_none_or(|b| rank < b.5 || (rank == b.5 && r > b.2))
                    {
                        best = Some((i, part.weapon.clone(), r, part.tank, capped, rank));
                    }
                }
            }
            let Some((i, weapon, r, tank, capped, _)) = best else {
                break;
            };
            let id = shares[i].piece.clone();
            if !self.take_loss_entry(&id, weapon.as_deref()) {
                break;
            }
            let mut carried = 0;
            if capped && let Some(left) = halftracks.as_deref_mut() {
                *left -= 1;
                let p = self.pieces.get_mut(&id).expect("a piece");
                carried = p.motorised.min(1);
                p.motorised -= carried;
            }
            if tank {
                self.mark_destroyed_tank(&id, weapon.as_deref(), &mut destroyed);
            }
            shares[i].points -= 1;
            absorbed += r;
            removed += 1;
            match per_piece.iter_mut().find(|(p, ..)| *p == id) {
                Some((_, n, m)) => {
                    *n += 1;
                    *m += carried;
                }
                None => per_piece.push((id, 1, carried)),
            }
        }
        events.extend(destroyed);
        for (id, n, m) in per_piece {
            self.note_losses_with(&id, n, m, events);
        }
        (absorbed, removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, pid};
    use crate::order::Order;
    use crate::piece::Location;
    use crate::state::Modules;

    /// 1 RTR's A9 cruisers at C4021 absorbing 9 damage by protection.
    fn shoot_1_rtr(modules: Modules) -> (GameState, Vec<Event>, u32) {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = GameState::with_modules(&data, 5, modules).unwrap();
        crate::state::test_support::auto_place(&data, &mut state);
        let rtr = pid("unit:cw:1-rtr");
        state.put(&rtr, Location::Hex(hex("C4021")));
        let toe = state.pieces[&rtr].toe;
        let mut shares = vec![Share {
            piece: rtr,
            points: toe,
        }];
        let mut events = Vec::new();
        let (_, removed) = state.absorb(&data, &mut shares, |p| p.protection, 9, &mut events);
        (state, events, removed)
    }

    #[test]
    fn tanks_destroyed_in_combat_stay_as_a_destroyed_tank_marker_14_5() {
        let (state, events, removed) = shoot_1_rtr(Modules::ALL);
        assert!(removed > 0);
        let markers = &state.destroyed_tanks[&hex("C4021")];
        assert_eq!(markers.len(), 1, "{markers:?}");
        assert_eq!(markers[0].side, Side::Cw);
        assert_eq!(markers[0].points, removed);
        let weapon = markers[0].weapon.clone();
        assert!(events.contains(&Event::TanksDestroyed {
            piece: pid("unit:cw:1-rtr"),
            hex: hex("C4021"),
            weapon,
            points: removed,
        }));
    }

    #[test]
    fn without_the_module_destroyed_tanks_leave_nothing() {
        let (state, events, removed) = shoot_1_rtr(Modules::default());
        assert!(removed > 0);
        assert!(state.destroyed_tanks.is_empty());
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::TanksDestroyed { .. }))
        );
    }

    /// 1 RTR made up as 4 A9, 2 A10 and 1 A13 listed, 2 points unlisted,
    /// in the Commonwealth's movement segment with nothing pending.
    fn mixed_1_rtr() -> (Data, GameState, PieceId) {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = crate::movement::test_support::moving(&data, 3, Side::Cw);
        let rtr = pid("unit:cw:1-rtr");
        let p = state.pieces.get_mut(&rtr).unwrap();
        p.weapons = [("A9", 4), ("A10", 2), ("A13", 1)]
            .into_iter()
            .map(|(weapon, points)| cna_data::oa::ToeEntry {
                weapon: weapon.into(),
                points,
            })
            .collect();
        p.toe = 9;
        (data, state, rtr)
    }

    fn entries(state: &GameState, id: &PieceId) -> Vec<(String, u32)> {
        state.pieces[id]
            .weapons
            .iter()
            .map(|w| (w.weapon.clone(), w.points))
            .collect()
    }

    fn set(piece: &PieceId, weapons: &[&str]) -> Order {
        Order::SetLossOrder {
            piece: piece.clone(),
            weapons: weapons.iter().map(|w| w.to_string()).collect(),
        }
    }

    fn e(list: &[(&str, u32)]) -> Vec<(String, u32)> {
        list.iter().map(|(w, n)| (w.to_string(), *n)).collect()
    }

    #[test]
    fn with_no_loss_order_the_largest_entry_goes_first_then_the_unlisted() {
        let (_, mut state, rtr) = mixed_1_rtr();
        assert_eq!(state.take_loss(&rtr, 3, &mut Vec::new()), 3);
        // A9 4 -> 2 while it outnumbers the 2 unlisted; then an unlisted point.
        assert_eq!(
            entries(&state, &rtr),
            e(&[("A9", 2), ("A10", 2), ("A13", 1)])
        );
        assert_eq!(state.pieces[&rtr].toe, 6);
    }

    #[test]
    fn losses_follow_the_owner_s_order_then_the_engine_s_12_45() {
        let (data, mut state, rtr) = mixed_1_rtr();
        let events = state
            .apply(&data, Side::Cw, set(&rtr, &["A13", "A10"]))
            .unwrap();
        assert_eq!(
            events,
            vec![Event::LossOrderSet {
                piece: rtr.clone(),
                weapons: vec!["A13".into(), "A10".into()],
            }]
        );
        assert_eq!(state.pieces[&rtr].loss_order, ["A13", "A10"]);
        let mut events = Vec::new();
        assert_eq!(state.take_loss(&rtr, 2, &mut events), 2);
        assert_eq!(
            entries(&state, &rtr),
            e(&[("A9", 4), ("A10", 1), ("A13", 0)])
        );
        // The list runs out: the largest entry, as with none.
        assert_eq!(state.take_loss(&rtr, 2, &mut events), 2);
        assert_eq!(
            entries(&state, &rtr),
            e(&[("A9", 3), ("A10", 0), ("A13", 0)])
        );
        assert_eq!(state.pieces[&rtr].toe, 5);
    }

    /// Armour protection per point of each of `id`'s weapon entries.
    fn protection(data: &Data, state: &GameState, id: &PieceId) -> Vec<(String, u32)> {
        state
            .parts(data, &state.pieces[id])
            .iter()
            .filter_map(|p| Some((p.weapon.clone()?, p.protection)))
            .collect()
    }

    /// `rtr` absorbing `damage` by armour protection, as anti-armour
    /// fire's damage is (14.4x); returns the points removed.
    fn absorb_damage(data: &Data, state: &mut GameState, rtr: &PieceId, damage: u32) -> u32 {
        let mut shares = vec![Share {
            piece: rtr.clone(),
            points: state.pieces[rtr].toe,
        }];
        let (absorbed, removed) =
            state.absorb(data, &mut shares, |p| p.protection, damage, &mut Vec::new());
        assert!(absorbed >= damage, "{absorbed} absorbed of {damage}");
        removed
    }

    #[test]
    fn damage_is_absorbed_by_the_owner_s_order_first_14_4x() {
        let (data, mut state, rtr) = mixed_1_rtr();
        let rates = protection(&data, &state, &rtr);
        let rate = |w: &str| rates.iter().find(|(n, _)| n == w).unwrap().1;
        // The cruiser the engine would spend first, and a cheaper one.
        let (best, _) = rates.iter().max_by_key(|(_, r)| *r).unwrap().clone();
        let (cheap, low) = rates.iter().min_by_key(|(_, r)| *r).unwrap().clone();
        assert!(low > 0 && low < rate(&best), "{rates:?}");
        state.modules.absorb_by_loss_order = true;
        state
            .apply(&data, Side::Cw, set(&rtr, &[cheap.as_str()]))
            .unwrap();
        let before = entries(&state, &rtr);
        // Damage the best point alone would absorb: the owner's cheap
        // points take it instead, as many as it needs (14.4x's "three
        // light tanks" rather than "one cruiser").
        let damage = rate(&best);
        let removed = absorb_damage(&data, &mut state, &rtr, damage);
        assert_eq!(removed, damage.div_ceil(low));
        for ((w, was), (_, now)) in before.iter().zip(entries(&state, &rtr)) {
            let lost = if *w == cheap { removed } else { 0 };
            assert_eq!(now, was - lost, "{w}");
        }
    }

    #[test]
    fn without_the_module_or_an_order_the_best_protected_point_absorbs_first() {
        for (module, order) in [(false, true), (true, false)] {
            let (data, mut state, rtr) = mixed_1_rtr();
            let rates = protection(&data, &state, &rtr);
            let top = rates.iter().map(|(_, r)| *r).max().unwrap();
            let (cheap, _) = rates.iter().min_by_key(|(_, r)| *r).unwrap().clone();
            state.modules.absorb_by_loss_order = module;
            if order {
                state
                    .apply(&data, Side::Cw, set(&rtr, &[cheap.as_str()]))
                    .unwrap();
            }
            let before = entries(&state, &rtr);
            assert_eq!(absorb_damage(&data, &mut state, &rtr, top), 1);
            // The one point lost is among the best protected.
            let lost: Vec<String> = before
                .iter()
                .zip(entries(&state, &rtr))
                .filter(|((_, was), (_, now))| now != was)
                .map(|((w, _), _)| w.clone())
                .collect();
            assert_eq!(lost.len(), 1, "module {module}, order {order}");
            assert!(
                rates.contains(&(lost[0].clone(), top)),
                "{lost:?} of {rates:?}"
            );
        }
    }

    #[test]
    fn an_empty_list_clears_the_loss_order() {
        let (data, mut state, rtr) = mixed_1_rtr();
        state.apply(&data, Side::Cw, set(&rtr, &["A13"])).unwrap();
        state.apply(&data, Side::Cw, set(&rtr, &[])).unwrap();
        assert!(state.pieces[&rtr].loss_order.is_empty());
        let json = serde_json::to_value(&state.pieces[&rtr]).unwrap();
        assert!(
            json.get("loss_order").is_none(),
            "an unset order is not saved"
        );
        state.take_loss(&rtr, 1, &mut Vec::new());
        assert_eq!(
            entries(&state, &rtr),
            e(&[("A9", 3), ("A10", 2), ("A13", 1)])
        );
    }

    #[test]
    fn a_loss_order_names_the_player_s_own_entries_once() {
        let (data, mut state, rtr) = mixed_1_rtr();
        let axis = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && !p.weapons.is_empty())
            .map(|p| (p.id.clone(), p.weapons[0].weapon.clone()))
            .unwrap();
        let refused = |state: &mut GameState, player, order| {
            let before = state.clone();
            let err = state.apply(&data, player, order).unwrap_err();
            assert_eq!(*state, before);
            err.to_string()
        };
        let why = refused(&mut state, Side::Cw, set(&axis.0, &[axis.1.as_str()]));
        assert!(why.contains("not one of"), "{why}");
        let why = refused(&mut state, Side::Cw, set(&rtr, &["A9", "Matilda"]));
        assert!(why.contains("no Matilda entry"), "{why}");
        let why = refused(&mut state, Side::Cw, set(&rtr, &["A9", "A10", "A9"]));
        assert!(why.contains("named twice"), "{why}");
        // Only the player to act gives it (5.2).
        let why = refused(&mut state, Side::Axis, set(&axis.0, &[axis.1.as_str()]));
        assert!(why.contains("may not act"), "{why}");
    }

    #[test]
    fn the_enemy_never_sees_a_loss_order_3_61() {
        let (data, mut state, rtr) = mixed_1_rtr();
        state.apply(&data, Side::Cw, set(&rtr, &["A13"])).unwrap();
        let aliases = crate::view::Aliases::new([7; 32]);
        let id = rtr.as_str();
        let own = crate::view::view_for(&state, &data, &aliases, Some(Side::Cw), false).unwrap();
        assert_eq!(own["pieces"][id]["loss_order"], serde_json::json!(["A13"]));
        for viewer in [Some(Side::Axis), None] {
            let v = crate::view::view_for(&state, &data, &aliases, viewer, false).unwrap();
            assert!(v["pieces"][id].get("loss_order").is_none(), "{viewer:?}");
        }
        let set = Event::LossOrderSet {
            piece: rtr.clone(),
            weapons: vec!["A13".into()],
        };
        let seen = |viewer| {
            crate::view::events_for(&state, &aliases, std::slice::from_ref(&set), viewer, false)
        };
        assert_eq!(seen(Some(Side::Cw)).len(), 1);
        assert!(seen(Some(Side::Axis)).is_empty());
        assert!(seen(None).is_empty());
    }
}
