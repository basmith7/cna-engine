//! Desert raiders (§27): a raider moves hidden (27.31), may be spotted
//! entering an enemy-controlled hex (27.41–27.44), and an LRDG may raid
//! Rommel (27.6x). Raiders are formed by Task 3 of the desert raiders
//! plan; until cna's data has their counters only tests make one.

use crate::clock::{Phase, Segment};
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::GameState;
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::map::HexId;
use cna_data::raid::RaidResult;
use cna_data::scenario::Side;

/// A raider's CPA (27.12).
pub(crate) const RAIDER_CPA: u32 = 50;

/// What a raid on Rommel costs (27.61).
const RAID_CP: u32 = 10;

/// The Axis initiative rating a raid's 2 pins (27.92).
pub(crate) const PINNED_AXIS_INITIATIVE: u8 = 3;

/// The die that spots a raider (27.41).
const SPOTTING_DIE: u8 = 6;

pub(crate) fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Raids on Rommel a scenario allows (27.64): four in a campaign game
/// (cna's four, each named a campaign), one in any shorter scenario.
pub(crate) fn raid_cap(data: &Data) -> u32 {
    if data.slug().contains("campaign") { 4 } else { 1 }
}

impl GameState {
    /// A raider never goes past its CPA of 50 (27.38), and moves only
    /// with other raiders: its owner moves it in secret (27.31), which a
    /// stack seen on the map cannot share.
    pub(crate) fn raider_cap(&self, pieces: &[PieceId], cost: Cp) -> Result<(), Illegal> {
        let raiders = pieces
            .iter()
            .filter(|id| self.pieces[*id].kind == PieceKind::Raider)
            .count();
        if raiders > 0 && raiders < pieces.len() {
            return Err(Illegal::new(
                "27.31",
                "a raider moves in secret, apart from any other unit".to_owned(),
            ));
        }
        for id in pieces {
            let p = &self.pieces[id];
            if p.kind == PieceKind::Raider && p.cp_own_half() + cost > Cp::whole(RAIDER_CPA) {
                return Err(Illegal::new(
                    "27.38",
                    format!(
                        "{id} has spent {} and the path costs {cost}: a raider never exceeds its CPA of {RAIDER_CPA}",
                        p.cp_own_half()
                    ),
                ));
            }
        }
        Ok(())
    }

    /// 27.41: each raider of a stack that has just entered `hex`, if an
    /// enemy combat unit could exert a ZOC over it, rolls one die on the
    /// game's RNG; a 6 spots and eliminates it. Declarations are not
    /// asked for: a hidden raider prompts none. Whether any raider fell,
    /// which stops the stack.
    pub(crate) fn spot_raiders(
        &mut self,
        data: &Data,
        pieces: &[PieceId],
        hex: &HexId,
        events: &mut Vec<Event>,
    ) -> bool {
        let mut fell = false;
        for id in pieces {
            let p = &self.pieces[id];
            if p.kind != PieceKind::Raider || self.zoc_at(data, hex, p.side) == Zoc::Clear {
                continue;
            }
            let roll = self.rng.d6();
            let eliminated = roll == SPOTTING_DIE;
            if eliminated {
                let p = self.pieces.get_mut(id).expect("listed");
                p.hidden = false;
                p.location = Location::Eliminated;
                fell = true;
            }
            events.push(Event::RaiderSpotted {
                piece: id.clone(),
                hex: hex.clone(),
                roll,
                eliminated,
            });
        }
        fell
    }

    /// `Order::RaidRommel` (27.6x): a Commonwealth raider next to
    /// Rommel's hex in its movement segment spends 10 CP and rolls two
    /// dice on the Raid on Rommel Table. The raider stays in its own hex:
    /// no unit enters an enemy-held one (8.1x).
    pub(crate) fn raid_rommel(
        &mut self,
        data: &Data,
        player: Side,
        id: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        let Some(p) = self.pieces.get(id) else {
            return Err(Illegal::new("27.61", format!("{id}: no such piece")));
        };
        if p.side != player || player != Side::Cw || p.kind != PieceKind::Raider {
            return Err(Illegal::new(
                "27.61",
                format!("{id}: only a Commonwealth player's LRDG raids Rommel"),
            ));
        }
        if self.clock.phase != Phase::MovementCombat
            || self.clock.segment != Some(Segment::Movement)
            || self.phasing() != Some(player)
        {
            return Err(Illegal::new(
                "27.61",
                "a raid on Rommel is made in the raider's movement segment",
            ));
        }
        let Some(at) = p.hex().cloned() else {
            return Err(Illegal::new("27.61", format!("{id} is not on the map")));
        };
        let Some((rommel, hex)) = self
            .rommel_on_map()
            .filter(|_| !self.rommel_recalled)
            .and_then(|r| Some((r.id.clone(), r.hex()?.clone())))
        else {
            return Err(Illegal::new("27.61", "Rommel is not on the map"));
        };
        if !data.map.adjacent(&at, &hex) {
            return Err(Illegal::new(
                "27.61",
                format!("{id} at {at} is not next to Rommel's hex {hex}"),
            ));
        }
        let cap = raid_cap(data);
        if self.rommel_raids >= cap {
            return Err(Illegal::new(
                "27.64",
                format!("this scenario allows {cap} raid(s) on Rommel and all are made"),
            ));
        }
        let mut events = Vec::new();
        self.charge(id, Cp::whole(RAID_CP), false, &mut events);
        self.rommel_raids += 1;
        let roll = self.rng.d6() + self.rng.d6();
        let row = data
            .raid_on_rommel
            .row(roll)
            .expect("the Raid on Rommel Table covers 2 to 12");
        let guarded = self.pieces.values().any(|q| {
            q.hex() == Some(&hex)
                && q.side == Side::Axis
                && matches!(q.kind, PieceKind::Combat | PieceKind::Hq)
        });
        let mut lrdg_eliminated = false;
        match row.result {
            RaidResult::RommelRemoved => {
                let r = self.pieces.get_mut(&rommel).expect("on the map");
                r.location = Location::Eliminated;
                r.contact = false;
            }
            RaidResult::LrdgEliminated => {
                lrdg_eliminated = guarded || !data.raid_on_rommel.lrdg_eliminated_unless_hex_empty;
            }
            RaidResult::AxisInitiative3Temporary => {
                let turns = row.turns.unwrap_or(0);
                self.axis_initiative_pinned_through =
                    Some(self.clock.game_turn.saturating_add(turns));
            }
            RaidResult::None => {}
        }
        // An LRDG the raid leaves with no CP is eliminated too (27.61).
        if self.pieces[id].cp_own_half() >= Cp::whole(RAIDER_CPA) {
            lrdg_eliminated = true;
        }
        let p = self.pieces.get_mut(id).expect("checked");
        // A raid puts the counter on the map (27.31).
        p.hidden = false;
        if lrdg_eliminated {
            p.location = Location::Eliminated;
        }
        events.push(Event::RommelRaided {
            piece: id.clone(),
            hex,
            roll,
            result: row.result,
            lrdg_eliminated,
        });
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::test_support::{moving, mv, pid};
    use crate::order::Order;
    use crate::rng::GameRng;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// A Commonwealth raider, hidden, at `hex`: 1 KRRC's counter made
    /// into one, as no LRDG counter is in cna's data yet.
    fn raider(state: &mut GameState, hex: &HexId) -> PieceId {
        let mut p = state.pieces[&pid("unit:cw:1-krrc")].clone();
        p.id = PieceId::from("unit:cw:lrdg-test".to_owned());
        p.kind = PieceKind::Raider;
        p.toe = 0;
        p.weapons.clear();
        p.equivalent = None;
        p.cpa = None;
        p.parent = None;
        p.attached_to = None;
        p.hidden = true;
        p.location = Location::Hex(hex.clone());
        let id = p.id.clone();
        state.pieces.insert(id.clone(), p);
        id
    }

    fn empty(state: &GameState, h: &HexId) -> bool {
        state.pieces_at(h).is_empty()
    }

    /// An empty hex in reach of an Axis combat unit's ZOC and an empty
    /// one next to it in no enemy's, from which a raider may step in.
    fn approach(data: &Data, state: &GameState) -> (HexId, HexId) {
        for p in state.pieces.values() {
            if p.side != Side::Axis || p.kind != PieceKind::Combat {
                continue;
            }
            let Some(ax) = p.hex() else { continue };
            for near in data.map.neighbours(ax) {
                if !empty(state, &near) || state.zoc_at(data, &near, Side::Cw) == Zoc::Clear {
                    continue;
                }
                for far in data.map.neighbours(&near) {
                    if !empty(state, &far) || state.zoc_at(data, &far, Side::Cw) != Zoc::Clear {
                        continue;
                    }
                    let mut s = state.clone();
                    let id = raider(&mut s, &far);
                    if s
                        .apply(data, Side::Cw, mv(&[id.as_str()], &[&near.to_string()]))
                        .is_ok()
                    {
                        return (far, near);
                    }
                }
            }
        }
        panic!("no hex next to an Axis combat unit to approach");
    }

    #[test]
    fn a_raider_is_a_free_mover_with_cpa_fifty() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (far, _) = approach(&data, &state);
        let id = raider(&mut state, &far);
        let p = &state.pieces[&id];
        assert_eq!(state.cpa(p), Some(50));
        let before = state.stacked(&data, &far);
        state.pieces.get_mut(&id).unwrap().location = Location::Eliminated;
        assert_eq!(state.stacked(&data, &far), before, "no stacking points");
        state.pieces.get_mut(&id).unwrap().location = Location::Hex(far.clone());
        assert!(!state.could_exert(&data, &far), "no ZOC");
    }

    #[test]
    fn a_raider_entering_an_enemy_zoc_is_spotted_on_a_six_and_survives_one_to_five() {
        let data = graziani();
        let base = moving(&data, 3, Side::Cw);
        let (far, near) = approach(&data, &base);
        let (mut spotted, mut survived) = (0, 0);
        for seed in 0..60 {
            let mut s = base.clone();
            s.rng = GameRng::new(seed);
            let id = raider(&mut s, &far);
            let events = s
                .apply(&data, Side::Cw, mv(&[id.as_str()], &[&near.to_string()]))
                .unwrap();
            let rolls: Vec<(u8, bool)> = events
                .iter()
                .filter_map(|e| match e {
                    Event::RaiderSpotted {
                        roll, eliminated, ..
                    } => Some((*roll, *eliminated)),
                    _ => None,
                })
                .collect();
            assert_eq!(rolls.len(), 1, "seed {seed}: {events:?}");
            let (roll, eliminated) = rolls[0];
            assert_eq!(eliminated, roll == 6);
            let p = &s.pieces[&id];
            if eliminated {
                assert_eq!(p.location, Location::Eliminated);
                assert!(!p.hidden);
                spotted += 1;
            } else {
                assert_eq!(p.location, Location::Hex(near.clone()));
                assert!(p.hidden, "a raider not spotted stays hidden");
                survived += 1;
            }
        }
        assert!(spotted > 0 && survived > 0, "{spotted} {survived}");
    }

    #[test]
    fn an_enemy_moves_into_a_hidden_raider_s_hex_as_if_it_were_empty_27_42() {
        // A raider holds no ground: an Axis unit may enter its hex, and
        // nothing the Axis is told (a refusal, an attack's target, the
        // combat segment) gives the hidden counter away.
        let data = graziani();
        let base = moving(&data, 3, Side::Axis);
        let (mover, to) = base
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .find_map(|p| {
                let at = p.hex()?;
                data.map.neighbours(at).into_iter().find_map(|n| {
                    let mut s = base.clone();
                    let ok = empty(&s, &n)
                        && s.apply(&data, Side::Axis, mv(&[p.id.as_str()], &[&n.to_string()]))
                            .is_ok();
                    ok.then(|| (p.id.clone(), n))
                })
            })
            .expect("an Axis unit with an empty hex to step into");
        let mut state = base.clone();
        let id = raider(&mut state, &to);
        assert_eq!(state.holder(&to), None, "a raider holds no hex");
        state
            .apply(&data, Side::Axis, mv(&[mover.as_str()], &[&to.to_string()]))
            .unwrap();
        assert_eq!(state.pieces[&mover].hex(), Some(&to));
        assert_eq!(state.pieces[&id].hex(), Some(&to));
    }

    #[test]
    fn a_raider_moves_apart_from_other_units_27_31() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (far, near) = approach(&data, &state);
        let id = raider(&mut state, &far);
        let other = pid("unit:cw:1-krrc");
        let p = state.pieces.get_mut(&other).unwrap();
        p.location = Location::Hex(far.clone());
        p.attached_to = None;
        let before = serde_json::to_string(&state).unwrap();
        let err = state
            .apply(
                &data,
                Side::Cw,
                mv(&[id.as_str(), other.as_str()], &[&near.to_string()]),
            )
            .unwrap_err();
        assert_eq!(err.case, "27.31", "{err:?}");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
    }

    #[test]
    fn a_raider_outside_enemy_zoc_rolls_nothing() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (far, near) = approach(&data, &state);
        let id = raider(&mut state, &near);
        let draws = state.rng.draws();
        let events = state
            .apply(&data, Side::Cw, mv(&[id.as_str()], &[&far.to_string()]))
            .unwrap();
        assert_eq!(state.rng.draws(), draws, "{events:?}");
        assert!(state.pieces[&id].hidden);
    }

    #[test]
    fn a_raider_never_exceeds_cpa_fifty() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (far, near) = approach(&data, &state);
        let id = raider(&mut state, &near);
        state.pieces.get_mut(&id).unwrap().cp_spent = Cp::whole(50);
        let err = state
            .apply(&data, Side::Cw, mv(&[id.as_str()], &[&far.to_string()]))
            .unwrap_err();
        assert_eq!(err.case, "27.38");
    }

    /// A Rommel stand-in at an Axis combat unit's hex (or alone in an
    /// empty hex when `alone`), and a raider next to him.
    fn with_rommel(data: &Data, state: &mut GameState, alone: bool) -> (PieceId, PieceId) {
        let (far, near) = approach(data, state);
        let ax = data
            .map
            .neighbours(&near)
            .into_iter()
            .find(|h| {
                state
                    .pieces_at(h)
                    .iter()
                    .any(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            })
            .unwrap();
        let (rommel_hex, raider_hex) = if alone { (near, far) } else { (ax, near) };
        let model = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .unwrap();
        let mut r = model.clone();
        r.id = PieceId::from("unit:de:rommel".to_owned());
        r.kind = PieceKind::Commander;
        r.toe = 0;
        r.weapons.clear();
        r.equivalent = None;
        r.attached_to = None;
        r.location = Location::Hex(rommel_hex);
        let rommel = r.id.clone();
        state.pieces.insert(rommel.clone(), r);
        let id = raider(state, &raider_hex);
        (rommel, id)
    }

    fn raid(state: &mut GameState, data: &Data, id: &PieceId) -> Result<Event, Illegal> {
        let events = state.apply(data, Side::Cw, Order::RaidRommel { piece: id.clone() })?;
        Ok(events
            .into_iter()
            .find(|e| matches!(e, Event::RommelRaided { .. }))
            .expect("a raid event"))
    }

    #[test]
    fn a_raid_on_rommel_spends_ten_cp_and_applies_each_row() {
        let data = graziani();
        let base = moving(&data, 3, Side::Cw);
        let turn = base.clock.game_turn;
        let mut seen = std::collections::BTreeSet::new();
        for alone in [false, true] {
            for seed in 0..300 {
                let mut s = base.clone();
                let (rommel, id) = with_rommel(&data, &mut s, alone);
                s.rng = GameRng::new(seed);
                let Event::RommelRaided {
                    roll,
                    result,
                    lrdg_eliminated,
                    ..
                } = raid(&mut s, &data, &id).unwrap()
                else {
                    unreachable!()
                };
                seen.insert(roll);
                let p = &s.pieces[&id];
                assert_eq!(p.cp_spent, Cp::whole(10));
                assert!(!p.hidden, "a raid puts the counter on the map");
                assert_eq!(s.rommel_raids, 1);
                let gone = |id: &PieceId| s.pieces[id].location == Location::Eliminated;
                assert_eq!(gone(&rommel), roll == 12, "roll {roll}");
                assert_eq!(result == RaidResult::RommelRemoved, roll == 12);
                let costly = matches!(roll, 3 | 9 | 11);
                assert_eq!(lrdg_eliminated, costly && !alone, "roll {roll} alone {alone}");
                assert_eq!(gone(&id), lrdg_eliminated);
                let pinned = (roll == 2).then_some(turn + 2);
                assert_eq!(s.axis_initiative_pinned_through, pinned, "roll {roll}");
            }
        }
        assert_eq!(seen.len(), 11, "every total from 2 to 12: {seen:?}");
    }

    #[test]
    fn an_lrdg_the_raid_leaves_with_no_cp_is_eliminated() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (_, id) = with_rommel(&data, &mut state, true);
        state.pieces.get_mut(&id).unwrap().cp_spent = Cp::whole(40);
        let Event::RommelRaided {
            lrdg_eliminated, ..
        } = raid(&mut state, &data, &id).unwrap()
        else {
            unreachable!()
        };
        assert!(lrdg_eliminated);
        assert_eq!(state.pieces[&id].location, Location::Eliminated);
    }

    #[test]
    fn a_second_raid_in_a_short_scenario_and_a_fifth_in_the_campaign_are_refused() {
        let data = graziani();
        assert_eq!(raid_cap(&data), 1);
        for campaign in ["campaign-game", "short-campaign-game", "desert-fox-campaign"] {
            assert_eq!(raid_cap(&Data::load(campaign).unwrap()), 4, "{campaign}");
        }
        assert_eq!(raid_cap(&Data::load("rommels-arrival").unwrap()), 1);
        let mut state = moving(&data, 3, Side::Cw);
        let (_, id) = with_rommel(&data, &mut state, true);
        state.rommel_raids = 1;
        let before = serde_json::to_string(&state).unwrap();
        let err = raid(&mut state, &data, &id).unwrap_err();
        assert_eq!(err.case, "27.64");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
    }

    #[test]
    fn only_a_raider_next_to_rommel_raids_him() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (_, id) = with_rommel(&data, &mut state, false);
        let err = raid(&mut state, &data, &pid("unit:cw:1-krrc")).unwrap_err();
        assert_eq!(err.case, "27.61");
        state.pieces.get_mut(&id).unwrap().location = Location::Eliminated;
        assert_eq!(raid(&mut state, &data, &id).unwrap_err().case, "27.61");
    }

    #[test]
    fn a_raid_s_two_pins_the_axis_initiative_at_three_for_two_turns() {
        let data = Data::load("operation-crusader").unwrap();
        let mut state = GameState::new(&data, 3).unwrap();
        crate::state::test_support::auto_place(&data, &mut state);
        assert_eq!(state.axis_initiative_rating(&data), 6);
        let turn = state.clock.game_turn;
        state.axis_initiative_pinned_through = Some(turn + 2);
        assert_eq!(state.axis_initiative_rating(&data), 3);
        state.clock.game_turn = turn + 2;
        assert_eq!(state.axis_initiative_rating(&data), 3);
        state.clock.game_turn = turn + 3;
        assert_eq!(state.axis_initiative_rating(&data), 6);
    }
}
