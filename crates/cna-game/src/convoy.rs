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

impl GameState {
    pub(crate) fn naval_convoy_stage(&mut self, data: &Data, events: &mut Vec<Event>) {
        if !self.modules.abstract_supply {
            return;
        }
        self.motorisation_losses(data, events);
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
