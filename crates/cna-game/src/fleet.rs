//! The Commonwealth's Mediterranean Fleet (SPI 30.1x, cna
//! `rules/90-special.md`, *The Mediterranean Fleet*), behind the `fleet`
//! module. Each warship of `cw-fleet-schedule.json` arrives in Alexandria
//! in the OpStage the schedule gives. In phase E the Commonwealth places
//! the ships it chooses at sea (`Order::Station`) and passes; a ship not
//! placed is in port that OpStage. A ship may be out three OpStages in a
//! row at most, then spends two in port for each it was out.
//!
//! Where a ship may be: a sea hex, or a coastal one, reached by sea from
//! Alexandria and no farther west than hex row xx29 on sheet B (R-019:
//! the row is the operative limit, and is where the 100-sea-hex count
//! ends).
//!
//! Off-shore bombardment (30.2x): a ship at sea fires once an OpStage,
//! in a barrage step of the Commonwealth's choosing (`Order::Barrage`
//! naming the ship as a firer), its gun rating as actual barrage points
//! needing no ammunition, at an enemy unit in its own coastal hex; a
//! battleship or heavy cruiser (cna's "4-point ships", by class) may
//! instead fire at one in an adjacent hex at half its guns, a half point
//! rounding up (as R-121 rounds a halved point; only a damaged ship has
//! an odd rating). A ship that fired comes back to Alexandria and spends
//! the next two OpStages there. Strikes on the fleet, damage, repair and
//! the Valletta shuttle are not simulated yet (`NOT_SIMULATED.md`).

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, Piece, PieceId, PieceKind, Port};
use crate::state::GameState;
use cna_data::map::{HexId, Map, Terrain};
use cna_data::scenario::Side;
use cna_data::time::GameTime;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Alexandria's two hexes: the sea hexes touching them are 1 sea hex
/// out (R-019's count). Alexandria is not in `places.json`.
const ALEXANDRIA: [&str; 2] = ["E3613", "E3714"];

/// The farthest west a ship goes: hex row xx29 on sheet B (R-019).
const WESTERN_SHEET: char = 'B';
const WESTERN_ROW: u8 = 29;

/// Most OpStages a ship may spend at sea in a row (30.1x).
pub const MAX_AT_SEA: u8 = 3;

/// OpStages in port owed for each OpStage at sea (30.1x).
pub const PORT_PER_SEA: u8 = 2;

/// OpStages in Alexandria owed by a ship that fired (30.25).
pub const PORT_AFTER_FIRING: u8 = 2;

/// One ship's port record (30.1x).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortRecord {
    /// OpStages at sea in a row, this one included once it is stationed
    /// and phase E closes.
    #[serde(default)]
    pub at_sea: u8,
    /// OpStages in port still owed before it may sail.
    #[serde(default)]
    pub owed: u8,
    /// Stationed in this phase E.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stationed: bool,
    /// Bombarded this OpStage (30.25): it fires no more, and goes back to
    /// Alexandria for the next two.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fired: bool,
}

/// A ship's piece id: `ship:cw:<name>`.
pub fn ship_id(name: &str) -> PieceId {
    PieceId::from(format!("ship:cw:{name}"))
}

/// The schedule's row for a ship's piece.
fn ship_row<'a>(data: &'a Data, id: &PieceId) -> Option<&'a cna_data::fleet::Ship> {
    data.fleet.rows.iter().find(|r| ship_id(&r.ship) == *id)
}

/// Battleships and heavy cruisers, cna's "4-point ships", reach an
/// adjacent hex (30.2x).
fn reaches_adjacent(class: &str) -> bool {
    matches!(class, "BB" | "CA")
}

/// Sea distance from Alexandria of every sea hex reached by sea, the
/// sea hexes touching Alexandria being 1 (R-019).
fn sea_distances(map: &Map) -> BTreeMap<HexId, u32> {
    let is_sea = |h: &HexId| map.hex(h).is_some_and(|x| x.terrain == Terrain::Sea);
    let mut dist = BTreeMap::new();
    let mut queue = VecDeque::new();
    for a in ALEXANDRIA {
        let a = HexId::parse(a).expect("a hex id");
        for n in map.neighbours(&a) {
            if is_sea(&n) && !dist.contains_key(&n) {
                dist.insert(n.clone(), 1);
                queue.push_back(n);
            }
        }
    }
    while let Some(h) = queue.pop_front() {
        let d = dist[&h];
        for n in map.neighbours(&h) {
            if is_sea(&n) && !dist.contains_key(&n) {
                dist.insert(n.clone(), d + 1);
                queue.push_back(n);
            }
        }
    }
    dist
}

/// Why a ship may not be placed in `hex`, if it may not (30.1x, R-019).
fn station_refusal(map: &Map, hex: &HexId) -> Option<Illegal> {
    let Some(h) = map.hex(hex) else {
        return Some(Illegal::new("30.1x", format!("{hex}: not on the map")));
    };
    if h.terrain != Terrain::Sea && !h.coastal {
        return Some(Illegal::new(
            "30.1x",
            format!("{hex}: a ship is placed in a sea or coastal hex"),
        ));
    }
    if hex.sheet() < WESTERN_SHEET || (hex.sheet() == WESTERN_SHEET && hex.col() < WESTERN_ROW) {
        return Some(Illegal::new(
            "R-019",
            format!(
                "{hex}: west of hex row xx{WESTERN_ROW} on sheet {WESTERN_SHEET}, out of naval range"
            ),
        ));
    }
    let dist = sea_distances(map);
    let reached = if h.terrain == Terrain::Sea {
        dist.contains_key(hex)
    } else {
        map.neighbours(hex).iter().any(|n| dist.contains_key(n))
    };
    if !reached {
        return Some(Illegal::new(
            "30.1x",
            format!("{hex}: not reached by sea from Alexandria"),
        ));
    }
    None
}

impl GameState {
    /// The warships the schedule has deployed by now and not yet in
    /// play land in Alexandria (30.59). A scenario starting later has
    /// every ship deployed by its start.
    fn ships_arrive(&mut self, data: &Data, events: &mut Vec<Event>) {
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        let mut ids = Vec::new();
        for row in data.fleet.rows.iter().filter(|r| r.arrives() <= now) {
            let id = ship_id(&row.ship);
            if self.pieces.contains_key(&id) {
                continue;
            }
            let p = crate::setup::supply_piece(
                Side::Cw,
                PieceKind::Ship,
                id.clone(),
                Location::Port(Port::Alexandria),
                None,
            );
            self.pieces.insert(id.clone(), p);
            self.fleet.insert(id.clone(), PortRecord::default());
            ids.push(id);
        }
        if !ids.is_empty() {
            events.push(Event::Arrived {
                pieces: ids,
                at: Location::Port(Port::Alexandria),
            });
        }
    }

    /// Whether `ship` may be placed at sea this phase E (30.1x): in port
    /// with no port time owed, or at sea fewer than three OpStages in a
    /// row.
    fn free_to_sail(&self, ship: &PieceId) -> bool {
        let Some(r) = self.fleet.get(ship) else {
            return false;
        };
        match self.pieces.get(ship).map(|p| &p.location) {
            Some(Location::Port(_)) => r.owed == 0,
            Some(Location::Hex(_)) => !r.fired && r.at_sea < MAX_AT_SEA,
            _ => false,
        }
    }

    /// Phase E opens: with the module on, ships due arrive, and the phase
    /// waits for the Commonwealth when some ship is free to sail.
    pub(crate) fn fleet_phase_opens(&mut self, data: &Data, events: &mut Vec<Event>) -> bool {
        if !self.modules.fleet {
            return false;
        }
        self.ships_arrive(data, events);
        let ids: Vec<PieceId> = self.fleet.keys().cloned().collect();
        ids.iter().any(|id| self.free_to_sail(id))
    }

    /// Phase E closes: a ship placed this phase spends the OpStage at
    /// sea; any other goes back to (or stays in) Alexandria and serves a
    /// stage of its port time, owing two for each stage it was out.
    pub(crate) fn close_fleet_phase(&mut self, events: &mut Vec<Event>) {
        if !self.modules.fleet {
            return;
        }
        let mut back = Vec::new();
        for (id, r) in self.fleet.iter_mut() {
            if r.stationed {
                r.stationed = false;
                r.at_sea += 1;
                continue;
            }
            let p = self
                .pieces
                .get_mut(id)
                .expect("a ship's record has its piece");
            if matches!(p.location, Location::Hex(_)) {
                p.location = Location::Port(Port::Alexandria);
                r.owed = (PORT_PER_SEA * r.at_sea).max(if r.fired { PORT_AFTER_FIRING } else { 0 });
                r.fired = false;
                r.at_sea = 0;
                back.push(id.clone());
            }
            r.owed = r.owed.saturating_sub(1);
        }
        if !back.is_empty() {
            events.push(Event::InPort { ships: back });
        }
    }

    /// Phase E: `ship` goes to sea in `hex` this OpStage (30.1x).
    pub(crate) fn station(
        &mut self,
        data: &Data,
        player: Side,
        ship: &PieceId,
        hex: &HexId,
    ) -> Result<Vec<Event>, Illegal> {
        if self.clock.phase != crate::clock::Phase::CwFleet {
            return Err(Illegal::new(
                "30.1x",
                format!("ships are placed in phase E, not at {}", self.clock),
            ));
        }
        if player != Side::Cw || !self.fleet.contains_key(ship) {
            return Err(Illegal::new(
                "30.1x",
                format!("{ship} is not a Commonwealth ship in play"),
            ));
        }
        let r = self.fleet[ship];
        if !r.stationed && !self.free_to_sail(ship) {
            return Err(if r.fired {
                Illegal::new(
                    "30.25",
                    format!(
                        "{ship} fired: it spends the next {PORT_AFTER_FIRING} OpStages in Alexandria"
                    ),
                )
            } else if r.at_sea >= MAX_AT_SEA {
                Illegal::new(
                    "30.1x",
                    format!("{ship} has been at sea {MAX_AT_SEA} OpStages in a row"),
                )
            } else {
                Illegal::new(
                    "30.1x",
                    format!("{ship} owes {} more OpStages in port", r.owed),
                )
            });
        }
        if let Some(e) = station_refusal(&data.map, hex) {
            return Err(e);
        }
        self.pieces.get_mut(ship).expect("checked").location = Location::Hex(hex.clone());
        self.fleet.get_mut(ship).expect("checked").stationed = true;
        Ok(vec![Event::Stationed {
            ship: ship.clone(),
            hex: hex.clone(),
        }])
    }

    /// The actual barrage points `ship` fires at an enemy unit in `at`
    /// (30.2x): its guns into its own coastal hex, or, a battleship or
    /// heavy cruiser, half of them (a half point up) into an adjacent hex.
    pub(crate) fn bombardment_points(
        &self,
        data: &Data,
        ship: &Piece,
        at: &HexId,
    ) -> Result<u32, Illegal> {
        let row = ship_row(data, &ship.id).filter(|_| self.modules.fleet);
        let (Some(row), Some(r)) = (row, self.fleet.get(&ship.id)) else {
            return Err(Illegal::new(
                "30.2x",
                format!("{}: not a ship in play", ship.id),
            ));
        };
        if r.fired {
            return Err(Illegal::new(
                "30.25",
                format!(
                    "{} has fired this OpStage: a ship fires once a stage",
                    ship.id
                ),
            ));
        }
        let Some(from) = ship.hex() else {
            return Err(Illegal::new(
                "30.2x",
                format!(
                    "{} is in port: a ship fires from the hex it is placed in",
                    ship.id
                ),
            ));
        };
        if from == at {
            if data.map.hex(at).is_some_and(|h| h.coastal) {
                return Ok(row.gun_rating);
            }
            return Err(Illegal::new(
                "30.2x",
                format!("{at}: a ship bombards a coastal hex"),
            ));
        }
        if !data.map.adjacent(from, at) {
            return Err(Illegal::new(
                "30.2x",
                format!(
                    "{at} is neither {}'s hex {from} nor adjacent to it",
                    ship.id
                ),
            ));
        }
        if !reaches_adjacent(&row.class) {
            return Err(Illegal::new(
                "30.2x",
                format!(
                    "{} is a {}: only battleships and heavy cruisers bombard an adjacent hex",
                    ship.id, row.class
                ),
            ));
        }
        Ok(row.gun_rating.div_ceil(2))
    }

    /// Whether some ship may bombard an enemy unit now (30.2x): opens a
    /// barrage step that has no land fire in it.
    pub(crate) fn ships_may_fire(&self, data: &Data) -> bool {
        if !self.modules.fleet {
            return false;
        }
        self.fleet.keys().any(|id| {
            let ship = &self.pieces[id];
            self.pieces.values().any(|t| {
                t.side != ship.side
                    && self.is_target(data, t)
                    && t.hex()
                        .is_some_and(|at| self.bombardment_points(data, ship, at).is_ok())
            })
        })
    }

    /// The ships among a barrage plot are marked as having fired (30.25).
    pub(crate) fn ships_fired(&mut self, firers: &[PieceId]) {
        for id in firers {
            if let Some(r) = self.fleet.get_mut(id) {
                r.fired = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Phase;
    use crate::order::Order;
    use crate::raiders::tests::graziani;
    use crate::state::Modules;
    use crate::state::test_support::{auto_place, pass_stage};

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    fn fleet_on() -> Modules {
        Modules {
            fleet: true,
            ..Modules::default()
        }
    }

    /// Plays on, passing, to the next phase E the engine stops at.
    fn to_phase_e(data: &Data, state: &mut GameState) {
        for _ in 0..12 {
            if state.clock.phase == Phase::CwFleet {
                return;
            }
            if state.clock.phase == Phase::InitiativeDeclaration {
                let p = state.to_act().unwrap();
                state
                    .apply(data, p, Order::DeclareInitiative { player_a: p })
                    .unwrap();
                continue;
            }
            pass_stage(data, state);
        }
        panic!("no phase E at {}", state.clock);
    }

    /// Graziani's set-up with `modules`, placed, at phase A of GT1/1.
    fn set_up(data: &Data, modules: Modules) -> GameState {
        let mut state = GameState::with_modules(data, 7, modules).unwrap();
        auto_place(data, &mut state);
        state
    }

    fn station(ship: &PieceId, at: &str) -> Order {
        Order::Station {
            ship: ship.clone(),
            hex: hex(at),
        }
    }

    /// A sea hex next to Alexandria, 1 sea hex out.
    fn off_alexandria(data: &Data) -> HexId {
        sea_distances(&data.map)
            .into_iter()
            .find(|(_, d)| *d == 1)
            .unwrap()
            .0
    }

    #[test]
    fn the_schedules_first_ships_arrive_in_alexandria_30_59() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        let ships: Vec<_> = state
            .pieces
            .values()
            .filter(|p| p.kind == PieceKind::Ship)
            .collect();
        assert_eq!(ships.len(), 11, "GT1/1's eleven ships");
        assert!(ships.iter().all(|p| p.side == Side::Cw
            && p.location == Location::Port(Port::Alexandria)
            && p.equivalent.is_none()
            && !p.kind.holds_ground()));
        assert!(state.pieces.contains_key(&ship_id("valiant")));
        assert!(!state.pieces.contains_key(&ship_id("barham")), "GT8/3");
        assert_eq!(state.fleet.len(), 11);
    }

    #[test]
    fn later_ships_arrive_by_the_schedule_30_59() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        state.clock.game_turn = 8;
        state.clock.stage = 3;
        let mut events = Vec::new();
        assert!(state.fleet_phase_opens(&data, &mut events));
        assert_eq!(
            state.fleet.len(),
            16,
            "Barham, York, Ajax, Marne, Partridge"
        );
        assert!(state.pieces.contains_key(&ship_id("barham")));
    }

    #[test]
    fn without_the_module_there_are_no_ships_and_phase_e_is_skipped() {
        let data = graziani();
        let mut state = set_up(&data, Modules::default());
        let p = state.to_act().unwrap();
        let events = state
            .apply(&data, p, Order::DeclareInitiative { player_a: p })
            .unwrap();
        assert!(events.iter().any(|e| matches!(e,
            Event::PhaseSkipped { clock } if clock.phase == Phase::CwFleet)));
        assert!(state.fleet.is_empty());
        assert!(state.pieces.values().all(|p| p.kind != PieceKind::Ship));
    }

    #[test]
    fn phase_e_opens_with_the_module_and_waits_for_the_commonwealth() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        assert_eq!(state.clock.phase, Phase::CwFleet);
        assert_eq!(state.to_act(), Some(Side::Cw));
        let err = state.apply(&data, Side::Axis, Order::Pass).unwrap_err();
        assert_eq!(err.case, "5.2");
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(state.clock.phase, Phase::ReserveDesignation);
    }

    #[test]
    fn phase_e_is_skipped_when_no_ship_is_free_to_sail() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        for r in state.fleet.values_mut() {
            r.owed = 4;
        }
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        pass_stage(&data, &mut state);
        let p = state.to_act().unwrap();
        let events = state
            .apply(&data, p, Order::DeclareInitiative { player_a: p })
            .unwrap();
        assert!(events.iter().any(|e| matches!(e,
            Event::PhaseSkipped { clock } if clock.phase == Phase::CwFleet)));
        assert!(state.fleet.values().all(|r| r.owed == 2), "a stage served");
    }

    #[test]
    fn a_station_west_of_b_xx29_or_off_the_sea_is_refused_r_019() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        let valiant = ship_id("valiant");
        let dist = sea_distances(&data.map);
        // The westernmost sea hexes allowed and the first refused.
        let on_row = |col: u8| {
            dist.keys()
                .find(|h| h.sheet() == 'B' && h.col() == col)
                .unwrap()
                .clone()
        };
        let west = on_row(28);
        let err = state
            .apply(&data, Side::Cw, station(&valiant, west.as_str()))
            .unwrap_err();
        assert_eq!(err.case, "R-019");
        let derna_sea = data
            .map
            .neighbours(&hex("B5925"))
            .into_iter()
            .find(|h| dist.contains_key(h))
            .unwrap();
        assert!(
            state
                .apply(&data, Side::Cw, station(&valiant, derna_sea.as_str()))
                .is_err()
        );
        // Sheet A is beyond the range altogether.
        let a = data
            .map
            .hexes()
            .find(|h| h.sheet == 'A' && h.terrain == Terrain::Sea)
            .unwrap()
            .id
            .clone();
        assert!(
            state
                .apply(&data, Side::Cw, station(&valiant, a.as_str()))
                .is_err()
        );
        // Cairo is inland.
        let err = state
            .apply(&data, Side::Cw, station(&valiant, "E1730"))
            .unwrap_err();
        assert_eq!(err.case, "30.1x");
        let edge = on_row(29);
        let events = state
            .apply(&data, Side::Cw, station(&valiant, edge.as_str()))
            .unwrap();
        assert_eq!(
            events,
            vec![Event::Stationed {
                ship: valiant.clone(),
                hex: edge.clone()
            }]
        );
        // A coastal hex in range: Sidi Barrani.
        state
            .apply(&data, Side::Cw, station(&ship_id("dido"), "C4131"))
            .unwrap();
        assert_eq!(state.pieces[&valiant].hex(), Some(&edge));
    }

    #[test]
    fn row_xx29_lies_about_100_sea_hexes_out_r_019() {
        let data = graziani();
        let dist = sea_distances(&data.map);
        let nearest = |col: u8| {
            dist.iter()
                .filter(|(h, _)| h.sheet() == 'B' && h.col() == col)
                .map(|(_, d)| *d)
                .min()
                .unwrap()
        };
        // R-019's count: row xx30 at 100, xx29 at 101; the row is the limit.
        assert!((98..=103).contains(&nearest(29)), "{}", nearest(29));
        assert!(nearest(25) > 100);
    }

    #[test]
    fn the_fourth_stage_at_sea_in_a_row_is_refused_30_1x() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        let valiant = ship_id("valiant");
        let sea = off_alexandria(&data);
        for _ in 0..3 {
            to_phase_e(&data, &mut state);
            state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap();
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
        }
        assert_eq!(state.fleet[&valiant].at_sea, 3);
        to_phase_e(&data, &mut state);
        let err = state
            .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
            .unwrap_err();
        assert_eq!(err.case, "30.1x");
        assert!(err.reason.contains("at sea 3"), "{}", err.reason);
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(
            state.pieces[&valiant].location,
            Location::Port(Port::Alexandria)
        );
    }

    #[test]
    fn a_ship_back_from_three_stages_out_may_not_sail_for_six_30_1x() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        let valiant = ship_id("valiant");
        let sea = off_alexandria(&data);
        for _ in 0..3 {
            to_phase_e(&data, &mut state);
            state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap();
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
        }
        // Stages 4 to 9 in port; the other ships keep phase E open.
        for stage in 4..=9 {
            to_phase_e(&data, &mut state);
            let err = state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap_err();
            assert_eq!(err.case, "30.1x", "stage {stage}");
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
        }
        to_phase_e(&data, &mut state);
        assert_eq!((state.clock.game_turn, state.clock.stage), (4, 1));
        state
            .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
            .unwrap();
    }

    #[test]
    fn a_ship_one_stage_out_owes_two_in_port_30_1x() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        let valiant = ship_id("valiant");
        let sea = off_alexandria(&data);
        to_phase_e(&data, &mut state);
        state
            .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
            .unwrap();
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        // Stage 2: not placed, it is back in port, the first of its two.
        to_phase_e(&data, &mut state);
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert_eq!(
            state.pieces[&valiant].location,
            Location::Port(Port::Alexandria)
        );
        to_phase_e(&data, &mut state);
        let err = state
            .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
            .unwrap_err();
        assert!(err.reason.contains("owes 1"), "{}", err.reason);
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        to_phase_e(&data, &mut state);
        state
            .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
            .unwrap();
    }

    #[test]
    fn a_ship_is_stationed_only_in_phase_e_by_the_commonwealth() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        let sea = off_alexandria(&data);
        let mut later = state.clone();
        later.apply(&data, Side::Cw, Order::Pass).unwrap();
        let p = later.to_act().unwrap();
        let err = later
            .apply(&data, p, station(&ship_id("valiant"), sea.as_str()))
            .unwrap_err();
        assert_eq!(err.case, "30.1x");
        let err = state
            .apply(&data, Side::Cw, station(&ship_id("no-such"), sea.as_str()))
            .unwrap_err();
        assert_eq!(err.case, "30.1x");
    }

    #[test]
    fn ships_and_their_stations_are_open_to_the_axis_3_61() {
        use crate::view::{Aliases, events_for, leaks, view_for};
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        let valiant = ship_id("valiant");
        let sea = off_alexandria(&data);
        let events = state
            .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
            .unwrap();
        let aliases = Aliases::new([1; 32]);
        let seen = events_for(&state, &aliases, &events, Some(Side::Axis), false);
        assert_eq!(seen[0]["stationed"]["hex"], serde_json::json!(sea.as_str()));
        let v = view_for(&state, &data, &aliases, Some(Side::Axis), false).unwrap();
        assert_eq!(
            v["pieces"][valiant.as_str()]["location"],
            serde_json::json!({ "hex": sea.as_str() })
        );
        assert_eq!(v["fleet"][valiant.as_str()]["at_sea"], serde_json::json!(0));
        assert_eq!(leaks(&state, &v, Side::Cw), Vec::<String>::new());
    }

    /// A ship off a coast the Axis holds is no lone piece to capture: it
    /// holds no ground (30.1x), so an enemy ZOC never takes it.
    #[test]
    fn a_ship_beside_the_axis_is_never_captured() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        let dido = ship_id("dido");
        state
            .apply(&data, Side::Cw, station(&dido, "C4221"))
            .unwrap();
        let events = pass_stage(&data, &mut state);
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::Captured { piece, .. } if *piece == dido)),
            "{events:?}"
        );
        assert_ne!(state.pieces[&dido].location, Location::Eliminated);
    }

    mod bombardment {
        use super::*;
        use crate::combat::segment::test_support::at_sollum;
        use crate::movement::test_support::{moving, pid};
        use crate::order::Fire;

        const TARGET: &str = "unit:it:viii-libyan-bn";

        /// Sollum's combat with the fleet in play: the ships in
        /// Alexandria, the gun positions answered, the barrage step open.
        fn at_barrage(data: &Data) -> GameState {
            let mut state = at_sollum(data);
            state.modules.fleet = true;
            state.ships_arrive(data, &mut Vec::new());
            for side in [Side::Cw, Side::Axis] {
                state
                    .apply(data, side, Order::GunPositions { forward: vec![] })
                    .unwrap();
            }
            assert_eq!(state.clock.segment, Some(crate::clock::Segment::Barrage));
            // The target steps onto the coast beside Sollum (C4021).
            let coast = data
                .map
                .neighbours(&hex("C4021"))
                .into_iter()
                .find(|n| {
                    data.map.hex(n).unwrap().coastal
                        && data
                            .map
                            .neighbours(n)
                            .iter()
                            .any(|m| data.map.hex(m).unwrap().terrain == Terrain::Sea)
                })
                .unwrap();
            state.pieces.get_mut(&pid(TARGET)).unwrap().location = Location::Hex(coast);
            state
        }

        fn target_hex(state: &GameState) -> HexId {
            state.pieces[&pid(TARGET)].hex().unwrap().clone()
        }

        /// A sea hex beside the target's coastal hex.
        fn offshore(data: &Data, at: &HexId) -> HexId {
            data.map
                .neighbours(at)
                .into_iter()
                .find(|n| data.map.hex(n).unwrap().terrain == Terrain::Sea)
                .expect("the target's hex is on the coast")
        }

        fn put(state: &mut GameState, ship: &str, at: &HexId) -> PieceId {
            let id = ship_id(ship);
            state.pieces.get_mut(&id).unwrap().location = Location::Hex(at.clone());
            id
        }

        fn fire(ship: &PieceId, points: u32) -> Order {
            Order::Barrage {
                fires: vec![Fire {
                    piece: ship.clone(),
                    points,
                    target: pid(TARGET),
                }],
            }
        }

        /// The Commonwealth's plot, then the Axis's empty one: the barrage
        /// rolled. Returns the ship's barrage's points.
        fn resolve(data: &Data, state: &mut GameState, ship: &PieceId, points: u32) -> u32 {
            state.apply(data, Side::Cw, fire(ship, points)).unwrap();
            let events = state
                .apply(data, Side::Axis, Order::Barrage { fires: vec![] })
                .unwrap();
            events
                .iter()
                .find_map(|e| match e {
                    Event::Barrage {
                        firers,
                        points,
                        target,
                        ..
                    } if firers == &vec![ship.clone()] && *target == pid(TARGET) => Some(*points),
                    _ => None,
                })
                .expect("the ship's barrage")
        }

        #[test]
        fn a_ship_bombards_its_coastal_hex_with_its_guns_as_actual_points_30_2x() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let at = target_hex(&state);
            assert!(data.map.hex(&at).unwrap().coastal);
            let valiant = put(&mut state, "valiant", &at);
            assert_eq!(resolve(&data, &mut state, &valiant, 6), 6);
            assert!(state.fleet[&valiant].fired);
        }

        #[test]
        fn a_battleship_bombards_an_adjacent_hex_at_half_30_2x() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let sea = offshore(&data, &target_hex(&state));
            let valiant = put(&mut state, "valiant", &sea);
            let err = state.apply(&data, Side::Cw, fire(&valiant, 6)).unwrap_err();
            assert_eq!(err.case, "30.2x");
            assert!(err.reason.contains("its 3 gun points"), "{}", err.reason);
            assert_eq!(resolve(&data, &mut state, &valiant, 3), 3);
        }

        #[test]
        fn a_destroyer_or_light_cruiser_may_not_bombard_an_adjacent_hex_30_2x() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let at = target_hex(&state);
            let sea = offshore(&data, &at);
            for ship in ["airedale", "arethusa", "dido"] {
                let id = put(&mut state, ship, &sea);
                let err = state.apply(&data, Side::Cw, fire(&id, 1)).unwrap_err();
                assert_eq!(err.case, "30.2x", "{ship}");
                assert!(err.reason.contains("only battleships"), "{}", err.reason);
            }
            // In the coastal hex itself a destroyer fires its one point.
            let airedale = put(&mut state, "airedale", &at);
            assert_eq!(resolve(&data, &mut state, &airedale, 1), 1);
        }

        #[test]
        fn a_ship_in_port_or_out_of_reach_may_not_fire_30_2x() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let err = state
                .apply(&data, Side::Cw, fire(&ship_id("valiant"), 6))
                .unwrap_err();
            assert_eq!(err.case, "12.2x", "in Alexandria, off the map");
            let far = sea_distances(&data.map)
                .into_keys()
                .find(|h| !data.map.adjacent(h, &target_hex(&state)))
                .unwrap();
            let valiant = put(&mut state, "valiant", &far);
            let err = state.apply(&data, Side::Cw, fire(&valiant, 6)).unwrap_err();
            assert_eq!(err.case, "30.2x");
            assert!(err.reason.contains("neither"), "{}", err.reason);
        }

        #[test]
        fn a_ship_fires_once_a_stage_30_25() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let at = target_hex(&state);
            let valiant = put(&mut state, "valiant", &at);
            let twice = Order::Barrage {
                fires: vec![
                    Fire {
                        piece: valiant.clone(),
                        points: 6,
                        target: pid(TARGET),
                    };
                    2
                ],
            };
            let err = state.apply(&data, Side::Cw, twice).unwrap_err();
            assert_eq!(err.case, "30.25");
            resolve(&data, &mut state, &valiant, 6);
            // A later barrage step this stage: refused.
            let err = state
                .check_barrage(
                    &data,
                    Side::Cw,
                    &[Fire {
                        piece: valiant.clone(),
                        points: 6,
                        target: pid(TARGET),
                    }],
                )
                .unwrap_err();
            assert_eq!(err.case, "30.25");
            assert!(err.reason.contains("once a stage"), "{}", err.reason);
        }

        #[test]
        fn a_ship_that_fired_spends_the_next_two_stages_in_alexandria_30_25() {
            let data = graziani();
            let mut state = set_up(&data, fleet_on());
            let valiant = ship_id("valiant");
            let sea = off_alexandria(&data);
            to_phase_e(&data, &mut state);
            state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap();
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
            // It fires this stage (the firing itself is tested above).
            state.ships_fired(std::slice::from_ref(&valiant));
            // Stage 2: one stage out would let it stay; having fired, no.
            to_phase_e(&data, &mut state);
            let err = state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap_err();
            assert_eq!(err.case, "30.25");
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert_eq!(
                state.pieces[&valiant].location,
                Location::Port(Port::Alexandria)
            );
            assert!(!state.fleet[&valiant].fired);
            // Stage 3: the second in port.
            to_phase_e(&data, &mut state);
            let err = state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap_err();
            assert!(err.reason.contains("owes 1"), "{}", err.reason);
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
            // Stage 4: free to sail.
            to_phase_e(&data, &mut state);
            state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap();
        }

        #[test]
        fn a_ship_opens_a_barrage_step_on_a_coast_no_land_unit_fights_on_30_2x() {
            let data = graziani();
            let mut state = moving(&data, 3, Side::Cw);
            state.modules.fleet = true;
            state.ships_arrive(&data, &mut Vec::new());
            // One Axis unit left, on a coast in range no Commonwealth
            // land unit stands beside.
            let cw_near = |s: &GameState, h: &HexId| {
                s.pieces.values().any(|p| {
                    p.side == Side::Cw
                        && p.kind.holds_ground()
                        && p.hex().is_some_and(|q| q == h || data.map.adjacent(q, h))
                })
            };
            let coast = data
                .map
                .hexes()
                .filter(|h| h.coastal && station_refusal(&data.map, &h.id).is_none())
                .map(|h| h.id.clone())
                .find(|h| !cw_near(&state, h))
                .unwrap();
            for p in state.pieces.values_mut() {
                if p.side == Side::Axis && p.hex().is_some() {
                    p.location = Location::Eliminated;
                }
            }
            state.pieces.get_mut(&pid(TARGET)).unwrap().location = Location::Hex(coast.clone());
            let mut quiet = state.clone();
            quiet.apply(&data, Side::Cw, Order::EndSegment).unwrap();
            assert_ne!(
                quiet.clock.segment,
                Some(crate::clock::Segment::Barrage),
                "no ship in reach, no land contact: no barrage step"
            );
            put(&mut state, "valiant", &coast);
            state.apply(&data, Side::Cw, Order::EndSegment).unwrap();
            assert_eq!(state.clock.segment, Some(crate::clock::Segment::Barrage));
            assert_eq!(resolve(&data, &mut state, &ship_id("valiant"), 6), 6);
        }

        #[test]
        fn without_the_module_no_ship_fires() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let at = target_hex(&state);
            let valiant = put(&mut state, "valiant", &at);
            state.modules.fleet = false;
            assert!(!state.ships_may_fire(&data));
            let err = state.apply(&data, Side::Cw, fire(&valiant, 6)).unwrap_err();
            assert_eq!(err.case, "30.2x");
        }
    }
}
