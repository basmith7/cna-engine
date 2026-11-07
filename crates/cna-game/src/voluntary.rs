//! Voluntary Commonwealth withdrawals (20.9, 64.75 item 3, R-109) under
//! the `voluntary_withdrawals` module, in the campaign games (the points
//! victories ranked by ratio, `campaign::campaign`). Whenever the
//! Commonwealth is to act with nothing pending, it may withdraw a combat
//! unit (not an HQ: a non-HQ counter is a battalion or equivalent, as
//! 19.5's battalion row counts it) that is not anti-aircraft (its TOE not
//! all AA, as `setup::thin_anti_air` judges an AA unit), stands in Cairo
//! or Alexandria and holds 75 % of its maximum TOE. It leaves into
//! `GameState.withdrawn` as a scheduled withdrawal does, and earns half a
//! point for each whole Game-Turn (R-109: first OpStage to last) it is
//! away, at most three points a unit over the game. Bringing it back
//! puts it in Cairo as it left and costs two points; it may not be
//! withdrawn again for 26 Game-Turns (six months). No schedule row
//! brings it back. Points are kept in halves.

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind};
use crate::state::GameState;
use cna_data::scenario::Side;
use cna_data::weapons::WeaponClass;
use serde::{Deserialize, Serialize};

/// Half-points a unit may earn over the whole game (R-109: three points).
const CAP_HALVES: u32 = 6;
/// Half-points bringing a unit back costs (64.75: two points).
const RETURN_HALVES: u32 = 4;
/// Game-Turns after a return before the unit may go again (64.75: six
/// months of weekly Game-Turns).
const LOCKOUT_TURNS: u8 = 26;

/// One unit's voluntary withdrawals.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voluntary {
    /// The Game-Turn it left in, while it is away.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub away_since: Option<u8>,
    /// Half-points earned by absences that have ended.
    #[serde(default)]
    pub banked: u32,
    /// Times it was brought back.
    #[serde(default)]
    pub returns: u32,
    /// The Game-Turn of its last return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returned: Option<u8>,
}

/// The voluntary withdrawal orders open to the Commonwealth now: what
/// `Order::WithdrawVoluntarily` and `Order::ReturnWithdrawn` accept, and
/// the net half-points earned so far.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct VoluntaryOptions {
    pub withdrawable: Vec<PieceId>,
    pub returnable: Vec<PieceId>,
    pub halves: i32,
}

impl VoluntaryOptions {
    pub fn is_empty(&self) -> bool {
        self.withdrawable.is_empty() && self.returnable.is_empty()
    }
}

impl Voluntary {
    /// Half-points earned to Game-Turn `now`: the whole Game-Turns
    /// between the one it left in and `now`, both excluded, on top of
    /// what is banked, capped at three points.
    fn earned(&self, now: u8) -> u32 {
        let away = self
            .away_since
            .map_or(0, |gt| u32::from(now.saturating_sub(gt).saturating_sub(1)));
        (self.banked + away).min(CAP_HALVES)
    }

    /// Net half-points to Game-Turn `now`: earned less the returns' cost.
    pub fn net_halves(&self, now: u8) -> i32 {
        self.earned(now) as i32 - (self.returns * RETURN_HALVES) as i32
    }
}

impl GameState {
    fn voluntary_open(&self, data: &Data, player: Side) -> Result<(), Illegal> {
        if !self.modules.voluntary_withdrawals || crate::campaign::campaign(data).is_none() {
            return Err(Illegal::new(
                "64.75",
                "voluntary withdrawals are a campaign game's rule",
            ));
        }
        if player != Side::Cw {
            return Err(Illegal::new(
                "20.9",
                "only the Commonwealth withdraws voluntarily",
            ));
        }
        Ok(())
    }

    /// Whether `id` is a unit that may be withdrawn voluntarily now.
    fn voluntary_refusal(&self, data: &Data, id: &PieceId) -> Option<String> {
        let Some(p) = self.pieces.get(id) else {
            return Some(format!("{id} is not in play"));
        };
        if p.side != Side::Cw {
            return Some(format!("{id} is not a Commonwealth unit"));
        }
        let hq =
            p.id.unit()
                .and_then(|u| data.oa.unit(&u))
                .is_some_and(|u| u.echelon.is_some());
        if p.kind != PieceKind::Combat || hq {
            return Some(format!("{id} is not a combat battalion or equivalent"));
        }
        let aa = !p.weapons.is_empty()
            && p.weapons.iter().all(|w| {
                p.nation
                    .and_then(|n| data.weapons.resolve(n, &w.weapon))
                    .is_some_and(|r| r.class == WeaponClass::AntiAir)
            });
        if aa {
            return Some(format!("{id} is an anti-aircraft unit"));
        }
        if !Self::withdrawal_hexes(data).contains(&p.location) {
            return Some(format!("{id} is not in Cairo or Alexandria"));
        }
        if p.toe * 4 < self.toe_ceiling(data, p) * 3 {
            return Some(format!("{id} is under 75 % of its TOE"));
        }
        let now = self.clock.game_turn;
        if let Some(back) = self.voluntary.get(id).and_then(|v| v.returned)
            && now < back.saturating_add(LOCKOUT_TURNS)
        {
            return Some(format!(
                "{id} came back in Game-Turn {back}: not again before Game-Turn {}",
                back.saturating_add(LOCKOUT_TURNS)
            ));
        }
        None
    }

    /// `Order::WithdrawVoluntarily` (20.9, 64.75).
    pub(crate) fn withdraw_voluntarily(
        &mut self,
        data: &Data,
        player: Side,
        id: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        self.voluntary_open(data, player)?;
        if let Some(reason) = self.voluntary_refusal(data, id) {
            return Err(Illegal::new("64.75", reason));
        }
        self.release_attached(id);
        let p = self.pieces.remove(id).expect("checked in play");
        self.withdrawn.insert(id.clone(), p);
        self.voluntary.entry(id.clone()).or_default().away_since = Some(self.clock.game_turn);
        Ok(vec![Event::WithdrawnVoluntarily { piece: id.clone() }])
    }

    /// `Order::ReturnWithdrawn` (64.75): back to Cairo as it left.
    pub(crate) fn return_withdrawn(
        &mut self,
        data: &Data,
        player: Side,
        id: &PieceId,
    ) -> Result<Vec<Event>, Illegal> {
        self.voluntary_open(data, player)?;
        let now = self.clock.game_turn;
        let Some(v) = self
            .voluntary
            .get_mut(id)
            .filter(|v| v.away_since.is_some())
        else {
            return Err(Illegal::new(
                "64.75",
                format!("{id} was not withdrawn voluntarily"),
            ));
        };
        v.banked = v.earned(now);
        v.away_since = None;
        v.returns += 1;
        v.returned = Some(now);
        let mut p = self.withdrawn.remove(id).expect("away in withdrawn");
        p.location = Location::Hex(
            data.map
                .place("cairo")
                .expect("places.json has Cairo")
                .hex
                .clone(),
        );
        self.pieces.insert(id.clone(), p);
        self.attach_where_stacked(Some(std::slice::from_ref(id)));
        Ok(vec![Event::ReturnedFromWithdrawal { piece: id.clone() }])
    }

    /// What `player` may withdraw or bring back now, for the browser:
    /// the Commonwealth's alone, in a campaign game, while it is to act
    /// with nothing pending.
    pub fn voluntary_options(&self, data: &Data, player: Side) -> VoluntaryOptions {
        if self.voluntary_open(data, player).is_err()
            || self.to_act() != Some(player)
            || self.pending_refusal().is_some()
        {
            return VoluntaryOptions::default();
        }
        VoluntaryOptions {
            withdrawable: self
                .pieces
                .keys()
                .filter(|id| self.voluntary_refusal(data, id).is_none())
                .cloned()
                .collect(),
            returnable: self
                .voluntary
                .iter()
                .filter(|(_, v)| v.away_since.is_some())
                .map(|(id, _)| id.clone())
                .collect(),
            halves: self.withdrawal_halves(),
        }
    }

    /// The Commonwealth's net withdrawal half-points to now (64.75).
    pub fn withdrawal_halves(&self) -> i32 {
        let now = self.clock.game_turn;
        self.voluntary.values().map(|v| v.net_halves(now)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;
    use crate::state::Modules;

    fn data() -> Data {
        Data::load("campaign-game").unwrap()
    }

    fn game(data: &Data) -> GameState {
        let mut s = GameState::with_modules(data, 7, Modules::ALL).unwrap();
        crate::state::test_support::auto_place(data, &mut s);
        s
    }

    fn cairo(data: &Data) -> Location {
        Location::Hex(data.map.place("cairo").unwrap().hex.clone())
    }

    /// A Commonwealth non-HQ combat unit, not AA, at full TOE, moved to
    /// `at`.
    fn battalion(state: &mut GameState, data: &Data, at: Location) -> PieceId {
        let ids: Vec<PieceId> = state
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::Combat)
            .filter(|p| p.location != Location::Eliminated)
            .map(|p| p.id.clone())
            .collect();
        let id = ids
            .into_iter()
            .find(|id| {
                let mut probe = state.clone();
                let max = probe.toe_ceiling(data, &probe.pieces[id]);
                let p = probe.pieces.get_mut(id).unwrap();
                p.location = cairo(data);
                p.toe = max;
                max > 0 && probe.voluntary_refusal(data, id).is_none()
            })
            .expect("a battalion");
        let max = state.toe_ceiling(data, &state.pieces[&id]);
        let p = state.pieces.get_mut(&id).unwrap();
        p.location = at;
        p.toe = max;
        id
    }

    fn withdraw(state: &mut GameState, data: &Data, id: &PieceId) -> Result<Vec<Event>, Illegal> {
        state.withdraw_voluntarily(data, Side::Cw, id)
    }

    #[test]
    fn a_ready_battalion_in_cairo_withdraws() {
        let data = data();
        let mut state = game(&data);
        let id = battalion(&mut state, &data, cairo(&data));
        let gt = state.clock.game_turn;
        let events = withdraw(&mut state, &data, &id).unwrap();
        assert_eq!(
            events,
            vec![Event::WithdrawnVoluntarily { piece: id.clone() }]
        );
        assert!(!state.pieces.contains_key(&id));
        assert!(state.withdrawn.contains_key(&id));
        assert_eq!(state.voluntary[&id].away_since, Some(gt));
    }

    #[test]
    fn the_order_is_applied_through_apply() {
        let data = data();
        let mut state = game(&data);
        let id = battalion(&mut state, &data, cairo(&data));
        // Find a moment the Commonwealth is to act with nothing pending.
        for _ in 0..200 {
            if state.to_act() == Some(Side::Cw) && state.pending_refusal().is_none() {
                break;
            }
            let who = state.to_act().unwrap();
            if state.apply(&data, who, Order::Pass).is_err() {
                state
                    .apply(&data, who, Order::DeclareInitiative { player_a: Side::Cw })
                    .unwrap();
            }
        }
        assert_eq!(state.to_act(), Some(Side::Cw));
        state.pieces.get_mut(&id).unwrap().location = cairo(&data);
        state
            .apply(
                &data,
                Side::Cw,
                Order::WithdrawVoluntarily { piece: id.clone() },
            )
            .unwrap();
        assert!(state.withdrawn.contains_key(&id));
        state
            .apply(
                &data,
                Side::Cw,
                Order::ReturnWithdrawn { piece: id.clone() },
            )
            .unwrap();
        assert_eq!(state.pieces[&id].location, cairo(&data));
    }

    #[test]
    fn the_options_list_what_may_go_and_come_back() {
        let data = data();
        let mut state = game(&data);
        let id = battalion(&mut state, &data, cairo(&data));
        for _ in 0..200 {
            if state.to_act() == Some(Side::Cw) && state.pending_refusal().is_none() {
                break;
            }
            let who = state.to_act().unwrap();
            if state.apply(&data, who, Order::Pass).is_err() {
                state
                    .apply(&data, who, Order::DeclareInitiative { player_a: Side::Cw })
                    .unwrap();
            }
        }
        state.pieces.get_mut(&id).unwrap().location = cairo(&data);
        let o = state.voluntary_options(&data, Side::Cw);
        assert!(o.withdrawable.contains(&id));
        assert!(o.returnable.is_empty());
        assert!(state.voluntary_options(&data, Side::Axis).is_empty());
        withdraw(&mut state, &data, &id).unwrap();
        let o = state.voluntary_options(&data, Side::Cw);
        assert!(!o.withdrawable.contains(&id));
        assert_eq!(o.returnable, vec![id]);
    }

    #[test]
    fn refused_weak_aa_away_axis_or_module_off() {
        let data = data();
        let base = game(&data);
        // Under 75 %.
        let mut s = base.clone();
        let id = battalion(&mut s, &data, cairo(&data));
        let max = s.toe_ceiling(&data, &s.pieces[&id].clone());
        s.pieces.get_mut(&id).unwrap().toe = (max * 3).div_ceil(4) - 1;
        let e = withdraw(&mut s, &data, &id).unwrap_err();
        assert!(e.to_string().contains("75 %"), "{e}");
        // Outside Cairo and Alexandria.
        let mut s = base.clone();
        let id = battalion(
            &mut s,
            &data,
            Location::Box(cna_data::scenario::OffMapBox::Tripoli),
        );
        let e = withdraw(&mut s, &data, &id).unwrap_err();
        assert!(e.to_string().contains("Cairo"), "{e}");
        // Alexandria is a place it may leave from.
        let mut s = base.clone();
        let alex = Location::Hex(data.map.place("alexandria-west").unwrap().hex.clone());
        let id = battalion(&mut s, &data, alex);
        withdraw(&mut s, &data, &id).unwrap();
        // The Axis may not.
        let mut s = base.clone();
        let id = battalion(&mut s, &data, cairo(&data));
        let e = s.withdraw_voluntarily(&data, Side::Axis, &id).unwrap_err();
        assert!(e.to_string().contains("Commonwealth"), "{e}");
        // Not a Commonwealth unit.
        let mut s = base.clone();
        let axis = s
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && p.kind == PieceKind::Combat)
            .unwrap()
            .id
            .clone();
        s.pieces.get_mut(&axis).unwrap().location = cairo(&data);
        let e = withdraw(&mut s, &data, &axis).unwrap_err();
        assert!(e.to_string().contains("not a Commonwealth"), "{e}");
        // Module off.
        let mut s = base.clone();
        s.modules.voluntary_withdrawals = false;
        let id = battalion(&mut s, &data, cairo(&data));
        let e = withdraw(&mut s, &data, &id).unwrap_err();
        assert!(e.to_string().contains("campaign"), "{e}");
        // Not a campaign game.
        let italian = Data::load("italian-campaign").unwrap();
        let mut s = game(&italian);
        let id = s
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::Combat)
            .unwrap()
            .id
            .clone();
        let e = s.withdraw_voluntarily(&italian, Side::Cw, &id).unwrap_err();
        assert!(e.to_string().contains("campaign"), "{e}");
    }

    #[test]
    fn an_anti_aircraft_unit_is_refused() {
        let data = data();
        let mut s = game(&data);
        let id = battalion(&mut s, &data, cairo(&data));
        let aa = data
            .weapons
            .rows()
            .find(|r| r.class == WeaponClass::AntiAir && r.nation == cna_data::oa::Nation::Cw)
            .unwrap();
        let name = data
            .weapons
            .oa_name(&aa.id)
            .unwrap_or(&aa.weapon)
            .to_owned();
        let p = s.pieces.get_mut(&id).unwrap();
        p.nation = Some(cna_data::oa::Nation::Cw);
        p.weapons = vec![cna_data::oa::ToeEntry {
            weapon: name,
            points: p.toe,
        }];
        let e = withdraw(&mut s, &data, &id).unwrap_err();
        assert!(e.to_string().contains("anti-aircraft"), "{e}");
    }

    #[test]
    fn an_hq_is_refused() {
        let data = data();
        let mut s = game(&data);
        let hq = s
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Cw
                    && p.id
                        .unit()
                        .and_then(|u| data.oa.unit(&u))
                        .is_some_and(|u| u.echelon.is_some())
            })
            .unwrap()
            .id
            .clone();
        s.pieces.get_mut(&hq).unwrap().location = cairo(&data);
        let e = withdraw(&mut s, &data, &hq).unwrap_err();
        assert!(e.to_string().contains("battalion"), "{e}");
    }

    #[test]
    fn points_accrue_by_whole_game_turn_and_cap_at_three() {
        let v = Voluntary {
            away_since: Some(10),
            ..Voluntary::default()
        };
        // Left in GT 10: GT 10 is not whole; GT 11 is once GT 12 opens.
        assert_eq!(v.net_halves(10), 0);
        assert_eq!(v.net_halves(11), 0);
        assert_eq!(v.net_halves(12), 1);
        assert_eq!(v.net_halves(15), 4);
        assert_eq!(v.net_halves(17), 6);
        assert_eq!(v.net_halves(40), 6, "three points at most (R-109)");
    }

    #[test]
    fn a_return_costs_two_points_and_lands_in_cairo_as_it_left() {
        let data = data();
        let mut state = game(&data);
        let id = battalion(&mut state, &data, cairo(&data));
        let before = state.pieces[&id].clone();
        let gt = state.clock.game_turn;
        withdraw(&mut state, &data, &id).unwrap();
        state.clock.game_turn = gt + 5; // four whole Game-Turns away
        assert_eq!(state.withdrawal_halves(), 4);
        let events = state.return_withdrawn(&data, Side::Cw, &id).unwrap();
        assert_eq!(
            events,
            vec![Event::ReturnedFromWithdrawal { piece: id.clone() }]
        );
        assert_eq!(state.withdrawal_halves(), 0, "two earned less two");
        let mut back = state.pieces[&id].clone();
        back.attached_to = before.attached_to.clone();
        assert_eq!(back, before);
        assert!(!state.withdrawn.contains_key(&id));
        // A second return of a unit not away is refused.
        assert!(state.return_withdrawn(&data, Side::Cw, &id).is_err());
        // Later Game-Turns earn nothing while it is back.
        state.clock.game_turn = gt + 20;
        assert_eq!(state.withdrawal_halves(), 0);
    }

    #[test]
    fn not_again_within_26_game_turns_and_the_cap_spans_the_game() {
        let data = data();
        let mut state = game(&data);
        let id = battalion(&mut state, &data, cairo(&data));
        let gt = state.clock.game_turn;
        withdraw(&mut state, &data, &id).unwrap();
        state.clock.game_turn = gt + 5;
        state.return_withdrawn(&data, Side::Cw, &id).unwrap();
        state.clock.game_turn = gt + 30;
        let e = withdraw(&mut state, &data, &id).unwrap_err();
        assert!(e.to_string().contains("not again"), "{e}");
        state.clock.game_turn = gt + 31;
        withdraw(&mut state, &data, &id).unwrap();
        // Four halves banked; six weeks more reach only the cap of six.
        state.clock.game_turn = gt + 38;
        assert_eq!(state.withdrawal_halves(), 6 - 4);
    }

    #[test]
    fn the_victory_total_counts_them() {
        let data = data();
        let mut state = game(&data);
        let id = battalion(&mut state, &data, cairo(&data));
        withdraw(&mut state, &data, &id).unwrap();
        state.clock.game_turn += 7; // six whole Game-Turns: three points
        // The same board without the record: only the points differ.
        let mut same = state.clone();
        same.voluntary.clear();
        let without = crate::victory::evaluate(&data, &same).points.unwrap();
        let with = crate::victory::evaluate(&data, &state).points.unwrap();
        assert_eq!(with.cw, without.cw + 3);
        assert_eq!(with.axis, without.axis);
    }
}
