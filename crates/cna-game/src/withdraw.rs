//! Commonwealth withdrawals (20.8x, 4.43): under the `withdrawals`
//! module, each `withdraws` row due in an OpStage is checked once, after
//! that stage's last phase (R-155). A named row takes its counter and
//! every unit under it on its OA sheet still in play, less the row's
//! `less` units; an `hq_only` counter goes alone; a `pick` adds units of
//! a slot kind the Commonwealth chooses (R-154). A withdrawal leaves when
//! every unit in it stands in Cairo or Alexandria and together they hold
//! 75 % of their combined maximum TOE, a picked unit tested alone
//! (R-156), and, under §32, the row's motorisation points in Cairo or
//! Alexandria go with it (R-157). Otherwise every unit in it is
//! eliminated. A schedule row that brings a withdrawn unit back brings
//! it as it left (`arrival.rs`), and with `returning_transport` on, a
//! return printed "Tpt: See ..." brings back the motorisation points
//! that left on the withdrawals it names (R-157).

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::oa::{Oa, UnitId};
use cna_data::scenario::Side;
use cna_data::schedule::{Pick, Row};
use cna_data::time::GameTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A withdrawal pick the Commonwealth may make now (20.82, R-154): what
/// `Order::PickWithdrawal` accepts. `candidates` are in the order the
/// engine would take them if no pick is made (in Cairo or Alexandria
/// first, then the strongest), so their first `count` are its choice as
/// things stand; `picked` is the pick already made this OpStage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PickOption {
    pub fills: String,
    pub count: u32,
    pub candidates: Vec<PieceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picked: Option<Vec<PieceId>>,
}

/// Motorisation points that left with an OpStage's withdrawals (R-157),
/// kept until a return that names that OpStage brings them back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportOut {
    pub at: GameTime,
    pub points: u32,
}

/// The places a withdrawal leaves from (20.81).
const PLACES: [&str; 3] = ["cairo", "alexandria-east", "alexandria-west"];

/// A unit's TOE and its maximum, for the 75 % test (20.82).
fn ready(toe: u32, max: u32) -> bool {
    toe * 4 >= max * 3
}

/// The OpStage before `at`, if there is one.
fn previous(at: GameTime) -> Option<GameTime> {
    match (at.game_turn, at.opstage) {
        (1, 1) => None,
        (gt, 1) => Some(GameTime::new(gt - 1, 3)),
        (gt, st) => Some(GameTime::new(gt, st - 1)),
    }
}

fn subtree(oa: &Oa, head: &UnitId, less: &[UnitId], out: &mut BTreeSet<UnitId>) {
    if less.contains(head) || !out.insert(head.clone()) {
        return;
    }
    for c in oa.children(head) {
        subtree(oa, c, less, out);
    }
}

impl GameState {
    pub(crate) fn withdrawal_hexes(data: &Data) -> Vec<Location> {
        PLACES
            .iter()
            .map(|p| {
                Location::Hex(
                    data.map
                        .place(p)
                        .expect("places.json has Cairo and Alexandria")
                        .hex
                        .clone(),
                )
            })
            .collect()
    }

    /// The pieces a row names, by its OA sheets (R-154): still in play.
    pub(crate) fn withdrawal_members(&self, data: &Data, row: &Row) -> Vec<PieceId> {
        let mut units = BTreeSet::new();
        for head in &row.units {
            if row.hq_only.contains(head) {
                units.insert(head.clone());
            } else {
                subtree(&data.oa, head, &row.less, &mut units);
            }
        }
        self.pieces
            .values()
            .filter(|p| p.side == row.side && p.location != Location::Eliminated)
            .filter(|p| p.id.unit().is_some_and(|u| units.contains(&u)))
            .map(|p| p.id.clone())
            .collect()
    }

    /// Commonwealth units in play that fill `kind` (R-142), outside `not`.
    fn pick_candidates(&self, data: &Data, kind: &str, not: &[PieceId]) -> Vec<&Piece> {
        self.pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.location != Location::Eliminated)
            .filter(|p| !not.contains(&p.id))
            .filter(|p| {
                p.id.unit()
                    .and_then(|u| data.oa.unit(&u))
                    .is_some_and(|u| u.fills.iter().any(|f| f == kind))
            })
            .collect()
    }

    /// TOE and maximum TOE summed over the units in `ids` (R-156).
    fn strength(&self, data: &Data, ids: &[PieceId]) -> (u32, u32) {
        ids.iter()
            .map(|id| &self.pieces[id])
            .filter(|p| matches!(p.kind, PieceKind::Combat | PieceKind::Hq))
            .fold((0, 0), |(t, m), p| {
                (t + p.toe, m + self.toe_ceiling(data, p))
            })
    }

    /// Strength per mille, for ranking picked units.
    fn permille(&self, data: &Data, p: &Piece) -> u32 {
        let max = self.toe_ceiling(data, p);
        (p.toe * 1000).checked_div(max).unwrap_or(1000)
    }

    /// The picks `player` may make now, for the browser: the
    /// Commonwealth's alone, while it is to act with nothing pending in
    /// an OpStage with a `pick` row, and only where it has a choice (more
    /// units of the kind than the row takes).
    pub fn withdrawal_pick_options(&self, data: &Data, player: Side) -> Vec<PickOption> {
        if !self.modules.withdrawals
            || player != Side::Cw
            || self.to_act() != Some(player)
            || self.pending_refusal().is_some()
        {
            return Vec::new();
        }
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        data.schedule
            .withdrawals(Side::Cw, now)
            .filter_map(|row| {
                let pick = row.pick.as_ref()?;
                let named = self.withdrawal_members(data, row);
                let candidates = self.auto_pick_from(data, pick, &named, usize::MAX);
                (candidates.len() > pick.count as usize).then(|| PickOption {
                    fills: pick.fills.clone(),
                    count: pick.count,
                    candidates,
                    picked: self.withdrawal_picks.get(&pick.fills).cloned(),
                })
            })
            .collect()
    }

    /// The units a `pick` sends when the player chose none: those in
    /// Cairo or Alexandria first, then the strongest, then by id.
    fn auto_pick(&self, data: &Data, pick: &Pick, named: &[PieceId]) -> Vec<PieceId> {
        self.auto_pick_from(data, pick, named, pick.count as usize)
    }

    /// The first `n` candidates for `pick`, in `auto_pick`'s order.
    fn auto_pick_from(
        &self,
        data: &Data,
        pick: &Pick,
        named: &[PieceId],
        n: usize,
    ) -> Vec<PieceId> {
        let places = Self::withdrawal_hexes(data);
        let mut c = self.pick_candidates(data, &pick.fills, named);
        c.sort_by_key(|p| {
            (
                !places.contains(&p.location),
                std::cmp::Reverse(self.permille(data, p)),
                p.id.clone(),
            )
        });
        c.into_iter().take(n).map(|p| p.id.clone()).collect()
    }

    /// `Order::PickWithdrawal` (20.82, R-154).
    pub(crate) fn pick_withdrawal(
        &mut self,
        data: &Data,
        player: Side,
        fills: &str,
        pieces: &[PieceId],
    ) -> Result<Vec<Event>, Illegal> {
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        let row = data
            .schedule
            .withdrawals(Side::Cw, now)
            .find(|r| r.pick.as_ref().is_some_and(|p| p.fills == fills))
            .filter(|_| self.modules.withdrawals && player == Side::Cw)
            .ok_or_else(|| {
                Illegal::new(
                    "20.82",
                    format!("no withdrawal at {now} lets {player} pick {fills} units"),
                )
            })?;
        let pick = row.pick.as_ref().expect("found by its pick");
        let distinct: BTreeSet<&PieceId> = pieces.iter().collect();
        if distinct.len() != pieces.len() || pieces.len() != pick.count as usize {
            return Err(Illegal::new(
                "20.82",
                format!(
                    "the withdrawal takes {} different {fills} units",
                    pick.count
                ),
            ));
        }
        let named = self.withdrawal_members(data, row);
        let fit: BTreeSet<&PieceId> = self
            .pick_candidates(data, fills, &named)
            .into_iter()
            .map(|p| &p.id)
            .collect();
        if let Some(bad) = pieces.iter().find(|id| !fit.contains(id)) {
            return Err(Illegal::new(
                "20.82",
                format!("{bad} is not a Commonwealth {fills} unit in play outside the row"),
            ));
        }
        self.withdrawal_picks
            .insert(fills.to_owned(), pieces.to_vec());
        Ok(vec![Event::WithdrawalPicked {
            fills: fills.to_owned(),
            pieces: pieces.to_vec(),
        }])
    }

    /// At phase A: the withdrawals of the OpStage just ended (R-155).
    pub(crate) fn withdraw_after_stage(&mut self, data: &Data, events: &mut Vec<Event>) {
        if !self.modules.withdrawals {
            return;
        }
        let picks = std::mem::take(&mut self.withdrawal_picks);
        let Some(at) = previous(GameTime::new(self.clock.game_turn, self.clock.stage))
            .filter(|at| *at >= data.scenario.start)
        else {
            return;
        };
        for row in data.schedule.withdrawals(Side::Cw, at) {
            self.check_withdrawal(
                data,
                row,
                picks.get(row.pick.as_ref().map_or("", |p| &p.fills)),
                events,
            );
        }
    }

    /// One row's check (R-155–R-157).
    pub(crate) fn check_withdrawal(
        &mut self,
        data: &Data,
        row: &Row,
        picked: Option<&Vec<PieceId>>,
        events: &mut Vec<Event>,
    ) {
        let places = Self::withdrawal_hexes(data);
        let named = self.withdrawal_members(data, row);
        // The named units go as one; each picked unit alone (R-156).
        let mut groups: Vec<(Vec<PieceId>, bool)> = Vec::new();
        if !named.is_empty() {
            groups.push((named.clone(), false));
        }
        if let Some(pick) = &row.pick {
            let chosen = picked
                .filter(|ids| {
                    ids.iter().all(|id| {
                        self.pieces
                            .get(id)
                            .is_some_and(|p| p.location != Location::Eliminated)
                    })
                })
                .cloned()
                .unwrap_or_else(|| self.auto_pick(data, pick, &named));
            groups.extend(chosen.into_iter().map(|id| (vec![id], true)));
        }
        if groups.is_empty() {
            return;
        }
        // Under §32 the row's motorisation points go from Cairo or
        // Alexandria with it (R-157).
        let needed = match (self.modules.abstract_supply, row.transport) {
            (true, Some(t)) => t.motorisation,
            _ => 0,
        };
        let keys: Vec<String> = places.iter().filter_map(Location::key).collect();
        let held: u32 = self
            .motorisation
            .get(&Side::Cw)
            .map_or(0, |m| keys.iter().filter_map(|k| m.get(k)).sum());
        let transported = held >= needed;
        let mut taken = false;
        for (ids, alone) in groups {
            let reason = if !ids
                .iter()
                .all(|id| places.contains(&self.pieces[id].location))
            {
                Some("not in Cairo or Alexandria".to_owned())
            } else if !self.strong_enough(data, &ids, alone, row, &named) {
                Some("under 75 % of its TOE".to_owned())
            } else if !transported {
                Some(format!("short of {needed} motorisation points"))
            } else {
                None
            };
            for id in &ids {
                self.release_attached(id);
            }
            match reason {
                Some(reason) => {
                    for id in &ids {
                        self.pieces.get_mut(id).expect("a piece").location = Location::Eliminated;
                    }
                    events.push(Event::WithdrawalFailed {
                        pieces: ids,
                        reason,
                    });
                }
                None => {
                    let motorisation = if taken { 0 } else { needed };
                    if !taken {
                        self.take_motorisation(&keys, needed);
                        taken = true;
                        if self.modules.returning_transport && needed > 0 {
                            self.transport_out.push(TransportOut {
                                at: row.at,
                                points: needed,
                            });
                        }
                    }
                    for id in &ids {
                        let p = self.pieces.remove(id).expect("a piece");
                        self.withdrawn.insert(id.clone(), p);
                    }
                    events.push(Event::Withdrawn {
                        pieces: ids,
                        motorisation,
                    });
                }
            }
        }
    }

    /// 75 % of the combined maximum TOE (20.82, R-156); a picked unit
    /// also serves when it is the strongest of its kind and none is at
    /// 75 %.
    fn strong_enough(
        &self,
        data: &Data,
        ids: &[PieceId],
        alone: bool,
        row: &Row,
        named: &[PieceId],
    ) -> bool {
        let (toe, max) = self.strength(data, ids);
        if ready(toe, max) {
            return true;
        }
        let Some(pick) = row.pick.as_ref().filter(|_| alone) else {
            return false;
        };
        let mine = self.permille(data, &self.pieces[&ids[0]]);
        let others = self.pick_candidates(data, &pick.fills, named);
        others
            .iter()
            .all(|p| !ready(p.toe, self.toe_ceiling(data, p)) && self.permille(data, p) <= mine)
    }

    /// Takes `n` Commonwealth motorisation points from `keys`, in order.
    fn take_motorisation(&mut self, keys: &[String], mut n: u32) {
        let Some(m) = self.motorisation.get_mut(&Side::Cw) else {
            return;
        };
        for k in keys {
            if n == 0 {
                break;
            }
            if let Some(have) = m.get_mut(k) {
                let t = n.min(*have);
                *have -= t;
                n -= t;
                if *have == 0 {
                    m.remove(k);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;
    use crate::setup;
    use crate::state::Modules;

    fn italian() -> Data {
        Data::load("italian-campaign").unwrap()
    }

    fn game(data: &Data) -> GameState {
        let mut s = GameState::with_modules(data, 7, Modules::ALL).unwrap();
        crate::state::test_support::auto_place(data, &mut s);
        s
    }

    fn row(data: &Data, gt: u8, st: u8) -> Row {
        data.schedule
            .withdrawals(Side::Cw, GameTime::new(gt, st))
            .next()
            .unwrap()
            .clone()
    }

    fn cairo(data: &Data) -> Location {
        Location::Hex(data.map.place("cairo").unwrap().hex.clone())
    }

    /// Puts `head`'s sheet in play at `at`, replacing what is there.
    fn put(state: &mut GameState, data: &Data, head: &str, at: Location) -> Vec<PieceId> {
        let head = UnitId::parse(head).unwrap();
        let pieces = setup::reinforcement(
            Side::Cw,
            &head,
            &data.oa,
            &data.chars,
            GameTime::new(13, 3),
            at,
        )
        .unwrap();
        pieces
            .into_iter()
            .map(|p| {
                let id = p.id.clone();
                state.pieces.insert(id.clone(), p);
                id
            })
            .collect()
    }

    fn motorise(state: &mut GameState, data: &Data, n: u32) {
        let key = cairo(data).key().unwrap();
        state.motorisation.insert(Side::Cw, [(key, n)].into());
    }

    fn check(
        state: &mut GameState,
        data: &Data,
        r: &Row,
        picked: Option<Vec<PieceId>>,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        state.check_withdrawal(data, r, picked.as_ref(), &mut events);
        events
    }

    #[test]
    fn a_ready_brigade_in_cairo_leaves_with_its_motorisation() {
        let data = italian();
        let mut state = game(&data);
        let r = row(&data, 13, 3);
        let ids = put(&mut state, &data, "unit:cw:5-in-hq", cairo(&data));
        assert!(ids.len() > 1, "the HQ and its sheet");
        motorise(&mut state, &data, 12);
        let events = check(&mut state, &data, &r, None);
        let Event::Withdrawn {
            pieces,
            motorisation,
        } = &events[0]
        else {
            panic!("{events:?}");
        };
        assert_eq!(*motorisation, 10);
        let mut want = ids.clone();
        want.sort();
        assert_eq!(*pieces, want);
        for id in &ids {
            assert!(!state.pieces.contains_key(id));
            assert!(state.withdrawn.contains_key(id));
        }
        let key = cairo(&data).key().unwrap();
        assert_eq!(state.motorisation[&Side::Cw][&key], 2);
    }

    #[test]
    fn under_strength_or_away_from_cairo_or_unmotorised_is_eliminated() {
        let data = italian();
        for case in ["weak", "away", "trucks"] {
            let mut state = game(&data);
            let r = row(&data, 13, 3);
            let at = if case == "away" {
                Location::Hex(data.map.place("alexandria-east").unwrap().hex.clone())
            } else {
                cairo(&data)
            };
            let ids = put(&mut state, &data, "unit:cw:5-in-hq", at);
            if case == "away" {
                // One unit elsewhere fails the whole brigade.
                state.pieces.get_mut(&ids[1]).unwrap().location =
                    Location::Box(cna_data::scenario::OffMapBox::Tripoli);
            }
            if case == "weak" {
                for id in &ids {
                    let p = state.pieces.get_mut(id).unwrap();
                    p.toe /= 2;
                }
            }
            motorise(&mut state, &data, if case == "trucks" { 9 } else { 10 });
            let events = check(&mut state, &data, &r, None);
            assert!(
                matches!(&events[0], Event::WithdrawalFailed { pieces, .. } if pieces.len() == ids.len()),
                "{case}: {events:?}"
            );
            for id in &ids {
                assert_eq!(state.pieces[id].location, Location::Eliminated, "{case}");
            }
            assert!(state.withdrawn.is_empty());
        }
    }

    #[test]
    fn hq_only_goes_alone_and_less_units_stay() {
        let data = Data::load("campaign-game").unwrap();
        let mut state = game(&data);
        // GT36/3: the 70th Division's HQ alone, the 16th Brigade whole.
        let r = row(&data, 36, 3);
        let div = put(
            &mut state,
            &data,
            "unit:cw:70th-infantry-div-hq",
            cairo(&data),
        );
        let members = state.withdrawal_members(&data, &r);
        let head = PieceId::from("unit:cw:70th-infantry-div-hq".to_owned());
        assert!(members.contains(&head));
        let bde = UnitId::parse("unit:cw:16th-infantry-bde-hq").unwrap();
        let mut under16 = BTreeSet::new();
        subtree(&data.oa, &bde, &[], &mut under16);
        for id in &div {
            let unit = id.unit().unwrap();
            assert_eq!(
                members.contains(id),
                *id == head || under16.contains(&unit),
                "{id}"
            );
        }
        // GT15/2: the 4th Indian Division less its 5th and 7th Brigades.
        let r = row(&data, 15, 2);
        let mut state = game(&data);
        let div = put(&mut state, &data, "unit:cw:4-in-hq", cairo(&data));
        let members = state.withdrawal_members(&data, &r);
        let mut left = BTreeSet::new();
        for b in ["unit:cw:5-in-hq", "unit:cw:7-in-hq"] {
            subtree(&data.oa, &UnitId::parse(b).unwrap(), &[], &mut left);
        }
        assert!(!left.is_empty());
        assert!(members.len() < div.len());
        for id in &div {
            assert_eq!(
                members.contains(id),
                !left.contains(&id.unit().unwrap()),
                "{id}"
            );
        }
    }

    /// GT97/2's row: the 22nd Guards Brigade HQ and 3 infantry battalions.
    fn guards() -> (Data, GameState, Row) {
        let data = Data::load("campaign-game").unwrap();
        let mut state = game(&data);
        state.clock.game_turn = 97;
        state.clock.stage = 2;
        let r = row(&data, 97, 2);
        put(&mut state, &data, "unit:cw:22-gds", cairo(&data));
        motorise(&mut state, &data, 21);
        (data, state, r)
    }

    fn battalions(state: &GameState, data: &Data, r: &Row) -> Vec<PieceId> {
        let named = state.withdrawal_members(data, r);
        state
            .pick_candidates(data, "infantry-battalion", &named)
            .iter()
            .map(|p| p.id.clone())
            .collect()
    }

    /// Plays on until the Commonwealth is to act, still in GT97/2.
    fn to_cw(state: &mut GameState, data: &Data) {
        while state.to_act() != Some(Side::Cw) {
            let who = state.to_act().unwrap();
            let order = if state.clock.phase == crate::clock::Phase::InitiativeDeclaration {
                Order::DeclareInitiative { player_a: Side::Cw }
            } else {
                Order::Pass
            };
            state.apply(data, who, order).unwrap();
        }
        assert_eq!((state.clock.game_turn, state.clock.stage), (97, 2));
    }

    #[test]
    fn the_pick_is_offered_to_the_commonwealth_alone_while_it_acts() {
        let (data, mut state, r) = guards();
        let view = |state: &GameState, side| {
            crate::view::view_for(
                state,
                &data,
                &crate::view::Aliases::new([0; 32]),
                Some(side),
                false,
            )
            .unwrap()
        };
        to_cw(&mut state, &data);
        let offered = state.withdrawal_pick_options(&data, Side::Cw);
        assert_eq!(offered.len(), 1, "{offered:?}");
        let o = &offered[0];
        assert_eq!(
            (o.fills.as_str(), o.count, o.picked.as_ref()),
            ("infantry-battalion", 3, None)
        );
        // Every battalion outside the row, the engine's own choice first.
        let mut all = battalions(&state, &data, &r);
        all.sort();
        let mut listed = o.candidates.clone();
        listed.sort();
        assert_eq!(listed, all);
        let named = state.withdrawal_members(&data, &r);
        let pick = data
            .schedule
            .withdrawals(Side::Cw, GameTime::new(97, 2))
            .next()
            .unwrap()
            .pick
            .clone()
            .unwrap();
        assert_eq!(o.candidates[..3], state.auto_pick(&data, &pick, &named)[..]);
        // Never to the Axis, which sees neither the offer nor the pick.
        assert!(state.withdrawal_pick_options(&data, Side::Axis).is_empty());
        assert!(
            view(&state, Side::Axis)
                .get("withdrawal_pick_options")
                .is_none()
        );
        let shown = view(&state, Side::Cw);
        assert_eq!(shown["withdrawal_pick_options"][0]["count"], 3);
        // A pick made is shown back to the Commonwealth.
        let three = o.candidates[o.candidates.len() - 3..].to_vec();
        state
            .apply(
                &data,
                Side::Cw,
                Order::PickWithdrawal {
                    fills: "infantry-battalion".into(),
                    pieces: three.clone(),
                },
            )
            .unwrap();
        let offered = state.withdrawal_pick_options(&data, Side::Cw);
        assert_eq!(offered[0].picked.as_ref(), Some(&three));
        assert!(view(&state, Side::Axis).get("withdrawal_picks").is_none());
        // Not while the Axis acts, nor in another OpStage, nor off the module.
        let (data, mut state, _) = guards();
        assert_eq!(state.to_act(), Some(Side::Axis));
        assert!(state.withdrawal_pick_options(&data, Side::Cw).is_empty());
        to_cw(&mut state, &data);
        state.clock.stage = 1;
        assert!(state.withdrawal_pick_options(&data, Side::Cw).is_empty());
        state.clock.stage = 2;
        state.modules.withdrawals = false;
        assert!(state.withdrawal_pick_options(&data, Side::Cw).is_empty());
    }

    #[test]
    fn a_pick_names_battalions_of_the_kind_and_each_is_tested_alone() {
        let (data, mut state, r) = guards();
        let bns = battalions(&state, &data, &r);
        assert!(bns.len() >= 4, "{bns:?}");
        let three: Vec<PieceId> = bns[..3].to_vec();
        let pick = |pieces: Vec<PieceId>| Order::PickWithdrawal {
            fills: "infantry-battalion".into(),
            pieces,
        };
        to_cw(&mut state, &data);
        // Illegal: two of them, one twice, the Axis, the HQ itself.
        assert!(
            state
                .apply(&data, Side::Cw, pick(bns[..2].to_vec()))
                .is_err()
        );
        let twice = vec![bns[0].clone(), bns[0].clone(), bns[1].clone()];
        assert!(state.apply(&data, Side::Cw, pick(twice)).is_err());
        assert!(state.apply(&data, Side::Axis, pick(three.clone())).is_err());
        let hq = PieceId::from("unit:cw:22-gds".to_owned());
        let with_hq = vec![hq, bns[0].clone(), bns[1].clone()];
        assert!(state.apply(&data, Side::Cw, pick(with_hq)).is_err());
        // Legal.
        let events = state.apply(&data, Side::Cw, pick(three.clone())).unwrap();
        assert!(matches!(&events[0], Event::WithdrawalPicked { pieces, .. } if *pieces == three));
        // The picked battalions go to Cairo; one is under strength.
        for id in &three {
            state.pieces.get_mut(id).unwrap().location = cairo(&data);
        }
        let weak = three[2].clone();
        state.pieces.get_mut(&weak).unwrap().toe = 0;
        let picked = state.withdrawal_picks.get("infantry-battalion").cloned();
        let events = check(&mut state, &data, &r, picked);
        let left: Vec<&PieceId> = events
            .iter()
            .flat_map(|e| match e {
                Event::Withdrawn { pieces, .. } => pieces.iter().collect(),
                _ => vec![],
            })
            .collect();
        assert_eq!(left.len(), 3, "the HQ and two battalions: {events:?}");
        assert!(events.iter().any(
            |e| matches!(e, Event::WithdrawalFailed { pieces, .. } if *pieces == vec![weak.clone()])
        ));
        assert_eq!(state.pieces[&weak].location, Location::Eliminated);
    }

    #[test]
    fn the_check_comes_after_the_stage_and_is_deterministic() {
        let data = italian();
        let run = || {
            let mut state = game(&data);
            state.clock.game_turn = 13;
            state.clock.stage = 3;
            let ids = put(&mut state, &data, "unit:cw:5-in-hq", cairo(&data));
            motorise(&mut state, &data, 10);
            let events = crate::state::test_support::pass_stage(&data, &mut state);
            assert_eq!((state.clock.game_turn, state.clock.stage), (14, 1));
            assert!(events.iter().any(|e| matches!(e, Event::Withdrawn { .. })));
            assert!(ids.iter().all(|id| !state.pieces.contains_key(id)));
            serde_json::to_string(&state).unwrap()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn off_the_module_nothing_withdraws() {
        let data = italian();
        let mut state = game(&data);
        state.modules.withdrawals = false;
        state.clock.game_turn = 13;
        state.clock.stage = 3;
        put(&mut state, &data, "unit:cw:5-in-hq", cairo(&data));
        let events = crate::state::test_support::pass_stage(&data, &mut state);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::Withdrawn { .. } | Event::WithdrawalFailed { .. }))
        );
    }

    #[test]
    fn a_return_comes_back_as_it_left() {
        let data = Data::load("campaign-game").unwrap();
        let mut state = game(&data);
        let r = row(&data, 13, 3);
        let ids = put(&mut state, &data, "unit:cw:5-in-hq", cairo(&data));
        motorise(&mut state, &data, 10);
        let hq = ids[0].clone();
        state.pieces.get_mut(&hq).unwrap().toe = 1;
        check(&mut state, &data, &r, None);
        assert!(!state.pieces.contains_key(&hq));
        // The 4th Indian Division, its 5th Brigade under it, comes back
        // at GT28/3.
        let back = GameTime::new(28, 3);
        assert!(
            data.schedule
                .arrivals(Side::Cw, back)
                .any(|x| x.units[0].as_str() == "unit:cw:4-in-hq")
        );
        state.clock.game_turn = back.game_turn;
        state.clock.stage = back.opstage;
        let mut events = Vec::new();
        state.arrive(&data, &mut events);
        assert_eq!(state.pieces[&hq].toe, 1);
        assert!(!state.withdrawn.contains_key(&hq));
    }

    /// The 1st Armoured Brigade leaves at GT23/3 with 30 motorisation
    /// points, and comes back at GT31/1 "Tpt: See 23/3".
    fn armoured_round_trip(modules: Modules) -> (u32, u32, u32) {
        let data = Data::load("campaign-game").unwrap();
        let mut state = game(&data);
        state.modules = modules;
        let r = row(&data, 23, 3);
        let ids = put(&mut state, &data, "unit:cw:1-armd-bde-hq", cairo(&data));
        motorise(&mut state, &data, 35);
        let key = cairo(&data).key().unwrap();
        let held = |s: &GameState| s.motorisation[&Side::Cw].get(&key).copied().unwrap_or(0);
        let events = check(&mut state, &data, &r, None);
        assert!(matches!(&events[0], Event::Withdrawn { .. }), "{events:?}");
        assert!(!state.pieces.contains_key(&ids[0]));
        let out = held(&state);
        state.clock.game_turn = 31;
        state.clock.stage = 1;
        let mut events = Vec::new();
        state.arrive(&data, &mut events);
        assert!(state.pieces.contains_key(&ids[0]), "the brigade is back");
        let back = held(&state);
        // A second return naming 23/3 would find nothing left.
        assert!(state.transport_out.is_empty());
        (35, out, back)
    }

    #[test]
    fn a_return_brings_back_the_transport_that_left_r_157() {
        assert_eq!(armoured_round_trip(Modules::ALL), (35, 5, 35));
    }

    #[test]
    fn without_returning_transport_the_points_stay_gone_r_157() {
        let modules = Modules {
            returning_transport: false,
            ..Modules::ALL
        };
        assert_eq!(armoured_round_trip(modules), (35, 5, 5));
    }
}
