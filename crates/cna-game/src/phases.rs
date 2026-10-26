//! The sequence of play as the state walks it: initiative (7.1x), the
//! initiative declaration and weather (phases A and B), and the phases
//! this mission passes through. `advance` runs the clock from wherever it
//! stands to the next moment that waits for an order.

use crate::clock::Phase;
use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceKind};
use crate::state::{GameState, Weather};
use cna_data::cp::Cp;
use cna_data::initiative::AxisSituation;
use cna_data::oa::Nation;
use cna_data::scenario::Side;
use cna_data::time::GameTime;
use cna_data::weather::WeatherKind;

impl GameState {
    /// The first moment of play after set-up.
    pub(crate) fn start_play(&mut self, data: &Data) -> Vec<Event> {
        let mut events = Vec::new();
        self.advance(data, &mut events);
        events
    }

    /// Phase A: the holder names Player A (7.1x).
    pub(crate) fn declare_initiative(
        &mut self,
        data: &Data,
        player_a: Side,
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != Phase::InitiativeDeclaration {
            return Err(Illegal::new(
                "7.1x",
                format!("initiative is declared in phase A, not at {}", self.clock),
            ));
        }
        self.player_a = Some(player_a);
        let mut events = vec![Event::InitiativeDeclared { player_a }];
        self.clock = self.clock.next();
        self.advance(data, &mut events);
        Ok(events)
    }

    /// Nothing more this phase (F or G).
    pub(crate) fn pass(&mut self, data: &Data) -> Result<Vec<Event>, Illegal> {
        match self.clock.phase {
            Phase::ReserveDesignation
            | Phase::MovementCombat
            | Phase::TruckConvoy
            | Phase::Rail
            | Phase::Repair => {}
            Phase::Organisation => {
                // Player A hands phase C to Player B; B's pass ends it.
                if !self.organised {
                    self.organised = true;
                    return Ok(Vec::new());
                }
                self.organised = false;
                let mut events = Vec::new();
                self.clock = self.clock.next();
                self.advance(data, &mut events);
                return Ok(events);
            }
            Phase::InitiativeDeclaration => {
                return Err(Illegal::new(
                    "7.1x",
                    "the initiative holder must declare who is Player A",
                ));
            }
            _ => {
                return Err(Illegal::new(
                    "5.2",
                    format!("nothing to pass at {}", self.clock),
                ));
            }
        }
        if self.clock.segment == Some(crate::clock::Segment::CloseAssault) {
            return self.end_close_assault_step(data);
        }
        let mut events = Vec::new();
        self.before_leaving_g(data, &mut events)?;
        if self.clock.phase == Phase::TruckConvoy {
            self.escape_unguarded(data, &mut events);
        }
        self.cycle = 0;
        self.continuing.clear();
        self.clock = self.clock.end_phase();
        self.advance(data, &mut events);
        Ok(events)
    }

    /// Runs the clock on until a phase that waits for a player (A, F or
    /// G) or the end of the scenario.
    fn advance(&mut self, data: &Data, events: &mut Vec<Event>) {
        loop {
            let clock = self.clock;
            if GameTime::new(clock.game_turn, clock.stage) > data.scenario.end {
                let report = crate::victory::evaluate(data, self);
                self.over = Some(report.clone());
                events.push(Event::GameOver { report });
                return;
            }
            match clock.phase {
                Phase::InitiativeDeclaration => {
                    self.count_construction(data, events);
                    self.start_stage();
                    if clock.stage == 1 {
                        self.naval_convoy_stage(data, events);
                    }
                    self.determine_initiative(data, events);
                    events.push(Event::PhaseStarted { clock });
                    return;
                }
                Phase::Weather => self.roll_weather(data, events),
                Phase::ReserveDesignation => {
                    events.push(Event::PhaseStarted { clock });
                    return;
                }
                Phase::MovementCombat => {
                    events.push(Event::PhaseStarted { clock });
                    self.open_movement_segment(data, events);
                    return;
                }
                Phase::Arrival => {
                    if !self.arrive(data, events) {
                        events.push(Event::PhaseSkipped { clock });
                    }
                }
                // Phase C opens for supply under the module, and for
                // repaired points waiting to be absorbed (20.41).
                Phase::Organisation
                    if self.modules.abstract_supply
                        || self.modules.construction
                        || !self.repaired.is_empty() =>
                {
                    events.push(Event::PhaseStarted { clock });
                    return;
                }
                Phase::TruckConvoy
                    if {
                        self.marched.clear();
                        self.trucks_to_move() || self.prisoners_to_handle()
                    } =>
                {
                    events.push(Event::PhaseStarted { clock });
                    return;
                }
                Phase::Rail if self.rail_to_move(data) => {
                    events.push(Event::PhaseStarted { clock });
                    return;
                }
                Phase::Repair if self.repairs_to_try(data) => {
                    events.push(Event::PhaseStarted { clock });
                    return;
                }
                Phase::Organisation
                | Phase::CwFleet
                | Phase::TruckConvoy
                | Phase::Rail
                | Phase::Repair
                | Phase::Patrol => events.push(Event::PhaseSkipped { clock }),
            }
            self.clock = self.clock.next();
        }
    }

    /// The bookkeeping between OpStages: unspent CP are lost (6.16), a
    /// unit that spent none rests (6.24), reserve ends (18.2x), and
    /// engaged markers come off (15.81).
    fn start_stage(&mut self) {
        self.railed.clear();
        self.field_rolled.clear();
        self.rail_east = None;
        for p in self.pieces.values_mut() {
            if p.cp_spent == Cp::ZERO {
                p.cohesion.rest();
            }
            p.cp_spent = Cp::ZERO;
            p.cp_enemy_half = Cp::ZERO;
            p.reserve = None;
            p.released = None;
            p.landed = false;
            p.flown = false;
            p.fuelled = false;
            p.escorted = false;
            p.breakdown_halves = 0;
            p.breakdown_checked = None;
            p.fuelled_over = false;
            p.engaged = false;
        }
    }

    /// Who holds initiative this Game-Turn (7.1x): the scenario's holder
    /// while its grant lasts, else one die plus the rating each, ties
    /// re-rolled. Decided at the first OpStage of a Game-Turn, and again
    /// at the OpStage where a scenario's grant runs out mid-turn.
    fn determine_initiative(&mut self, data: &Data, events: &mut Vec<Event>) {
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        let granted = |t: GameTime| {
            data.scenario
                .initiative
                .filter(|i| t <= i.through)
                .map(|i| i.side)
        };
        if let Some(side) = granted(now) {
            if self.initiative != Some(side) || now.opstage == 1 {
                self.initiative = Some(side);
                events.push(Event::InitiativeDetermined {
                    holder: side,
                    rolled: false,
                });
            }
            return;
        }
        let grant_just_ended =
            now.opstage > 1 && granted(GameTime::new(now.game_turn, now.opstage - 1)).is_some();
        if now.opstage != 1 && !grant_just_ended && self.initiative.is_some() {
            return;
        }
        let axis_rating = data
            .initiative
            .axis(self.axis_situation())
            .expect("initiative-ratings.json has every situation");
        let cw_rating = data
            .initiative
            .commonwealth(now.game_turn)
            .expect("initiative-ratings.json covers the scenario's turns");
        let holder = loop {
            let axis = self.rng.d6();
            events.push(Event::DieRolled {
                purpose: "initiative".into(),
                value: axis,
            });
            let cw = self.rng.d6();
            events.push(Event::DieRolled {
                purpose: "initiative".into(),
                value: cw,
            });
            match (axis + axis_rating).cmp(&(cw + cw_rating)) {
                std::cmp::Ordering::Greater => break Side::Axis,
                std::cmp::Ordering::Less => break Side::Cw,
                std::cmp::Ordering::Equal => continue,
            }
        };
        self.initiative = Some(holder);
        events.push(Event::InitiativeDetermined {
            holder,
            rolled: true,
        });
    }

    /// German land combat units on the map, holding boxes excepted
    /// (7.1x). Rommel is not on any OA sheet yet, so his row is never
    /// reached.
    fn axis_situation(&self) -> AxisSituation {
        let germans = self.pieces.values().any(|p| {
            p.nation == Some(Nation::De)
                && p.kind == PieceKind::Combat
                && matches!(p.location, Location::Hex(_))
        });
        if germans {
            AxisSituation::GermanLandCombatUnitsNoRommel
        } else {
            AxisSituation::NoGermanCombatUnits
        }
    }

    /// Phase B (29.6): two dice read sequentially on the season's row;
    /// a storm rolls one die more for the sheets it covers (29.7).
    fn roll_weather(&mut self, data: &Data, events: &mut Vec<Event>) {
        let a = self.rng.d6();
        let b = self.rng.d6();
        for v in [a, b] {
            events.push(Event::DieRolled {
                purpose: "weather".into(),
                value: v,
            });
        }
        let kind = data
            .weather
            .roll(self.clock.game_turn, a * 10 + b)
            .expect("weather.json covers every reading of the scenario's turns");
        let sheets = |state: &mut GameState, events: &mut Vec<Event>| {
            let die = state.rng.d6();
            events.push(Event::DieRolled {
                purpose: "storm location".into(),
                value: die,
            });
            data.weather
                .storm_sheets(die)
                .expect("foul-weather-location.json has every die")
                .to_vec()
        };
        self.weather = match kind {
            WeatherKind::Normal => Weather::Normal,
            WeatherKind::Hot => Weather::Hot,
            WeatherKind::Sandstorm => Weather::Sandstorm {
                sheets: sheets(self, events),
            },
            WeatherKind::Rainstorm => Weather::Rainstorm {
                sheets: sheets(self, events),
            },
        };
        events.push(Event::WeatherSet {
            weather: self.weather.clone(),
        });
    }
}
