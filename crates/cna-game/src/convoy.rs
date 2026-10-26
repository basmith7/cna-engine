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

/// The most supply units one Axis convoy lane carries a convoy stage
/// (32.6x).
const LANE_SUPPLY: u32 = 3;

/// Supply units sunk from `units` by a `percent` convoy result (32.65):
/// under 20 % only motorisation and replacement points are hit, and a
/// convoy carries neither yet (R-127); from 20 % that share, fractions
/// down, at least one.
fn supply_sunk(units: u32, percent: u32) -> u32 {
    if percent < 20 {
        return 0;
    }
    (units * percent / 100).max(1).min(units)
}

impl GameState {
    /// A Commonwealth or Axis HQ whose OA echelon is division: the
    /// engine's reading of a full division (§19) for 32.64.
    pub(crate) fn is_division_hq(&self, data: &Data, p: &crate::piece::Piece) -> bool {
        p.kind == crate::piece::PieceKind::Hq
            && p.id.unit().and_then(|u| data.oa.unit(&u)?.echelon)
                == Some(cna_data::oa::Echelon::Division)
    }

    /// The Commonwealth attacks the Axis convoy on `lane` carrying
    /// `units` supply units (32.63–32.65): bomb points from the chart's
    /// row for the westernmost Commonwealth division HQ on the map; none
    /// reached, no attack (R-126). Two dice, read in order, come from a
    /// stream of their own, seeded by the game and the turn, so bombing
    /// never shifts the dice of play. Returns the units sunk.
    fn bomb_convoy(&mut self, data: &Data, lane: u8, units: u32, events: &mut Vec<Event>) -> u32 {
        let west = self
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && self.is_division_hq(data, p))
            .filter_map(|p| p.hex())
            .map(|h| (h.sheet(), h.col()))
            .min();
        let Some((sheet, col)) = west else {
            return 0;
        };
        let Some(band) = data.convoy_bombing.bomb_points(lane, sheet, col) else {
            return 0;
        };
        let salt = (u64::from(self.clock.game_turn) << 8) | 0x80 | u64::from(lane);
        let mut rng =
            crate::rng::GameRng::new(self.rng.seed() ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let reading = rng.d6() * 10 + rng.d6();
        let percent = data
            .convoy_bombardment
            .percent(band.from, reading)
            .unwrap_or(0);
        let sunk = supply_sunk(units, percent);
        events.push(Event::ConvoyBombed {
            lane,
            bomb_points: band.from,
            reading,
            percent,
            units,
            sunk,
        });
        sunk
    }

    /// Phase D of the arrival turn's first OpStage: the planned supply
    /// units appear loaded (40 ammunition, 60 fuel), with a dummy for
    /// every three real ones (32.4x, 32.1x). The Commonwealth's land in
    /// Cairo, one in three in Alexandria (its west hex, E3613, where the
    /// Commonwealth's supply starts). The Axis's come by convoy to the
    /// Tripoli box on lane 2, at most three a stage, the rest sailing with
    /// the next turn's convoy, and may be bombed on the way (32.6x); the
    /// dummies count only the units that land (R-128). Arrivals do not check stacking, as
    /// reinforcements do not. True when anything arrived.
    pub(crate) fn supply_arrivals(&mut self, data: &Data, events: &mut Vec<Event>) -> bool {
        use crate::piece::{Load, Location, MAX_AMMO, MAX_FUEL, PieceId, PieceKind};
        use cna_data::scenario::OffMapBox;
        if !self.modules.abstract_supply || self.clock.stage != 1 {
            return false;
        }
        let turn = self.clock.game_turn;
        let hex = |name: &str| {
            Location::Hex(
                data.map
                    .place(name)
                    .unwrap_or_else(|| panic!("places.json has {name}"))
                    .hex
                    .clone(),
            )
        };
        let mut any = false;
        for side in [Side::Axis, Side::Cw] {
            let due: u32 = self
                .planned
                .iter()
                .filter(|p| p.side == side && p.arrives == turn)
                .map(|p| p.units)
                .sum();
            self.planned
                .retain(|p| !(p.side == side && p.arrives == turn));
            let sailing = match side {
                Side::Axis => due.min(LANE_SUPPLY),
                Side::Cw => due,
            };
            if due > sailing {
                self.planned.push(Planned {
                    side,
                    arrives: turn + 1,
                    units: due - sailing,
                });
            }
            let units = match side {
                Side::Axis if sailing > 0 => {
                    let lane = data
                        .convoy_lanes
                        .to_box("tripoli")
                        .expect("convoy-lanes.json has a Tripoli lane")
                        .lane;
                    sailing - self.bomb_convoy(data, lane, sailing, events)
                }
                _ => sailing,
            };
            if units == 0 {
                continue;
            }
            let (main, second, to_second) = match side {
                Side::Cw => (hex("cairo"), hex("alexandria-west"), units / 3),
                Side::Axis => (
                    Location::Box(OffMapBox::Tripoli),
                    Location::Box(OffMapBox::Tripoli),
                    0,
                ),
            };
            let next = |state: &GameState, dummy: bool| {
                (1..)
                    .map(|n| {
                        if dummy {
                            PieceId::dummy_supply(side, n)
                        } else {
                            PieceId::supply(side, n)
                        }
                    })
                    .find(|id| !state.pieces.contains_key(id))
                    .expect("a free number")
            };
            let mut at_main = Vec::new();
            let mut at_second = Vec::new();
            for i in 0..units + units / 3 {
                let dummy = i >= units;
                let id = next(self, dummy);
                let (at, list) = if !dummy && i < to_second {
                    (second.clone(), &mut at_second)
                } else {
                    (main.clone(), &mut at_main)
                };
                let (kind, load) = if dummy {
                    (PieceKind::DummySupplyUnit, None)
                } else {
                    (
                        PieceKind::SupplyUnit,
                        Some(Load {
                            ammo: MAX_AMMO,
                            fuel: MAX_FUEL,
                        }),
                    )
                };
                let mut p = crate::setup::supply_piece(side, kind, id.clone(), at, load);
                p.landed = true;
                self.pieces.insert(id.clone(), p);
                list.push(id);
            }
            for (pieces, at) in [(at_second, second), (at_main, main)] {
                if !pieces.is_empty() {
                    events.push(Event::Arrived { pieces, at });
                }
            }
            any = true;
        }
        any
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
    fn commonwealth_supply_arrives_in_cairo_and_alexandria_with_dummies_32_4x() {
        use crate::clock::Phase;
        use crate::piece::PieceKind;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_turn(&data, 5);
        state.clock.phase = Phase::Arrival;
        // Set-up ran GT1's planning, which may be due now too.
        state.planned.clear();
        state.planned.push(Planned {
            side: Side::Cw,
            arrives: 5,
            units: 7,
        });
        let count = |s: &GameState, k: PieceKind| {
            s.pieces
                .values()
                .filter(|p| p.side == Side::Cw && p.kind == k)
                .count()
        };
        let (real, dummies) = (
            count(&state, PieceKind::SupplyUnit),
            count(&state, PieceKind::DummySupplyUnit),
        );
        let mut events = Vec::new();
        assert!(state.supply_arrivals(&data, &mut events));
        assert_eq!(count(&state, PieceKind::SupplyUnit), real + 7);
        assert_eq!(count(&state, PieceKind::DummySupplyUnit), dummies + 2);
        let alexandria = data.map.place("alexandria-west").unwrap().hex.clone();
        let Event::Arrived { pieces, at } = &events[0] else {
            panic!("{events:?}");
        };
        assert_eq!(*at, crate::piece::Location::Hex(alexandria));
        assert_eq!(pieces.len(), 2);
        assert!(
            pieces
                .iter()
                .all(|id| state.pieces[id].load.unwrap().fuel == 60)
        );
        assert!(state.planned.is_empty());
        // Nothing more in a later OpStage.
        state.clock.stage = 2;
        assert!(!state.supply_arrivals(&data, &mut Vec::new()));
    }

    #[test]
    fn an_axis_convoy_lands_three_in_tripoli_and_the_rest_sail_next_turn_32_6x() {
        use crate::clock::Phase;
        use crate::piece::{Location, PieceKind};
        use cna_data::scenario::OffMapBox;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_turn(&data, 5);
        state.clock.phase = Phase::Arrival;
        state.planned.clear();
        state.planned.push(Planned {
            side: Side::Axis,
            arrives: 5,
            units: 5,
        });
        let in_tripoli = |s: &GameState, k: PieceKind| {
            s.pieces
                .values()
                .filter(|p| p.kind == k && p.location == Location::Box(OffMapBox::Tripoli))
                .count()
        };
        let before = in_tripoli(&state, PieceKind::SupplyUnit);
        assert!(state.supply_arrivals(&data, &mut Vec::new()));
        assert_eq!(in_tripoli(&state, PieceKind::SupplyUnit), before + 3);
        assert_eq!(
            state.planned,
            vec![Planned {
                side: Side::Axis,
                arrives: 6,
                units: 2
            }]
        );
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
    fn axis_convoy_of(state: &mut GameState, units: u32) {
        state.clock.phase = crate::clock::Phase::Arrival;
        state.planned.clear();
        state.planned.push(Planned {
            side: Side::Axis,
            arrives: state.clock.game_turn,
            units,
        });
    }

    fn bombed(events: &[Event]) -> Vec<&Event> {
        events
            .iter()
            .filter(|e| matches!(e, Event::ConvoyBombed { .. }))
            .collect()
    }

    #[test]
    fn a_convoy_no_division_reaches_is_not_attacked_r_126() {
        let data = Data::load("grazianis-offensive").unwrap();
        let mut state = at_turn(&data, 5);
        axis_convoy_of(&mut state, 3);
        let mut events = Vec::new();
        assert!(state.supply_arrivals(&data, &mut events));
        assert!(bombed(&events).is_empty());
    }

    #[test]
    fn a_division_far_west_bombs_the_tripoli_convoy_and_dummies_count_survivors_32_6x() {
        use crate::piece::{Location, PieceKind};
        use cna_data::map::HexId;
        let data = Data::load("grazianis-offensive").unwrap();
        let mut sunk_seen = false;
        for seed in 0..40 {
            let mut state = GameState::with_modules(&data, seed, Modules::ALL).unwrap();
            crate::state::test_support::auto_place(&data, &mut state);
            state.clock = Clock::start(5);
            let hq = state
                .pieces
                .values()
                .find(|p| p.side == Side::Cw && state.is_division_hq(&data, p))
                .expect("a Commonwealth division HQ")
                .id
                .clone();
            state.put(&hq, Location::Hex(HexId::parse("A2010").unwrap()));
            axis_convoy_of(&mut state, 3);
            let count =
                |s: &GameState, k: PieceKind| s.pieces.values().filter(|p| p.kind == k).count();
            let (real0, dummy0) = (
                count(&state, PieceKind::SupplyUnit),
                count(&state, PieceKind::DummySupplyUnit),
            );
            let mut events = Vec::new();
            state.supply_arrivals(&data, &mut events);
            let hits = bombed(&events);
            assert_eq!(hits.len(), 1, "seed {seed}");
            let Event::ConvoyBombed {
                lane,
                bomb_points,
                percent,
                sunk,
                ..
            } = hits[0]
            else {
                unreachable!()
            };
            assert_eq!((*lane, *bomb_points), (2, 201));
            let expect = if *percent >= 20 {
                (3 * percent / 100).max(1)
            } else {
                0
            };
            assert_eq!(*sunk, expect, "seed {seed}");
            sunk_seen |= *sunk > 0;
            let landed = 3 - *sunk as usize;
            assert_eq!(count(&state, PieceKind::SupplyUnit), real0 + landed);
            assert_eq!(
                count(&state, PieceKind::DummySupplyUnit),
                dummy0 + landed / 3
            );
        }
        assert!(sunk_seen, "some seed sinks a supply unit");
    }

    #[test]
    fn twenty_percent_or_more_sinks_at_least_one_and_less_sinks_nothing_r_127() {
        assert_eq!(supply_sunk(3, 0), 0);
        assert_eq!(supply_sunk(3, 5), 0);
        assert_eq!(supply_sunk(3, 10), 0);
        assert_eq!(supply_sunk(3, 20), 1);
        assert_eq!(supply_sunk(3, 50), 1);
        assert_eq!(supply_sunk(10, 50), 5);
        assert_eq!(supply_sunk(0, 50), 0);
    }
}
