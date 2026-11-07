# cna-engine: the Air Game as a module

**Date:** 2026-11-06
**Mission:** 7 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** proposed; waiting on Brian's review (PR label `needs-brian`).
Nothing in this spec is built. Read against cna 111220a, the commit
`vendor/cna` pins on main.

Written by the autopilot under Brian's delegation of 2026-09-26: every
decision below is Claude's, is cheap to reverse, and can be reversed with
one PR. The mission itself (the Air Game, SPI §33–47, once cna has restated
it) is Brian's, from `AUTOPILOT.md`'s *Later, in no fixed order* list. cna
finished restating it (its Mission 7, `rules/air/`, 399 cases) on
2026-11-06. It is a large mission, so Brian reviews this spec before any
building, as he did Mission 2's. It is written while Mission 6's spec
(the Logistics Game, PR #148) also waits for him; the two are independent
by design (*Order against Mission 6*, below), so either can be approved
first.

## Goal

Graziani's Offensive, and then every scenario, playable with the Air Game
in place of the abstract air rules. Today nothing flies: §32 stands in for
the air war (32.57 and 32.59's truck losses, 32.6's convoy bombing chart,
32.7's strikes on the fleet in `fleet.rs`, 32.8's stripped AA). When this
mission is done, a game started with the Air Game switch on tracks every
aircraft by type in squadrons with named pilots and ground crews (SGSUs)
at air facilities (§34–36); flies missions within range, fuelled, refitted
and armed by the SGSUs within each facility's capacity (§37–38); resolves
three land support air phases an Operations Stage's worth at a time, both
sides deploying together (§39–42): fighters on CAP and scramble, air-to-air
combat by pairings on the TacAir Kill Table, flak, then bombing, strafing,
reconnaissance, transport and airdrops (§40–46); and once a Game-Turn
flies the strategic war over Malta and the Axis convoy lanes (§41.6, §43,
§44), with §47's abstract logistics feeding the aircraft.

How this moves **the goal** (an open-source, self-hostable digital Campaign
for North Africa, two players in a browser, the engine enforcing every rule,
each ruling a switch): the Air Game is a third of the printed game and the
one no group plays by hand, because every aircraft is a line on a sheet.
Without it the engine plays a game where Malta, the convoy war and the
desert air forces are a die roll. It lands in `cna-game`'s state machine,
so the Mission 4 server serves it unchanged, and its secrets (missions
deployed blind, scramble targets, the choice to fight) are exactly what the
server's per-side views were built to keep.

## Non-goals

- **The Air Game with the Logistics Game.** This mission plays the Air
  Game with §32 kept and changed by §47 (cna's
  `rules/air/90-abstract-logistics.md`), which 33.0 allows. Air with full
  logistics (per-aircraft fuel and ammunition from dumps, port efficiency,
  pipelines, §58 dropped) is slice 8, built only after Mission 6 is whole.
- **Construction of air facilities (24.7).** Building airfields, landing
  strips and alighting areas waits for a later slice; the facilities a
  scenario starts with, and their repair by bombing levels, are in.
- **Optional rules and variants.** 40.1's *make your own aces* (campaign
  game only, optional) and cna's recorded variants V-009 to V-025 are not
  built unless Brian asks.
- **Paper bookkeeping.** Squadron sheets are the engine's state; the
  browser shows squadrons, not facsimiles of sheets.
- **AI players.**

## What "done" means

1. `cna-play new --air` and the browser's new-game screen (with the Air
   Game ticked) start any scenario cna gives an air set-up for
   (`sides.*.air`, today all four small scenarios), with its
   `abstractions.air_and_logistics` block skipped and its
   `abstractions.logistics` block applied (§32 kept): aircraft by type,
   pilots by rating, SGSUs, refitted counts, Malta's aircraft, AA points and
   facility capacity.
2. Every aircraft is tracked by squadron with its readied, refitted,
   fuelled and armed flags; a mission refused for range, capability,
   capacity, weather, an adjacent enemy or missing maintenance names its
   case.
3. Each Operations Stage's land support air phase runs between the fleet
   phase and reserve designation: both sides' missions sealed, revealed
   together; interception and scramble; air-to-air combat pairing by pairing
   on `tacair-kill.json` with pilot ratings and the maneuver adjustment;
   aborts; flak on `anti-aircraft-results.json`; then bombing
   (`air-bombardment.json`), strafing (`strafing.json`), reconnaissance,
   transport and airdrops, with their effects on the land game (pinned
   units, lost TOE, lost motorisation points, cut roads and railways, lost
   facility capacity). Night missions are their own pass.
4. Once a Game-Turn: strategic air planning (land support or strategic, per
   squadron), Axis raids on Malta through the availability levels, the
   convoy stage (Commonwealth convoy reconnaissance, cover, flak suppression
   and bombing per lane, under 47.6 and R-127, R-144), and strategic air
   recovery.
5. Reinforcements, pilots, SGSUs and Commonwealth withdrawals arrive and
   leave on their schedules; aircraft are refitted on `aircraft-refit.json`;
   shot-down aircraft and pilots are recovered on `pilot-plane-recovery.json`.
6. With the module on, 32.57, 32.59, 32.6, 32.7 and 32.8 are off and §47's
   depots, costs, air transport of supply units and abstract convoys are on,
   exactly as cna's `#switch` lists; every other §32 rule, 32.37's air lift
   included, stays. With it off, every game plays as today.
7. Every air ruling (R-031, R-111–R-116, R-127, R-144, and those cna adds)
   is a `Ruleset` switch or a `NOT_SIMULATED.md` row reading "as decided"
   with the module that builds it. An Air golden game for Graziani's
   Offensive and its Playwright twin match `replay` byte for byte; every old
   golden game and save replays unchanged.
8. `NOT_SIMULATED.md`'s rows for the Air Game read what the module does and
   what it still leaves out.

## The one switch

cna's `rules/air/00-overview-and-sequence.md#switch` names what the Air
Game throws out. The engine follows it exactly, in its *Air Game on,
Logistics Game off* column:

| Abstract rule | With `air_game` on |
|---|---|
| 32.57, 32.59 abstract motorisation losses (`convoy.rs`'s monthly losses) | off from slice 1; motorisation points are lost to strafing and bombing as trucks (47.5) |
| 32.6 simplified Axis convoys (`convoy.rs`'s bombing chart) | the chart is off from slice 1 (convoys arrive unbombed until slice 4, a `NOT_SIMULATED.md` line); replaced in slice 4 by 47.6's abstract convoys (no tonnage; three supply units, three depots, 25 motorisation points per lane), bombed under 41.6 with 47.64's results (R-127) |
| 32.7 strikes on the fleet (`fleet.rs`'s `roll_for_strikes`, `strike_the_fleet`) | off from slice 1; the rest of `fleet` (§30's ships, phase E, bombardment) stays, and the fleet is bombed and torpedoed by missions (41.34, 41.7, 47.7) |
| 32.8 AA modifications (the set-up's thinning, and `captured.rs`'s 32.81 drop of captured Axis AA) | off from slice 1; every AA unit and point stays and fires flak (R-114) |
| the rest of §32, 32.37's air lift included | stays, changed by §47: depots (47.14, 47.4), aircraft supply within 15 CP (47.13), §47.2's costs, depot movement (47.3), 47.35's aircraft carrying supply units beside 32.37 |
| §58 (Logistics Game's abstract air) | never applies with the Air Game; irrelevant here, as the Logistics Game is off |

How the switch is built:

- **A module, not a ruleset switch.** `Modules` gains `air_game: bool` with
  `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`. Saves
  and logs made before it read it as `false` and replay unchanged
  (AGENTS.md, *Determinism and saves*).
- **Requires `abstract_supply`** for this mission: `with_modules` refuses
  `air_game` without it (and, until slice 8, with Mission 6's
  `logistics_game`), naming 33.0. The abstract air rules are gated in
  slice 1 at their call sites, not by refusing whole modules: the code that
  runs them is gated only on `abstract_supply` (32.57/32.59 and 32.6 in
  `convoy.rs`, 32.7 in `fleet.rs`, 32.8 in `setup` and `captured.rs`), so
  each gains a `!air_game` test there. What replaces them lands slice by
  slice; until then the gap is logged and listed in `NOT_SIMULATED.md`.
- **The set-up's abstraction block.** With the module on, set-up applies
  `abstractions.logistics` in place of `abstractions.air_and_logistics`
  (today read unconditionally by `Scenario::land_only`, `setup.rs` and
  `unattached.rs`): motorisation from medium and heavy trucks, with 47.52's
  second- and third-line trucks at air facilities counted in. That reaches
  R-102 (`NOT_SIMULATED.md`: "not reached"), built in slice 1.
- **Not in `Modules::ALL`** until the mission's last slice: new games keep
  today's abstract air while the module is partial. `--air` and a browser
  checkbox opt in.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Which § numbers | cna's `::: spi` badges, which are SPI's: §34 aircraft, §35 squadrons and SGSUs, §36 facilities, §37 flight, §38 maintenance, §39 missions, §40 fighters, §41 bombing, §42 other missions, §43 Mediterranean bases, §44 Malta, §45 air combat, §46 flak, §47 abstract logistics | Errors cite SPI cases |
| Order against Mission 6 | Build on §32 plus §47 first; Air with the Logistics Game is slice 8, after Mission 6 | 33.0 allows the Air Game without the Logistics Game, and cna restates §47 for exactly that; the two missions then never block each other |
| First scenario | Graziani's Offensive: six Game-Turns, Italian aircraft only (no German bombers, so no §43 percentages), Malta with 31 aircraft and 17 AA points, a typed air set-up (`sides.axis.air`, `sides.cw.air`) | The scenario every golden plays and the smallest air order of battle. It cannot exercise the Axis's strategic missions: its Malta commitment allows level I only, the Axis may not base in Italy or Sicily, and convoy cover flies only from there (44.25–44.27, 40.24, 41.63), so slice 4 tests those on Operation Crusader |
| Aircraft | Each aircraft is a record in its squadron (`Aircraft { type_id, readied, refitted, fuelled, armed, pilot }`), keyed by an id the engine numbers at set-up; no map counter for grounded aircraft (34.71) | 34.0 tracks every aircraft by type; per-aircraft flags are what §38 and 37.24 read |
| Squadrons | `Squadron { nation, class, sgsu, aircraft, pilots }`, its location its SGSU's (35.11); ready capacity plus a third in reserve by 35.23's chart (R-116 for the Commonwealth before July 1941); German and Italian aircraft never share one (35.28) | 35.0: the squadron is the only air organisation |
| Forming squadrons at set-up | The set-up gives aircraft by type, not squadrons. The engine forms them: one squadron per SGSU the set-up gives (Graziani's: 39 Axis, 14 Commonwealth in Africa), aircraft dealt one type per squadron where the counts allow, within 35.23's sizes, in the chart's type order, placed at facilities within capacity by the same placement step as land units (each side may regroup before play by order) | 35.21 advises one type per squadron; a fixed default keeps scripted games short, an order keeps the player's choice |
| Pilots | Pilots are counts per squadron by rating (1–4, Marseille 6, R-115), assigned to aircraft only for the stage they fly; zero-rated pilots unlimited | 40.1x: pilots have no counters; ratings are all combat reads |
| Air facilities | `Facility { kind, at, capacity, max, owner_by_occupation }` per hex or box; Malta's as their own boxes with 18 aircraft per level (44.1x) | 36.x engine notes |
| Range and paths | Hex distance from `data/map` and `seams.json`; a mission names its path (default: a shortest path, ties by hex id) because interception reads it (40.27); off-map legs from the request below | 37.1x: "an engine computes distance from the map" |
| Simultaneous deployment | The land support air phase is a sealed step for both sides, as sealed combat options already are: each side's missions are an order the other cannot see, revealed when both have committed; scramble targets and 45.14's "fight or not" are sealed the same way | 39.0, 40.3x, 45.14; the server already redacts sealed orders (Mission 4 slice 1) |
| Air-to-air combat | The defender's layout (34.75) is the engine's: aircraft listed by type and squadron, never by pilot; the attacker's allocation is an order (default: each attacker against the defender with the lowest TacAir, covering every defender at least once); resolution pairing by pairing, firing order per 45.0's sequence, two ordered dice per shot | 45.x as restated; a default keeps scripts short, the order keeps the choice |
| Flak | Per target group, AA points summed, never divided by ten; day and night apart; R-111 (map flak heavy), R-113 (two density shifts) as decided; a gun-class unit pays 1 ammunition per target group fired at (47.21); pure flak units stack free in cities and at airfields (46.17) | 46.x engine notes |
| Bombing and strafing targets | Under §32+§47 a target that does not exist is not offered: supply dumps read as supply units (their ammunition and fuel lost by the percentage), ports' efficiency and the water pipeline wait for slice 8; trucks are motorisation points (47.5) | 47.5; Request 2 asks cna to confirm the mapping |
| Reconnaissance replies | The engine reports the revealed units truthfully; 42.24's "off by up to 2" is an order the owner may give within the bound before the reply goes out (default: the true figure) | Keeps the bluff the rule allows without a wait in every scripted game |
| Strategic air | Once a Game-Turn, after initiative and before the convoy stage: planning (each squadron land support or strategic, sealed), Malta raids (`axis-malta-availability.json`, `axis-malta-commitment.json`), convoy stage per lane, recovery at turn end | 33.0's turn outline and `logistics-sequence.json`'s `air_game_only` steps |
| Clock | Today the engine runs the naval convoy stage before initiative, inside stage 1's phase A, with no player acting (`phases.rs`, `convoy.rs`). With the module on the turn opens as `logistics-sequence.json` orders it: initiative, strategic air planning (sealed), the Malta raid, an interactive convoy stage with its air segments, then phase A; the land support air phase once a stage for both sides together, between phases E and F (R-031 as decided), strategic air recovery at turn end; mapped from `logistics-sequence.json`'s `air_game_only` rows | Off, the clock is today's, so old logs replay |
| Abstract convoys (47.6) | `convoy.rs` keeps its lanes; with the module on, a convoy is the turn's supply units, depots, motorisation points and replacement points by lane, without tonnage, bombed by convoy missions with 47.64's results | 47.6x as restated |
| Rulings | R-113 gets a `Ruleset` switch (a probe tells its options apart: flak kills over a dense target); R-111 is a row, since no printed map or box flak is typed yet and its option 3 has nothing to apply (Request 6); the rest are `NOT_SIMULATED.md` rows "as decided" with the module, in the slice that builds them | AGENTS.md: every ruling a switch or a row |
| Probes | Flak density (R-113): aircraft lost over a Graziani target by option, in the shape of `probes/R-018.json` | Probes are why rulings are switches |
| Limited intelligence | The enemy sees mission counters on target hexes (one per class, 34.71), not their numbers until combat; squadrons, pilots and facilities' grounded aircraft are the owner's; reconnaissance results reach only the scout | 3.61 and 34.7x |
| UI | An air layer on the map (facilities, SGSUs, mission counters); an air panel per phase: squadrons and maintenance, mission deployment (target, mission, path), the combat layout and allocation, results; strategic planning and convoy panels beside `FleetPanel` | "Shows up in the UI's panels", as in Missions 5 and 6 |

## Crates touched

- `cna-data`: loaders for `aircraft-characteristics`, `air-missions`,
  `aircraft-refit`, `air-reinforcement-schedule`,
  `axis-air-reinforcement-schedule`, `pilot-arrival`, `scramble`,
  `tacair-kill`, `maneuver-adjustment`, `strafing`, `air-bombardment`,
  `anti-aircraft-results`, `air-reconnaissance`, `convoy-reconnaissance`,
  `axis-convoy-bombing`, `axis-malta-availability`,
  `axis-malta-commitment`, `malta-construction`, `pilot-plane-recovery`,
  `harbour-mining`;
  the scenarios' `sides.*.air` blocks.
- `cna-rules`: pure helpers (range, kill numbers, maneuver shifts,
  strafing and bombardment columns, flak columns, refit rolls) and the
  R-113 switch.
- `cna-game`: an `air` module tree (`aircraft`, `squadron`, `facility`,
  `maintenance`, `mission`, `combat`, `flak`, `bombing`, `strafing`,
  `strategic`, `malta`, `reinforce`), `Modules::air_game`, the clock's air
  phases; `Order` and `Event` gain variants; `view` redacts them.
- `cna-probe`: the flak density probe (R-113).
- `cna-play`: `--air`.
- `cna-web` and `web/`: the air layer, panels and the new-game checkbox.

## Testing

- **Rule tests per case**, failing first, each refusal naming its case.
- **Determinism and replay.** Every slice adds a property test (random
  Graziani Air games from a seed, as `determinism.rs` does) that two
  replays of the same log match byte for byte; the old goldens and every
  Playwright fixture stay pinned and must not change: the proof the module
  is off for them.
- **Conservation.** A property test that aircraft and pilots leave a side
  only by a named event (shot down, crashed, destroyed on the ground,
  withdrawn), and that every aircraft is in exactly one squadron.
- **Leaks.** The existing `leaks` test replays the Air golden and finds
  nothing either side may not know; sealed missions are checked hidden
  until revealed.
- **Golden game.** `tests/games/graziani-air.json`, pinned by `.sha256`,
  extended per slice; `regenerate_` rewrites it, per AGENTS.md's bump
  steps.
- **Playwright.** Each slice's panel driven by clicks to the pinned hash;
  one new spec, `air.spec.ts`.
- **Rulings.** The test that every ruling has a switch or a row keeps
  passing at every slice.

## Slices

One PR each, in order; each gets its own plan at its start except slice 1,
whose plan is `docs/plans/2026-11-06-air-game.md` together with the task
list for the rest. Each leaves a playable game.

1. **The switch, set-up and maintenance for Graziani's Offensive.**
   `air_game`, requiring `abstract_supply`; 32.57/32.59, 32.6's chart, 32.7's
   strikes and 32.8 gated off at their call sites; `abstractions.logistics`
   applied (R-102); the tables loaded; facilities, with a set-up step for
   those the players place; squadrons formed and SGSUs placed; pilots;
   Malta's aircraft, AA and capacity; §38 refuelling, refitting and arming
   within capacity at §47's costs; the sealed deployment step of the land
   support air phase, with transfers (37.1x, double range) its only
   mission, so transfers are secret from the start; the air layer and the
   squadron panel; `--air`. Playable: aircraft sit, are serviced and
   transfer. Needs from cna: Requests 1, 3, 4 and 5.
2. **Missions without opposition.** Every land support mission in the
   sealed step; range, capability and restriction checks; bombing troops
   (pinned), trucks (motorisation points), fortifications, roads, railways,
   ships and facilities on the Air Bombardment Table; mining harbours
   (41.39, `harbour-mining.json`); strafing; reconnaissance; transport and
   airdrops; return to base, crash landing, emergency flight; facilities
   losing capacity to barrage, captured or destroyed by an occupying land
   unit, damaged by desert raiders (36.14–36.15); an SGSU reacting or
   eliminated when an enemy combat unit moves next to it (35.12) and its
   grounded aircraft captured (35.27). Playable: bombers fly and land units
   feel it. Needs: Request 2.
3. **Air combat and flak.** CAP (offensive, defensive, air ZOC),
   interception along paths, scramble, the fight-or-not choice, aborts,
   pairings and the kill table, screened aircraft, recovering pilots and
   aircraft, flak (its 47.21 ammunition, 46.17's stacking) and flak
   suppression, flak destruction bombing (41.33), night missions; pilots
   moving squadrons, retraining and sitting out after recovery (40.13–40.14,
   40.17); R-113 with its probe.
4. **The strategic air war.** Strategic air planning; Axis raids on Malta
   through the availability levels; Malta's facilities, construction rolls
   and AA; the convoy stage: reconnaissance, cover, flak suppression,
   convoy bombing and torpedoes; 47.6's abstract convoys with what they
   carry, so depots (47.14, 47.4x) and motorisation arrivals (47.53) land
   here, and R-127, R-144; bombing Tripoli; the off-map facilities'
   rebuilding roll (36.5); strategic recovery. The Axis's raids and convoy
   cover are tested on Operation Crusader, since Graziani's allows neither.
5. **Reinforcements and §47.** Aircraft by schedule, pilots by roll,
   SGSUs called in within their limits, Commonwealth withdrawals, refit
   rolls, depot movement and stacking (47.3x, 47.8), air transport of
   supply units (47.35).
   After this slice Graziani's Air Game is whole and the module joins
   `Modules::ALL` if Brian says so (question 1).
6. **The other scenarios.** Rommel's Arrival, Operation Crusader and The
   Last Chance (and the campaigns that extend them) with the German air
   force: §43's Mediterranean basing percentages and Crete, Marseille,
   USAAF SGSUs, each scenario's air special rules, the facilities its
   players place at set-up. Needs nothing known beyond Request 1.
7. **Docs.** README, `NOT_SIMULATED.md`, this Status, the queue.
8. **Air with the Logistics Game**, after Mission 6: per-aircraft fuel and
   ammunition (38.2x) from facility stocks, dumps, ports and pipelines as
   targets, `abstractions.air_and_logistics`, §58 off. Its own plan then.

## Risks

- **Interaction depth.** A land support air phase with a dozen missions,
  interceptions and scrambles is the most interactive step in the game.
  Each sealed step and pending choice is a state the clock can stop in;
  the plan keeps them to four per phase (deploy, scramble, fight-or-not,
  allocation), each with a default so a scripted game can pass them.
- **Order verbosity.** Hundreds of aircraft per side; orders name
  squadrons and counts, not aircraft, and the engine picks which aircraft
  (readied first, by id).
- **State size.** About 400 Axis and 175 Commonwealth aircraft in
  Graziani's; each a few bytes. Acceptable.
- **Golden churn.** As in Mission 5: the new golden re-pinned per slice,
  the old ones never.
- **Data cna has not typed.** Starting air facilities, off-map air
  distances, the set-ups' aircraft ids and SGSU upkeep under §47
  (Requests 1, 3, 4 and 5) block slice 1; slice 1 starts on the tables and
  squadron state meanwhile.

## Requests for cna this mission makes

To be posted to `PROGRESS.md` **Requests for cna** with this PR:

1. **The air facilities each scenario starts with, typed throughout.**
   Rommel's Arrival, Crusader and The Last Chance list facilities in
   `sides.*.air`, but Graziani's lists none (its Axis supply says "spread
   over any Axis airfields"); some entries are free text ("three, anywhere
   on sheets A to C", "the four off-map airfields"); and Malta's capacity
   comes in mixed units (`facility_capacity_sgsu: 5` in Graziani,
   `facility_capacity_total` 8 and 14 elsewhere) against 44.14's levels of
   18 aircraft with no SGSUs, while Graziani's Malta planes still carry
   `sgsu` counts. Needed: per scenario, each facility's kind, hex or box,
   capacity, or a typed placement rule where the players choose; the
   off-map facilities' printed capacities (36.5); Malta's facilities with
   their standard levels. Blocks slice 1.
2. **Bombing and strafing targets under §47.** Which of 41.3x's and 40.6x's
   targets exist when the Air Game is played with abstract logistics:
   supply dumps (do supply units stand in for them, and what does a
   percentage take?), ports (efficiency is a Logistics Game number), the
   water pipeline, trucks in convoy versus attached motorisation points.
   Blocks slice 2.
3. **Air distances off the map.** Range from Malta, the Tripoli and
   Tunisia boxes, Italy, Sicily, Crete and the Commonwealth's off-map bases
   to map hexes: the Malta inset's distance box and 37.4's off-map entries
   as data (the convoy lanes' distances are already in `convoy-lanes.json`).
   Blocks slice 1's transfers off the map.
4. **Scenario aircraft keyed to the characteristics chart.** Every
   `sides.*.air` plane entry names its type as the set-up page prints it
   (`CR 42`, `Hurricane Mk I`, `Potez 63/11`, `Me Bf109F`); of the four
   scenarios' 82 distinct names only 9 match an `aircraft` in
   `aircraft-characteristics.json` exactly (the rest read `C.R. 42 Falco`,
   `Hurricane Mk. I`, `63.11`, `Bf. 109F`). Some have no row at all: The Last Chance's
   `Hurricane IIE`, `Wellington II`, `Wellington V`, `Baltimore II` and
   `III`, Crusader's `Blenheim IV F (second line)`. Needed: each entry's
   characteristics row id (`aircraft:it:cr-42-falco`), and for the types
   without a row, a ruling or a row. Blocks slice 1 (Graziani's 23 names,
   15 of them unmatched) and slice 6.
5. **SGSU upkeep and stores under §47.** 35.14 charges each SGSU 1 fuel
   and 1 water a stage and 1 stores a turn, and 38.36 a stores point per
   refit roll; 45.62 and 45.64's repairs cost stores and fuel. 47.0 keeps
   §32 for anything not mentioned and 47.2 replaces only the maintenance
   costs, but §32 has no water or stores. Needed: how each reads with the
   Air Game and abstract logistics (fuel from a supply unit, water and
   stores waived, or otherwise). Blocks slice 1's maintenance.
6. **Printed flak on the map and in the boxes, as data.** 41.85 and R-111
   speak of flak printed on the map and in the Tripoli-type boxes, but no
   file types it. Needed: each printed flak value and where it stands.
   Blocks R-111's build in slice 3.

## Brian's questions

At most three; each has a default the autopilot takes if Brian approves the
spec without answering.

1. **Should the Air Game become the default for new games once Graziani's
   is whole (after slice 5)?** *Recommended: no.* A tick-box at new game:
   it more than doubles the orders in a turn, and a group should choose it.
2. **Build the Air Game on §32 and §47 first, before the Logistics Game
   (Mission 6) is built?** *Recommended: yes.* cna restates §47 for exactly
   this combination, it unblocks this mission from #148, and Air with full
   logistics becomes one later slice.
3. **Should the engine form squadrons at set-up, or ask each player?**
   *Recommended: form them, changeable.* One type per squadron, filled to
   size in type order, one SGSU each; a regroup order before play changes
   it. Asking would make every Air game start with a few hundred clicks.
