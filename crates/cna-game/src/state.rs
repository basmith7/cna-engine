//! The game state: everything that changes during a game, and `apply`,
//! the only way it changes. An order is validated in full before anything
//! moves; a rejected order leaves the state as it was and never enters
//! the log.

use crate::clock::Clock;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::order::{LoggedOrder, Order};
use crate::piece::{Piece, PieceId};
use crate::rng::GameRng;
use crate::setup;
use anyhow::Result;
use cna_data::map::HexId;
use cna_data::scenario::Side;
use cna_rules::ruleset::Ruleset;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The OpStage's weather (29.6): the result and, for a storm, the sheets
/// it covers (29.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Weather {
    Normal,
    Hot,
    Sandstorm { sheets: Vec<char> },
    Rainstorm { sheets: Vec<char> },
}

/// A decision one player owes before anything else may happen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pending {
    /// This side has pieces with a free placement to make (59.2).
    SetUp { side: Side },
}

impl Pending {
    pub fn side(&self) -> Side {
        match self {
            Pending::SetUp { side } => *side,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub scenario: String,
    /// The `vendor/cna` commit the game was made from.
    pub rules_commit: String,
    pub ruleset: Ruleset,
    pub clock: Clock,
    /// Who holds initiative this Game-Turn (7.1x).
    pub initiative: Option<Side>,
    /// Who acts first this OpStage.
    pub player_a: Option<Side>,
    pub weather: Weather,
    pub pieces: BTreeMap<PieceId, Piece>,
    pub pending: Option<Pending>,
    /// Movement segments completed in the current phase G (8.2x).
    #[serde(default)]
    pub cycle: u8,
    /// Hexes a piece has moved into this movement segment: where the
    /// stacking ceiling binds when it ends (9.14, 9.31).
    #[serde(default)]
    pub entered: BTreeSet<HexId>,
    /// The scenario's last OpStage is over.
    pub over: bool,
    pub rng: GameRng,
    pub log: Vec<LoggedOrder>,
}

impl GameState {
    /// A new game: the set-up expanded to pieces, the Axis to place its
    /// free pieces first (our choice: 59 and 60 do not say who sets up
    /// first), then the Commonwealth; then GT1 OpStage 1 phase A.
    pub fn new(data: &Data, seed: u64) -> Result<GameState> {
        let setup = setup::expand(&data.scenario, &data.oa, &data.chars, &data.map)?;
        let pieces = setup
            .pieces
            .into_iter()
            .map(|p| (p.id.clone(), p))
            .collect();
        let mut state = GameState {
            scenario: data.slug().to_owned(),
            rules_commit: data.rules_commit.clone(),
            ruleset: Ruleset::default(),
            clock: Clock::opening(data.scenario.start),
            initiative: None,
            player_a: None,
            weather: Weather::Normal,
            pieces,
            pending: None,
            cycle: 0,
            entered: BTreeSet::new(),
            over: false,
            rng: GameRng::new(seed),
            log: Vec::new(),
        };
        state.pending = state.next_set_up();
        if state.pending.is_none() {
            state.start_play(data);
        }
        Ok(state)
    }

    /// The player the engine is waiting on, or none when the game is over.
    pub fn to_act(&self) -> Option<Side> {
        if self.over {
            return None;
        }
        if let Some(p) = &self.pending {
            return Some(p.side());
        }
        match self.clock.half {
            None => self.initiative,
            Some(crate::clock::Half::A) => self.player_a,
            Some(crate::clock::Half::B) => self.player_a.map(Side::other),
        }
    }

    /// Applies one order from `player`. Fully validated first: on `Err`
    /// nothing has changed.
    pub fn apply(
        &mut self,
        data: &Data,
        player: Side,
        order: Order,
    ) -> Result<Vec<Event>, Illegal> {
        self.check(data).expect("the Data is this game's");
        match self.to_act() {
            None => return Err(Illegal::new("60.8", "the game is over")),
            Some(s) if s != player => {
                return Err(Illegal::new(
                    "5.2",
                    format!("{player} may not act: waiting on {s} at {}", self.clock),
                ));
            }
            Some(_) => {}
        }
        let events = self.apply_checked(data, player, &order)?;
        self.log.push(LoggedOrder { player, order });
        Ok(events)
    }

    fn apply_checked(
        &mut self,
        data: &Data,
        player: Side,
        order: &Order,
    ) -> Result<Vec<Event>, Illegal> {
        match (&self.pending, order) {
            (Some(Pending::SetUp { .. }), Order::Place { piece, hex }) => {
                self.place(data, player, piece, hex)
            }
            (Some(Pending::SetUp { side }), _) => Err(Illegal::new(
                "59.2",
                format!("{side} still has pieces to place"),
            )),
            (None, Order::Place { .. }) => {
                Err(Illegal::new("59.2", "set-up is over; nothing is pending"))
            }
            (None, Order::DeclareInitiative { player_a }) => {
                self.declare_initiative(data, *player_a)
            }
            (None, Order::Move { pieces, path }) => self.move_pieces(data, player, pieces, path),
            (None, Order::EndSegment) => self.end_segment(data),
            (None, Order::Pass) => self.pass(data),
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::piece::Location;
    use cna_data::scenario::Locator;

    /// Hexes a pending piece's placement allows, in id order (the
    /// placement area only; stacking and `not_within` are the engine's).
    pub fn candidates(data: &Data, piece: &Piece) -> Vec<HexId> {
        let Location::Pending(p) = &piece.location else {
            return Vec::new();
        };
        let map = &data.map;
        let mut v: Vec<HexId> = match &p.locator {
            Locator::Hexes(h) => h.clone(),
            Locator::Within { of, hexes } => map.within(of, *hexes).into_iter().collect(),
            Locator::Region(c) => map
                .hexes()
                .filter(|h| map.country(&h.id) == Some(*c))
                .map(|h| h.id.clone())
                .collect(),
            Locator::Sheets(s) => map
                .hexes()
                .filter(|h| s.contains(&h.sheet))
                .map(|h| h.id.clone())
                .collect(),
            _ => Vec::new(),
        };
        // The `constraint` sentences the grammar cannot check, honoured by
        // hand: Axis supply on sheet C stays in Libya; the Commonwealth's
        // on sheet D goes next to Mersa Matruh (D3714).
        if piece.side == Side::Axis {
            v.retain(|h| map.country(h) == Some(cna_data::map::Country::Libya));
        }
        if piece.side == Side::Cw && matches!(&p.locator, Locator::Sheets(s) if s == &['D']) {
            let matruh = HexId::parse("D3714").unwrap();
            v.retain(|h| map.adjacent(h, &matruh));
        }
        v.sort();
        v
    }

    /// A game with every piece placed, waiting on GT1 OpStage 1 phase A.
    pub fn set_up(data: &Data, seed: u64) -> GameState {
        let mut state = GameState::new(data, seed).unwrap();
        auto_place(data, &mut state);
        state
    }

    /// The holder declares itself Player A, then both players pass
    /// through F and G, until the OpStage changes; the events of every
    /// order, in order.
    pub fn pass_stage(data: &Data, state: &mut GameState) -> Vec<Event> {
        let (gt, stage) = (state.clock.game_turn, state.clock.stage);
        let mut events = Vec::new();
        while !state.over && (state.clock.game_turn, state.clock.stage) == (gt, stage) {
            let player = state.to_act().unwrap();
            let order = if state.clock.phase == crate::clock::Phase::InitiativeDeclaration {
                Order::DeclareInitiative { player_a: player }
            } else {
                Order::Pass
            };
            events.extend(state.apply(data, player, order).unwrap());
        }
        events
    }

    /// Places every pending piece on the first hex the engine accepts,
    /// returning the orders it took.
    pub fn auto_place(data: &Data, state: &mut GameState) -> Vec<LoggedOrder> {
        let mut orders = Vec::new();
        while let Some(Pending::SetUp { side }) = state.pending.clone() {
            let piece = state
                .pieces
                .values()
                .find(|p| p.side == side && p.is_pending())
                .expect("pending piece")
                .clone();
            let mut placed = false;
            for hex in candidates(data, &piece) {
                let order = Order::Place {
                    piece: piece.id.clone(),
                    hex,
                };
                if state.apply(data, side, order.clone()).is_ok() {
                    orders.push(LoggedOrder {
                        player: side,
                        order,
                    });
                    placed = true;
                    break;
                }
            }
            assert!(placed, "no legal hex for {}", piece.id);
        }
        orders
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Phase;
    use crate::order::Order;
    use crate::piece::Location;

    fn graziani() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn pid(s: &str) -> PieceId {
        PieceId::from(cna_data::oa::UnitId::parse(s).unwrap())
    }

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    fn place(piece: &str, hex_id: &str) -> Order {
        Order::Place {
            piece: pid(piece),
            hex: hex(hex_id),
        }
    }

    #[test]
    fn a_new_game_waits_on_the_axis_set_up() {
        let data = graziani();
        let state = GameState::new(&data, 1).unwrap();
        assert_eq!(state.pending, Some(Pending::SetUp { side: Side::Axis }));
        assert_eq!(state.pieces.values().filter(|p| p.is_pending()).count(), 59);
        assert_eq!(state.clock, Clock::start(1));
        assert_eq!(state.to_act(), Some(Side::Axis));
    }

    #[test]
    fn a_new_game_records_the_data_s_rules_commit() {
        // The commit comes with the data; the engine never asks git.
        let mut data = graziani();
        data.rules_commit = "0".repeat(40);
        let state = GameState::new(&data, 1).unwrap();
        assert_eq!(state.rules_commit, data.rules_commit);
    }

    #[test]
    fn a_place_outside_the_placement_area_is_illegal() {
        let data = graziani();
        let mut state = GameState::new(&data, 1).unwrap();
        let before = serde_json::to_string(&state).unwrap();
        // The Saharan Detachment: within 3 hexes of C0716. C0716 itself is
        // in; C0720 is four hexes away; D0101 is another sheet.
        let err = state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:saharan-detachment-hq", "C0720"),
            )
            .unwrap_err();
        assert_eq!(err.case, "59.2");
        let err = state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:saharan-detachment-hq", "D0101"),
            )
            .unwrap_err();
        assert_eq!(err.case, "59.2");
        // "Anywhere in Libya": an Egyptian hex is out.
        let err = state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:xxi-corps-artillery-regt", "D3714"),
            )
            .unwrap_err();
        assert_eq!(err.case, "59.2");
        // A hex not on the map at all.
        let err = state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:xxi-corps-artillery-regt", "C9999"),
            )
            .unwrap_err();
        assert_eq!(err.case, "59.2");
        // A piece that is not pending, or not the player's.
        let err = state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:63rd-cirene-div-hq", "C4321"),
            )
            .unwrap_err();
        assert_eq!(err.case, "59.2");
        let err = state
            .apply(&data, Side::Axis, place("unit:cw:8-med", "D3714"))
            .unwrap_err();
        assert_eq!(err.case, "59.2");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        let events = state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:saharan-detachment-hq", "C0716"),
            )
            .unwrap();
        assert_eq!(
            events,
            [Event::Placed {
                piece: pid("unit:it:saharan-detachment-hq"),
                hex: hex("C0716")
            }]
        );
        assert_eq!(
            state.pieces[&pid("unit:it:saharan-detachment-hq")].location,
            Location::Hex(hex("C0716"))
        );
        assert_eq!(state.log.len(), 1);
    }

    #[test]
    fn a_place_over_the_stacking_ceiling_is_illegal() {
        let data = graziani();
        let mut state = GameState::new(&data, 1).unwrap();
        // C4218 (clear, ceiling 6) holds the 1st CCNN division with its
        // HQ: 5 SP (9.13). One "anywhere in Libya" battalion (1 SP) fits;
        // a second does not.
        let target = hex("C4218");
        assert_eq!(state.stacked(&data, &target), 5);
        state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:1st-bn-1-arty-rgppmnto", "C4218"),
            )
            .unwrap();
        let before = serde_json::to_string(&state).unwrap();
        let err = state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:2nd-bn-1-arty-rgppmnto", "C4218"),
            )
            .unwrap_err();
        assert_eq!(err.case, "9.14");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        // The same battalion onto an empty Libyan hex is fine.
        state
            .apply(
                &data,
                Side::Axis,
                place("unit:it:2nd-bn-1-arty-rgppmnto", "C4219"),
            )
            .unwrap();
    }

    #[test]
    fn not_within_an_enemy_unit_is_checked() {
        let data = graziani();
        let mut state = GameState::new(&data, 1).unwrap();
        // Axis supply units: sheet C, not within 4 hexes of an enemy unit.
        // The Commonwealth starts at C3922 (1 KRRC and others).
        let supply = PieceId::supply(Side::Axis, 16);
        assert!(state.pieces[&supply].is_pending());
        let near = |h: &str| Order::Place {
            piece: supply.clone(),
            hex: hex(h),
        };
        let before = serde_json::to_string(&state).unwrap();
        let err = state.apply(&data, Side::Axis, near("C3922")).unwrap_err();
        assert_eq!(err.case, "59.2");
        assert!(err.reason.contains("enemy"), "{}", err.reason);
        let err = state.apply(&data, Side::Axis, near("C3722")).unwrap_err();
        assert_eq!(err.case, "59.2");
        assert!(data.map.within(&hex("C3922"), 4).contains(&hex("C3722")));
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        // Well clear of every Commonwealth unit (C3410 is Libya, sheet C).
        state.apply(&data, Side::Axis, near("C3410")).unwrap();
    }

    #[test]
    fn a_place_on_a_hex_with_no_ceiling_is_illegal() {
        let data = graziani();
        let mut state = GameState::new(&data, 1).unwrap();
        // Axis supply may go anywhere on sheet C; C5001 is sea (8.37: no
        // stacking column), and far from the Commonwealth.
        let supply = PieceId::supply(Side::Axis, 16);
        assert_eq!(data.map.hex(&hex("C5001")).unwrap().terrain.name(), "sea");
        let before = serde_json::to_string(&state).unwrap();
        let err = state
            .apply(
                &data,
                Side::Axis,
                Order::Place {
                    piece: supply,
                    hex: hex("C5001"),
                },
            )
            .unwrap_err();
        assert_eq!(err.case, "8.37", "{}", err.reason);
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
    }

    fn skipped(events: &[Event]) -> Vec<char> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::PhaseSkipped { clock } => Some(clock.phase.letter()),
                _ => None,
            })
            .collect()
    }

    fn started(events: &[Event]) -> Vec<Clock> {
        events
            .iter()
            .filter_map(|e| match e {
                Event::PhaseStarted { clock } => Some(*clock),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn in_game_turn_one_only_the_axis_declares_initiative() {
        let data = graziani();
        let mut state = test_support::set_up(&data, 3);
        assert_eq!(state.initiative, Some(Side::Axis));
        assert_eq!(state.to_act(), Some(Side::Axis));
        let before = serde_json::to_string(&state).unwrap();
        let err = state
            .apply(
                &data,
                Side::Cw,
                Order::DeclareInitiative { player_a: Side::Cw },
            )
            .unwrap_err();
        assert_eq!(err.case, "5.2");
        let err = state.apply(&data, Side::Axis, Order::Pass).unwrap_err();
        assert_eq!(err.case, "7.1x");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative { player_a: Side::Cw },
            )
            .unwrap();
        assert_eq!(state.player_a, Some(Side::Cw));
        assert_eq!(events[0], Event::InitiativeDeclared { player_a: Side::Cw });
        // Weather in B, then C, D, E skipped, then Player A's phase F.
        assert!(events.iter().any(|e| matches!(e, Event::WeatherSet { .. })));
        assert_eq!(skipped(&events), ['C', 'D', 'E']);
        let f = started(&events);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].phase, Phase::ReserveDesignation);
        assert_eq!(f[0].half, Some(crate::clock::Half::A));
        assert_eq!(state.clock, f[0]);
        assert_eq!(state.to_act(), Some(Side::Cw));
    }

    #[test]
    fn weather_is_rolled_in_phase_b_and_logged() {
        let data = graziani();
        // Seed 3's first weather reading, pinned: the dice are the first
        // two draws after set-up (set-up draws none).
        let mut state = test_support::set_up(&data, 3);
        let mut rng = state.rng.clone();
        let reading = rng.two_dice_sequential();
        let kind = data.weather.roll(1, reading).unwrap();
        let events = state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        let dice: Vec<u8> = events
            .iter()
            .filter_map(|e| match e {
                Event::DieRolled { purpose, value } if purpose == "weather" => Some(*value),
                _ => None,
            })
            .collect();
        assert_eq!(dice[..2], [reading / 10, reading % 10]);
        let set = events
            .iter()
            .find_map(|e| match e {
                Event::WeatherSet { weather } => Some(weather.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(set, state.weather);
        match (kind, &set) {
            (cna_data::weather::WeatherKind::Normal, Weather::Normal)
            | (cna_data::weather::WeatherKind::Hot, Weather::Hot) => assert_eq!(dice.len(), 2),
            (cna_data::weather::WeatherKind::Sandstorm, Weather::Sandstorm { sheets })
            | (cna_data::weather::WeatherKind::Rainstorm, Weather::Rainstorm { sheets }) => {
                assert_eq!(dice.len(), 3);
                assert_eq!(sheets, data.weather.storm_sheets(dice[2]).unwrap());
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_storm_names_its_sheets() {
        // Search seeds for an autumn storm in GT1 OpStage 1 and pin one.
        let data = graziani();
        let base = test_support::set_up(&data, 0);
        let mut found = None;
        for seed in 0..200u64 {
            let mut rng = GameRng::new(seed);
            let reading = rng.two_dice_sequential();
            if reading >= 55 {
                found = Some(seed);
                break;
            }
        }
        let seed = found.expect("a storm within 200 seeds");
        assert_eq!(seed, 1, "pinned");
        let mut state = base.clone();
        state.rng = GameRng::new(seed);
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        match &state.weather {
            Weather::Sandstorm { sheets } | Weather::Rainstorm { sheets } => {
                assert!(!sheets.is_empty());
                assert!(sheets.iter().all(|s| ('A'..='E').contains(s)));
            }
            w => panic!("expected a storm, got {w:?}"),
        }
    }

    #[test]
    fn pass_ends_f_and_g_and_the_rest_is_skipped() {
        let data = graziani();
        let mut state = test_support::set_up(&data, 3);
        state
            .apply(
                &data,
                Side::Axis,
                Order::DeclareInitiative {
                    player_a: Side::Axis,
                },
            )
            .unwrap();
        // F: Pass moves to G, movement segment.
        let events = state.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(skipped(&events), []);
        assert_eq!(state.clock.phase, Phase::MovementCombat);
        assert_eq!(state.clock.segment, Some(crate::clock::Segment::Movement));
        assert_eq!(state.to_act(), Some(Side::Axis));
        // G: Pass ends the phase; H, J, K, L are skipped; Player B's F.
        let events = state.apply(&data, Side::Axis, Order::Pass).unwrap();
        assert_eq!(skipped(&events), ['H', 'J', 'K', 'L']);
        assert_eq!(state.clock.phase, Phase::ReserveDesignation);
        assert_eq!(state.clock.half, Some(crate::clock::Half::B));
        assert_eq!(state.to_act(), Some(Side::Cw));
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(skipped(&events), ['H', 'J', 'K', 'L']);
        // Then OpStage 2's phase A, the Axis still holding initiative.
        assert_eq!(state.clock.stage, 2);
        assert_eq!(state.clock.phase, Phase::InitiativeDeclaration);
        assert_eq!(state.initiative, Some(Side::Axis));
        assert_eq!(state.to_act(), Some(Side::Axis));
    }

    #[test]
    fn from_game_turn_two_initiative_is_rolled_with_the_ratings() {
        let data = graziani();
        // GT1 OpStage 3, Player B's phase G: the next Pass opens GT2.
        let base = {
            let mut s = test_support::set_up(&data, 0);
            for _ in 0..2 {
                test_support::pass_stage(&data, &mut s);
            }
            while (s.clock.phase, s.clock.half)
                != (Phase::MovementCombat, Some(crate::clock::Half::B))
            {
                let player = s.to_act().unwrap();
                let order = if s.clock.phase == Phase::InitiativeDeclaration {
                    Order::DeclareInitiative { player_a: player }
                } else {
                    Order::Pass
                };
                s.apply(&data, player, order).unwrap();
            }
            assert_eq!((s.clock.game_turn, s.clock.stage), (1, 3));
            s
        };
        // Seeds were searched for a tie on the first pair (Commonwealth 3
        // + die equals Axis 1 + die): seed 2 is the first, pinned.
        let mut tie_seed = None;
        for seed in 0..100u64 {
            let mut rng = GameRng::new(seed);
            if 1 + rng.d6() == 3 + rng.d6() {
                tie_seed = Some(seed);
                break;
            }
        }
        let seed = tie_seed.unwrap();
        assert_eq!(seed, 2, "pinned");
        for seed in [seed, 0] {
            let mut state = base.clone();
            state.rng = GameRng::new(seed);
            let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert_eq!(state.clock, Clock::start(2));
            let dice: Vec<u8> = events
                .iter()
                .filter_map(|e| match e {
                    Event::DieRolled { purpose, value } if purpose == "initiative" => Some(*value),
                    _ => None,
                })
                .collect();
            let mut rng = GameRng::new(seed);
            let (mut axis, mut cw) = (rng.d6(), rng.d6());
            let mut expect = vec![axis, cw];
            while axis + 1 == cw + 3 {
                axis = rng.d6();
                cw = rng.d6();
                expect.extend([axis, cw]);
            }
            assert_eq!(dice, expect);
            let holder = if axis + 1 > cw + 3 {
                Side::Axis
            } else {
                Side::Cw
            };
            assert_eq!(state.initiative, Some(holder));
            assert!(events.contains(&Event::InitiativeDetermined {
                holder,
                rolled: true
            }));
            assert_eq!(state.to_act(), Some(holder));
        }
    }

    #[test]
    fn the_game_ends_after_game_turn_six() {
        let data = graziani();
        let mut state = test_support::set_up(&data, 11);
        let mut stages = 0;
        while !state.over {
            let events = test_support::pass_stage(&data, &mut state);
            stages += 1;
            if state.over {
                assert_eq!(events.last(), Some(&Event::GameOver));
            }
        }
        assert_eq!(stages, 18);
        assert_eq!(state.to_act(), None);
        let before = serde_json::to_string(&state).unwrap();
        let err = state.apply(&data, Side::Axis, Order::Pass).unwrap_err();
        assert_eq!(err.case, "60.8");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        let json = serde_json::to_string(&state).unwrap();
        let back: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
    }

    #[test]
    fn play_starts_when_nothing_is_pending() {
        let data = graziani();
        let mut state = GameState::new(&data, 1).unwrap();
        let orders = test_support::auto_place(&data, &mut state);
        assert_eq!(orders.len(), 59);
        assert!(state.pieces.values().all(|p| !p.is_pending()));
        assert_eq!(state.pending, None);
        assert_eq!(state.clock, Clock::start(1));
        assert_eq!(state.clock.phase, Phase::InitiativeDeclaration);
        assert_eq!(state.initiative, Some(Side::Axis));
        assert_eq!(state.to_act(), Some(Side::Axis));
        assert_eq!(state.log.len(), 59);
        // The Axis placed first, then the Commonwealth.
        let first_cw = orders.iter().position(|o| o.player == Side::Cw).unwrap();
        assert!(orders[..first_cw].iter().all(|o| o.player == Side::Axis));
        assert!(orders[first_cw..].iter().all(|o| o.player == Side::Cw));
    }

    #[test]
    fn an_order_from_the_wrong_player_is_illegal_and_changes_nothing() {
        let data = graziani();
        let mut state = GameState::new(&data, 7).unwrap();
        assert_eq!(state.to_act(), Some(Side::Axis));
        let before = serde_json::to_string(&state).unwrap();
        let err = state.apply(&data, Side::Cw, Order::Pass).unwrap_err();
        assert_eq!(err.case, "5.2");
        assert_eq!(serde_json::to_string(&state).unwrap(), before);
        assert!(state.log.is_empty());
    }

    #[test]
    fn a_state_with_pieces_round_trips_through_json() {
        let data = graziani();
        let state = GameState::new(&data, 7).unwrap();
        assert_eq!(state.pieces.len(), 328);
        let json = serde_json::to_string(&state).unwrap();
        let back: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["scenario"], "grazianis-offensive");
        assert_eq!(v["rules_commit"].as_str().unwrap().len(), 40);
        assert_eq!(v["ruleset"]["R-001"], 1);
        assert_eq!(v["rng"]["seed"], 7);
    }
}
