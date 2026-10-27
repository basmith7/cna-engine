//! Rommel (31.0, R-135): his counter on the map lifts the Axis initiative
//! rating (7.1x), and at the start of each Game-Turn he is present a
//! two-dice 12 recalls him to Germany for the turn: out of play, he
//! neither moves nor can be caught, and comes back to the hex he left.

use crate::data::Data;
use crate::event::Event;
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use crate::zoc::Zoc;
use cna_data::cp::Cp;
use cna_data::initiative::AxisSituation;
use cna_data::map::HexId;
use cna_data::oa::{Commander, Nation};
use cna_data::scenario::Side;

impl GameState {
    /// Rommel's piece on a map hex, the holding boxes excepted (7.1x).
    pub(crate) fn rommel_on_map(&self) -> Option<&Piece> {
        self.pieces
            .values()
            .find(|p| p.kind == PieceKind::Commander && p.hex().is_some())
    }

    /// Rommel commanding on the map: there, and not recalled this turn.
    pub(crate) fn rommel_at(&self, hex: &HexId) -> bool {
        !self.rommel_recalled && self.rommel_on_map().and_then(Piece::hex) == Some(hex)
    }

    /// 17.28, 31.0: Axis units sharing Rommel's hex add his
    /// `stack_morale_bonus` to their adjusted morale, past +3 if need be.
    pub(crate) fn rommel_morale_bonus(&self, data: &Data, ids: &[PieceId]) -> i32 {
        let Some(c) = self.rommel_on_map().and_then(|p| commander(data, p)) else {
            return 0;
        };
        let with_him = ids.iter().any(|id| {
            let p = &self.pieces[id];
            p.side == Side::Axis && p.hex().is_some_and(|h| self.rommel_at(h))
        });
        if with_him { c.stack_morale_bonus } else { 0 }
    }

    /// The Axis initiative rating (7.1x): Rommel's row while he is on the
    /// map, his `initiative_while_recalled` while he is in Germany, else
    /// by the German combat units on the map.
    pub(crate) fn axis_initiative_rating(&self, data: &Data) -> u8 {
        if let Some(c) = self.rommel_on_map().and_then(|p| commander(data, p)) {
            if self.rommel_recalled {
                return u8::try_from(c.initiative_while_recalled).unwrap_or(0);
            }
            return data
                .initiative
                .axis(AxisSituation::RommelOnMap)
                .expect("initiative-ratings.json has every situation");
        }
        let germans = self.pieces.values().any(|p| {
            p.nation == Some(Nation::De) && p.kind == PieceKind::Combat && p.hex().is_some()
        });
        let situation = if germans {
            AxisSituation::GermanLandCombatUnitsNoRommel
        } else {
            AxisSituation::NoGermanCombatUnits
        };
        data.initiative
            .axis(situation)
            .expect("initiative-ratings.json has every situation")
    }

    /// 31.0's recall roll, at a Game-Turn's start while Rommel is on the
    /// map: two dice, and his `recall_roll` total sends him to Germany for
    /// the turn. He is back the next turn unless it comes up again.
    pub(crate) fn roll_rommel_recall(&mut self, data: &Data, events: &mut Vec<Event>) {
        self.rommel_recalled = false;
        let Some(c) = self.rommel_on_map().and_then(|p| commander(data, p)) else {
            return;
        };
        let target = c.recall_roll;
        let a = self.rng.d6();
        let b = self.rng.d6();
        for value in [a, b] {
            events.push(Event::DieRolled {
                purpose: "Rommel's recall".into(),
                value,
            });
        }
        if u32::from(a + b) == target {
            self.rommel_recalled = true;
            events.push(Event::RommelRecalled);
        }
    }

    /// R-137: as an OpStage begins, every Axis unit in the hex of a
    /// commanding Rommel borrows his `escort_cpa_bonus`; everyone else
    /// has none.
    pub(crate) fn mark_rommel_escort(&mut self, data: &Data) {
        let bonus = self
            .rommel_on_map()
            .filter(|_| !self.rommel_recalled)
            .and_then(|r| Some((r.hex()?.clone(), commander(data, r)?.escort_cpa_bonus)));
        for p in self.pieces.values_mut() {
            p.rommel_cpa = match &bonus {
                Some((hex, n))
                    if p.side == Side::Axis
                        && matches!(p.kind, PieceKind::Combat | PieceKind::Hq)
                        && p.hex() == Some(hex) =>
                {
                    *n
                }
                _ => 0,
            };
        }
    }

    /// R-137: a unit no longer in a commanding Rommel's hex loses his
    /// bonus at once and earns the DP it spared: those for CP spent past
    /// its own CPA but inside the raised one (6.21).
    pub(crate) fn part_from_rommel(&mut self, events: &mut Vec<Event>) {
        let hex = self
            .rommel_on_map()
            .filter(|_| !self.rommel_recalled)
            .and_then(Piece::hex)
            .cloned();
        let parted: Vec<PieceId> = self
            .pieces
            .values()
            .filter(|p| p.rommel_cpa > 0 && (hex.is_none() || p.hex() != hex.as_ref()))
            .map(|p| p.id.clone())
            .collect();
        for id in parted {
            let p = &self.pieces[&id];
            let over = |cpa: Option<u32>| {
                p.cp_spent
                    .saturating_sub(Cp::whole(cpa.unwrap_or(0)))
                    .floor()
            };
            let dp = over(self.base_cpa(p)) - over(self.cpa(p));
            let p = self.pieces.get_mut(&id).expect("listed");
            p.rommel_cpa = 0;
            // A unit gone from the map has no cohesion left to charge.
            if dp > 0 && p.hex().is_some() {
                p.cohesion.earn_dp(dp);
                events.push(Event::Disorganised {
                    piece: id,
                    dp,
                    level: p.cohesion.level,
                });
            }
        }
    }

    /// R-138: a commanding Rommel alone in an enemy ZOC in the enemy's
    /// phase, with no reaction left to him (`offered` lists those still
    /// offered one), is captured (10.29): removed for good, and with no
    /// strength he yields no prisoner points.
    pub(crate) fn capture_lone_rommel(
        &mut self,
        data: &Data,
        offered: &[PieceId],
        events: &mut Vec<Event>,
    ) {
        let Some(p) = self.rommel_on_map().filter(|_| !self.rommel_recalled) else {
            return;
        };
        let (id, side) = (p.id.clone(), p.side);
        let hex = p.hex().expect("on the map").clone();
        if offered.contains(&id)
            || Some(side) == self.phasing()
            || self.friendly_combat_in(&hex, side, &[])
            || self.zoc_at(data, &hex, side) != Zoc::Exerts
        {
            return;
        }
        let p = self.pieces.get_mut(&id).expect("listed");
        p.contact = false;
        p.location = Location::Eliminated;
        events.push(Event::Captured { piece: id, hex });
    }
}

fn commander<'a>(data: &'a Data, p: &Piece) -> Option<&'a Commander> {
    data.oa.unit(&p.id.unit()?)?.commander.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;
    use crate::rng::GameRng;
    use crate::state::Pending;
    use crate::state::test_support::auto_place;
    use cna_data::oa::UnitId;

    fn crusader() -> (Data, GameState) {
        let data = Data::load("operation-crusader").unwrap();
        let mut state = GameState::new(&data, 3).unwrap();
        auto_place(&data, &mut state);
        (data, state)
    }

    fn rommel() -> PieceId {
        PieceId::from(UnitId::parse("unit:de:rommel").unwrap())
    }

    #[test]
    fn rommel_on_the_map_rates_the_axis_six_and_three_when_recalled() {
        let (data, mut state) = crusader();
        assert_eq!(state.axis_initiative_rating(&data), 6);
        state.rommel_recalled = true;
        assert_eq!(state.axis_initiative_rating(&data), 3);
        state.rommel_recalled = false;
        state.pieces.get_mut(&rommel()).unwrap().location = Location::Eliminated;
        // German combat units but no Rommel.
        assert_eq!(state.axis_initiative_rating(&data), 3);
    }

    #[test]
    fn a_twelve_recalls_rommel_and_anything_else_keeps_him() {
        let (data, state) = crusader();
        let mut recalled = 0;
        for seed in 0..400 {
            let mut s = state.clone();
            s.rng = GameRng::new(seed);
            s.rommel_recalled = true;
            let mut events = Vec::new();
            s.roll_rommel_recall(&data, &mut events);
            let dice: Vec<u8> = events
                .iter()
                .filter_map(|e| match e {
                    Event::DieRolled { value, .. } => Some(*value),
                    _ => None,
                })
                .collect();
            assert_eq!(dice.len(), 2);
            let twelve = dice == [6, 6];
            assert_eq!(s.rommel_recalled, twelve, "seed {seed}");
            assert_eq!(events.contains(&Event::RommelRecalled), twelve);
            recalled += usize::from(twelve);
        }
        assert!(recalled > 0, "no 12 in 400 seeds");
    }

    #[test]
    fn no_rommel_no_roll() {
        let (data, mut state) = crusader();
        state.pieces.get_mut(&rommel()).unwrap().location = Location::Eliminated;
        let draws = state.rng.draws();
        let mut events = Vec::new();
        state.roll_rommel_recall(&data, &mut events);
        assert!(events.is_empty());
        assert_eq!(state.rng.draws(), draws);
    }

    #[test]
    fn axis_units_with_rommel_fight_a_point_braver() {
        let (data, mut state) = crusader();
        let hex = state.pieces[&rommel()].hex().unwrap().clone();
        let with: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex) && p.kind != PieceKind::Commander)
            .map(|p| p.id.clone())
            .collect();
        let away: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.hex().is_some_and(|h| h != &hex))
            .map(|p| p.id.clone())
            .take(3)
            .collect();
        assert!(!with.is_empty() && !away.is_empty());
        assert_eq!(state.rommel_morale_bonus(&data, &with), 1);
        assert_eq!(state.rommel_morale_bonus(&data, &away), 0);
        state.rommel_recalled = true;
        assert_eq!(state.rommel_morale_bonus(&data, &with), 0);
    }

    #[test]
    fn a_recalled_rommel_does_not_move() {
        let (_, mut state) = crusader();
        state.rommel_recalled = true;
        let p = state.pieces[&rommel()].clone();
        assert_eq!(state.able_to_move(&p).unwrap_err().case, "31.0");
        state.rommel_recalled = false;
        assert!(state.able_to_move(&p).is_ok());
    }

    /// The Axis units in Rommel's hex, him excepted.
    fn with_him(state: &GameState) -> Vec<PieceId> {
        let hex = state.pieces[&rommel()].hex().unwrap().clone();
        state
            .pieces
            .values()
            .filter(|p| p.hex() == Some(&hex) && p.kind != PieceKind::Commander)
            .map(|p| p.id.clone())
            .collect()
    }

    // R-137: every unit in his hex as an OpStage begins moves on CPA +5;
    // nobody else, and not Rommel himself.
    #[test]
    fn units_with_rommel_at_a_stages_start_move_five_further() {
        let (_, state) = crusader();
        let with = with_him(&state);
        assert!(!with.is_empty());
        for id in &with {
            let p = &state.pieces[id];
            assert_eq!(p.rommel_cpa, 5, "{id}");
            assert_eq!(state.cpa(p), state.base_cpa(p).map(|c| c + 5), "{id}");
        }
        let away = state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.hex().is_some() && !with.contains(&p.id))
            .unwrap();
        assert_eq!(away.rommel_cpa, 0);
        assert_eq!(state.cpa(away), state.base_cpa(away));
        assert_eq!(state.pieces[&rommel()].rommel_cpa, 0);
    }

    // R-137: a recalled Rommel gives nobody the bonus.
    #[test]
    fn a_recalled_rommel_gives_no_bonus() {
        let (data, mut state) = crusader();
        state.rommel_recalled = true;
        state.mark_rommel_escort(&data);
        assert!(state.pieces.values().all(|p| p.rommel_cpa == 0));
    }

    // R-137: parting loses the bonus at once and charges the DP it had
    // spared, one per CP spent past the unit's own CPA, at most five.
    #[test]
    fn parting_from_rommel_charges_the_spared_dp() {
        let (_, mut state) = crusader();
        let id = with_him(&state)
            .into_iter()
            .find(|id| state.base_cpa(&state.pieces[id]).is_some())
            .unwrap();
        let base = state.base_cpa(&state.pieces[&id]).unwrap();
        let elsewhere = state
            .pieces
            .values()
            .find_map(|p| {
                p.hex()
                    .filter(|h| *h != state.pieces[&rommel()].hex().unwrap())
            })
            .unwrap()
            .clone();
        let mut events = Vec::new();
        // Spent three past its own CPA, inside the raised one: no DP yet.
        state.charge(&id, Cp::whole(base + 3), false, &mut events);
        assert!(events.is_empty(), "{events:?}");
        let level = state.pieces[&id].cohesion.level;
        // Still together: nothing happens.
        state.part_from_rommel(&mut events);
        assert_eq!(state.pieces[&id].rommel_cpa, 5);
        state.pieces.get_mut(&id).unwrap().location = Location::Hex(elsewhere);
        state.part_from_rommel(&mut events);
        let p = &state.pieces[&id];
        assert_eq!(p.rommel_cpa, 0);
        assert_eq!(state.cpa(p), Some(base));
        assert_eq!(p.cohesion.level, level - 3);
        assert!(events.contains(&Event::Disorganised {
            piece: id.clone(),
            dp: 3,
            level: level - 3,
        }));
        // Eliminated with him, a unit just loses the bonus.
        let other = with_him(&state)
            .into_iter()
            .find(|q| state.pieces[q].rommel_cpa > 0)
            .unwrap();
        state.charge(&other, Cp::whole(30), false, &mut Vec::new());
        state.pieces.get_mut(&other).unwrap().location = Location::Eliminated;
        let mut events = Vec::new();
        state.part_from_rommel(&mut events);
        assert_eq!(state.pieces[&other].rommel_cpa, 0);
        assert!(events.is_empty(), "{events:?}");
    }

    // R-137: 8.17's cap and whether a unit is motorised go by its own CPA.
    #[test]
    fn the_foot_cap_and_mobility_ignore_the_bonus() {
        let (data, mut state) = crusader();
        let foot = state
            .pieces
            .values()
            .find(|p| p.kind == PieceKind::Combat && state.base_cpa(p) == Some(10))
            .unwrap()
            .id
            .clone();
        state.pieces.get_mut(&foot).unwrap().rommel_cpa = 5;
        let p = &state.pieces[&foot];
        assert_eq!(state.cpa(p), Some(15));
        assert_eq!(
            state.mobility(&data, p),
            cna_rules::movement::Mobility::Foot
        );
        let (cap, case, _) = state.voluntary_cap(&foot).unwrap();
        assert_eq!((cap, case), (Cp::whole(15), "8.1x"));
    }

    // 31.0: Rommel may react when an enemy unit comes next to him, like
    // a motorised combat unit; not while he is in Germany.
    #[test]
    fn rommel_is_offered_a_reaction() {
        let (data, mut state) = crusader();
        let at = state.pieces[&rommel()].hex().unwrap().clone();
        let (enemy, next) = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::Combat && p.hex().is_some())
            .find_map(|p| {
                let free = data
                    .map
                    .neighbours(&at)
                    .into_iter()
                    .find(|n| state.pieces.values().all(|q| q.hex() != Some(n)))?;
                Some((p.id.clone(), free))
            })
            .unwrap();
        let was = state.pieces[&enemy].hex().unwrap().clone();
        state.pieces.get_mut(&enemy).unwrap().location = Location::Hex(next.clone());
        let mover = std::slice::from_ref(&enemy);
        let offered = state.reactors_next_to(&data, &next, &was, mover, false);
        assert!(offered.contains(&rommel()), "{offered:?}");
        state.rommel_recalled = true;
        let offered = state.reactors_next_to(&data, &next, &was, mover, false);
        assert!(!offered.contains(&rommel()), "{offered:?}");
    }

    // 31.0: Rommel retreating alone goes through enemy ZOC freely, where
    // a move would stop him at the first hex (10.29).
    #[test]
    fn rommel_retreats_alone_through_enemy_zoc() {
        use crate::reach::Purpose;
        let (data, mut state) = crusader();
        let clear = |h: &HexId| {
            data.map
                .hex(h)
                .is_some_and(|x| x.terrain.name() == "clear" || x.terrain.name() == "gravel")
        };
        let empty = |s: &GameState, h: &HexId| {
            data.map
                .within(h, 3)
                .iter()
                .all(|n| s.pieces.values().all(|p| p.hex() != Some(n)))
        };
        // E holds an enemy unit; A is two hexes from it; B and C are next
        // to both E and each other, B next to A: A → B → C runs through
        // E's ZOC.
        let (e, a, b, c) = data
            .map
            .hexes()
            .map(|h| h.id.clone())
            .filter(|e| clear(e) && empty(&state, e))
            .find_map(|e| {
                let ring = data.map.neighbours(&e);
                if ring.len() < 6 || !ring.iter().all(&clear) {
                    return None;
                }
                let b = ring[0].clone();
                let c = ring.iter().find(|c| data.map.adjacent(c, &b))?.clone();
                let a = data.map.neighbours(&b).into_iter().find(|a| {
                    clear(a) && !ring.contains(a) && a != &e && !data.map.adjacent(a, &c)
                })?;
                Some((e, a, b, c))
            })
            .expect("open desert");
        state.zoc.insert(e.clone(), true);
        // A Commonwealth stack that exerts a ZOC moves to E whole.
        let src = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw)
            .filter_map(|p| p.hex().cloned())
            .find(|h| state.could_exert(&data, h))
            .expect("a Commonwealth stack exerting a ZOC");
        for p in state.pieces.values_mut() {
            if p.hex() == Some(&src) {
                p.location = Location::Hex(e.clone());
            }
        }
        for h in [&b, &c] {
            assert_eq!(
                state.zoc_at(&data, h, Side::Axis),
                crate::zoc::Zoc::Exerts,
                "{h}"
            );
        }

        state.pieces.get_mut(&rommel()).unwrap().location = Location::Hex(a.clone());
        let me = [rommel()];
        let path = [b.clone(), c.clone()];
        let refused = state
            .plan_with(&data, &me, &a, &path, Purpose::Move)
            .unwrap_err();
        assert_eq!(refused.case, "10.29");
        for purpose in [Purpose::RetreatBeforeAssault, Purpose::ForcedRetreat] {
            let plan = state.plan_with(&data, &me, &a, &path, purpose).unwrap();
            assert_eq!(plan.steps.len(), 2, "{purpose:?}");
            assert!(!plan.ends_in_zoc && !plan.break_off, "{purpose:?}");
        }
        state.rommel_recalled = true;
        assert!(
            state
                .plan_with(&data, &me, &a, &path, Purpose::RetreatBeforeAssault)
                .is_err()
        );
    }

    /// R-138's scene: the Commonwealth moving in Operation Crusader,
    /// Rommel alone at R in open desert, and a Commonwealth stack that
    /// exerts a ZOC two hexes off at S, N between them; Q is next to R
    /// and out of N's reach. The stack moving S → N puts him in its ZOC.
    struct Scene {
        data: Data,
        state: GameState,
        stack: Vec<PieceId>,
        n: HexId,
        q: HexId,
    }

    fn lone_rommel_scene() -> Scene {
        let data = Data::load("operation-crusader").unwrap();
        let mut state = GameState::new(&data, 3).unwrap();
        auto_place(&data, &mut state);
        // Pass to the Commonwealth's phase G movement segment.
        while !(state.clock.phase == crate::clock::Phase::MovementCombat
            && state.clock.segment == Some(crate::clock::Segment::Movement)
            && state.phasing() == Some(Side::Cw)
            && state.pending.is_none())
        {
            let player = state.to_act().unwrap();
            let order = match &state.pending {
                Some(Pending::DeclareZoc { .. }) => Order::DeclareZoc { exerting: vec![] },
                _ if state.clock.phase == crate::clock::Phase::InitiativeDeclaration => {
                    Order::DeclareInitiative { player_a: Side::Cw }
                }
                _ => Order::Pass,
            };
            state.apply(&data, player, order).unwrap();
        }
        let clear = |h: &HexId| {
            data.map
                .hex(h)
                .is_some_and(|x| x.terrain.name() == "clear" || x.terrain.name() == "gravel")
        };
        let empty = |s: &GameState, h: &HexId| {
            data.map
                .within(h, 4)
                .iter()
                .all(|n| s.pieces.values().all(|p| p.hex() != Some(n)))
        };
        let (r, n, s, q) = data
            .map
            .hexes()
            .map(|h| h.id.clone())
            // Map C, east of Marble Arch where the Commonwealth may go (8.1x).
            .filter(|r| r.to_string().starts_with('C') && clear(r) && empty(&state, r))
            .find_map(|r| {
                let ring = data.map.neighbours(&r);
                if ring.len() < 6 || !ring.iter().all(&clear) {
                    return None;
                }
                let n = ring[0].clone();
                let s = data
                    .map
                    .neighbours(&n)
                    .into_iter()
                    .find(|s| clear(s) && s != &r && !ring.contains(s))?;
                let q = ring
                    .iter()
                    .find(|q| *q != &n && !data.map.adjacent(q, &n))?
                    .clone();
                Some((r, n, s, q))
            })
            .expect("open desert");
        let src = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::Combat)
            .filter(|p| state.mobility(&data, p).motorised())
            .filter_map(|p| p.hex().cloned())
            .find(|h| state.could_exert(&data, h))
            .expect("a motorised Commonwealth stack exerting a ZOC");
        let mut stack = Vec::new();
        for p in state.pieces.values_mut() {
            if p.hex() == Some(&src) {
                p.location = Location::Hex(s.clone());
                stack.push(p.id.clone());
            }
        }
        state.pieces.get_mut(&rommel()).unwrap().location = Location::Hex(r);
        Scene {
            data,
            state,
            stack,
            n,
            q,
        }
    }

    fn approach(scene: &mut Scene) -> Vec<Event> {
        let order = Order::Move {
            pieces: scene.stack.clone(),
            path: vec![scene.n.clone()],
            assault: false,
            fuel_from: None,
        };
        scene.state.apply(&scene.data, Side::Cw, order).unwrap()
    }

    fn captured(events: &[Event]) -> bool {
        events
            .iter()
            .any(|e| matches!(e, Event::Captured { piece, .. } if piece == &rommel()))
    }

    // R-138: an enemy combat unit coming next to a lone Rommel offers him
    // his reaction first; passing it leaves him alone in its ZOC, and
    // 10.29 takes him, for good.
    #[test]
    fn a_lone_rommel_who_passes_his_reaction_is_captured() {
        let mut scene = lone_rommel_scene();
        let events = approach(&mut scene);
        assert!(!captured(&events), "{events:?}");
        assert!(
            matches!(&scene.state.pending, Some(Pending::Reaction { eligible, .. }) if eligible.contains(&rommel())),
            "{:?}",
            scene.state.pending
        );
        let events = scene
            .state
            .apply(&scene.data, Side::Axis, Order::Pass)
            .unwrap();
        assert!(captured(&events), "{events:?}");
        assert_eq!(scene.state.pieces[&rommel()].location, Location::Eliminated);
    }

    // R-138: reacting out of the ZOC saves him.
    #[test]
    fn a_lone_rommel_who_reacts_away_escapes() {
        let mut scene = lone_rommel_scene();
        approach(&mut scene);
        let order = Order::React {
            pieces: vec![rommel()],
            path: vec![scene.q.clone()],
        };
        let events = scene.state.apply(&scene.data, Side::Axis, order).unwrap();
        assert!(!captured(&events), "{events:?}");
        assert!(
            events.contains(&Event::ContactBroken {
                piece: rommel(),
                cost: Cp::whole(2),
            }),
            "{events:?}"
        );
        assert_eq!(
            scene.state.pieces[&rommel()].location,
            Location::Hex(scene.q.clone())
        );
    }

    // R-138: a lone Rommel who may not react (here in contact already,
    // 8.5x) is captured the moment the enemy's move puts him in its ZOC.
    #[test]
    fn a_lone_rommel_who_may_not_react_is_captured_on_contact() {
        let mut scene = lone_rommel_scene();
        scene.state.pieces.get_mut(&rommel()).unwrap().contact = true;
        let events = approach(&mut scene);
        assert!(captured(&events), "{events:?}");
        assert_eq!(scene.state.pieces[&rommel()].location, Location::Eliminated);
    }

    // R-138: with a friendly combat unit beside him he is never captured
    // this way; he may react along with it.
    #[test]
    fn rommel_stacked_with_a_combat_unit_is_not_captured() {
        let mut scene = lone_rommel_scene();
        let r = scene.state.pieces[&rommel()].hex().unwrap().clone();
        let friend = scene
            .state
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat && p.hex().is_some())
            .map(|p| p.id.clone())
            .unwrap();
        scene.state.pieces.get_mut(&friend).unwrap().location = Location::Hex(r.clone());
        scene.state.pieces.get_mut(&rommel()).unwrap().contact = true;
        let mut events = approach(&mut scene);
        if scene.state.pending.is_some() {
            events.extend(
                scene
                    .state
                    .apply(&scene.data, Side::Axis, Order::Pass)
                    .unwrap(),
            );
        }
        assert!(!captured(&events), "{events:?}");
        assert_eq!(scene.state.pieces[&rommel()].location, Location::Hex(r));
    }

    // R-138: still alone in an enemy ZOC when the retreat-before-assault
    // step closes, he is taken.
    #[test]
    fn a_lone_rommel_who_does_not_retreat_before_assault_is_captured() {
        let mut scene = lone_rommel_scene();
        for id in &scene.stack {
            scene.state.pieces.get_mut(id).unwrap().location = Location::Hex(scene.n.clone());
        }
        scene.state.clock.segment = Some(crate::clock::Segment::RetreatBeforeAssault);
        scene.state.pending = Some(Pending::RetreatBeforeAssault { side: Side::Axis });
        let events = scene
            .state
            .apply(&scene.data, Side::Axis, Order::Pass)
            .unwrap();
        assert!(captured(&events), "{events:?}");
        assert_eq!(scene.state.pieces[&rommel()].location, Location::Eliminated);
    }
}
