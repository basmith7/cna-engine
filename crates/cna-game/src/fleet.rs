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
//! the next two OpStages there.
//!
//! Strikes on the fleet (32.7x, R-032): the first time phase E comes
//! round the Axis rolls a die (+2 in a campaign game) for its strikes, a
//! number the Commonwealth never sees (3.61). As phase E closes, one
//! strike falls on Alexandria harbour when the Axis has one left, some
//! ship is in port and map sheet E has no rainstorm or sandstorm: the
//! engine makes the Axis's choice to strike, as it makes a patrol
//! defender's, and always strikes when it may. A die (+2 in a campaign
//! game) picks the column, counted from the left, of the Air Bombardment
//! Table's units rows; two dice read in order there give damage points,
//! laid one at a time on the ship in port with the most guns left (the
//! lower id on a tie), the Axis's choice again made by the engine. The
//! strike's dice come from a stream of their own, seeded by the game and
//! the OpStage, as convoy bombing's do, so they never shift play's dice.
//!
//! Damage (30.3x): a ship's guns drop point for point and its AA in
//! proportion, rounded to the nearest point (a half up); it fires at its
//! guns left. Below half its printed guns it may not leave port; at none
//! it is sunk and leaves play. Repair (30.37): a damaged ship gets a gun
//! point back for each six OpStages in a row it spends in Alexandria,
//! counted as phase E closes, the count starting again whenever it sails.
//! The Valletta shuttle is not simulated (`NOT_SIMULATED.md`).

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

/// OpStages in a row in Alexandria for a gun point of repair (30.37).
pub const STAGES_PER_REPAIR: u8 = 6;

/// The map sheet whose storms keep the Axis from striking (32.73).
const STRIKE_SHEET: &str = "E3613";

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
    /// Gun points lost (30.3x).
    #[serde(default, skip_serializing_if = "crate::raiders::is_zero")]
    pub damage: u32,
    /// OpStages in a row in Alexandria towards the next gun point of
    /// repair (30.37), counted while damaged.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub repairing: u8,
}

fn is_zero_u8(n: &u8) -> bool {
    *n == 0
}

/// A ship's ratings as damaged (30.3x): guns less the damage, AA in the
/// same proportion of its printed AA, rounded to the nearest point (a
/// half up).
pub fn current_ratings(row: &cna_data::fleet::Ship, damage: u32) -> (u32, u32) {
    let guns = row.gun_rating.saturating_sub(damage);
    if row.gun_rating == 0 {
        return (0, row.aa_rating);
    }
    let aa = (2 * row.aa_rating * guns + row.gun_rating) / (2 * row.gun_rating);
    (guns, aa)
}

/// Below half its printed guns a ship may not leave port (30.3x).
fn below_half(row: &cna_data::fleet::Ship, damage: u32) -> bool {
    2 * current_ratings(row, damage).0 < row.gun_rating
}

/// A die stream of the fleet's own, seeded by the game, the OpStage and
/// `tag`, so its rolls never shift the dice of play (as convoy bombing's).
pub(crate) fn fleet_dice(seed: u64, game_turn: u8, stage: u8, tag: u64) -> crate::rng::GameRng {
    let salt = (1 << 40) | (tag << 32) | (u64::from(game_turn) << 8) | u64::from(stage);
    crate::rng::GameRng::new(seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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

/// Every hex a ship may be placed in (30.1x, R-019): what
/// `station_refusal` accepts, with the sea distances computed once.
fn station_hexes(map: &Map) -> Vec<HexId> {
    let dist = sea_distances(map);
    map.hexes()
        .filter(|h| h.terrain == Terrain::Sea || h.coastal)
        .filter(|h| {
            let id = &h.id;
            !(id.sheet() < WESTERN_SHEET || (id.sheet() == WESTERN_SHEET && id.col() < WESTERN_ROW))
        })
        .filter(|h| {
            if h.terrain == Terrain::Sea {
                dist.contains_key(&h.id)
            } else {
                map.neighbours(&h.id).iter().any(|n| dist.contains_key(n))
            }
        })
        .map(|h| h.id.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The ships the Commonwealth may place this phase E and the hexes any
/// of them may go to (30.1x): what `Order::Station` accepts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StationOptions {
    pub ships: Vec<PieceId>,
    pub hexes: Vec<HexId>,
}

impl GameState {
    /// What `player` may station now; `None` outside its phase E or with
    /// no ship free to sail.
    pub fn station_options(&self, data: &Data, player: Side) -> Option<StationOptions> {
        if !self.modules.fleet
            || self.clock.phase != crate::clock::Phase::CwFleet
            || self.pending.is_some()
            || player != Side::Cw
            || self.to_act() != Some(Side::Cw)
        {
            return None;
        }
        let ships: Vec<PieceId> = self
            .fleet
            .iter()
            .filter(|(id, r)| r.stationed || self.free_to_sail(data, id))
            .map(|(id, _)| id.clone())
            .collect();
        if ships.is_empty() {
            return None;
        }
        Some(StationOptions {
            ships,
            hexes: station_hexes(&data.map),
        })
    }
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
    /// with no port time owed and at least half its guns, or at sea fewer
    /// than three OpStages in a row.
    fn free_to_sail(&self, data: &Data, ship: &PieceId) -> bool {
        let Some(r) = self.fleet.get(ship) else {
            return false;
        };
        match self.pieces.get(ship).map(|p| &p.location) {
            Some(Location::Port(_)) => {
                r.owed == 0 && ship_row(data, ship).is_some_and(|row| !below_half(row, r.damage))
            }
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
        self.roll_for_strikes(data, events);
        if self.before_first_sail(data) {
            return false;
        }
        let ids: Vec<PieceId> = self.fleet.keys().cloned().collect();
        ids.iter().any(|id| self.free_to_sail(data, id))
    }

    /// Whether the scenario still bars every warship from sailing
    /// (`first_sail`: Graziani's Offensive's 60.45, Rommel's Arrival's
    /// 61.37). Ships in port meanwhile serve their port time as usual.
    fn before_first_sail(&self, data: &Data) -> bool {
        let now = GameTime::new(self.clock.game_turn, self.clock.stage);
        data.scenario
            .side(Side::Cw)
            .fleet
            .as_ref()
            .and_then(|f| f.first_sail)
            .is_some_and(|first| now < first)
    }

    /// The first phase E with the fleet in play: the Axis rolls one die,
    /// +2 in a campaign game, for its strikes on the fleet (32.72).
    fn roll_for_strikes(&mut self, data: &Data, events: &mut Vec<Event>) {
        if self.fleet_strikes.contains_key(&Side::Axis) || self.fleet.is_empty() {
            return;
        }
        let mut dice = fleet_dice(self.rng.seed(), self.clock.game_turn, self.clock.stage, 1);
        let strikes = dice.d6() + if data.is_campaign() { 2 } else { 0 };
        self.fleet_strikes.insert(Side::Axis, strikes);
        events.push(Event::FleetStrikesRolled { strikes });
    }

    /// Ships in Alexandria harbour now.
    fn in_harbour(&self) -> Vec<PieceId> {
        self.fleet
            .keys()
            .filter(|id| self.pieces[*id].location == Location::Port(Port::Alexandria))
            .cloned()
            .collect()
    }

    /// The close of phase E: an Axis strike on the harbour, if it has one
    /// left, some ship is in port and sheet E has no storm (32.73, R-032).
    fn strike_the_fleet(&mut self, data: &Data, events: &mut Vec<Event>) {
        let left = self.fleet_strikes.get(&Side::Axis).copied().unwrap_or(0);
        let storm = self.weather_at(&HexId::parse(STRIKE_SHEET).expect("a hex id"))
            != cna_rules::movement::Weather::Normal;
        if left == 0 || storm || self.in_harbour().is_empty() {
            return;
        }
        self.fleet_strikes.insert(Side::Axis, left - 1);
        let mut dice = fleet_dice(self.rng.seed(), self.clock.game_turn, self.clock.stage, 2);
        let column = dice.d6() + if data.is_campaign() { 2 } else { 0 };
        let reading = dice.d6() * 10 + dice.d6();
        self.resolve_strike(data, column, reading, events);
    }

    /// A strike read in `column` (1 the leftmost) on `reading`: its damage
    /// laid a point at a time on the ship in harbour with the most guns
    /// left, the lower id on a tie (32.74, the Axis's choice made by the
    /// engine); a ship left with none is sunk (30.3x).
    pub(crate) fn resolve_strike(
        &mut self,
        data: &Data,
        column: u8,
        reading: u8,
        events: &mut Vec<Event>,
    ) {
        let damage = data
            .fleet_bombardment
            .in_column(usize::from(column.saturating_sub(1)), reading)
            .unwrap_or(0);
        let (hits, sunk) = self.lay_harbour_damage(data, damage);
        events.push(Event::FleetStruck {
            column,
            reading,
            damage,
            hits,
            sunk,
        });
    }

    /// `damage` gun points laid a point at a time on the ship in harbour
    /// with the most guns left, the lower id on a tie (32.74, the Axis's
    /// choice made by the engine); a ship left with none is sunk (30.3x).
    /// The points each ship took, and the ships sunk.
    pub(crate) fn lay_harbour_damage(
        &mut self,
        data: &Data,
        damage: u32,
    ) -> (BTreeMap<PieceId, u32>, Vec<PieceId>) {
        let guns = |s: &Self, id: &PieceId| {
            ship_row(data, id).map_or(0, |row| current_ratings(row, s.fleet[id].damage).0)
        };
        let mut hits = BTreeMap::new();
        for _ in 0..damage {
            let Some(target) = self
                .in_harbour()
                .into_iter()
                .filter(|id| guns(self, id) > 0)
                .max_by(|a, b| guns(self, a).cmp(&guns(self, b)).then_with(|| b.cmp(a)))
            else {
                break;
            };
            self.fleet.get_mut(&target).expect("in harbour").damage += 1;
            *hits.entry(target).or_insert(0) += 1;
        }
        let sunk: Vec<PieceId> = hits
            .keys()
            .filter(|id| guns(self, id) == 0)
            .cloned()
            .collect();
        for id in &sunk {
            self.fleet.remove(id);
            self.pieces.get_mut(id).expect("a ship's piece").location = Location::Eliminated;
        }
        (hits, sunk)
    }

    /// A damaged ship in Alexandria counts the OpStage towards repair: a
    /// gun point back at six in a row (30.37).
    fn repair_ships(&mut self, events: &mut Vec<Event>, data: &Data) {
        for (id, r) in self.fleet.iter_mut() {
            if r.damage == 0 || self.pieces[id].location != Location::Port(Port::Alexandria) {
                r.repairing = 0;
                continue;
            }
            r.repairing += 1;
            if r.repairing >= STAGES_PER_REPAIR {
                r.repairing = 0;
                r.damage -= 1;
                let guns = ship_row(data, id).map_or(0, |row| current_ratings(row, r.damage).0);
                events.push(Event::ShipRepaired {
                    ship: id.clone(),
                    guns,
                });
            }
        }
    }

    /// Phase E closes: a ship placed this phase spends the OpStage at
    /// sea; any other goes back to (or stays in) Alexandria and serves a
    /// stage of its port time, owing two for each stage it was out.
    ///
    /// Then a damaged ship in port counts the stage towards repair, and
    /// the Axis strikes (32.73, R-032): repair first, so a ship struck
    /// this stage starts its six from the next.
    pub(crate) fn close_fleet_phase(&mut self, data: &Data, events: &mut Vec<Event>) {
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
        self.repair_ships(events, data);
        self.make_chariot_raid(data, events);
        self.strike_the_fleet(data, events);
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
        if !r.stationed && !self.free_to_sail(data, ship) {
            let crippled = ship_row(data, ship).is_some_and(|row| below_half(row, r.damage));
            return Err(
                if crippled && matches!(self.pieces[ship].location, Location::Port(_)) {
                    Illegal::new(
                        "30.3x",
                        format!(
                            "{ship} is below half its guns: it may not leave port until repaired"
                        ),
                    )
                } else if r.fired {
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
                },
            );
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
        let guns = current_ratings(row, r.damage).0;
        if from == at {
            if data.map.hex(at).is_some_and(|h| h.coastal) {
                return Ok(guns);
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
        Ok(guns.div_ceil(2))
    }

    /// Whether some ship may bombard an enemy unit now (30.2x): opens a
    /// barrage step that has no land fire in it.
    pub(crate) fn ships_may_fire(&self, data: &Data) -> bool {
        if !self.modules.fleet {
            return false;
        }
        self.blockship_may_fire(data)
            || self.fleet.keys().any(|id| {
                let ship = &self.pieces[id];
                self.pieces.values().any(|t| {
                    // What the step's opening reveals must be open to both
                    // sides (3.61): a counter's kind, not its TOE.
                    t.side != ship.side
                        && matches!(t.kind, PieceKind::Combat | PieceKind::Hq)
                        && t.hex()
                            .is_some_and(|at| self.bombardment_points(data, ship, at).is_ok())
                })
            })
    }

    /// The ships among a barrage plot, and the *San Giorgio*, are marked
    /// as having fired (30.25, R-151).
    pub(crate) fn ships_fired(&mut self, firers: &[PieceId]) {
        for id in firers {
            if self.pieces.get(id).map(|p| p.kind) == Some(PieceKind::Blockship) {
                self.blockship_fired = true;
            }
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
    use crate::state::Modules;
    use crate::state::test_support::{auto_place, pass_stage};

    fn hex(s: &str) -> HexId {
        HexId::parse(s).unwrap()
    }

    /// Graziani's Offensive with its first-sail bar lifted, so the
    /// fleet's rules can be tried from GT1.
    fn graziani() -> Data {
        let mut data = crate::raiders::tests::graziani();
        let cw = data.scenario.sides.get_mut(&Side::Cw).unwrap();
        cw.fleet.as_mut().unwrap().first_sail = None;
        data
    }

    #[test]
    fn no_warship_sails_before_the_scenario_s_first_sail_60_45() {
        let data = crate::raiders::tests::graziani();
        let mut state = set_up(&data, fleet_on());
        let mut skipped = Vec::new();
        while state.clock.game_turn < 2 {
            for e in pass_stage(&data, &mut state) {
                match e {
                    Event::PhaseSkipped { clock } if clock.phase == Phase::CwFleet => {
                        skipped.push(clock.stage)
                    }
                    Event::PhaseStarted { clock } => {
                        assert_ne!(clock.phase, Phase::CwFleet, "phase E opened at {clock}")
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(skipped, [1, 2, 3], "phase E skipped all of GT1");
        assert_eq!(state.fleet.len(), 11, "the ships are in Alexandria");
        to_phase_e(&data, &mut state);
        assert_eq!((state.clock.game_turn, state.clock.stage), (2, 1));
        let ship = ship_id("valiant");
        state
            .apply(
                &data,
                Side::Cw,
                station(&ship, &off_alexandria(&data).to_string()),
            )
            .unwrap();
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
    fn station_options_are_exactly_what_station_accepts_30_1x() {
        let data = graziani();
        let mut state = set_up(&data, fleet_on());
        to_phase_e(&data, &mut state);
        let o = state.station_options(&data, Side::Cw).expect("phase E");
        assert_eq!(o.ships.len(), 11);
        assert!(state.station_options(&data, Side::Axis).is_none());
        let valiant = ship_id("valiant");
        let offered: std::collections::BTreeSet<_> = o.hexes.iter().collect();
        for h in data.map.hexes() {
            let r = state
                .clone()
                .apply(&data, Side::Cw, station(&valiant, h.id.as_str()));
            assert_eq!(r.is_ok(), offered.contains(&h.id), "{}: {r:?}", h.id);
        }
        // A ship owing port time is not offered.
        state.fleet.get_mut(&valiant).unwrap().owed = 2;
        let o = state.station_options(&data, Side::Cw).unwrap();
        assert!(!o.ships.contains(&valiant));
        // Nor outside phase E.
        state.apply(&data, Side::Cw, Order::Pass).unwrap();
        assert!(state.station_options(&data, Side::Cw).is_none());
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

    mod strikes {
        use super::*;
        use crate::view::{Aliases, events_for, leaks, view_for};

        /// Graziani's set-up with the fleet, at its first phase E, the
        /// Axis's strikes set to `strikes`.
        fn at_phase_e(data: &Data, strikes: u8) -> GameState {
            let mut state = set_up(data, fleet_on());
            to_phase_e(data, &mut state);
            state.fleet_strikes.insert(Side::Axis, strikes);
            state
        }

        fn struck(events: &[Event]) -> Option<&Event> {
            events
                .iter()
                .find(|e| matches!(e, Event::FleetStruck { .. }))
        }

        fn damage(state: &GameState, ship: &str) -> u32 {
            state.fleet[&ship_id(ship)].damage
        }

        #[test]
        fn a_commonwealth_preview_never_shows_a_strike_to_come_32_72() {
            let data = graziani();
            let preview = |strikes: Option<u8>| {
                let mut state = set_up(&data, fleet_on());
                to_phase_e(&data, &mut state);
                match strikes {
                    Some(n) => {
                        state.fleet_strikes.insert(Side::Axis, n);
                    }
                    None => {
                        state.fleet_strikes.clear();
                    }
                }
                let mut known = crate::view::as_known_to(&state, Side::Cw);
                let events = known.apply(&data, Side::Cw, Order::Pass).unwrap();
                assert!(struck(&events).is_none(), "{events:?}");
                serde_json::to_string(&events).unwrap()
            };
            assert_eq!(preview(Some(0)), preview(Some(2)));
            assert_eq!(preview(Some(0)), preview(None));
        }

        #[test]
        fn the_axis_rolls_its_strikes_at_the_first_phase_e_and_keeps_them_secret_32_72() {
            let data = graziani();
            let mut state = set_up(&data, fleet_on());
            assert!(state.fleet_strikes.is_empty());
            to_phase_e(&data, &mut state);
            let n = state.fleet_strikes[&Side::Axis];
            assert!((1..=6).contains(&n), "one die: {n}");
            let aliases = Aliases::new([1; 32]);
            let cw = view_for(&state, &data, &aliases, Some(Side::Cw), false).unwrap();
            assert!(
                cw.get("fleet_strikes")
                    .is_none_or(|m| m.get("axis").is_none())
            );
            assert_eq!(leaks(&state, &cw, Side::Axis), Vec::<String>::new());
            let axis = view_for(&state, &data, &aliases, Some(Side::Axis), false).unwrap();
            assert_eq!(axis["fleet_strikes"]["axis"], serde_json::json!(n));
            let rolled = [Event::FleetStrikesRolled { strikes: n }];
            assert!(events_for(&state, &aliases, &rolled, Some(Side::Cw), false).is_empty());
            assert!(events_for(&state, &aliases, &rolled, None, false).is_empty());
            assert_eq!(
                events_for(&state, &aliases, &rolled, Some(Side::Axis), false).len(),
                1
            );
            // Rolled once: the next phase E leaves it be.
            state.fleet_strikes.insert(Side::Axis, 9);
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
            to_phase_e(&data, &mut state);
            assert_eq!(
                state.fleet_strikes[&Side::Axis],
                8,
                "struck once, not rerolled"
            );
        }

        #[test]
        fn one_strike_a_stage_and_none_while_sheet_e_has_a_storm_32_73() {
            let data = graziani();
            let mut state = at_phase_e(&data, 3);
            state.weather = crate::state::Weather::Rainstorm { sheets: vec!['E'] };
            let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert!(struck(&events).is_none(), "{events:?}");
            assert_eq!(state.fleet_strikes[&Side::Axis], 3);
            to_phase_e(&data, &mut state);
            state.weather = crate::state::Weather::Sandstorm { sheets: vec!['E'] };
            let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert!(struck(&events).is_none());
            // A storm elsewhere does not stop it.
            to_phase_e(&data, &mut state);
            state.weather = crate::state::Weather::Sandstorm { sheets: vec!['C'] };
            let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert!(struck(&events).is_some());
            assert_eq!(state.fleet_strikes[&Side::Axis], 2, "one a stage");
        }

        #[test]
        fn no_strike_without_one_left_or_a_ship_in_harbour_32_73() {
            let data = graziani();
            let mut state = at_phase_e(&data, 0);
            let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert!(struck(&events).is_none());
            let mut state = at_phase_e(&data, 2);
            let sea = off_alexandria(&data);
            let ids: Vec<PieceId> = state.fleet.keys().cloned().collect();
            for id in &ids {
                state
                    .apply(&data, Side::Cw, station(id, sea.as_str()))
                    .unwrap();
            }
            let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert!(struck(&events).is_none(), "the harbour is empty");
            assert_eq!(state.fleet_strikes[&Side::Axis], 2);
        }

        #[test]
        fn damage_goes_a_point_at_a_time_to_the_most_guns_in_harbour_32_74() {
            let data = graziani();
            let mut state = at_phase_e(&data, 0);
            // A ship at sea is out of the strike's reach.
            let sea = off_alexandria(&data);
            state
                .apply(&data, Side::Cw, station(&ship_id("dido"), sea.as_str()))
                .unwrap();
            let mut events = Vec::new();
            // Column 8 (a 6 in a campaign game), 65: 5 damage points.
            state.resolve_strike(&data, 8, 65, &mut events);
            // Valiant 6, 5, 4 to 3; tied with Arethusa's 3, the lower id
            // first: Arethusa to 2; then Valiant to 2.
            assert_eq!(damage(&state, "valiant"), 4);
            assert_eq!(damage(&state, "arethusa"), 1);
            assert_eq!(damage(&state, "dido"), 0);
            assert_eq!(
                events,
                vec![Event::FleetStruck {
                    column: 8,
                    reading: 65,
                    damage: 5,
                    hits: BTreeMap::from([(ship_id("arethusa"), 1), (ship_id("valiant"), 4)]),
                    sunk: vec![],
                }]
            );
            // The leftmost column, 62: nothing.
            let mut events = Vec::new();
            state.resolve_strike(&data, 1, 62, &mut events);
            assert!(
                matches!(&events[0], Event::FleetStruck { damage: 0, hits, .. } if hits.is_empty())
            );
        }

        #[test]
        fn a_damaged_ships_aa_drops_in_proportion_30_3x() {
            let data = graziani();
            let valiant = ship_row(&data, &ship_id("valiant")).unwrap();
            assert_eq!(current_ratings(valiant, 0), (6, 12));
            assert_eq!(current_ratings(valiant, 4), (2, 4));
            assert_eq!(current_ratings(valiant, 6), (0, 0));
            let bedouin = ship_row(&data, &ship_id("bedouin")).unwrap();
            assert_eq!(current_ratings(bedouin, 0), (1, 3));
        }

        #[test]
        fn a_ship_below_half_its_guns_may_not_leave_port_30_3x() {
            let data = graziani();
            let mut state = at_phase_e(&data, 0);
            let sea = off_alexandria(&data);
            let valiant = ship_id("valiant");
            state.fleet.get_mut(&valiant).unwrap().damage = 4;
            let err = state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap_err();
            assert_eq!(err.case, "30.3x");
            assert!(err.reason.contains("below half"), "{}", err.reason);
            // At exactly half it may.
            state.fleet.get_mut(&valiant).unwrap().damage = 3;
            state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap();
        }

        #[test]
        fn a_ship_with_no_guns_left_is_sunk_30_3x() {
            let data = graziani();
            let mut state = at_phase_e(&data, 0);
            let sea = off_alexandria(&data);
            let ids: Vec<PieceId> = state.fleet.keys().cloned().collect();
            let airedale = ship_id("airedale");
            for id in ids.iter().filter(|id| **id != airedale) {
                state
                    .apply(&data, Side::Cw, station(id, sea.as_str()))
                    .unwrap();
            }
            let mut events = Vec::new();
            // Column 1, 63: a point, Airedale's one gun.
            state.resolve_strike(&data, 1, 63, &mut events);
            assert!(
                matches!(&events[0], Event::FleetStruck { sunk, .. } if *sunk == vec![airedale.clone()])
            );
            assert_eq!(state.pieces[&airedale].location, Location::Eliminated);
            assert!(!state.fleet.contains_key(&airedale));
            // Gone from play: it may not be stationed, nor arrives again.
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
            to_phase_e(&data, &mut state);
            let err = state
                .apply(&data, Side::Cw, station(&airedale, sea.as_str()))
                .unwrap_err();
            assert!(err.reason.contains("not a Commonwealth ship in play"));
        }

        #[test]
        fn a_ship_fires_at_its_guns_left_30_3x() {
            let data = graziani();
            let mut state = at_phase_e(&data, 0);
            let valiant = ship_id("valiant");
            let sea = off_alexandria(&data);
            state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap();
            state.fleet.get_mut(&valiant).unwrap().damage = 1;
            let coast = data
                .map
                .neighbours(&sea)
                .into_iter()
                .find(|h| data.map.hex(h).unwrap().coastal)
                .unwrap();
            let ship = state.pieces[&valiant].clone();
            assert_eq!(
                state.bombardment_points(&data, &ship, &sea),
                Err(Illegal::new(
                    "30.2x",
                    format!("{sea}: a ship bombards a coastal hex")
                ))
            );
            // Five guns left: half of them, a half up, into the next hex.
            assert_eq!(state.bombardment_points(&data, &ship, &coast), Ok(3));
            state.pieces.get_mut(&valiant).unwrap().location = Location::Hex(coast.clone());
            let ship = state.pieces[&valiant].clone();
            assert_eq!(state.bombardment_points(&data, &ship, &coast), Ok(5));
        }

        #[test]
        fn a_gun_point_comes_back_after_six_stages_in_alexandria_30_37() {
            let data = graziani();
            let mut state = at_phase_e(&data, 0);
            let valiant = ship_id("valiant");
            state.fleet.get_mut(&valiant).unwrap().damage = 2;
            for stage in 1..=5 {
                let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
                assert!(
                    !events
                        .iter()
                        .any(|e| matches!(e, Event::ShipRepaired { .. })),
                    "stage {stage}"
                );
                to_phase_e(&data, &mut state);
            }
            assert_eq!(state.fleet[&valiant].repairing, 5);
            let events = state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert!(events.contains(&Event::ShipRepaired {
                ship: valiant.clone(),
                guns: 5
            }));
            assert_eq!(damage(&state, "valiant"), 1);
            assert_eq!(state.fleet[&valiant].repairing, 0);
        }

        #[test]
        fn repair_starts_again_when_the_ship_sails_30_37() {
            let data = graziani();
            let mut state = at_phase_e(&data, 0);
            let valiant = ship_id("valiant");
            let sea = off_alexandria(&data);
            state.fleet.get_mut(&valiant).unwrap().damage = 1;
            for _ in 0..5 {
                state.apply(&data, Side::Cw, Order::Pass).unwrap();
                to_phase_e(&data, &mut state);
            }
            assert_eq!(state.fleet[&valiant].repairing, 5);
            state
                .apply(&data, Side::Cw, station(&valiant, sea.as_str()))
                .unwrap();
            state.apply(&data, Side::Cw, Order::Pass).unwrap();
            assert_eq!(state.fleet[&valiant].repairing, 0, "it sailed");
            assert_eq!(damage(&state, "valiant"), 1);
            // Back in port: six more from scratch.
            for _ in 0..6 {
                to_phase_e(&data, &mut state);
                state.apply(&data, Side::Cw, Order::Pass).unwrap();
            }
            assert_eq!(damage(&state, "valiant"), 0);
        }

        #[test]
        fn without_the_module_no_strike_is_rolled_or_made() {
            let data = graziani();
            let mut state = set_up(&data, Modules::default());
            for _ in 0..3 {
                if state.clock.phase == Phase::InitiativeDeclaration {
                    let p = state.to_act().unwrap();
                    state
                        .apply(&data, p, Order::DeclareInitiative { player_a: p })
                        .unwrap();
                }
                let events = pass_stage(&data, &mut state);
                assert!(struck(&events).is_none());
            }
            assert!(state.fleet_strikes.is_empty());
        }
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
        #[test]
        fn a_ship_in_the_barraged_hex_pays_no_combat_cp() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let at = target_hex(&state);
            let ship = put(&mut state, "valiant", &at);
            state.apply(&data, Side::Cw, fire(&ship, 6)).unwrap();
            let events = state
                .apply(&data, Side::Axis, Order::Barrage { fires: vec![] })
                .unwrap();
            assert!(
                !events
                    .iter()
                    .any(|e| matches!(e, Event::CombatCp { piece, .. } if *piece == ship)),
                "a ship is charged combat CP: {events:?}"
            );
            assert!(!state.combat.charged.contains_key(&ship));
        }

        #[test]
        fn whether_a_ship_may_fire_reads_only_what_both_sides_see() {
            // 3.61: the step's opening must not tell the Commonwealth an
            // enemy's TOE, so a spent enemy unit still counts.
            let data = graziani();
            let mut state = at_barrage(&data);
            let at = target_hex(&state);
            let off = offshore(&data, &at);
            put(&mut state, "valiant", &off);
            let with = state.ships_may_fire(&data);
            state.pieces.get_mut(&pid(TARGET)).unwrap().toe = 0;
            assert_eq!(state.ships_may_fire(&data), with);
            assert!(with);
        }

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
        fn the_barrage_options_offer_each_ship_exactly_the_fire_apply_accepts_30_2x() {
            let data = graziani();
            let mut state = at_barrage(&data);
            let at = target_hex(&state);
            let sea = offshore(&data, &at);
            let valiant = put(&mut state, "valiant", &sea);
            let dido = put(&mut state, "dido", &at);
            let airedale = put(&mut state, "airedale", &sea);
            let crate::combat::options::SealedOptions::Barrage { firers } =
                state.sealed_options(&data, Side::Cw).unwrap()
            else {
                panic!("the barrage step");
            };
            let offered = |id: &PieceId| firers.iter().find(|f| &f.piece == id);
            assert_eq!(offered(&valiant).unwrap().ship_points[&pid(TARGET)], 3);
            assert!(offered(&dido).is_some());
            assert!(offered(&airedale).is_none(), "a destroyer out of reach");
            let enemies: Vec<PieceId> = state
                .pieces
                .values()
                .filter(|p| p.side == Side::Axis && p.hex().is_some())
                .map(|p| p.id.clone())
                .collect();
            for ship in [&valiant, &dido, &airedale] {
                for t in &enemies {
                    for points in 1..=8 {
                        let r = state.clone().apply(
                            &data,
                            Side::Cw,
                            Order::Barrage {
                                fires: vec![Fire {
                                    piece: ship.clone(),
                                    points,
                                    target: t.clone(),
                                }],
                            },
                        );
                        let want = offered(ship)
                            .and_then(|f| f.ship_points.get(t))
                            .is_some_and(|n| *n == points);
                        assert_eq!(r.is_ok(), want, "{ship} {points} at {t}: {r:?}");
                    }
                }
            }
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
