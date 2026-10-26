//! Guards and escape (28.2x): a captor turns infantry TOE points into
//! guard points standing with its prisoners; prisoners short of guards at
//! the end of their captor's truck convoy phase escape, rejoining their
//! side if friends are near and otherwise waiting as escapees until an
//! enemy recaptures them. Guards are points, not counters, and do not
//! fight (`NOT_SIMULATED.md`): an enemy combat unit entering a hex with
//! no captor's combat unit in it frees the prisoners and takes the
//! guards.

use crate::clock::Phase;
use crate::combat::strength::Class;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{PieceId, PieceKind};
use crate::prisoners::PRISONER_CPA;
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::movement::Mobility;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// Prisoners one guard keeps on the move (28.17, E-029).
pub const PER_GUARD: u32 = 5;

/// The guards `points` marching prisoners need (28.17, E-029).
pub fn guards_needed(points: u32) -> u32 {
    points.div_ceil(PER_GUARD)
}

/// Adds `points` to `map[side][hex]`.
pub(crate) fn add(
    map: &mut BTreeMap<Side, BTreeMap<HexId, u32>>,
    side: Side,
    hex: &HexId,
    points: u32,
) {
    if points > 0 {
        *map.entry(side).or_default().entry(hex.clone()).or_default() += points;
    }
}

/// Takes `points` from `map[side][hex]`, dropping empty entries; the
/// caller has checked there are that many.
pub(crate) fn take(
    map: &mut BTreeMap<Side, BTreeMap<HexId, u32>>,
    side: Side,
    hex: &HexId,
    points: u32,
) {
    if points == 0 {
        return;
    }
    let mine = map.get_mut(&side).expect("checked");
    let left = mine.get_mut(hex).expect("checked");
    *left -= points;
    if *left == 0 {
        mine.remove(hex);
    }
    if mine.is_empty() {
        map.remove(&side);
    }
}

/// `map[side][hex]`, or zero.
pub(crate) fn held(map: &BTreeMap<Side, BTreeMap<HexId, u32>>, side: Side, hex: &HexId) -> u32 {
    map.get(&side)
        .and_then(|m| m.get(hex))
        .copied()
        .unwrap_or(0)
}

impl GameState {
    /// Guards are detailed and released in phase C or in their owner's
    /// truck convoy phase, where its prisoners march (28.21).
    fn guard_phase(&self, player: Side) -> Result<(), Illegal> {
        let ok = self.clock.phase == Phase::Organisation
            || (self.clock.phase == Phase::TruckConvoy && self.phasing() == Some(player));
        if ok {
            Ok(())
        } else {
            Err(Illegal::new(
                "28.21",
                format!(
                    "guards are detailed in phase C or their side's phase H, not at {}",
                    self.clock
                ),
            ))
        }
    }

    /// `player`'s combat unit `piece` on the map, and its hex.
    fn guard_unit(&self, player: Side, piece: &PieceId) -> Result<HexId, Illegal> {
        let p = self
            .pieces
            .get(piece)
            .filter(|p| p.side == player && p.kind == PieceKind::Combat)
            .ok_or_else(|| {
                Illegal::new(
                    "28.21",
                    format!("{piece} is not one of {player}'s combat units"),
                )
            })?;
        p.hex()
            .cloned()
            .ok_or_else(|| Illegal::new("28.21", format!("{piece} is not on the map")))
    }

    /// `Order::DetailGuards` (28.21): `points` of `piece`'s infantry TOE
    /// points become guard points in its hex. A unit keeps at least one
    /// TOE point.
    pub(crate) fn detail_guards(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        points: u32,
    ) -> Result<Vec<Event>, Illegal> {
        self.guard_phase(player)?;
        let hex = self.guard_unit(player, piece)?;
        let p = &self.pieces[piece];
        let infantry: Vec<(Option<String>, u32)> = self
            .parts(data, p)
            .into_iter()
            .filter(|part| part.class == Class::Infantry)
            .map(|part| (part.weapon, part.points))
            .collect();
        let have: u32 = infantry.iter().map(|(_, n)| n).sum();
        if points == 0 || points > have {
            return Err(Illegal::new(
                "28.21",
                format!("{piece} has {have} infantry TOE points, not {points}"),
            ));
        }
        if points >= p.toe {
            return Err(Illegal::new(
                "28.21",
                format!("{piece} would have no TOE points left"),
            ));
        }
        let mut left = points;
        for (weapon, n) in infantry {
            for _ in 0..n.min(left) {
                let took = self.take_loss_entry(piece, weapon.as_deref());
                debug_assert!(took, "an infantry part has its points");
                left -= 1;
            }
        }
        add(&mut self.guards, player, &hex, points);
        Ok(vec![Event::GuardsDetailed {
            piece: piece.clone(),
            hex,
            points,
        }])
    }

    /// `Order::ReleaseGuards` (28.22): `points` of the guards in
    /// `piece`'s hex rejoin it as infantry TOE points, never above its
    /// printed strength (19.61).
    pub(crate) fn release_guards(
        &mut self,
        data: &Data,
        player: Side,
        piece: &PieceId,
        points: u32,
    ) -> Result<Vec<Event>, Illegal> {
        self.guard_phase(player)?;
        let hex = self.guard_unit(player, piece)?;
        let there = held(&self.guards, player, &hex);
        if points == 0 || points > there {
            return Err(Illegal::new(
                "28.22",
                format!("{there} guard points stand in {hex}, not {points}"),
            ));
        }
        let p = &self.pieces[piece];
        let Some(part) = self
            .parts(data, p)
            .into_iter()
            .find(|part| part.class == Class::Infantry)
        else {
            return Err(Illegal::new(
                "28.22",
                format!("{piece} has no infantry for guards to rejoin"),
            ));
        };
        let ceiling = self.toe_ceiling(data, p);
        if p.toe + points > ceiling {
            return Err(Illegal::new(
                "19.61",
                format!(
                    "{piece} would hold {} TOE points: at most {ceiling}",
                    p.toe + points
                ),
            ));
        }
        let unit = self.pieces.get_mut(piece).expect("checked");
        unit.toe += points;
        if let Some(name) = part.weapon {
            unit.weapons
                .iter_mut()
                .find(|w| w.weapon == name)
                .expect("a listed part is a weapon entry")
                .points += points;
        }
        take(&mut self.guards, player, &hex, points);
        Ok(vec![Event::GuardsReleased {
            piece: piece.clone(),
            hex,
            points,
        }])
    }

    /// At the end of the phasing side's truck convoy phase its prisoners
    /// short of guards escape (28.23): a hex marched into this phase keeps
    /// five a guard (28.17), one standing still keeps all with a guard.
    /// Escapees within 8 CP of a friendly combat unit rejoin their side
    /// (28.24); the rest wait in the hex (28.25).
    pub(crate) fn escape_unguarded(&mut self, data: &Data, events: &mut Vec<Event>) {
        let Some(captor) = self.phasing() else {
            return;
        };
        let own = captor.other();
        let held_here: Vec<(HexId, u32)> = self
            .prisoners
            .get(&captor)
            .map(|m| m.iter().map(|(h, n)| (h.clone(), *n)).collect())
            .unwrap_or_default();
        for (hex, points) in held_here {
            let guards = held(&self.guards, captor, &hex);
            let kept = if self.marched.contains(&hex) {
                points.min(guards * PER_GUARD)
            } else if guards > 0 {
                points
            } else {
                0
            };
            let gone = points - kept;
            if gone == 0 {
                continue;
            }
            take(&mut self.prisoners, captor, &hex, gone);
            let near = self.friends_within(data, own, &hex, Cp::whole(PRISONER_CPA));
            let (rejoined, escapees) = if near { (gone, 0) } else { (0, gone) };
            if rejoined > 0 {
                *self.escaped.entry(own).or_default() += rejoined;
            }
            add(&mut self.escapees, own, &hex, escapees);
            events.push(Event::PrisonersEscaped {
                side: own,
                hex,
                rejoined,
                escapees,
            });
        }
    }

    /// Whether a combat unit of `side` stands within `cpa` of `from`, a
    /// foot path that enters no enemy unit's hex nor an enemy-controlled
    /// one without a friend in it (28.24, traced as 32.17's).
    fn friends_within(&self, data: &Data, side: Side, from: &HexId, cpa: Cp) -> bool {
        let friends: BTreeSet<&HexId> = self
            .pieces
            .values()
            .filter(|p| p.side == side && p.kind == PieceKind::Combat)
            .filter_map(|p| p.hex())
            .collect();
        if friends.is_empty() {
            return false;
        }
        let blocked = self.trace_blocked(data, side);
        let mut best: BTreeMap<HexId, Cp> = BTreeMap::from([(from.clone(), Cp::ZERO)]);
        let mut queue = BinaryHeap::from([Reverse((Cp::ZERO, from.clone()))]);
        while let Some(Reverse((cp, at))) = queue.pop() {
            if friends.contains(&at) {
                return true;
            }
            if best.get(&at).is_some_and(|b| *b < cp) {
                continue;
            }
            for n in data.map.neighbours(&at) {
                let Some(c) = self.trace_step(data, Mobility::Foot, &blocked, &at, &n) else {
                    continue;
                };
                let next = cp + c;
                if next > cpa || best.get(&n).is_some_and(|b| *b <= next) {
                    continue;
                }
                best.insert(n.clone(), next);
                queue.push(Reverse((next, n)));
            }
        }
        false
    }

    /// `pieces` entered `at`: where the enemy holds prisoners there with
    /// no combat unit of its own, the prisoners go free and the guards
    /// are taken (28.24, guards not fighting); enemy escapees there are
    /// recaptured (28.26). Only a combat unit's arrival counts.
    pub(crate) fn free_prisoners(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        at: &HexId,
        events: &mut Vec<Event>,
    ) {
        let Some(side) = pieces
            .iter()
            .map(|id| &self.pieces[id])
            .find(|p| p.kind == PieceKind::Combat)
            .map(|p| p.side)
        else {
            return;
        };
        let enemy = side.other();
        let theirs = held(&self.prisoners, enemy, at);
        if theirs > 0 && !self.friendly_combat_in(at, enemy, &[]) {
            take(&mut self.prisoners, enemy, at, theirs);
            *self.escaped.entry(side).or_default() += theirs;
            events.push(Event::PrisonersFreed {
                side,
                hex: at.clone(),
                points: theirs,
            });
            let guards = held(&self.guards, enemy, at);
            take(&mut self.guards, enemy, at, guards);
            self.capture_prisoners(data, side, at, guards, events);
        }
        let escapees = held(&self.escapees, enemy, at);
        if escapees > 0 {
            take(&mut self.escapees, enemy, at, escapees);
            self.capture_prisoners(data, side, at, escapees, events);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{hex, moving, mv, pid};
    use crate::order::Order;

    const KRRC: &str = "unit:cw:1-krrc";

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// The Commonwealth's phase H of GT1 OpStage 1.
    fn convoy_phase(data: &Data) -> GameState {
        let mut state = moving(data, 3, Side::Cw);
        state.clock.phase = Phase::TruckConvoy;
        state.clock.segment = None;
        state
    }

    fn detail(piece: &str, points: u32) -> Order {
        Order::DetailGuards {
            piece: pid(piece),
            points,
        }
    }

    fn infantry(data: &Data, state: &GameState, piece: &str) -> u32 {
        state
            .parts(data, &state.pieces[&pid(piece)])
            .iter()
            .filter(|p| p.class == Class::Infantry)
            .map(|p| p.points)
            .sum()
    }

    #[test]
    fn infantry_points_become_guards_and_rejoin_28_21_28_22() {
        let data = graziani();
        let mut state = convoy_phase(&data);
        let at = state.pieces[&pid(KRRC)].hex().unwrap().clone();
        let (toe, inf) = (state.pieces[&pid(KRRC)].toe, infantry(&data, &state, KRRC));
        let events = state.apply(&data, Side::Cw, detail(KRRC, 2)).unwrap();
        assert_eq!(
            events,
            vec![Event::GuardsDetailed {
                piece: pid(KRRC),
                hex: at.clone(),
                points: 2,
            }]
        );
        assert_eq!(state.pieces[&pid(KRRC)].toe, toe - 2);
        assert_eq!(infantry(&data, &state, KRRC), inf - 2);
        assert_eq!(held(&state.guards, Side::Cw, &at), 2);
        let release = Order::ReleaseGuards {
            piece: pid(KRRC),
            points: 2,
        };
        state.apply(&data, Side::Cw, release.clone()).unwrap();
        assert_eq!(state.pieces[&pid(KRRC)].toe, toe);
        assert_eq!(infantry(&data, &state, KRRC), inf);
        assert!(state.guards.is_empty());
        let err = state.apply(&data, Side::Cw, release).unwrap_err();
        assert_eq!(err.case, "28.22");
    }

    #[test]
    fn guards_are_detailed_only_in_phase_c_or_their_sides_phase_h() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let err = state.apply(&data, Side::Cw, detail(KRRC, 1)).unwrap_err();
        assert_eq!(err.case, "28.21");
        let toe = state.pieces[&pid(KRRC)].toe;
        let err = convoy_phase(&data)
            .apply(&data, Side::Cw, detail(KRRC, toe))
            .unwrap_err();
        assert_eq!(err.case, "28.21");
    }

    #[test]
    fn a_march_takes_a_guard_to_every_five_prisoners_28_17() {
        let data = graziani();
        let mut state = convoy_phase(&data);
        let (from, to) = (hex("E2020"), hex("E2021"));
        add(&mut state.prisoners, Side::Cw, &from, 12);
        add(&mut state.guards, Side::Cw, &from, 2);
        let march = Order::MarchPrisoners {
            from: from.clone(),
            path: vec![to.clone()],
            points: 12,
        };
        let err = state.apply(&data, Side::Cw, march.clone()).unwrap_err();
        assert_eq!(err.case, "28.17");
        add(&mut state.guards, Side::Cw, &from, 2);
        state.apply(&data, Side::Cw, march).unwrap();
        assert_eq!(held(&state.guards, Side::Cw, &from), 1);
        assert_eq!(held(&state.guards, Side::Cw, &to), 3);
    }

    /// A hex next to an Axis combat unit with nothing in it.
    fn by_the_axis(state: &GameState, data: &Data) -> HexId {
        state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .filter_map(|p| p.hex())
            .flat_map(|h| data.map.neighbours(h))
            .find(|n| state.holder(n).is_none())
            .unwrap()
    }

    #[test]
    fn unguarded_prisoners_escape_at_the_end_of_phase_h_28_23_28_25() {
        let data = graziani();
        let mut state = convoy_phase(&data);
        let (far, near, camp) = (hex("E2020"), by_the_axis(&state, &data), hex("E2220"));
        add(&mut state.prisoners, Side::Cw, &far, 6);
        add(&mut state.prisoners, Side::Cw, &near, 4);
        add(&mut state.prisoners, Side::Cw, &camp, 12);
        add(&mut state.guards, Side::Cw, &camp, 1);
        let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert!(events.contains(&Event::PrisonersEscaped {
            side: Side::Axis,
            hex: far.clone(),
            rejoined: 0,
            escapees: 6,
        }));
        assert!(events.contains(&Event::PrisonersEscaped {
            side: Side::Axis,
            hex: near.clone(),
            rejoined: 4,
            escapees: 0,
        }));
        // One guard keeps a camp standing still (28.17).
        assert_eq!(state.prisoners[&Side::Cw], [(camp, 12)].into());
        assert_eq!(state.escapees[&Side::Axis], [(far, 6)].into());
        assert_eq!(state.escaped[&Side::Axis], 4);
    }

    #[test]
    fn prisoners_marched_this_phase_keep_five_a_guard_28_17() {
        let data = graziani();
        let mut state = convoy_phase(&data);
        let at = hex("E2020");
        add(&mut state.prisoners, Side::Cw, &at, 12);
        add(&mut state.guards, Side::Cw, &at, 2);
        state.marched.insert(at.clone());
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(held(&state.prisoners, Side::Cw, &at), 10);
        assert_eq!(held(&state.escapees, Side::Axis, &at), 2);
    }

    #[test]
    fn an_enemy_entering_frees_unprotected_prisoners_and_recaptures_escapees_28_26() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (first, second) = (hex("C3923"), hex("C3924"));
        add(&mut state.prisoners, Side::Axis, &first, 5);
        add(&mut state.guards, Side::Axis, &first, 1);
        add(&mut state.escapees, Side::Axis, &second, 4);
        let events = state
            .apply(&data, Side::Cw, mv(&[KRRC], &["C3923", "C3924"]))
            .unwrap();
        assert!(events.contains(&Event::PrisonersFreed {
            side: Side::Cw,
            hex: first.clone(),
            points: 5,
        }));
        assert_eq!(state.escaped[&Side::Cw], 5);
        assert!(state.guards.is_empty() && state.escapees.is_empty());
        assert_eq!(state.prisoners[&Side::Cw], [(first, 1), (second, 4)].into());
    }
}
