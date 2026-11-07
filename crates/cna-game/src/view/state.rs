//! `view_for`: the state as one side may see it.
//!
//! Three layers, each relative to the viewer, never to whose move it is:
//!
//! 1. **Always:** `log` and `rng` go; a sealed step's answers from sides
//!    other than the viewer go until the step is revealed (11.0); the
//!    owner's secrets the hot-seat view already kept go for the other
//!    side (3.61: supply loads, motorisation points pooled or attached,
//!    repaired points, whether an unrevealed minefield is real, what a
//!    project builds).
//! 2. **Limited intelligence** (3.61, unless the game is played with open
//!    information): an enemy counter shows only its face. The tables
//!    below say what each field does for the enemy; `ENEMY_PIECE_KEEPS`
//!    is an allow-list, so a new `Piece` field is hidden from the enemy
//!    until someone decides otherwise, and a test fails on a field of
//!    `GameState`, `Piece` or `CombatSegment` with no row.
//! 3. **Aliases:** every enemy supply id in the result, values and keys,
//!    is replaced by its alias (`Aliases`), unless information is open.
//!
//! Hidden movement (27.31), with or without open information: a piece of
//! the other side marked `hidden` goes from the view, with every key,
//! element and entry that names it; `events_for` drops each event that
//! names it.
//!
//! Once the game is over nothing is secret: everything bar `log` and
//! `rng` is shown to everyone.
//!
//! `GameState`, enemy's part (layer 2):
//!
//! | Field | Enemy sees |
//! |---|---|
//! | `scenario`, `rules_commit`, `ruleset`, `modules`, `clock`, `initiative`, `rommel_recalled`, `raiders_lost`, `player_a`, `weather`, `cycle`, `over`, `organised`, `attachment_tracked`, `rail_runs`, `rail_supply_way`, `bizerta_open`, `blockship_fired` | as is |
//! | `convoy_lanes` | the Axis only: R-144 shows the Commonwealth only the lanes it bombs |
//! | `fortifications`, `projects` (bar `item`, layer 1), `minefields` (bar `real`, layer 1), `escapees`, `marched`, `zoc`, `air_lifted`, `fleet`, `stage_record` | as is: on the board, or known to both (30.1x: the ships and their port time are open; who fought and what was barraged this stage) |
//! | `prisoners`, `prisoners_out`, `prisoners_off_map`, `guards`, `escaped`, `fleet_strikes`, `chariot_raid` | own only: 3.61 keeps POW and guard numbers secret, R-145 the count of escapees back with their own side, 32.72 the Axis's strikes on the fleet, and 30.41 its chariot raid's plan |
//! | `motorisation`, `repaired`, `pending_motorisation`, `motorisation_to_place` | own only (layer 1) |
//! | `axis_held_since` | the Axis only (layer 1): it starts only while the holders trace to loaded supply units (64.71) |
//! | `works_to_place` | own entries only (layer 1): its counts of real fields placed would tell which are dummies |
//! | `withdrawn`, `withdrawal_picks` | the Commonwealth only (layer 1): a withdrawn unit's TOE, and the pick before its check (20.8x) |
//! | `transport_out` | as is: the printed motorisation that left with a withdrawal, as the public `withdrawn` event shows it |
//! | `planned`, `broken_down`, `field_rolled`, `destroyed_tanks`, `destroyed_rolled` | own entries only: the enemy's plans, wrecks' and destroyed tanks' contents and repair rolls are its own |
//! | `split` | own entries only: who detached from whom is composition |
//! | `scuttled`, `raided`, `spot_chances`, `spot_tried`, `railed`, `moved`, `continuing`, `shifted`, `escorts` | as is, aliased: what moved where is on the board (a raid puts its raider on the map, 27.5x) |
//! | `pieces` | `ENEMY_PIECE_KEEPS` only (the face, and the board's markers: `contact`, `engaged`, `reserve`, `garrison_of`); a dummy supply unit's `kind` is `supply-unit`; a piece still to be placed is `location: "pending"` without its placement |
//! | `pending` | as is, aliased; `forced-retreat.raw` (a function of TOE) only for the side retreating |
//! | `combat` | `forward`, `pinned`, `retreated`, `barraged`, `assaults` as is (on the board, or totals; an assault's `probe` shows in how it is resolved, 15.9x); `plots`, `assignments` own only; `no_armour` the phasing side's only; `charged`, `probing`, `barrage_involved` own entries only (CP spent, judgments on TOE, and which units fired: "never the units behind them") |
//! | `rng`, `log` | never (layer 1) |
//! | `pieces.*.unit_type`, `name`, `abbreviation` (added) | as is: the counter face |
//! | `pieces.*.shell` (added) | own only: a shell is a judgment on composition |

use super::Aliases;
use crate::combat::SealedStep;
use crate::data::Data;
use crate::piece::{PieceId, PieceKind};
use crate::state::{GameState, Pending};
use anyhow::{Context, Result};
use cna_data::scenario::Side;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// The `Piece` fields an enemy sees: the counter's face and the markers
/// the board shows.
pub const ENEMY_PIECE_KEEPS: &[&str] = &[
    "id",
    "side",
    "kind",
    "nation",
    "id_code",
    "cpa",
    "equivalent",
    "location",
    "garrison_of",
    "contact",
    "engaged",
    "reserve",
    // Added by the view: the counter's face.
    "unit_type",
    "name",
    "abbreviation",
];

/// The `Piece` fields an enemy does not see (3.61: status, composition,
/// attributes), so every field has a row.
pub const ENEMY_PIECE_REMOVES: &[&str] = &[
    "parent",
    "attached_to",
    "toe",
    "weapons",
    "morale",
    "status",
    "group",
    "entry",
    "load",
    "rommel_cpa",
    "cp_spent",
    "cp_enemy_half",
    "cohesion",
    "released",
    "released_assaulted",
    "landed",
    "fuelled",
    "fuelled_over",
    "motorised",
    "escorted",
    "breakdown_halves",
    "breakdown_checked",
    "flown",
    "shell",
    // The whole piece goes from the enemy's view while hidden (27.31).
    "hidden",
];

/// Top-level fields shown to the enemy as they are (aliased).
#[cfg(test)]
const STATE_KEEPS: &[&str] = &[
    "scenario",
    "rules_commit",
    "ruleset",
    "modules",
    "clock",
    "initiative",
    "rommel_recalled",
    "raiders_lost",
    "player_a",
    "weather",
    "cycle",
    "over",
    "organised",
    "attachment_tracked",
    "rail_runs",
    "rail_supply_way",
    "bizerta_open",
    "blockship_fired",
    "fortifications",
    // Set from the scenario's printed set-up and schedule: which units
    // began in the release cities, the stage's count, held repair hexes.
    "release",
    "repair_holds",
    "projects",
    "minefields",
    "escapees",
    "marched",
    "fleet",
    "stage_record",
    "zoc",
    "air_lifted",
    "scuttled",
    "raided",
    "spot_chances",
    "spot_tried",
    "railed",
    "moved",
    "continuing",
    "shifted",
    "escorts",
    "transport_out",
];

/// Top-level fields with an enemy's part removed (by this module's code).
#[cfg(test)]
const STATE_FILTERS: &[&str] = &[
    "axis_held_since",
    "convoy_lanes",
    "escaped",
    "fleet_strikes",
    "chariot_raid",
    "dummy_tanks",
    "prisoners",
    "prisoners_out",
    "prisoners_off_map",
    "guards",
    "motorisation",
    "repaired",
    "pending_motorisation",
    "motorisation_to_place",
    "works_to_place",
    "withdrawn",
    "withdrawal_picks",
    "planned",
    "broken_down",
    "field_rolled",
    "destroyed_tanks",
    "destroyed_rolled",
    "split",
    "pieces",
    "pending",
    "combat",
    "rng",
    "log",
];

#[cfg(test)]
const COMBAT_KEEPS: &[&str] = &["forward", "pinned", "retreated", "barraged", "assaults"];
#[cfg(test)]
const COMBAT_FILTERS: &[&str] = &[
    "no_armour",
    "plots",
    "assignments",
    "charged",
    "probing",
    "barrage_involved",
];

/// What `viewer` may see of `state` (`None`: a spectator, to whom both
/// sides are the enemy). `open`: the game is played with open
/// information, so layers 2 and 3 are skipped. Adds `to_act`,
/// `assaults_to_resolve`, `anti_armour_releases` (the phasing viewer's
/// anti-armour points it may move to a close assault, 14.23) and, per piece, `unit_type` from the
/// characteristics chart (null for supply units and ID codes the chart
/// lacks), the OA unit's `name` and `abbreviation` (null for supply
/// units), `shell: true` on a formation that is a shell (9.26), and in
/// phase C `raider_formings`, the viewer's raiders it may form (27.13),
/// and `parent_shifts`, its units' historical shifts of parent dated
/// this month (19.24), and `chariot_raid_plannable` when the viewer may
/// plan the chariot raid (30.41), and `dummy_tank_options`, the hexes
/// it may note a dummy tank formation in (16.4x), and, whenever the Commonwealth viewer
/// is to act in an OpStage whose withdrawal has a `pick`,
/// `withdrawal_pick_options`, the picks it may make (20.82),
/// and in phase L `patrol_options`, the viewer's units that may patrol
/// and where (16.x): built from who holds each hex, which both sides see;
/// and in phase E `station_options`, the ships the Commonwealth may
/// place at sea and the hexes open to them (30.1x); and in phase K
/// `facility_repairs`, the phasing viewer's facility rolls (22.3x), and
/// `destroyed_tank_repairs`, its destroyed tanks it may try (22.4x); and
/// at set-up `work_options`, where the viewer may place its set-up works
/// and whether it may pass on the rest (59.12); and `built_hexsides`, the
/// scenario's roads and railroads finished at set-up (`roads_built`).
pub fn view_for(
    state: &GameState,
    data: &Data,
    aliases: &Aliases,
    viewer: Option<Side>,
    open: bool,
) -> Result<Value> {
    let mut v = serde_json::to_value(state)?;
    let obj = v.as_object_mut().context("state is an object")?;
    obj.remove("log");
    obj.remove("rng");
    obj.insert("to_act".into(), json!(state.to_act()));
    obj.insert(
        "assaults_to_resolve".into(),
        json!(state.assaults_to_resolve()),
    );
    // 14.23: anti-armour points the phasing viewer may move to assault.
    let releases: Vec<Value> = if open || (viewer.is_some() && viewer == state.phasing()) {
        state
            .anti_armour_releases(data)
            .into_iter()
            .map(|(piece, target, points)| json!({"piece": piece, "target": target, "points": points}))
            .collect()
    } else {
        Vec::new()
    };
    obj.insert("anti_armour_releases".into(), json!(releases));
    // The viewer's raiders it may form now, and from which units (27.13).
    let formings: Vec<Value> = viewer
        .map(|side| state.raider_formings(data, side))
        .unwrap_or_default()
        .into_iter()
        .map(|(raider, from)| {
            let name = raider
                .unit()
                .and_then(|u| data.oa.unit(&u))
                .map(|u| u.abbreviation.clone().unwrap_or_else(|| u.name.clone()));
            json!({"raider": raider, "name": name, "from": from})
        })
        .collect();
    if !formings.is_empty() {
        obj.insert("raider_formings".into(), json!(formings));
    }
    // Phase C: the viewer's historical shifts of parent dated this
    // month (19.24).
    let shifts: Vec<Value> = viewer
        .map(|side| state.parent_shifts(data, side))
        .unwrap_or_default()
        .into_iter()
        .map(|(piece, to)| json!({"piece": piece, "to": to}))
        .collect();
    if !shifts.is_empty() {
        obj.insert("parent_shifts".into(), json!(shifts));
    }
    // Phase C: the Axis viewer may plan its chariot raid (30.41).
    if viewer.is_some_and(|side| state.chariot_raid_plannable(data, side)) {
        obj.insert("chariot_raid_plannable".into(), json!(true));
    }
    // Phase C: the hexes the viewer may note a dummy tank formation in
    // (16.4x): its own units' hexes, which it knows.
    let dummies = viewer
        .map(|side| state.dummy_tank_hexes_placeable(data, side))
        .unwrap_or_default();
    if !dummies.is_empty() {
        obj.insert("dummy_tank_options".into(), json!(dummies));
    }
    // The Commonwealth viewer's withdrawal picks it may make now, among
    // its own units (20.82, R-154).
    let picks = viewer
        .map(|side| state.withdrawal_pick_options(data, side))
        .unwrap_or_default();
    if !picks.is_empty() {
        obj.insert("withdrawal_pick_options".into(), json!(picks));
    }
    let patrols = viewer
        .map(|side| state.patrol_options(data, side))
        .unwrap_or_default();
    if !patrols.is_empty() {
        obj.insert("patrol_options".into(), json!(patrols));
    }
    // The scenario's roads and railroads built at set-up, which the map
    // as drawn shows unfinished (`roads_built`): open to everyone.
    if state.modules.roads_built
        && (!data.built.roads.is_empty() || !data.built.railroads.is_empty())
    {
        obj.insert(
            "built_hexsides".into(),
            json!({"roads": data.built.roads, "railroads": data.built.railroads}),
        );
    }
    if let Some(o) = viewer.and_then(|side| state.work_options(data, side)) {
        obj.insert("work_options".into(), o);
    }
    if let Some(o) = viewer.and_then(|side| state.station_options(data, side)) {
        obj.insert("station_options".into(), json!(o));
    }
    // Phase K: the facility rolls the viewer may make (22.3x).
    if viewer.is_some() && viewer == state.phasing() {
        let offers = state.facility_repairs(data);
        if !offers.is_empty() {
            obj.insert("facility_repairs".into(), json!(offers));
        }
        let offers = state.destroyed_tank_repairs(data);
        if !offers.is_empty() {
            obj.insert("destroyed_tank_repairs".into(), json!(offers));
        }
    }
    label(state, data, &mut v)?;
    if state.is_over() {
        return Ok(v);
    }
    let enemies: Vec<Side> = [Side::Axis, Side::Cw]
        .into_iter()
        .filter(|s| Some(*s) != viewer)
        .collect();
    let is_enemy = |side: Side| enemies.contains(&side);
    let enemy_piece = |id: &str| {
        state
            .pieces
            .get(&PieceId::from(id.to_owned()))
            .is_none_or(|p| is_enemy(p.side))
    };
    let names: Vec<String> = enemies.iter().map(|s| s.to_string()).collect();

    // Layer 1: sealed answers not yet revealed.
    if let Some(Pending::Sealed { step, .. }) = &state.pending {
        let combat = v["combat"].as_object_mut().context("combat")?;
        match step {
            SealedStep::GunPositions => {
                if let Some(f) = combat.get_mut("forward").and_then(Value::as_array_mut) {
                    f.retain(|id| !id.as_str().is_some_and(enemy_piece));
                }
            }
            SealedStep::Barrage | SealedStep::AssignForces => {
                let key = if matches!(step, SealedStep::Barrage) {
                    "plots"
                } else {
                    "assignments"
                };
                if let Some(m) = combat.get_mut(key).and_then(Value::as_object_mut) {
                    names.iter().for_each(|s| {
                        m.remove(s);
                    });
                }
            }
        }
    }
    // Layer 1: the owner's secrets. Withdrawn units keep their TOE for
    // their return, and the pick waits for the check (20.8x).
    if enemies.contains(&Side::Cw)
        && let Some(o) = v.as_object_mut()
    {
        o.remove("withdrawn");
        o.remove("withdrawal_picks");
    }
    // 64.71's count starts only while the Axis holder traces supply to a
    // loaded supply unit: shown to the Commonwealth, it would tell real
    // and loaded Axis supply units from empty ones and dummies (3.61).
    if enemies.contains(&Side::Axis)
        && let Some(o) = v.as_object_mut()
    {
        o.remove("axis_held_since");
    }
    for key in ["motorisation", "repaired", "motorisation_to_place"] {
        if let Some(m) = v.get_mut(key).and_then(Value::as_object_mut) {
            names.iter().for_each(|s| {
                m.remove(s);
            });
        }
    }
    let side_is_enemy = |o: &Map<String, Value>| {
        o.get("side")
            .and_then(Value::as_str)
            .is_some_and(|s| names.iter().any(|n| n == s))
    };
    if let Some(m) = v.get_mut("minefields").and_then(Value::as_object_mut) {
        for field in m.values_mut().filter_map(Value::as_object_mut) {
            let revealed = field.get("revealed").and_then(Value::as_bool) == Some(true);
            if side_is_enemy(field) && !revealed {
                field.remove("real");
            }
        }
    }
    if let Some(w) = v.get_mut("works_to_place").and_then(Value::as_array_mut) {
        w.retain(|q| !q.as_object().is_some_and(side_is_enemy));
    }
    if let Some(m) = v.get_mut("projects").and_then(Value::as_object_mut) {
        for project in m.values_mut().filter_map(Value::as_object_mut) {
            if side_is_enemy(project) {
                project.remove("item");
            }
        }
    }
    if let Some(m) = v
        .get_mut("pending_motorisation")
        .and_then(Value::as_object_mut)
    {
        m.retain(|id, _| !enemy_piece(id));
    }
    for p in state.pieces.values().filter(|p| is_enemy(p.side)) {
        if let Some(o) = v["pieces"][p.id.as_str()].as_object_mut() {
            for key in ["load", "motorised", "escorted"] {
                o.remove(key);
            }
        }
    }
    // Hidden movement (27.31) is a rule, not limited intelligence: an
    // enemy piece off the map goes even with open information.
    let hidden = hidden_from(state, viewer);
    if !hidden.is_empty() {
        strip_hidden(&mut v, &hidden);
    }
    if open {
        return Ok(v);
    }

    // Layer 2: limited intelligence.
    for p in state.pieces.values().filter(|p| is_enemy(p.side)) {
        let Some(o) = v["pieces"]
            .get_mut(p.id.as_str())
            .and_then(Value::as_object_mut)
        else {
            continue;
        };
        o.retain(|k, _| ENEMY_PIECE_KEEPS.contains(&k.as_str()));
        if p.kind == PieceKind::DummySupplyUnit {
            o.insert("kind".into(), json!(PieceKind::SupplyUnit));
        }
        if o.get("location")
            .is_some_and(|l| l.get("pending").is_some())
        {
            o.insert("location".into(), json!("pending"));
        }
    }
    for key in ["planned", "field_rolled", "destroyed_rolled"] {
        if let Some(items) = v.get_mut(key).and_then(Value::as_array_mut) {
            items.retain(|item| {
                let side = item.get("side").or_else(|| item.get(0));
                !side
                    .and_then(Value::as_str)
                    .is_some_and(|s| names.iter().any(|n| n == s))
            });
        }
    }
    for key in ["broken_down", "destroyed_tanks"] {
        if let Some(m) = v.get_mut(key).and_then(Value::as_object_mut) {
            for wrecks in m.values_mut().filter_map(Value::as_array_mut) {
                wrecks.retain(|w| !w.as_object().is_some_and(side_is_enemy));
            }
            m.retain(|_, w| w.as_array().is_none_or(|a| !a.is_empty()));
        }
    }
    for key in [
        "prisoners",
        "prisoners_out",
        "prisoners_off_map",
        "guards",
        "escaped",
        "fleet_strikes",
        "chariot_raid",
        "dummy_tanks",
    ] {
        if let Some(m) = v.get_mut(key).and_then(Value::as_object_mut) {
            names.iter().for_each(|s| {
                m.remove(s);
            });
        }
    }
    // An empty map would itself tell that a raid is planned (30.41), or
    // that the enemy has dummy tank formations (16.4).
    for key in ["chariot_raid", "dummy_tanks"] {
        if let Some(o) = v.as_object_mut()
            && o.get(key)
                .is_some_and(|m| m.as_object().is_some_and(Map::is_empty))
        {
            o.remove(key);
        }
    }
    if names.iter().any(|n| n == "axis")
        && let Some(o) = v.as_object_mut()
    {
        o.remove("convoy_lanes");
    }
    if let Some(m) = v.get_mut("split").and_then(Value::as_object_mut) {
        m.retain(|id, _| !enemy_piece(id));
    }
    if let Some(Pending::ForcedRetreat { side, .. }) = &state.pending
        && is_enemy(*side)
        && let Some(p) = v
            .get_mut("pending")
            .and_then(|p| p.get_mut("forced-retreat"))
        && let Some(o) = p.as_object_mut()
    {
        o.remove("raw");
    }
    if let Some(combat) = v.get_mut("combat").and_then(Value::as_object_mut) {
        for key in ["plots", "assignments"] {
            if let Some(m) = combat.get_mut(key).and_then(Value::as_object_mut) {
                names.iter().for_each(|s| {
                    m.remove(s);
                });
            }
        }
        if let Some(m) = combat.get_mut("charged").and_then(Value::as_object_mut) {
            m.retain(|id, _| !enemy_piece(id));
        }
        for key in ["probing", "barrage_involved"] {
            if let Some(a) = combat.get_mut(key).and_then(Value::as_array_mut) {
                a.retain(|id| !id.as_str().is_some_and(enemy_piece));
            }
        }
        // 14.23's allowances tell how many points were aimed (3.61):
        // the phasing side's own.
        if state.phasing().is_none_or(is_enemy) {
            combat.remove("no_armour");
        }
    }

    // Layer 3: aliases.
    aliases.alias_json(viewer, &mut v);
    Ok(v)
}

/// `state` with every piece hidden from `viewer` off the board, for
/// trying an order on (a preview): it then answers as `viewer`'s own
/// view would have it, so a hidden raider's hex walks as empty and
/// announces nothing (27.31, 27.42). A hex where `viewer` was told of a
/// raider (`spot_chances`) keeps its raiders, so a try previews as made.
pub fn as_known_to(state: &GameState, viewer: Side) -> GameState {
    let mut s = state.clone();
    let told: BTreeSet<_> = s.spot_chances.values().cloned().collect();
    for p in s.pieces.values_mut() {
        if p.hidden && p.side != viewer && !p.hex().is_some_and(|h| told.contains(h)) {
            p.location = crate::piece::Location::Eliminated;
        }
    }
    // The Axis's strikes on the fleet are its own (32.72): the copy holds
    // none, rolled already, so a preview never shows a strike to come.
    if s.modules.fleet && viewer != Side::Axis {
        s.fleet_strikes.insert(Side::Axis, 0);
    }
    // Its chariot raid's plan too (30.41).
    if viewer != Side::Axis {
        s.chariot_raid.remove(&Side::Axis);
    }
    // And the enemy's dummy tank formations (16.4).
    s.dummy_tanks.remove(&viewer.other());
    s
}

/// The ids of pieces `viewer` may not see at all: the other side's
/// hidden ones (27.31).
pub(super) fn hidden_from(state: &GameState, viewer: Option<Side>) -> BTreeSet<&str> {
    state
        .pieces
        .values()
        .filter(|p| p.hidden && Some(p.side) != viewer)
        .map(|p| p.id.as_str())
        .collect()
}

/// Whether `v` names any of `hidden`, as a key or a string anywhere.
pub(super) fn names_any(v: &Value, hidden: &BTreeSet<&str>) -> bool {
    match v {
        Value::String(s) => hidden.contains(s.as_str()),
        Value::Array(a) => a.iter().any(|x| names_any(x, hidden)),
        Value::Object(o) => o
            .iter()
            .any(|(k, x)| hidden.contains(k.as_str()) || names_any(x, hidden)),
        _ => false,
    }
}

/// Removes every mention of a hidden piece: a key naming it, an array
/// element that is its id, and an object or element whose own field is
/// its id (a `moved` entry, a pending step).
fn strip_hidden(v: &mut Value, hidden: &BTreeSet<&str>) {
    let names = |x: &Value| match x {
        Value::String(s) => hidden.contains(s.as_str()),
        Value::Object(o) => o
            .values()
            .any(|f| f.as_str().is_some_and(|s| hidden.contains(s))),
        _ => false,
    };
    match v {
        Value::Array(a) => {
            a.retain(|x| !names(x));
            a.iter_mut().for_each(|x| strip_hidden(x, hidden));
        }
        Value::Object(o) => {
            o.retain(|k, x| !hidden.contains(k.as_str()) && !names(x));
            o.values_mut().for_each(|x| strip_hidden(x, hidden));
        }
        _ => {}
    }
}

/// The labels every piece carries, and the shell flag (9.26).
fn label(state: &GameState, data: &Data, v: &mut Value) -> Result<()> {
    for p in state.pieces.values() {
        let unit_type = p
            .nation
            .zip(p.id_code.as_deref())
            .and_then(|(nation, code)| data.chars.row(nation, code))
            .map(|row| row.unit_type.clone());
        let oa = p.id.unit().and_then(|id| data.oa.unit(&id));
        let o = v["pieces"][p.id.as_str()]
            .as_object_mut()
            .context("a piece is an object")?;
        o.insert("unit_type".into(), json!(unit_type));
        o.insert("name".into(), json!(oa.map(|u| u.name.clone())));
        o.insert(
            "abbreviation".into(),
            json!(oa.and_then(|u| u.abbreviation.clone())),
        );
        if state.is_shell(data, &p.id) {
            o.insert("shell".into(), json!(true));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::{Log, replay};
    use std::collections::BTreeSet;

    fn data() -> Data {
        Data::load("grazianis-offensive").unwrap()
    }

    fn log(name: &str) -> Log {
        let path = cna_data::repo_root().join("tests/games").join(name);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// The module-on golden game after its first `n` orders.
    fn golden(data: &Data, n: usize) -> GameState {
        let mut l = log("graziani-m5.json");
        l.orders.truncate(n);
        replay(data, &l).unwrap()
    }

    fn keys(v: &Value) -> BTreeSet<String> {
        v.as_object().unwrap().keys().cloned().collect()
    }

    fn a() -> Aliases {
        Aliases::new([9; 32])
    }

    #[test]
    fn only_the_axis_sees_its_hold_on_the_cities_64_71() {
        let data = data();
        let mut s = golden(&data, 70);
        s.axis_held_since = Some(cna_data::time::GameTime::new(20, 2));
        let cw = view_for(&s, &data, &a(), Some(Side::Cw), false).unwrap();
        assert!(cw.get("axis_held_since").is_none());
        let axis = view_for(&s, &data, &a(), Some(Side::Axis), false).unwrap();
        assert!(axis.get("axis_held_since").is_some());
    }

    #[test]
    fn a_wreck_s_owner_shows_to_its_side_and_the_enemy_sees_neither_61_38() {
        let data = data();
        let mut s = golden(&data, 70);
        let unit = cna_data::oa::UnitId::parse("unit:cw:7-hus").unwrap();
        let hex = cna_data::map::HexId::parse("C4021").unwrap();
        s.broken_down.insert(
            hex.clone(),
            vec![crate::breakdown::Wreck {
                side: Side::Cw,
                nation: None,
                weapon: "Matilda".into(),
                points: 3,
                this_stage: 0,
                towed: 0,
                unit: Some(unit),
            }],
        );
        for open in [false, true] {
            let cw = view_for(&s, &data, &a(), Some(Side::Cw), open).unwrap();
            assert_eq!(cw["broken_down"]["C4021"][0]["unit"], "unit:cw:7-hus");
        }
        let axis = view_for(&s, &data, &a(), Some(Side::Axis), false).unwrap();
        assert!(axis["broken_down"].get("C4021").is_none());
        // Open information shows the enemy's wrecks whole, owner and all.
        let axis = view_for(&s, &data, &a(), Some(Side::Axis), true).unwrap();
        assert_eq!(axis["broken_down"]["C4021"][0]["unit"], "unit:cw:7-hus");
    }

    #[test]
    fn every_field_has_a_row() {
        let data = data();
        let mut s = golden(&data, 70);
        // Fill the optional fields so they serialise.
        s.rail_supply_way.get_or_insert(crate::rail::RailWay::East);
        s.rail_runs.insert(crate::rail::RailWay::East);
        let v = serde_json::to_value(&s).unwrap();
        let classified: BTreeSet<String> = STATE_KEEPS
            .iter()
            .chain(STATE_FILTERS)
            .map(|k| k.to_string())
            .collect();
        let unclassified: Vec<_> = keys(&v).difference(&classified).cloned().collect();
        assert!(unclassified.is_empty(), "GameState: {unclassified:?}");
        let pieces: BTreeSet<String> = ENEMY_PIECE_KEEPS
            .iter()
            .chain(ENEMY_PIECE_REMOVES)
            .map(|k| k.to_string())
            .collect();
        for p in v["pieces"].as_object().unwrap().values() {
            let left: Vec<_> = keys(p).difference(&pieces).cloned().collect();
            assert!(left.is_empty(), "Piece: {left:?}");
        }
        let combat: BTreeSet<String> = COMBAT_KEEPS
            .iter()
            .chain(COMBAT_FILTERS)
            .map(|k| k.to_string())
            .collect();
        let left: Vec<_> = keys(&v["combat"]).difference(&combat).cloned().collect();
        assert!(left.is_empty(), "CombatSegment: {left:?}");
    }

    #[test]
    fn own_pieces_keep_everything_and_enemy_ones_their_face() {
        let data = data();
        let s = golden(&data, 70);
        let full = view_for(&s, &data, &a(), Some(Side::Axis), true).unwrap();
        let v = view_for(&s, &data, &a(), Some(Side::Axis), false).unwrap();
        assert!(v.get("log").is_none() && v.get("rng").is_none());
        assert!(v.get("to_act").is_some() && v.get("assaults_to_resolve").is_some());
        for p in s.pieces.values() {
            let id = a().alias(Some(Side::Axis), &p.id);
            let seen = &v["pieces"][id.as_str()];
            if p.side == Side::Axis {
                assert_eq!(*seen, full["pieces"][p.id.as_str()], "{}", p.id);
                assert!(seen.get("toe").is_some());
            } else {
                for k in keys(seen) {
                    assert!(ENEMY_PIECE_KEEPS.contains(&k.as_str()), "{}: {k}", p.id);
                }
                for k in ["toe", "cohesion", "cp_spent", "morale", "status", "parent"] {
                    assert!(seen.get(k).is_none(), "{}: {k}", p.id);
                }
                assert!(seen.get("unit_type").is_some() && seen.get("name").is_some());
            }
        }
    }

    #[test]
    fn enemy_supply_is_aliased_and_dummies_look_real() {
        let data = data();
        let s = golden(&data, 70);
        let v = view_for(&s, &data, &a(), Some(Side::Cw), false).unwrap();
        let text = v.to_string();
        let dummies: Vec<&PieceId> = s
            .pieces
            .values()
            .filter(|p| p.side == Side::Axis && p.kind == PieceKind::DummySupplyUnit)
            .map(|p| &p.id)
            .collect();
        assert!(!dummies.is_empty());
        assert!(
            !text.contains("supply:axis:dummy"),
            "an Axis dummy id leaks"
        );
        for p in v["pieces"].as_object().unwrap().values() {
            if p["side"] == json!("axis") {
                assert_ne!(p["kind"], json!("dummy-supply-unit"), "a dummy kind leaks");
            }
        }
        for d in dummies {
            let alias = a().alias(Some(Side::Cw), d);
            assert_eq!(v["pieces"][alias.as_str()]["kind"], json!("supply-unit"));
        }
        // The Commonwealth's own dummies stay as they are.
        let own = s
            .pieces
            .values()
            .find(|p| p.side == Side::Cw && p.kind == PieceKind::DummySupplyUnit);
        if let Some(p) = own {
            assert_eq!(
                v["pieces"][p.id.as_str()]["kind"],
                json!("dummy-supply-unit")
            );
        }
    }

    #[test]
    fn an_enemy_piece_to_be_placed_shows_no_placement() {
        let data = data();
        let s = GameState::with_modules(&data, 1, crate::state::Modules::ALL).unwrap();
        let v = view_for(&s, &data, &a(), Some(Side::Cw), false).unwrap();
        let axis = s
            .pieces
            .values()
            .find(|p| {
                p.side == Side::Axis && matches!(p.location, crate::piece::Location::Pending(_))
            })
            .unwrap();
        let id = a().alias(Some(Side::Cw), &axis.id);
        assert_eq!(v["pieces"][id.as_str()]["location"], json!("pending"));
        let own = view_for(&s, &data, &a(), Some(Side::Axis), false).unwrap();
        assert!(own["pieces"][axis.id.as_str()]["location"]["pending"].is_object());
    }

    #[test]
    fn a_sealed_answer_is_hidden_from_the_other_viewer_even_with_open_information() {
        let data = data();
        let l = log("graziani-m5.json");
        // Find a prefix where a sealed step holds one side's answer.
        let mut s = GameState::with_modules(&data, l.seed, l.modules).unwrap();
        s.ruleset = l.ruleset;
        let mut found = false;
        for o in &l.orders {
            s.apply(&data, o.player, o.order.clone()).unwrap();
            if let Some(Pending::Sealed {
                step: SealedStep::Barrage,
                waiting,
            }) = &s.pending
                && let Some(answered) = s.combat.plots.keys().next()
            {
                let other = answered.other();
                assert!(waiting.contains(&other));
                for open in [true, false] {
                    let v = view_for(&s, &data, &a(), Some(other), open).unwrap();
                    assert!(v["combat"]["plots"].get(answered.to_string()).is_none());
                    let own = view_for(&s, &data, &a(), Some(*answered), open).unwrap();
                    assert!(own["combat"]["plots"].get(answered.to_string()).is_some());
                }
                found = true;
                break;
            }
        }
        assert!(found, "the golden game has a barrage plot pending");
    }

    #[test]
    fn open_information_shows_enemy_counters_whole_bar_owner_secrets() {
        let data = data();
        let s = golden(&data, 70);
        let v = view_for(&s, &data, &a(), Some(Side::Cw), true).unwrap();
        let p = s
            .pieces
            .values()
            .find(|p| p.side == Side::Axis && !p.kind.is_supply())
            .unwrap();
        assert!(v["pieces"][p.id.as_str()].get("toe").is_some());
        assert!(
            v.to_string().contains("supply:axis:"),
            "no aliases when open"
        );
        assert!(v["motorisation"].get("axis").is_none());
    }

    #[test]
    fn the_enemy_does_not_learn_which_units_fired_a_barrage() {
        let data = data();
        let s = golden(&data, 68);
        let fired: Vec<_> = s
            .combat
            .barrage_involved
            .iter()
            .filter(|id| s.pieces[*id].side == Side::Axis)
            .collect();
        assert!(!fired.is_empty(), "an Axis unit barraged by order 68");
        let v = view_for(&s, &data, &a(), Some(Side::Cw), false).unwrap();
        for id in fired {
            assert!(
                !v["combat"]["barrage_involved"]
                    .to_string()
                    .contains(id.as_str()),
                "{id}"
            );
        }
        let own = view_for(&s, &data, &a(), Some(Side::Axis), false).unwrap();
        let own = own["combat"]["barrage_involved"].to_string();
        for id in s.combat.barrage_involved.iter() {
            if s.pieces[id].side == Side::Axis {
                assert!(own.contains(id.as_str()), "{id}");
            }
        }
    }

    /// R-145: both sides see prisoners escape, but how many have rejoined
    /// their own side (to come back as replacement points) is that side's.
    #[test]
    fn pow_and_guard_numbers_are_their_owner_s() {
        let data = data();
        let mut s = golden(&data, 70);
        let h = cna_data::map::HexId::parse("C4807").unwrap();
        for side in [Side::Axis, Side::Cw] {
            s.prisoners.entry(side).or_default().insert(h.clone(), 3);
            s.guards.entry(side).or_default().insert(h.clone(), 1);
            s.prisoners_out.insert(side, 2);
            s.escaped.insert(side, 4);
        }
        let v = view_for(&s, &data, &a(), Some(Side::Cw), false).unwrap();
        for key in ["prisoners", "guards", "prisoners_out", "escaped"] {
            assert!(v[key].get("axis").is_none(), "{key}");
            assert!(v[key].get("cw").is_some(), "{key}");
        }
    }

    fn mentions(v: &Value, id: &str) -> bool {
        names_any(v, &BTreeSet::from([id]))
    }

    #[test]
    fn a_hidden_raider_is_gone_from_the_enemy_s_view_27_31() {
        let data = data();
        let mut s = golden(&data, 70);
        let full = view_for(&s, &data, &a(), Some(Side::Axis), true).unwrap();
        // A Commonwealth piece the board names somewhere besides `pieces`.
        let id = s
            .pieces
            .values()
            .filter(|p| p.side == Side::Cw && p.kind == PieceKind::Combat)
            .map(|p| p.id.clone())
            .find(|id| {
                let mut f = full.clone();
                f.as_object_mut().unwrap().remove("pieces");
                mentions(&f, id.as_str())
            })
            .unwrap();
        s.pieces.get_mut(&id).unwrap().hidden = true;
        for open in [false, true] {
            let v = view_for(&s, &data, &a(), Some(Side::Axis), open).unwrap();
            assert!(!mentions(&v, id.as_str()), "open {open}");
            let own = view_for(&s, &data, &a(), Some(Side::Cw), open).unwrap();
            assert!(own["pieces"].get(id.as_str()).is_some(), "open {open}");
        }
    }

    #[test]
    fn a_spectator_sees_both_sides_as_an_enemy() {
        let data = data();
        let s = golden(&data, 70);
        let v = view_for(&s, &data, &a(), None, false).unwrap();
        for p in s.pieces.values() {
            let id = a().alias(None, &p.id);
            assert!(v["pieces"][id.as_str()].get("toe").is_none(), "{}", p.id);
        }
    }

    #[test]
    fn a_finished_game_hides_nothing() {
        let data = data();
        let s = replay(&data, &log("graziani-short.json")).unwrap();
        assert!(s.is_over());
        let v = view_for(&s, &data, &a(), Some(Side::Cw), false).unwrap();
        let p = s.pieces.values().find(|p| p.side == Side::Axis).unwrap();
        assert!(v["pieces"][p.id.as_str()].get("toe").is_some());
    }
}
