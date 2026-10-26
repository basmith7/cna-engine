//! The naval convoy stage (5.2's stage II) under abstract supply: run by
//! the engine as each Game-Turn's first OpStage opens, since no player
//! acts in it yet. Slice 4 starts with the monthly motorisation losses
//! (32.5x); planning and arrivals follow.

use crate::data::Data;
use crate::event::Event;
use crate::state::GameState;
use cna_data::calendar::previous_month;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use serde::{Deserialize, Serialize};

/// Real supply units planned for `side`, due in Game-Turn `arrives`
/// (32.4x).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Planned {
    pub side: Side,
    pub arrives: u8,
    pub units: u32,
}

impl GameState {
    pub(crate) fn naval_convoy_stage(&mut self, data: &Data, events: &mut Vec<Event>) {
        if !self.modules.abstract_supply {
            return;
        }
        self.plan_supply(data, events);
        self.motorisation_losses(data, events);
    }

    /// Each turn's planning rolls (32.4x): the Axis one die under the
    /// tonnage letter of the month it plans in, units due two turns on;
    /// the Commonwealth under the supply period of the arrival month,
    /// four turns on. The dice come from a stream of their own, seeded by
    /// the game, the turn and the side, so planning never shifts the
    /// dice of play. A roll that brings nothing, or a month the charts
    /// print as a dash, plans nothing.
    fn plan_supply(&mut self, data: &Data, events: &mut Vec<Event>) {
        let turn = self.clock.game_turn;
        for side in [Side::Axis, Side::Cw] {
            let (later, month_turn) = match side {
                Side::Axis => (
                    data.simplified_supply.axis.arrive_turns_later,
                    u32::from(turn),
                ),
                Side::Cw => {
                    let l = data.simplified_supply.cw.arrive_turns_later;
                    (l, u32::from(turn) + l)
                }
            };
            let Some(week) = data.calendar.week(month_turn) else {
                continue;
            };
            let salt = (u64::from(turn) << 8) | u64::from(side == Side::Cw);
            let die = crate::rng::GameRng::new(
                self.rng.seed() ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            )
            .d6();
            let month = week.month_key();
            let units = match side {
                Side::Axis => data.simplified_supply.axis_units(&month, die),
                Side::Cw => data.simplified_supply.cw_units(&month, die),
            };
            let Some(units) = units.filter(|u| *u > 0) else {
                continue;
            };
            let arrives = turn.saturating_add(later as u8);
            self.planned.push(Planned {
                side,
                arrives,
                units,
            });
            events.push(Event::SupplyPlanned {
                side,
                die,
                units,
                arrives,
            });
        }
    }

    /// In a month's first naval convoy stage, each side loses the
    /// previous month's percentage of all its motorisation points, pooled
    /// and attached, boxes included (32.5x): fractions down, taken from
    /// the pools on its rear map (the Commonwealth's map E, the Axis's map
    /// A and the boxes) in key order, and no further than they hold.
    fn motorisation_losses(&mut self, data: &Data, events: &mut Vec<Event>) {
        let turn = u32::from(self.clock.game_turn);
        if !data.calendar.starts_month(turn) {
            return;
        }
        let Some(week) = data.calendar.week(turn) else {
            return;
        };
        let Some(row) = data.motorisation_losses.month(&previous_month(&week)) else {
            return;
        };
        for side in [Side::Axis, Side::Cw] {
            let percent = match side {
                Side::Axis => row.axis_percent,
                Side::Cw => row.cw_percent,
            };
            let attached: u32 = self
                .pieces
                .values()
                .filter(|p| p.side == side && p.hex().is_some())
                .map(|p| p.motorised)
                .sum();
            let pooled: u32 = self.motorisation.get(&side).map_or(0, |m| m.values().sum());
            let of = attached + pooled;
            let mut owed = of * percent / 100;
            if owed == 0 {
                continue;
            }
            let rear = |key: &str| match HexId::parse(key) {
                Ok(h) => h.sheet() == if side == Side::Cw { 'E' } else { 'A' },
                Err(_) => side == Side::Axis,
            };
            let mut lost = 0;
            if let Some(pools) = self.motorisation.get_mut(&side) {
                for (_, points) in pools.iter_mut().filter(|(k, _)| rear(k)) {
                    let n = owed.min(*points);
                    *points -= n;
                    owed -= n;
                    lost += n;
                }
                pools.retain(|_, p| *p > 0);
            }
            events.push(Event::MotorisationLost {
                side,
                percent,
                of,
                points: lost,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use crate::state::Modules;
    use std::collections::BTreeMap;

    fn at_turn(data: &Data, game_turn: u8) -> GameState {
        let mut state = GameState::with_modules(data, 5, Modules::ALL).unwrap();
        crate::state::test_support::auto_place(data, &mut state);
        state.clock = Clock::start(game_turn);
        state
    }

    #[test]
    fn november_takes_octobers_percentage_from_the_rear_pools_32_5x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_turn(&data, 7);
        for p in state.pieces.values_mut() {
            p.motorised = 0;
        }
        state.motorisation = BTreeMap::from([
            (
                Side::Cw,
                BTreeMap::from([("E3613".to_string(), 100), ("D3714".to_string(), 100)]),
            ),
            (Side::Axis, BTreeMap::from([("A0101".to_string(), 50)])),
        ]);
        let mut events = Vec::new();
        state.motorisation_losses(&data, &mut events);
        // October 1940: Commonwealth 6 %, Axis 2 %; fractions down.
        assert!(events.contains(&Event::MotorisationLost {
            side: Side::Cw,
            percent: 6,
            of: 200,
            points: 12
        }));
        assert!(events.contains(&Event::MotorisationLost {
            side: Side::Axis,
            percent: 2,
            of: 50,
            points: 1
        }));
        assert_eq!(state.motorisation[&Side::Cw]["E3613"], 88);
        assert_eq!(state.motorisation[&Side::Cw]["D3714"], 100);
    }

    #[test]
    fn each_turn_plans_axis_supply_two_turns_on_and_commonwealth_four_32_4x() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_turn(&data, 3);
        let draws = state.rng.draws();
        let mut events = Vec::new();
        state.plan_supply(&data, &mut events);
        assert_eq!(state.rng.draws(), draws, "planning uses its own dice");
        for e in &events {
            let Event::SupplyPlanned {
                side,
                die,
                units,
                arrives,
            } = e
            else {
                panic!("{e:?}");
            };
            // October 1940: the Axis letter B; the Commonwealth's period I.
            let want = match side {
                Side::Axis => (data.simplified_supply.axis_units("1940-10", *die), 5),
                Side::Cw => (data.simplified_supply.cw_units("1940-11", *die), 7),
            };
            assert_eq!((Some(*units), *arrives), want);
        }
        // The Commonwealth always gets at least one in period I.
        assert!(
            state
                .planned
                .iter()
                .any(|p| p.side == Side::Cw && p.arrives == 7)
        );
        // The same game plans the same way again.
        let mut again = at_turn(&data, 3);
        again.plan_supply(&data, &mut Vec::new());
        assert_eq!(again.planned, state.planned);
    }

    #[test]
    fn no_loss_mid_month_or_before_the_chart_starts() {
        let data = Data::load("grazianis-offensive").unwrap();
        for turn in [3, 8] {
            let mut state = at_turn(&data, turn);
            let before = state.motorisation.clone();
            let mut events = Vec::new();
            state.motorisation_losses(&data, &mut events);
            assert!(events.is_empty(), "GT{turn}: {events:?}");
            assert_eq!(state.motorisation, before);
        }
    }
}
