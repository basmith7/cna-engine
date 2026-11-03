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
//! ends). Ratings, bombardment, strikes on the fleet, damage, repair and
//! the Valletta shuttle are not simulated yet (`NOT_SIMULATED.md`).

use crate::data::Data;
use crate::event::{Event, Illegal};
use crate::piece::{Location, PieceId, PieceKind, Port};
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
}

/// A ship's piece id: `ship:cw:<name>`.
pub fn ship_id(name: &str) -> PieceId {
    PieceId::from(format!("ship:cw:{name}"))
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
            Some(Location::Hex(_)) => r.at_sea < MAX_AT_SEA,
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
                r.owed = PORT_PER_SEA * r.at_sea;
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
            return Err(if r.at_sea >= MAX_AT_SEA {
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
}
