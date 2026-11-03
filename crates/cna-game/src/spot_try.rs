//! The enemy's spotting try (27.42–27.44): a phasing combat unit that
//! enters a raider's hex is told a raider is there, unnamed, and its move
//! stops so it may spend 1 CP to try to spot it; a 6 eliminates the
//! raider. Once per stage per phasing unit. A raider entering an
//! enemy-controlled hex rolls on its own (`GameState::spot_raiders`).

use crate::clock::{Phase, Segment};
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::scenario::Side;

/// What a spotting try costs (27.43).
const TRY_CP: u32 = 1;

/// The die that spots a raider (27.43).
const SPOTTING_DIE: u8 = 6;

impl GameState {
    /// Enemy raiders in `hex` of `side`'s enemy, in id order.
    fn raiders_in(&self, hex: &HexId, side: Side) -> Vec<PieceId> {
        self.pieces
            .values()
            .filter(|p| {
                p.kind == PieceKind::Raider
                    && p.side != side
                    && matches!(&p.location, Location::Hex(h) if h == hex)
            })
            .map(|p| p.id.clone())
            .collect()
    }

    /// 27.42: the phasing stack `pieces` has entered `hex` in its
    /// movement; if an enemy raider stands there, its owner announces
    /// it, and each combat unit of the stack yet to try this stage gains
    /// a try there. Whether any did: the move then stops in `hex`.
    pub(crate) fn announce_raiders(
        &mut self,
        pieces: &[PieceId],
        hex: &HexId,
        events: &mut Vec<Event>,
    ) -> bool {
        let Some(side) = pieces.first().map(|id| self.pieces[id].side) else {
            return false;
        };
        if self.clock.phase != Phase::MovementCombat || self.raiders_in(hex, side).is_empty() {
            return false;
        }
        let tries: Vec<PieceId> = pieces
            .iter()
            .filter(|id| {
                self.pieces[*id].kind == PieceKind::Combat && !self.spot_tried.contains(*id)
            })
            .cloned()
            .collect();
        for id in &tries {
            self.spot_chances.insert(id.clone(), hex.clone());
        }
        events.push(Event::RaiderAnnounced {
            hex: hex.clone(),
            pieces: tries.clone(),
        });
        !tries.is_empty()
    }

    /// `Order::TrySpot` (27.43–27.44). Every refusal is about what the
    /// mover already knows, so none tells it where a raider is: a unit
    /// never told of one is refused before anything else is looked at.
    pub(crate) fn try_spot(&mut self, player: Side, id: &PieceId) -> Result<Vec<Event>, Illegal> {
        let bad = |why: String| Illegal::new("27.44", why);
        let Some(hex) = self
            .spot_chances
            .get(id)
            .filter(|_| self.pieces.get(id).is_some_and(|p| p.side == player))
            .cloned()
        else {
            return Err(bad(format!(
                "{id} has not been told of a raider in the hex it entered this stage, or has tried already"
            )));
        };
        if self.clock.phase != Phase::MovementCombat
            || self.clock.segment != Some(Segment::Movement)
            || self.phasing() != Some(player)
        {
            return Err(bad(
                "a spotting try is made in the mover's movement segment".to_owned(),
            ));
        }
        let p = &self.pieces[id];
        if p.hex() != Some(&hex) {
            return Err(bad(format!(
                "{id} has left {hex}: a try is made where the raider was announced"
            )));
        }
        if let Some((cap, case, why)) = self.voluntary_cap(id)
            && p.cp_own_half() + Cp::whole(TRY_CP) > cap
        {
            return Err(Illegal::new(
                case,
                format!(
                    "{id} has spent {} and the try costs {TRY_CP}, past {cap}: {why}",
                    p.cp_own_half()
                ),
            ));
        }
        let raiders = self.raiders_in(&hex, player);
        if raiders.is_empty() {
            return Err(bad(format!("no raider is left in {hex} to spot")));
        }
        let mut events = Vec::new();
        self.charge(id, Cp::whole(TRY_CP), false, &mut events);
        self.spot_chances.remove(id);
        self.spot_tried.insert(id.clone());
        let mut rolls = Vec::new();
        let mut eliminated = Vec::new();
        for r in raiders {
            let roll = self.rng.d6();
            rolls.push(roll);
            if roll == SPOTTING_DIE {
                self.pieces.get_mut(&r).expect("listed").hidden = false;
                self.lose_raider(&r);
                eliminated.push(r);
            }
        }
        events.push(Event::SpotTried {
            piece: id.clone(),
            hex,
            rolls,
            eliminated,
        });
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Data;
    use crate::movement::test_support::{moving, mv};
    use crate::order::Order;
    use crate::raiders::tests::{graziani, raider};
    use crate::rng::GameRng;
    use crate::view::{Aliases, events_for};

    fn try_spot(id: &PieceId) -> Order {
        Order::TrySpot { piece: id.clone() }
    }

    /// The Axis's movement segment, an Axis combat unit, and two empty
    /// hexes it may walk in one move, the first holding a hidden
    /// Commonwealth raider.
    fn crossing(data: &Data) -> (GameState, PieceId, HexId, HexId, PieceId) {
        let base = moving(data, 3, Side::Axis);
        let empty = |s: &GameState, h: &HexId| s.pieces_at(h).is_empty();
        for p in base.pieces.values() {
            if p.side != Side::Axis || p.kind != PieceKind::Combat {
                continue;
            }
            let Some(at) = p.hex() else { continue };
            for a in data.map.neighbours(at) {
                if !empty(&base, &a) {
                    continue;
                }
                for b in data.map.neighbours(&a) {
                    if &b == at || !empty(&base, &b) {
                        continue;
                    }
                    let mut s = base.clone();
                    let path = [a.to_string(), b.to_string()];
                    let path: Vec<&str> = path.iter().map(String::as_str).collect();
                    if s.apply(data, Side::Axis, mv(&[p.id.as_str()], &path)).is_ok() {
                        let mut state = base.clone();
                        let r = raider(&mut state, &a);
                        return (state, p.id.clone(), a, b, r);
                    }
                }
            }
        }
        panic!("no Axis unit with a two-hex walk");
    }

    fn walk(data: &Data, s: &mut GameState, mover: &PieceId, a: &HexId, b: &HexId) -> Vec<Event> {
        s.apply(
            data,
            Side::Axis,
            mv(&[mover.as_str()], &[&a.to_string(), &b.to_string()]),
        )
        .unwrap()
    }

    #[test]
    fn entering_a_raider_s_hex_announces_it_unnamed_and_stops_the_move_27_42() {
        let data = graziani();
        let (mut s, mover, a, b, r) = crossing(&data);
        let events = walk(&data, &mut s, &mover, &a, &b);
        assert_eq!(s.pieces[&mover].hex(), Some(&a), "the move stops at {a}");
        assert!(events.contains(&Event::RaiderAnnounced {
            hex: a.clone(),
            pieces: vec![mover.clone()],
        }));
        assert_eq!(s.spot_chances.get(&mover), Some(&a));
        // The Axis is told a raider is there, never which.
        let seen = serde_json::to_string(&events_for(
            &s,
            &Aliases::new([3; 32]),
            &events,
            Some(Side::Axis),
            false,
        ))
        .unwrap();
        assert!(seen.contains("raider-announced"), "{seen}");
        assert!(!seen.contains(r.as_str()), "{seen}");
        // It may walk on: the stop costs it nothing.
        s.apply(&data, Side::Axis, mv(&[mover.as_str()], &[&b.to_string()]))
            .unwrap();
    }

    #[test]
    fn a_try_costs_one_cp_and_a_six_eliminates_the_raider_27_43() {
        let data = graziani();
        let (base, mover, a, b, r) = crossing(&data);
        let (mut spotted, mut missed) = (0, 0);
        for seed in 0..40 {
            let mut s = base.clone();
            s.rng = GameRng::new(seed);
            walk(&data, &mut s, &mover, &a, &b);
            let cp = s.pieces[&mover].cp_own_half();
            let events = s.apply(&data, Side::Axis, try_spot(&mover)).unwrap();
            assert_eq!(s.pieces[&mover].cp_own_half(), cp + Cp::whole(1));
            let Some(Event::SpotTried {
                rolls, eliminated, ..
            }) = events.iter().find(|e| matches!(e, Event::SpotTried { .. }))
            else {
                panic!("{events:?}");
            };
            assert_eq!(rolls.len(), 1);
            let seen = serde_json::to_string(&events_for(
                &s,
                &Aliases::new([3; 32]),
                &events,
                Some(Side::Axis),
                false,
            ))
            .unwrap();
            assert!(seen.contains("spot-tried"), "{seen}");
            if rolls[0] == 6 {
                assert_eq!(eliminated, &vec![r.clone()]);
                assert_eq!(s.pieces[&r].location, Location::Eliminated);
                assert!(s.raiders_lost.contains_key(&r));
                spotted += 1;
            } else {
                assert!(eliminated.is_empty());
                assert!(s.pieces[&r].hidden);
                assert!(!seen.contains(r.as_str()), "{seen}");
                missed += 1;
            }
        }
        assert!(spotted > 0 && missed > 0, "{spotted} {missed}");
    }

    #[test]
    fn a_unit_tries_once_a_stage_and_only_where_it_was_told_27_44() {
        let data = graziani();
        let (mut s, mover, a, b, _) = crossing(&data);
        s.rng = GameRng::new(1);
        walk(&data, &mut s, &mover, &a, &b);
        let mut walked_on = s.clone();
        s.apply(&data, Side::Axis, try_spot(&mover)).unwrap();
        let err = s.apply(&data, Side::Axis, try_spot(&mover)).unwrap_err();
        assert_eq!(err.case, "27.44", "{err:?}");
        // Leaving and coming back gives no second try, and no stop.
        s.apply(&data, Side::Axis, mv(&[mover.as_str()], &[&b.to_string()]))
            .unwrap();
        s.apply(&data, Side::Axis, mv(&[mover.as_str()], &[&a.to_string()]))
            .unwrap();
        assert!(!s.spot_chances.contains_key(&mover));
        assert_eq!(
            s.apply(&data, Side::Axis, try_spot(&mover)).unwrap_err().case,
            "27.44"
        );
        // A unit that walked on has left the hex it was told of.
        walked_on
            .apply(&data, Side::Axis, mv(&[mover.as_str()], &[&b.to_string()]))
            .unwrap();
        let err = walked_on
            .apply(&data, Side::Axis, try_spot(&mover))
            .unwrap_err();
        assert_eq!(err.case, "27.44", "{err:?}");
    }

    #[test]
    fn a_unit_never_told_of_a_raider_learns_nothing_from_a_refusal() {
        // A raider moved in beside a unit already standing: no
        // announcement, so a try is refused exactly as in an empty hex.
        let data = graziani();
        let (base, mover, _, _, r) = crossing(&data);
        let at = base.pieces[&mover].hex().unwrap().clone();
        let mut with = base.clone();
        with.pieces.get_mut(&r).unwrap().location = Location::Hex(at);
        let mut without = base.clone();
        without.pieces.remove(&r);
        let draws = with.rng.draws();
        let a = with.apply(&data, Side::Axis, try_spot(&mover)).unwrap_err();
        let b = without.apply(&data, Side::Axis, try_spot(&mover)).unwrap_err();
        assert_eq!(a, b);
        assert_eq!(with.rng.draws(), draws);
    }

    #[test]
    fn the_tries_are_cleared_as_phase_g_ends() {
        let data = graziani();
        let (mut s, mover, a, b, r) = crossing(&data);
        walk(&data, &mut s, &mover, &a, &b);
        s.apply(&data, Side::Axis, try_spot(&mover)).unwrap();
        assert!(!s.spot_tried.is_empty());
        // No Commonwealth unit to fight: the Axis may pass phase G.
        for p in s.pieces.values_mut() {
            if p.side == Side::Cw && p.id != r {
                p.location = Location::Eliminated;
            }
        }
        let stage = s.clock.stage;
        s.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert!(s.clock.phase != Phase::MovementCombat || s.clock.stage != stage);
        assert!(s.spot_tried.is_empty() && s.spot_chances.is_empty());
    }
}
