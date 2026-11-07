//! The chariot raid (SPI 30.4x, cna `rules/90-special.md`, *Chariot
//! raid*), behind the `chariot_raid` module (with `fleet`, whose ships it
//! strikes). From July 1941, once in a campaign game, the Axis notes in
//! phase C (`Order::PlanChariotRaid`) a raid by the 10th Light Flotilla's
//! human torpedoes on Alexandria: its secret (30.41) until it is made.
//!
//! The raid is made in the seventh OpStage after the plan's. cna places
//! it in the land support air phase's mission-completion segment; §32
//! has no air phases, so it falls where the Axis's strikes on the fleet
//! do, as phase E closes, before the strike. One die on the Chariot Raid
//! Table (`chariot-raid.json`) gives how many dice to roll; their total
//! is damage points on the ships in Alexandria, laid as a strike's are
//! (`GameState::lay_harbour_damage`: the Axis's choice made by the
//! engine). No supplies are spent. The dice come from a stream of the
//! fleet's own, seeded by the game and the OpStage, as the strikes' do.

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::state::GameState;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};

/// OpStages from the plan's to the raid's (30.41).
pub const STAGES_TO_RAID: u32 = 7;

/// The first month a raid may be planned (30.41): July 1941.
const FIRST_MONTH: (u32, u32) = (1941, 7);

/// The die stream's tag: the strikes use 1 and 2.
const DICE_TAG: u64 = 3;

/// A chariot raid, once planned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Raid {
    /// The OpStage it was planned in.
    pub game_turn: u8,
    pub stage: u8,
    /// Made: no other raid this game.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub made: bool,
}

/// OpStages since the game's start, three a Game-Turn.
fn ordinal(game_turn: u8, stage: u8) -> u32 {
    u32::from(game_turn) * 3 + u32::from(stage)
}

impl GameState {
    /// Why `player` may not plan a chariot raid now, if it may not.
    fn chariot_raid_refusal(&self, data: &Data, player: Side) -> Option<Illegal> {
        let bad = |why: String| Some(Illegal::new("30.41", why));
        if !self.modules.chariot_raid || !self.modules.fleet {
            return bad("this game plays without the chariot raid".into());
        }
        if !data.is_campaign() {
            return bad("the chariot raid is made only in a campaign game".into());
        }
        if player != Side::Axis {
            return bad("only the Axis plans a chariot raid".into());
        }
        if self.clock.phase != Phase::Organisation {
            return bad(format!(
                "a chariot raid is planned in phase C, not at {}",
                self.clock
            ));
        }
        if self.chariot_raid.contains_key(&Side::Axis) {
            return bad("the chariot raid is made once a game".into());
        }
        let week = data.calendar.week(u32::from(self.clock.game_turn));
        if week.is_none_or(|w| (w.year, w.month) < FIRST_MONTH) {
            return bad("no chariot raid before July 1941".into());
        }
        None
    }

    /// Whether `player` may plan the chariot raid now, for the browser's
    /// phase C control.
    pub fn chariot_raid_plannable(&self, data: &Data, player: Side) -> bool {
        self.to_act() == Some(player) && self.chariot_raid_refusal(data, player).is_none()
    }

    /// `Order::PlanChariotRaid` (30.41): the raid is noted for the
    /// seventh OpStage after this one.
    pub(crate) fn plan_chariot_raid(
        &mut self,
        data: &Data,
        player: Side,
    ) -> Result<Vec<Event>, Illegal> {
        if let Some(refused) = self.chariot_raid_refusal(data, player) {
            return Err(refused);
        }
        let (game_turn, stage) = (self.clock.game_turn, self.clock.stage);
        self.chariot_raid.insert(
            Side::Axis,
            Raid {
                game_turn,
                stage,
                made: false,
            },
        );
        Ok(vec![Event::ChariotRaidPlanned { game_turn, stage }])
    }

    /// The close of phase E: the raid planned seven OpStages ago is made
    /// (30.4x), before any strike on the fleet.
    pub(crate) fn make_chariot_raid(&mut self, data: &Data, events: &mut Vec<Event>) {
        let Some(raid) = self.chariot_raid.get(&Side::Axis).copied() else {
            return;
        };
        let now = ordinal(self.clock.game_turn, self.clock.stage);
        if raid.made || now != ordinal(raid.game_turn, raid.stage) + STAGES_TO_RAID {
            return;
        }
        let mut dice = crate::fleet::fleet_dice(
            self.rng.seed(),
            self.clock.game_turn,
            self.clock.stage,
            DICE_TAG,
        );
        let roll = dice.d6();
        let n = data.chariot_raid.damage_dice(roll);
        let damage: u32 = (0..n).map(|_| u32::from(dice.d6())).sum();
        let (hits, sunk) = self.lay_harbour_damage(data, damage);
        self.chariot_raid
            .insert(Side::Axis, Raid { made: true, ..raid });
        events.push(Event::ChariotRaid {
            roll,
            dice: n,
            damage,
            hits,
            sunk,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order::Order;
    use crate::state::Modules;
    use crate::state::test_support::auto_place;
    use crate::view::{Aliases, events_for, view_for};

    /// Graziani's Offensive played as a campaign game (`is_campaign`
    /// reads the slug), its calendar moved to July 1941 when `july_1941`.
    fn campaign(july_1941: bool) -> Data {
        let mut data = crate::raiders::tests::graziani();
        data.scenario.id = "scenario:campaign-game".into();
        let cw = data.scenario.sides.get_mut(&Side::Cw).unwrap();
        cw.fleet.as_mut().unwrap().first_sail = None;
        if july_1941 {
            for w in &mut data.calendar.rows {
                (w.year, w.month) = FIRST_MONTH;
            }
        }
        data
    }

    /// The raid and the fleet, and construction to open phase C (as
    /// `abstract_supply` does in a new game).
    fn on() -> Modules {
        Modules {
            fleet: true,
            chariot_raid: true,
            construction: true,
            ..Modules::default()
        }
    }

    /// Plays from set-up to phase C with `side` to act.
    fn at_phase_c(data: &Data, modules: Modules, side: Side) -> GameState {
        let mut state = GameState::with_modules(data, 7, modules).unwrap();
        auto_place(data, &mut state);
        for _ in 0..20 {
            if state.clock.phase == Phase::Organisation && state.to_act() == Some(side) {
                return state;
            }
            let p = state.to_act().unwrap();
            let order = if state.clock.phase == Phase::InitiativeDeclaration {
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                }
            } else {
                Order::Pass
            };
            state.apply(data, p, order).unwrap();
        }
        panic!("no phase C for {side} at {}", state.clock);
    }

    fn refusal(state: &mut GameState, data: &Data, side: Side) -> String {
        state
            .apply(data, side, Order::PlanChariotRaid)
            .unwrap_err()
            .reason
    }

    #[test]
    fn refused_outside_a_campaign_game_30_41() {
        let mut data = campaign(true);
        data.scenario.id = "scenario:grazianis-offensive".into();
        let mut state = at_phase_c(&data, on(), Side::Axis);
        assert!(!state.chariot_raid_plannable(&data, Side::Axis));
        assert!(refusal(&mut state, &data, Side::Axis).contains("campaign game"));
    }

    #[test]
    fn refused_before_july_1941_30_41() {
        let data = campaign(false);
        let mut state = at_phase_c(&data, on(), Side::Axis);
        assert!(!state.chariot_raid_plannable(&data, Side::Axis));
        assert!(refusal(&mut state, &data, Side::Axis).contains("July 1941"));
    }

    #[test]
    fn refused_without_the_module() {
        let data = campaign(true);
        let modules = Modules {
            chariot_raid: false,
            ..on()
        };
        let mut state = at_phase_c(&data, modules, Side::Axis);
        assert!(refusal(&mut state, &data, Side::Axis).contains("without"));
    }

    #[test]
    fn refused_to_the_commonwealth_30_41() {
        let data = campaign(true);
        let mut state = at_phase_c(&data, on(), Side::Cw);
        assert!(!state.chariot_raid_plannable(&data, Side::Cw));
        assert!(refusal(&mut state, &data, Side::Cw).contains("only the Axis"));
    }

    #[test]
    fn refused_a_second_time_30_41() {
        let data = campaign(true);
        let mut state = at_phase_c(&data, on(), Side::Axis);
        assert!(state.chariot_raid_plannable(&data, Side::Axis));
        let events = state
            .apply(&data, Side::Axis, Order::PlanChariotRaid)
            .unwrap();
        assert_eq!(
            events,
            [Event::ChariotRaidPlanned {
                game_turn: 1,
                stage: 1
            }]
        );
        assert!(!state.chariot_raid_plannable(&data, Side::Axis));
        assert!(refusal(&mut state, &data, Side::Axis).contains("once a game"));
    }

    /// An OpStage played: its Game-Turn and stage, and its events.
    type Stage = ((u8, u8), Vec<Event>);

    /// Plans the raid at GT1/1 with the Axis's strikes spent, and plays
    /// on, passing, until the raid is made: the events of each stage.
    fn planned_and_played(data: &Data) -> (GameState, Vec<Stage>) {
        let mut state = at_phase_c(data, on(), Side::Axis);
        state
            .apply(data, Side::Axis, Order::PlanChariotRaid)
            .unwrap();
        // No strike to muddle the harbour's damage.
        state.fleet_strikes.insert(Side::Axis, 0);
        let mut stages = Vec::new();
        while state.clock.game_turn < 4 {
            let at = (state.clock.game_turn, state.clock.stage);
            let events = crate::state::test_support::pass_stage(data, &mut state);
            stages.push((at, events));
        }
        (state, stages)
    }

    fn raid(events: &[Event]) -> Option<&Event> {
        events
            .iter()
            .find(|e| matches!(e, Event::ChariotRaid { .. }))
    }

    #[test]
    fn made_in_the_seventh_stage_after_on_the_ships_in_alexandria_30_4x() {
        let data = campaign(true);
        let (state, stages) = planned_and_played(&data);
        let made: Vec<_> = stages
            .iter()
            .filter(|(_, e)| raid(e).is_some())
            .map(|(at, _)| *at)
            .collect();
        // GT1/1 + 7 OpStages: 1/2, 1/3, 2/1, 2/2, 2/3, 3/1, 3/2.
        assert_eq!(made, [(3, 2)], "once, in the seventh stage after");
        let events = &stages.iter().find(|(at, _)| *at == (3, 2)).unwrap().1;
        let Some(Event::ChariotRaid {
            roll,
            dice,
            damage,
            hits,
            sunk,
        }) = raid(events)
        else {
            unreachable!()
        };
        // Seed 7: a 1 gives three dice (30.46), totalling 8.
        assert_eq!((*roll, *dice, *damage), (1, 3, 8));
        assert_eq!(hits.values().sum::<u32>(), *damage);
        // Laid as a strike's are, on the most guns left, the lower id on
        // a tie: Valiant 6 to 3, then the 3s by id, Arethusa, Dido,
        // Euryalus, Valiant, then Arethusa again.
        let ship = |n: &str| crate::fleet::ship_id(n);
        assert_eq!(
            *hits,
            std::collections::BTreeMap::from([
                (ship("arethusa"), 2),
                (ship("dido"), 1),
                (ship("euryalus"), 1),
                (ship("valiant"), 4)
            ])
        );
        assert!(sunk.is_empty());
        assert_eq!(state.fleet[&ship("valiant")].damage, 4);
        assert!(state.chariot_raid[&Side::Axis].made);
    }

    #[test]
    fn hidden_from_the_commonwealth_until_made_30_41() {
        let data = campaign(true);
        let aliases = Aliases::new([1; 32]);
        let mut state = at_phase_c(&data, on(), Side::Axis);
        let shown = |s: &GameState, ev: &[Event], viewer: Option<Side>| {
            let v = view_for(s, &data, &aliases, viewer, false).unwrap();
            let e = events_for(s, &aliases, ev, viewer, false);
            (v.to_string(), serde_json::to_string(&e).unwrap())
        };
        let before = state.clone();
        let planned = state
            .apply(&data, Side::Axis, Order::PlanChariotRaid)
            .unwrap();
        for viewer in [Some(Side::Cw), None] {
            let (v, e) = shown(&state, &planned, viewer);
            assert_eq!(v, shown(&before, &[], viewer).0, "{viewer:?} sees the plan");
            assert_eq!(e, "[]", "{viewer:?} sees the plan's event");
        }
        let (v, e) = shown(&state, &planned, Some(Side::Axis));
        assert!(v.contains("\"chariot_raid\":{\"axis\""), "{v}");
        assert!(e.contains("chariot-raid-planned"), "{e}");
        // A Commonwealth preview made from what it knows holds no raid.
        let known = crate::view::as_known_to(&state, Side::Cw);
        assert!(known.chariot_raid.is_empty());
        // Made, the raid is public.
        state.fleet_strikes.insert(Side::Axis, 0);
        let mut all = Vec::new();
        while state.clock.game_turn < 4 {
            all.extend(crate::state::test_support::pass_stage(&data, &mut state));
        }
        let made: Vec<Event> = raid(&all).into_iter().cloned().collect();
        for viewer in [Some(Side::Cw), None] {
            let (_, e) = shown(&state, &made, viewer);
            assert!(e.contains("\"chariot-raid\""), "{viewer:?}: {e}");
        }
    }
}
