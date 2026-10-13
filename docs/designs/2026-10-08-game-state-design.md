# cna-engine: headless game state for Graziani's Offensive

**Date:** 2026-10-08
**Mission:** 2 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** proposed, waiting for Brian's review (PR labelled `needs-brian`).
Nothing in this mission is built until **Feedback** in
`docs/autopilot/PROGRESS.md` approves it.

Written by the autopilot under Brian's delegation of 2026-09-26: every
decision below is Claude's, is cheap to reverse, and can be reversed with
one PR. The order (one small scenario, a headless game state for it, then a
minimal UI) is Brian's, from the *Path to a playable game* agreed on
2026-10-07.

## Goal

A headless engine that loads **Graziani's Offensive** (SPI 60.22, Land Game
only per 60.92) from `vendor/cna`, places every unit as typed state, and
plays it: scripted orders go in, the engine enforces the turn sequence,
movement, stacking, zones of control, reserves and combat, rejects illegal
orders with the case number (or case group, where cna maps only the
group) they break, and writes a log from which the
whole game replays exactly. The state serialises to JSON and back.

How this moves **the goal** (two players, a browser, every rule enforced):
Mission 1 gave a rules core that resolves one combat in isolation. This
mission turns it into a *game*: a state that changes only through legal
orders. Mission 3's UI is a view of this state plus a way to send it orders,
and a two-player server later is the same state behind a network. Nothing
here is throwaway; every later mission adds systems to this state machine.

Why Graziani's Offensive: cna picked it as the first complete scenario
(cna `docs/designs/2026-10-08-scenarios-and-oa-design.md`): six
Game-Turns, every starting unit listed, and playable without the Air or
Logistics Games. It landed in cna on 2026-10-08 and this PR bumps
`vendor/cna` to it.

## Non-goals

- **A UI, a server, AI players.** Missions 3 and later.
- **Hidden information as views.** The state holds everything, secrets
  included. Sealed orders (barrage plots, force assignment) are collected
  from both players before they resolve, so the *order model* is right for
  a server; filtering the state per player (3.6x limited intelligence,
  dummy supply units) waits for the server.
- **Abstract supply and trucks (§32).** Every unit counts as supplied, with
  unlimited ammunition and fuel; supply units are placed but inert;
  motorisation points are not tracked. This is the "rest of the Land Game
  turn" mission in the queue. `NOT_SIMULATED.md` gains a *Systems* table
  listing it. Victory's supply clause is not §32 supply: it is
  R-110's route trace (Decisions, *Victory*).
- **Breakdown (§21), organisation (§20), engineering and construction
  (§24–26), repair (phase K), patrols (phase L), rail (phase J), the naval
  convoy and fleet (phases II, D, E), training.** Each phase still exists in
  the sequence and is passed through with a `PhaseSkipped` event, so adding
  one later changes only that phase.
- **The Air and Logistics Games.** Their scenario blocks are ignored; the
  `abstractions.air_and_logistics` block is applied, as 60.92 says.
- **The Italian Campaign** (`extends` Graziani's Offensive). The loader
  supports `extends`, so loading it is a test, but playing past Game-Turn 6
  needs the Axis schedule and supply.

## What "done" means

1. `cna-play new grazianis-offensive --seed 7` writes a start state with
   every unit of the set-up on its hex or box, and every free placement
   ("anywhere in Libya", "within 3 hexes of C0716") waiting for a set-up
   order from its owner.
2. A scripted game (`tests/games/*.json`: set-up orders and a few stages of
   moves, assaults and barrages by both sides) plays to its end; a golden
   test pins the final state and the log.
3. `cna-play replay log.json` rebuilds the same final state, byte for byte.
4. For every illegal-order case the spec lists below there is a test that
   the order is rejected, the state is unchanged, and the error names the
   case (`8.1x`, `10.2x`, …).
5. At the end of Game-Turn 6 the engine reports each side's victory level
   (60.8) with the conditions it checked.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Crates | `cna-data` gains loaders for the map, scenarios, OA sheets, unit characteristics and the reinforcement schedule. A new **`cna-game`** library holds the state, orders, sequence of play, movement and combat glue. A new **`cna-play`** binary drives it. `cna-rules` stays pure functions | Keeps I/O, rules and state apart, as Mission 1 did; `cna-game` compiles to WebAssembly for Mission 3 if it never touches the file system itself |
| Map | Hex ids stay cna's strings (`C3922`) in a `HexId` newtype; adjacency computed from `sheets.json` as `tools/map_geom.py` does; hexsides with no record are plain. Cross-sheet adjacency from cna's `data/map/seams.json` (359 pairs across A|B … D|E, with the hexside features of each pair); a seam pair is a hexside like any other | cna's data is the authority on the map; computing the join here would be a second copy of a map fact |
| Pieces | Every OA unit a deployment names (the counter plus the OA subtree it stands for, adjusted by `less`, `detached`, `attached`, `alone`) becomes one **piece** with a `UnitId`, its side, nation, `id_code`, parent, TOE points (printed, or `max_toe` for `N`), CPA, basic morale and location | The OA unit is the counter; the engine needs per-counter CP, cohesion and losses |
| Location | `Location::Hex(HexId)`, `Location::Box(Box)` (Tripoli, Tripolitania…), `Location::Pending(Placement)` for a free set-up, `Location::Eliminated` | Off-map boxes are where some Axis units start (8.8x) |
| Free set-up | A deployment whose placement is not a list of hexes or a place leaves its pieces `Pending`; the owner's `Place` orders put them on legal hexes (placement area, stacking, `not_within`). Play starts when no piece is pending. `constraint` text the grammar cannot check is logged as unchecked | 59.2 leaves these choices to the player |
| Clock | `Clock { game_turn, stage, phase, half: A/B, segment }` with phases A–L as cna's `phase` enum, and the movement-combat cycle's segments (movement, breakdown, combat steps 1–6, reserve release) as an enum | The letters match SPI 5.2 so logs and errors cite the printed sequence |
| Orders | One `Order` enum, each with the player who gives it: `Place`, `DeclareInitiative`, `DesignateReserve`, `Move { pieces, path }`, `EndSegment`, `DeclareZoc`, `React`, `Barrage`, `RetreatBeforeAssault`, `AssignForces` (sealed), `Assault`, `ReleaseReserve`, `Pass`. JSON via serde, externally tagged | A closed enum is what a UI and a server send; serde gives the JSON form free |
| Decisions the opponent owes | The state has a `pending: Option<Pending>` naming the player who must answer and what (ZOC declaration 10.16, reaction 8.51, retreat before assault, a sealed plot from both sides). While it is set, only that player's answer is legal | Turns SPI's interrupts into a strict turn order a headless driver and a server can both follow |
| Legality | `fn apply(&mut self, order) -> Result<Vec<Event>, Illegal>` validates fully before changing anything; `Illegal { case: "8.1x", reason }`. Rejected orders never enter the log | "Illegal ones rejected" with the reason a player can look up |
| Log and replay | The log is the scenario id, ruleset, seed, `vendor/cna` commit, and the accepted orders. Replay re-applies them; events are derived, not stored as truth. A test replays every golden game | Small, and a replay that disagrees with the state is a bug the test catches |
| Randomness | `rand_chacha::ChaCha8Rng` seeded per game, held in the state (serialised as seed plus draw count). Every die roll is an event | Reproducible; the Mission 1 rule (seeded ChaCha8 where sampling is needed) |
| Serialisation | `GameState` derives `Serialize`/`Deserialize`; static data (map, OA, tables) is not in the state, only the scenario id and the `vendor/cna` commit; loading checks the commit | A save is a few hundred kB, not the map |
| Rulesets in state | The state carries its `Ruleset`. From this mission on **each switch defaults to the option cna accepted** (the ruling's Decision), not the printed-text option. Today that changes one default: R-018 goes from option 3 to option 2, and its probe is regenerated | Rulings are now decided; a game should play the decided rules unless a group switches |
| Interpretive rulings | R-003, R-004, R-006, R-007, R-008 (contact, mixed formations, who reacts, reaction CP, "in combat") are implemented as decided and leave `NOT_SIMULATED.md` with a switch each **only if** cna lists more than one option worth playing; otherwise they move to a new "Implemented as decided" table there | The coverage test stays honest without inventing switches nobody wants |
| Movement costs | From `terrain-effects.json` with errata (E-031: a track halves the hex cost). Roads and tracks are hexside features; their rate applies only along a connected road or track hexside (8.33). Motorised or not from the CPA mark (`+`, `*`) and unit type | The data says it; the code does not retype a cost |
| Weather | Rolled each stage in phase B from `weather.json` (E-025 applied); sandstorm and rain change movement (§29) | Cheap, and it changes legal moves |
| Initiative | The Axis holds it through Game-Turn 1 (60.6); from Game-Turn 2 it is rolled with `initiative-ratings.json` | Data-driven |
| Reinforcements | Arrive from `reinforcement-schedule.json` in phase D of their stage (20.11–20.15): Commonwealth at Cairo, Axis at Tripoli; a listed parent brings its OA sheet less units with a later arrival of their own (4.42). The Benghazi diversion (55.1) waits for the Logistics Game | cna states the entry points; the schedule needs no hexes |
| Combat | Raw strengths are TOE points × ratings from `unit-characteristics.json` (per ID code) and, for tanks and guns, from `weapon-systems.json` (4.47–4.49): each TOE entry's `weapon` name maps through `oa_names[nation]` to a weapon row, whose ratings replace the ID-code row's for those points. A TOE name with no mapping is read as an ID code if one matches (the RECAM HQ's `vv`, the only case today), else it is a load error. Actual strength follows `60-combat.md` (raw ÷ 10, rounded). Resolution calls the Mission 1 rules core, sampling one result with the game's RNG | Combat is already in the core; this mission wires it to pieces and the map |
| Stacking points | From the unit's equivalent (division 5 … company 0, 9.4): the `equivalent` field of its `unit-characteristics.json` row (brigade, battalion, company, hq); for the two `by-unit` rows, the OA unit's own `equivalent`. HQs by 9.11–9.15 | cna states it per row since 18ed96e; the engine does not parse `unit_type` |
| Victory | Evaluated after Game-Turn 6 OpStage 3 from `victory.levels`: `hold` conditions are checked, and each holding unit's `supply_trace` (R-110): a path of any length a medium truck may enter, from the unit's hex to any hex of the named places, hexes or sheet, through no hex holding an enemy unit and no enemy-ZOC hex without a friendly unit | R-110 gives the Land-only meaning; it reuses slice 4's terrain prohibitions and ZOC, so no §32 supply is needed |

## Crates touched

- `cna-data`: `map`, `scenario`, `oa`, `characteristics`, `schedule`,
  `weather`, `initiative`, and `terrain` grows its CP and stacking columns.
- `cna-rules`: `ruleset` defaults; small pure helpers where a movement or
  combat computation has no state in it (movement cost of one step, ZOC
  eligibility, stacking sum).
- `cna-game` (new): `state`, `clock`, `order`, `event`, `setup`,
  `movement`, `zoc`, `reserve`, `combat`, `victory`, `log`.
- `cna-play` (new): `new`, `apply`, `replay`, `show`.
- `cna-probe`: unchanged except the regenerated R-018 probe.

## Testing

- **Loader tests:** every Graziani unit entry resolves to pieces; piece
  counts per side are pinned; every placement hex exists; `extends` loads
  the Italian Campaign.
- **Map tests:** neighbours of a few hand-checked hexes on each sheet;
  hexside lookup both ways; a seam pair from `seams.json` (C0633–D0701)
  is adjacent both ways with its slope.
- **Rule tests, one per case enforced:** a small state built in the test,
  one order, the expected result or the expected `Illegal` case. Movement
  costs are checked against hand-computed paths on the real map.
- **Serialisation:** every golden state round-trips through JSON unchanged.
- **Golden games:** scripted games in `tests/games/`, replayed in CI;
  the final state's hash is pinned.
- **Determinism:** the same seed and orders give the same state on every
  run; a property test applies random legal moves and replays them.
- Gates as before: `cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo run -p cna-probe -- check`.

## Slices

One PR each, in order; each has a plan in `docs/plans/`.

1. **Map** (`2026-10-08-m2-slice-1-map.md`): hexes, hexsides, places,
   regions, adjacency, terrain lookup.
2. **Scenario and pieces** (`…-slice-2-scenario.md`): OA, characteristics,
   schedule and scenario loaders; deployments expanded to pieces;
   placements resolved; free placements left pending.
3. **State, clock and orders** (`…-slice-3-state.md`): `GameState`, the
   sequence of play as a state machine with every phase, initiative,
   weather, set-up orders, the log, replay, JSON round trip, `cna-play`.
   Ruleset defaults follow cna's decisions.
4. **Movement** (`…-slice-4-movement.md`): CP and CPA (§6), terrain and
   hexsides (§8), roads and tracks, prohibitions, off-map boxes, stacking
   (§9), ZOC and contact (§10), break contact and disengagement, the two-hex
   continuation rule, reaction, reserves (§18), overspend DP and the 150 %
   cap, reinforcements.
5. **Combat** (`…-slice-5-combat.md`): the six combat steps wired to the
   rules core: position determination, barrage, retreat before assault,
   sealed force assignment, anti-armour, close assault; losses on TOE,
   retreats, captures, cohesion and the must-attack rule (10.3).
6. **Victory and the golden game** (`…-slice-6-victory.md`): 60.8
   evaluation, a scripted multi-stage game, docs (README,
   `NOT_SIMULATED.md`, this Status, the queue).

## Requests for cna this mission makes

Posted to `PROGRESS.md` **Requests for cna** with this PR. **All six were
answered on cna main by 18ed96e** (2026-10-08), which this PR now vendors:
(1) `data/map/seams.json`; (2) `data/tables/weapon-systems.json`; (3)
R-110, `victory.levels[].supply_trace`; (4) `equivalent` per
characteristics row; (5) the sequence now says phasing/non-phasing, and
the probe refund is right (6.3's asterisk: a defended probe costs 2 − 2 =
0, 15.9), so slice 5 follows `cp-costs.json`; (6) the track halves the
hex cost (E-031). The decisions above use the answers.

1. **Cross-sheet adjacency** for `data/map/`: which hexes face each other
   across each sheet edge, and the features of those hexsides (the `a|SIDE`
   edge records today). Blocks slice 4 moving a unit from sheet C to D.
2. **Weapon systems** (4.47–4.49): `data/tables/weapon-systems.json`, which
   `common.schema.json` already refers to. Blocks slice 5 for tanks and
   guns.
3. **The Land-Game-only meaning of "suppliable by convoy" and "a
   truck-convoy route"** in 60.8's victory conditions, as structured data or
   a ruling. Blocks the supply half of slice 6's victory check.
4. **A stacking equivalent per unit-characteristics row** (9.4): two
   `unit_type`s (`Engineer Bn/Coy-Eq`, `Construction (Road/RR)`) do not
   say whether they are battalions or companies. Slice 4 counts them as
   companies until then.
5. **Two combat inconsistencies inside cna:** `rules/20-sequence-of-play.md`
   gives retreat before assault to "Player B" and the assault order to
   "Player A", where `rules/60-combat.md` says non-phasing and phasing; and
   `cp-costs.json` gives the −4 refund to a defended probe, which the
   combat CP table does not. Slice 5 follows `60-combat.md`.
6. **The track cost:** `rules/40-movement.md` gives a track as 1 CP per hex
   while E-031 makes it half the hex cost. Which governs? Slice 4 follows
   the errata.
