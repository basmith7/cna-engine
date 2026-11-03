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

/// What a raid on a supply dump costs (27.5x; the Desert Raider Raids
/// Table's `cp_cost`).
const DUMP_RAID_CP: u32 = 5;

/// The highest supply-dump die that burns 10 % (the table's
/// `supply-dump` row).
const DUMP_HIT_MAX: u8 = 2;

/// The supply-dump die that eliminates a raider among combat units.
const GUARDED_DUMP_DEATH: u8 = 6;

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
    if data.slug().contains("campaign") {
        4
    } else {
        1
    }
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
                self.pieces.get_mut(id).expect("listed").hidden = false;
                self.lose_raider(id);
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
        // A raid puts the counter on the map (27.31).
        self.pieces.get_mut(id).expect("checked").hidden = false;
        if lrdg_eliminated {
            self.lose_raider(id);
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

    /// `Order::RaidDump` (27.55, 27.56, R-149): a raider in its movement
    /// segment, in or next to an enemy supply unit's hex outside a major
    /// city, spends 5 CP to raid it as a supply dump on the Desert Raider
    /// Raids Table (27.91). It raids from where it stands, as no unit
    /// enters an enemy-held hex (8.1x), and may move on afterwards.
    pub(crate) fn raid_dump(
        &mut self,
        data: &Data,
        player: Side,
        id: &PieceId,
        target: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        let Some(p) = self.pieces.get(id) else {
            return Err(Illegal::new("27.55", format!("{id}: no such piece")));
        };
        if p.side != player || p.kind != PieceKind::Raider {
            return Err(Illegal::new(
                "27.55",
                format!("{id}: only a desert raider raids a supply dump"),
            ));
        }
        if !self.modules.abstract_supply {
            return Err(Illegal::new(
                "R-149",
                "a supply unit is a dump only under abstract supply (§32)",
            ));
        }
        if self.clock.phase != Phase::MovementCombat
            || self.clock.segment != Some(Segment::Movement)
            || self.phasing() != Some(player)
        {
            return Err(Illegal::new(
                "27.51",
                "a raid is made in the raider's movement segment",
            ));
        }
        let Some(at) = p.hex().cloned() else {
            return Err(Illegal::new("27.55", format!("{id} is not on the map")));
        };
        let Some((hex, dummy, owner)) = self
            .pieces
            .get(target)
            .filter(|t| t.side != player && t.kind.is_supply())
            .and_then(|t| {
                let dummy = t.kind == PieceKind::DummySupplyUnit;
                Some((t.hex()?.clone(), dummy, t.side))
            })
        else {
            return Err(Illegal::new(
                "27.55",
                format!("{target}: no enemy supply unit on the map"),
            ));
        };
        if at != hex && !data.map.adjacent(&at, &hex) {
            return Err(Illegal::new(
                "27.55",
                format!("{id} at {at} is not in or next to {hex}"),
            ));
        }
        if data
            .map
            .hex(&hex)
            .is_some_and(|h| h.terrain == cna_data::map::Terrain::MajorCity)
        {
            return Err(Illegal::new(
                "27.55",
                format!("{hex} is a major city hex: no dump there is raided"),
            ));
        }
        if self.raided.contains(&(id.clone(), hex.clone())) {
            return Err(Illegal::new(
                "27.91",
                format!("{id} has raided at {hex} this stage"),
            ));
        }
        self.raider_cap(std::slice::from_ref(id), Cp::whole(DUMP_RAID_CP))?;
        let guards: Vec<&crate::piece::Piece> = self
            .pieces_at(&hex)
            .into_iter()
            .filter(|q| q.side == owner && q.kind == PieceKind::Combat)
            .collect();
        let guard_strength = (!guards.is_empty()).then(|| self.raw_defence(data, &guards));
        let mut events = Vec::new();
        self.charge(id, Cp::whole(DUMP_RAID_CP), false, &mut events);
        self.raided.insert((id.clone(), hex.clone()));
        // The chart: two dice equal to or above the guards' raw
        // close-assault defence get past them (27.91's note).
        let guard_roll = guard_strength.map(|_| self.rng.d6() + self.rng.d6());
        let mut raider_eliminated =
            matches!((guard_roll, guard_strength), (Some(r), Some(s)) if u32::from(r) < s);
        let roll = (!raider_eliminated).then(|| self.rng.d6());
        if roll == Some(GUARDED_DUMP_DEATH) && guard_strength.is_some() {
            raider_eliminated = true;
        }
        let (mut ammo_lost, mut fuel_lost, mut removed) = (0, 0, dummy);
        if roll.is_some_and(|r| r <= DUMP_HIT_MAX)
            && let Some(load) = self.pieces.get_mut(target).and_then(|t| t.load.as_mut())
        {
            // 10 % of each kind, rounded up (R-149).
            ammo_lost = load.ammo.div_ceil(10);
            fuel_lost = load.fuel.div_ceil(10);
            load.ammo -= ammo_lost;
            load.fuel -= fuel_lost;
            removed = load.ammo == 0 && load.fuel == 0;
        }
        if removed {
            // A dummy found is removed (27.56, 32.18); a unit left with
            // nothing is too (32.11).
            self.release_motorisation(target);
            self.pieces.get_mut(target).expect("checked").location = Location::Eliminated;
        }
        // A raid puts the counter on the map (27.5x).
        self.pieces.get_mut(id).expect("checked").hidden = false;
        if raider_eliminated {
            self.lose_raider(id);
        }
        events.push(Event::DumpRaided {
            piece: id.clone(),
            target: target.clone(),
            hex,
            guard_roll,
            guard_strength,
            roll,
            dummy,
            ammo_lost,
            fuel_lost,
            removed,
            raider_eliminated,
        });
        Ok(events)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::movement::test_support::{moving, mv, pid};
    use crate::order::Order;
    use crate::rng::GameRng;

    pub(crate) fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    /// A Commonwealth raider, hidden, at `hex`: 1 KRRC's counter made
    /// into one, as no LRDG counter is in cna's data yet.
    pub(crate) fn raider(state: &mut GameState, hex: &HexId) -> PieceId {
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
                    if s.apply(data, Side::Cw, mv(&[id.as_str()], &[&near.to_string()]))
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
        // A raider holds no ground: an Axis unit may enter its hex. It is
        // told a raider is there (27.42, `spot_try`), but nothing names
        // the hidden counter.
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
    fn a_raider_s_reach_stops_at_cpa_fifty_27_38() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (far, _) = approach(&data, &state);
        let id = raider(&mut state, &far);
        state.pieces.get_mut(&id).unwrap().cp_spent = Cp::whole(40);
        let reach = state
            .reachable(
                &data,
                Side::Cw,
                std::slice::from_ref(&id),
                crate::reach::Purpose::Move,
            )
            .unwrap();
        assert!(!reach.is_empty());
        assert!(
            reach.iter().all(|r| r.cp <= Cp::whole(10)),
            "{:?}",
            reach.iter().map(|r| r.cp).max()
        );
        // The dearest hex offered is one `apply` accepts.
        if let Some(r) = reach.iter().max_by_key(|r| r.cp) {
            let mut s = state.clone();
            let path: Vec<String> = r.path.iter().map(|h| h.to_string()).collect();
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            assert!(
                s.apply(&data, Side::Cw, mv(&[id.as_str()], &path)).is_ok(),
                "{}",
                r.hex
            );
        }
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
                assert_eq!(
                    lrdg_eliminated,
                    costly && !alone,
                    "roll {roll} alone {alone}"
                );
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
        for campaign in [
            "campaign-game",
            "short-campaign-game",
            "desert-fox-campaign",
        ] {
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

    #[test]
    fn the_dump_raid_constants_are_the_desert_raider_raids_table_s() {
        let t = cna_data::errata::load_table(&cna_data::source::Fs, "desert-raider-raids").unwrap();
        assert_eq!(t["cp_cost"], DUMP_RAID_CP);
        let row = |name: &str| {
            t["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["target"] == name)
                .unwrap()
                .clone()
        };
        let dump = row("supply-dump");
        let die = dump["die"].as_array().unwrap();
        assert_eq!(die[0]["to"], DUMP_HIT_MAX);
        assert_eq!(die[0]["result"], "destroy-percent");
        assert_eq!(die[0]["percent"], 10);
        assert_eq!(die[2]["from"], GUARDED_DUMP_DEATH);
        assert_eq!(die[2]["raider_eliminated_if_combat_units"], true);
        let guarded = row("supply-dump-guarded");
        assert_eq!(guarded["guard_check"]["dice"], 2);
        assert_eq!(
            guarded["guard_check"]["survive_if_sum_at_least"],
            "raw-close-assault-defence"
        );
    }

    /// An Axis supply unit (or dummy) holding 35 ammunition and 41 fuel
    /// at `hex`.
    fn dump(state: &mut GameState, hex: &HexId, dummy: bool) -> PieceId {
        let model = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .unwrap();
        let mut d = model.clone();
        d.id = if dummy {
            PieceId::dummy_supply(Side::Axis, 900)
        } else {
            PieceId::supply(Side::Axis, 900)
        };
        d.kind = if dummy {
            PieceKind::DummySupplyUnit
        } else {
            PieceKind::SupplyUnit
        };
        d.nation = None;
        d.id_code = None;
        d.toe = 0;
        d.weapons.clear();
        d.equivalent = None;
        d.parent = None;
        d.attached_to = None;
        d.load = (!dummy).then_some(crate::piece::Load { ammo: 35, fuel: 41 });
        d.location = Location::Hex(hex.clone());
        let id = d.id.clone();
        state.pieces.insert(id.clone(), d);
        id
    }

    /// A raider and an Axis dump next to it, alone or (`guarded`) in an
    /// Axis combat unit's hex, under abstract supply.
    fn dump_raid(
        data: &Data,
        state: &mut GameState,
        guarded: bool,
        dummy: bool,
    ) -> (PieceId, PieceId) {
        state.modules.abstract_supply = true;
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
        let (dump_hex, raider_hex) = if guarded { (ax, near) } else { (near, far) };
        let terrain = data.map.hex(&dump_hex).unwrap().terrain.name();
        assert_ne!(terrain, "major-city");
        let target = dump(state, &dump_hex, dummy);
        (raider(state, &raider_hex), target)
    }

    fn raid_dump(
        state: &mut GameState,
        data: &Data,
        id: &PieceId,
        target: &PieceId,
    ) -> Result<Event, Illegal> {
        let events = state.apply(
            data,
            Side::Cw,
            Order::RaidDump {
                piece: id.clone(),
                target: target.clone(),
            },
        )?;
        Ok(events
            .into_iter()
            .find(|e| matches!(e, Event::DumpRaided { .. }))
            .expect("a raid event"))
    }

    #[test]
    fn an_unguarded_raid_burns_ten_percent_of_each_kind_rounded_up_on_one_or_two() {
        let data = graziani();
        let base = moving(&data, 3, Side::Cw);
        let (mut hits, mut misses) = (0, 0);
        for seed in 0..40 {
            let mut s = base.clone();
            let (id, target) = dump_raid(&data, &mut s, false, false);
            s.rng = GameRng::new(seed);
            let Event::DumpRaided {
                guard_roll,
                roll,
                dummy,
                ammo_lost,
                fuel_lost,
                removed,
                raider_eliminated,
                ..
            } = raid_dump(&mut s, &data, &id, &target).unwrap()
            else {
                unreachable!()
            };
            assert_eq!(guard_roll, None);
            assert!(
                !dummy && !removed && !raider_eliminated,
                "unguarded: no 6 kills"
            );
            let roll = roll.expect("a die");
            let load = s.pieces[&target].load.unwrap();
            if roll <= 2 {
                hits += 1;
                // 3.5 and 4.1, rounded up.
                assert_eq!((ammo_lost, fuel_lost), (4, 5));
                assert_eq!((load.ammo, load.fuel), (31, 36));
            } else {
                misses += 1;
                assert_eq!((ammo_lost, fuel_lost), (0, 0));
                assert_eq!((load.ammo, load.fuel), (35, 41));
            }
            let p = &s.pieces[&id];
            assert_eq!(p.cp_spent, Cp::whole(5));
            assert!(!p.hidden, "a raid puts the counter on the map");
            assert!(p.hex().is_some());
        }
        assert!(hits > 0 && misses > 0, "{hits} {misses}");
    }

    #[test]
    fn a_dump_raid_leaving_a_unit_empty_removes_it_32_11() {
        let data = graziani();
        let base = moving(&data, 3, Side::Cw);
        let (s, target, e) = (0..40)
            .find_map(|seed| {
                let mut s = base.clone();
                let (id, target) = dump_raid(&data, &mut s, false, false);
                s.pieces.get_mut(&target).unwrap().load =
                    Some(crate::piece::Load { ammo: 1, fuel: 0 });
                s.rng = GameRng::new(seed);
                let e = raid_dump(&mut s, &data, &id, &target).unwrap();
                matches!(
                    e,
                    Event::DumpRaided {
                        roll: Some(1 | 2),
                        ..
                    }
                )
                .then_some((s, target, e))
            })
            .expect("a hit in 40 seeds");
        assert!(
            matches!(
                e,
                Event::DumpRaided {
                    removed: true,
                    ammo_lost: 1,
                    fuel_lost: 0,
                    ..
                }
            ),
            "{e:?}"
        );
        assert_eq!(s.pieces[&target].location, Location::Eliminated);
    }

    #[test]
    fn a_raided_dummy_is_revealed_and_removed_27_56() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (id, target) = dump_raid(&data, &mut state, false, true);
        let Event::DumpRaided {
            dummy,
            removed,
            ammo_lost,
            fuel_lost,
            ..
        } = raid_dump(&mut state, &data, &id, &target).unwrap()
        else {
            unreachable!()
        };
        assert!(dummy && removed);
        assert_eq!((ammo_lost, fuel_lost), (0, 0));
        assert_eq!(state.pieces[&target].location, Location::Eliminated);
    }

    #[test]
    fn a_guarded_dump_kills_a_raider_below_the_guards_strength_or_on_a_six() {
        let data = graziani();
        let mut base = moving(&data, 3, Side::Cw);
        let (id, target) = dump_raid(&data, &mut base, true, false);
        // One guard, thin enough that two dice sometimes get past it.
        let hex = base.pieces[&target].hex().unwrap().clone();
        let guards: Vec<PieceId> = base
            .pieces_at(&hex)
            .iter()
            .filter(|p| p.kind == PieceKind::Combat)
            .map(|p| p.id.clone())
            .collect();
        for (i, g) in guards.iter().enumerate() {
            let p = base.pieces.get_mut(g).unwrap();
            p.toe = 1;
            p.weapons.clear();
            if i > 0 {
                p.location = Location::Eliminated;
            }
        }
        let strength = |s: &GameState| s.raw_defence(&data, &[&s.pieces[&guards[0]]]);
        while strength(&base) < 5 {
            base.pieces.get_mut(&guards[0]).unwrap().toe += 1;
        }
        assert!(strength(&base) <= 9, "{}", strength(&base));
        let (mut fell, mut passes, mut fails, mut level) = (0, 0, 0, 0);
        for seed in 0..200 {
            let mut s = base.clone();
            s.rng = GameRng::new(seed);
            let Event::DumpRaided {
                guard_roll,
                guard_strength,
                roll,
                ammo_lost,
                raider_eliminated,
                ..
            } = raid_dump(&mut s, &data, &id, &target).unwrap()
            else {
                unreachable!()
            };
            let (dice, strength) = (guard_roll.unwrap(), guard_strength.unwrap());
            assert!(strength > 0);
            // The chart: a sum equal to or above the strength survives.
            let passed = u32::from(dice) >= strength;
            (passes, fails) = if passed {
                (passes + 1, fails)
            } else {
                (passes, fails + 1)
            };
            level += usize::from(u32::from(dice) == strength);
            assert_eq!(roll.is_some(), passed, "no dump die after a failed check");
            assert_eq!(raider_eliminated, !passed || roll == Some(6), "seed {seed}");
            if !passed {
                assert_eq!(ammo_lost, 0);
            }
            if raider_eliminated {
                fell += 1;
                assert_eq!(s.pieces[&id].location, Location::Eliminated);
                assert!(s.raiders_lost.contains_key(&id), "27.16's wait starts");
            }
        }
        assert!(
            fell > 0 && passes > 0 && fails > 0,
            "{fell} {passes} {fails}"
        );
        assert!(level > 0, "a sum equal to the strength was rolled");
    }

    #[test]
    fn a_dump_in_a_major_city_is_not_raided() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        state.modules.abstract_supply = true;
        let city = data
            .map
            .hexes()
            .find(|h| h.terrain.name() == "major-city")
            .unwrap()
            .id
            .clone();
        let beside = data.map.neighbours(&city)[0].clone();
        let target = dump(&mut state, &city, false);
        let id = raider(&mut state, &beside);
        let before = serde_json::to_string(&state).unwrap();
        let err = raid_dump(&mut state, &data, &id, &target).unwrap_err();
        assert_eq!(err.case, "27.55", "{err:?}");
        assert!(err.to_string().contains("major city"), "{err:?}");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
    }

    #[test]
    fn a_raider_raids_a_hex_once_a_stage() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (id, target) = dump_raid(&data, &mut state, false, false);
        raid_dump(&mut state, &data, &id, &target).unwrap();
        let before = serde_json::to_string(&state).unwrap();
        let err = raid_dump(&mut state, &data, &id, &target).unwrap_err();
        assert_eq!(err.case, "27.91", "{err:?}");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        // The movement segment after reserve release is the same stage.
        state.open_movement_segment(&data, &mut Vec::new());
        let err = raid_dump(&mut state, &data, &id, &target).unwrap_err();
        assert_eq!(err.case, "27.91", "{err:?}");
    }

    #[test]
    fn a_dump_raid_is_made_in_the_raider_s_movement_segment_and_within_cpa_fifty() {
        let data = graziani();
        let mut state = moving(&data, 3, Side::Cw);
        let (id, target) = dump_raid(&data, &mut state, false, false);
        let mut barrage = state.clone();
        barrage.clock.segment = Some(Segment::Barrage);
        assert_eq!(
            raid_dump(&mut barrage, &data, &id, &target)
                .unwrap_err()
                .case,
            "27.51"
        );
        state.pieces.get_mut(&id).unwrap().cp_spent = Cp::whole(46);
        assert_eq!(
            raid_dump(&mut state, &data, &id, &target).unwrap_err().case,
            "27.38"
        );
        // Only a raider raids, and only an enemy supply unit.
        state.pieces.get_mut(&id).unwrap().cp_spent = Cp::ZERO;
        let krrc = pid("unit:cw:1-krrc");
        assert_eq!(
            raid_dump(&mut state, &data, &krrc, &target)
                .unwrap_err()
                .case,
            "27.55"
        );
        assert_eq!(
            raid_dump(&mut state, &data, &id, &krrc).unwrap_err().case,
            "27.55"
        );
    }
}
