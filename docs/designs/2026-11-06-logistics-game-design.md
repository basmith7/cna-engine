# cna-engine: the Logistics Game as a module

**Date:** 2026-11-06
**Mission:** 6 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** proposed; waiting on Brian's review (PR label `needs-brian`).
Nothing in this spec is built.

Written by the autopilot under Brian's delegation of 2026-09-26: every
decision below is Claude's, is cheap to reverse, and can be reversed with
one PR. The mission itself (the Logistics Game, SPI §48–58, as a module
behind one switch, with the water and fuel probes) is Brian's, from
`AUTOPILOT.md`'s *Later, in no fixed order* list. It is a large mission, so
Brian reviews this spec before any building, as he did Mission 2's.

## Goal

Graziani's Offensive, and then every scenario, playable with the Logistics
Game in place of §32's abstract supply. Today the engine plays the Land Game
on §32: supply units carry two numbers (ammunition up to 40, fuel up to 60,
`piece::Load`), motorisation points stand where the medium trucks did,
fuel is paid per stage at 32.22's class rates, ammunition per
battalion-equivalent at 32.21's, the Axis receives supply units by the
simplified convoy table (`convoy.rs`, 32.6x), and there is no water and no
food. When this mission is done, a game started with the Logistics Game
switch on tracks the four supplies in points (fuel, ammunition, stores,
water) held by units, trucks and dumps; burns fuel per five CP moved
(49.13), spends ammunition per TOE Strength Point that fires (50.13);
waters units every stage from their own hex (52.4x, R-062) and feeds them
once a turn (51.1x); draws water from wells, oases and pipelines (52.1x–52.3x);
moves supplies on first-, second- and third-line trucks and stocks them in
dumps (53.x, 54.x); and lands the Axis's tonnage at ports by the convoy
system (55.x, 56.x), with §58's abstract air standing in for the Air Game.

How this moves **the goal** (an open-source, self-hostable digital Campaign
for North Africa, two players in a browser, the engine enforcing every rule,
each ruling a switch): the Logistics Game is what the printed game calls its
heart, and it is the part no group plays by hand to the end, because the
bookkeeping is the burden. An engine that keeps the books is the reason to
play CNA on a computer at all. Every rule lands in `cna-game`'s state
machine, so the Mission 4 server serves it unchanged; and the eleven
logistics rulings with real alternatives get probes, so the water and fuel
rulings cna decided without evidence (R-020–R-022, R-040–R-043) can be
looked at again with numbers next to them.

## Non-goals

- **The Air Game (§33–47).** The Logistics Game here is played with the
  Land Game only, as 48.0 allows: every *Air Game only* step of
  `logistics-sequence.json` is skipped and §58's abstract air rules stand
  in. Air facilities' trucks and supplies are left out at set-up (59.61).
- **Removing §32.** The `abstract_supply` module stays, unchanged, as the
  default for new games until Brian says otherwise (*Brian's questions*,
  1). The Logistics Game is a second supply mode beside it, not a
  rewrite of it.
- **The paper bookkeeping as such.** The TOE, Supply Dump and Truck Convoy
  sheets 49.0 asks players to keep are the engine's state; the browser shows
  the numbers, not facsimiles of the sheets.
- **Variants.** V-002 (no evaporation in tanks) and V-004 (stores in
  instalments) are cna's recorded house rules, not rulings; they are not
  built unless Brian asks.
- **Limited intelligence beyond the view's redaction.** As in Mission 5:
  dump contents, truck loads and unit stocks are the owner's; the enemy sees
  counters and what 3.61's table reveals.

## What "done" means

1. `cna-play new --logistics` and the browser's new-game screen (with the
   Logistics Game ticked) start any scenario cna gives a full set-up for with
   the base `sides` blocks plus its `abstractions.air` block (59.6, 59.61):
   dumps with all four supplies, trucks by line, the 59.4 free loading, units
   holding their starting stocks.
2. A unit with no fuel in its tanks or hex cannot move; a unit with no
   ammunition in its hex cannot fire, assault or defend and surrenders as
   R-050 says; a unit without water takes 52.5x's limits; a unit unfed takes
   51.2x's disorganisation and attrition. Each refusal or loss names its
   case; spending is logged per holder (unit, truck, dump) and replays byte
   for byte.
3. Trucks move in the truck convoy phase on their extended CPA (53.22),
   load and unload at 53.24's costs, never leapfrog (53.25); dumps hold
   what 54.13 allows and can be built, blown and captured.
4. The Axis's supply arrives by tonnage convoy through ports at their
   efficiency (55.1x, 56.x), bombed by §58's abstract attack; the
   Commonwealth's comes from Cairo (57.0, R-026).
5. Every logistics ruling (R-020–R-027, R-030, R-032, R-040–R-043,
   R-050–R-053, R-060–R-062, R-070–R-073, R-080–R-083, R-090–R-093, R-127,
   R-144) is a `Ruleset` switch or a `NOT_SIMULATED.md` row reading "as
   decided" with the module that builds it; the water and fuel probes are in
   `probes/`.
6. Each new decision is an order in the UI's panels; a Logistics golden game
   for Graziani's Offensive and its Playwright twin match `replay` byte for
   byte; every old golden game and save replays unchanged.
7. `NOT_SIMULATED.md`'s *Systems* row "The Logistics Game (§48–58): left
   out" is replaced by what the module does and what it still leaves out.

## The one switch

cna's `rules/logistics/00-overview-and-sequence.md#switch` names what full
logistics throws out of §32 and what it keeps. The engine follows it
exactly:

| §32 part | With `logistics_game` on |
|---|---|
| 32.1x supply units: load, draw range, capture, stacking | replaced: supply points of four kinds in units, trucks and dumps (49.x–54.x); no `PieceKind::SupplyUnit` is set up |
| 32.2x expenditure | replaced: 49.13–49.16 fuel, 50.12–50.15 ammunition, 51.1x stores, 52.4x water |
| 32.3x moving supply units: rail, sea and air lift | replaced: trucks (53.x), the railway (54.3x, R-071, R-072), tactical shipping (55.x, 56.3x) |
| 32.4x simplified supply planning | replaced: convoy arrival (phase D) and the Commonwealth base (57.0) |
| 32.5x motorisation points | replaced: truck points by line; trucks break down (21.x, `truck-characteristics.json`'s BAR) where motorisation points never did |
| 32.6x simplified Axis convoys | replaced by 56.x's tonnage convoys; 32.6's bombing step stays as §58.1's attack (R-032, R-093, R-127) |
| 32.7x bombing the fleet | stays (the Air Game is not played; 58.2) |
| 32.8x AA modification | stays (the Air Game is not played) |
| 32.9 road and track stacking ignored | stays |
| 3.6x limited intelligence | stays |

How the switch is built:

- **A module, not a ruleset switch.** `Modules` gains
  `logistics_game: bool` with `#[serde(default, skip_serializing_if =
  "std::ops::Not::not")]`, like every other module. A save or log made before
  it has no such field, reads as `false`, and replays as it does today
  (AGENTS.md, *Determinism and saves*).
- **Exclusive with `abstract_supply`.** `GameState::with_modules` refuses a
  module set with both on (an error naming 48.0's switch), and
  `Modules::supply_mode()` returns `None`, `Abstract` or `Logistics` so
  every call site that reads `abstract_supply` today (about 30 in
  `cna-game`: `fuel.rs`, `ammo.rs`, `movement.rs`, `setup.rs`, `convoy.rs`,
  `rail.rs`, `air_lift.rs`, `organisation.rs`, `campaign.rs`, `raiders.rs`,
  `captured.rs`, `repair.rs` and others) matches on the mode instead of a
  bool. That rename is slice 1's first commit, behaviour-neutral, pinned by
  the existing goldens.
- **Modules that read supply.** Each keeps its §32 branch and gains a
  Logistics one in the slice that touches it: `pre_play_convoys` (60.37
  plans tonnage, not supply units), `campaign_victory` (64.7x's "convoy
  route to a supplied dump" read literally, R-109's row), `fleet` (58.2
  unchanged), desert raiders' dump raids (§27 against dumps, not supply
  units), `breakdown` (truck points now break down), `captured_equipment`
  (unchanged). Until a module's Logistics branch lands, turning it on with
  `logistics_game` is refused, so no game runs a half-built mix.
- **Not in `Modules::ALL`** until the mission's last slice: new games keep
  playing §32 while the module is partial. `cna-play new --logistics` and a
  browser checkbox opt in.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Which § numbers | cna's `::: spi` badges, which are SPI's: §48 sequence, §49 fuel, §50 ammunition, §51 stores, §52 water, §53 trucks, §54 dumps, railway and equivalent weights, §55 ports, §56 Axis convoys and coastal shipping, §57 Commonwealth base, §58 abstract air; 59.4–59.6 set-up | Errors cite SPI cases |
| First scenario | Graziani's Offensive: six Game-Turns, its `air` abstraction is "no changes beyond 59.6", no pipeline (60.7), one port efficiency to read (Tobruk 2, R-027), coastal shipping in Tripoli (60.35), Cairo and Alexandria unlimited (60.44) | The smallest full set-up cna has, and the scenario every existing golden plays |
| Set-up data | The base `sides` blocks (dumps with `ammo`, `fuel`, `stores`, `water`; `trucks` by deployment and line) plus `abstractions.air`, which `cna-data` already parses (`Abstractions::air`) but `setup.rs` never applies; air-facility trucks and supplies dropped (59.61) | The full set-up is the scenario's printed one; the `air` block is what 59.6 says to change |
| Holders | One `Stock { fuel, ammo, stores, water }` type, held by a dump (a new piece kind with a hex), by each truck lot, and by each combat unit (its own carried supply: tanks, one fire per TOE point, 50.15.2). `BTreeMap`s keyed by piece id | One owner per point, every holder the same shape, order fixed |
| Trucks | Truck points are counts by class (`light`, `medium`, `heavy`) and line on a holder: first-line on a combat unit (no counter, 53.11), second and third line as a new `PieceKind::Convoy` counter (53.12–53.13), and dumps as a new `PieceKind::Dump` and `DummyDump`. Each lot carries a `Stock` or troops, never both, by `truck-characteristics.json` | 53.1x and 54.2; the lines only differ in where they stand |
| Fuel | Drawn at the start of each movement segment for the CP the mover declares, in blocks of five per segment (R-041), from its tanks or any source in its start hex the order names (default: tanks, then first-line trucks, then dump, ties by id); capacity is CPA in whole blocks times `fuel_rate` (R-043). A move whose declared CP run out stops; going further needs a new draw | 49.13–49.16 as restated; an order names its source, a default keeps scripts short |
| Ammunition | Paid at the instant of fire per TOE point committed, at `ammunition-consumption.json`'s *Logistics Game Played* rates (50.13), from the unit, its first-line trucks or a dump in its hex (50.15); pinned, retreated-in and withheld units pay none (R-013). A player may fire fewer points (50.14.3): the fire order gains an optional point count | 50.1x; the order already names the firing units |
| Out of ammunition | R-050 as decided: no fire, no defence, no ZOC, surrender when attacked or at the end of an enemy phase in an enemy ZOC, no entry into an enemy ZOC without a supplied friend | cna's accepted ruling; replaces R-120's §32 reading in this mode only |
| Water | Phase C's water-distribution segment and any point of the stage: a unit is watered from water in its own hex (R-062); wells drawn once a stage (R-020) on `water-availability.json` with the secret depletion die; poisoning and sweetening (52.16–52.17, R-021); oases unlimited (52.3); pipelines (52.2x, R-061) from the slice that has a scenario with one | 52.x; R-020's "per stage" yield is the table's |
| Lack of water | 52.51–52.53 with R-022 (guns are vehicles); the second dry stage's TOE loss logged per unit | cna's accepted rulings |
| Stores | The new stage IV once a Game-Turn: each unit is paid 51.11's rate from its hex; unfed units gain a disorganisation point (51.21), attrition at the end of each even unfed turn (R-052); prisoners and guards (51.12, 51.17, R-053); half rations (51.23) as an order | 51.x |
| Evaporation | Stage IV: 49.3's per-turn loss on dumps, trucks and units' tanks (R-040), the Commonwealth's 9 % replacing 6 %, hot weather's extra 5 % in phase B, every loss rounded down, sea convoys exempt (R-042); water the same (52.44) | cna's accepted rulings |
| Pasta | 52.6 with R-060: one Pasta Point a Game-Turn per Italian unit in stage IV, the CPA limit for the turn if unpaid | Italian units are Graziani's Axis |
| Sequence | `Clock` gains, only with the module on: stage IV (stores expenditure) after the naval convoy stage, and phase C's water-distribution, attrition, supply-distribution and tactical-shipping segments (R-030: any order, the player picks); the engine keeps its Land Game `Phase` letters and maps cna's Logistics letters through `logistics-sequence.json`'s `attaches_to` | Off, the clock is today's, so old logs replay; one name per phase across modes |
| Trucks moving | Convoys move in phase H (the Logistics Game's I) on the extended CPA (30, light 40; past it they break down, 53.22); first-line trucks move with their unit; loading and unloading cost 2 CP each way, free in phase C (53.24, `cp-costs.json`); a load carries the CP its trucks have spent (53.25) | 53.2x |
| Dumps | Built in phase C at R-018's chart cost (3 CP and 10 stores; dummy 2 CP); capacity by `supply-dump-capacity.json` (54.13), the Tripoli and Tunis boxes unlimited and not dumps (R-073); blown by 54.14 on `supply-dump-demolition.json` with E-032; captured whole by an enemy combat unit entering (49.11, 50.16, 51.15, R-051) | 54.1x; `construction.rs` already builds 24.9 dumps under §32 |
| Abandoned vehicles | 53.31–53.32: trucks out of fuel and left are markers a unit entering may capture | Closes the loop with fuel |
| Ports | Each port's efficiency level and capacity per OpStage (55.1x, R-080, R-081) from `port-capacity.json` and the scenario's `construction.ports` (Tobruk 2, R-027); blocking, clearing and mines (55.2x, R-082, R-083) | 55.x |
| Axis convoys | Phase A of the naval convoy stage reads `axis-convoy-level.json` and rolls `axis-convoy-capacity.json` for next turn's tonnage; the Axis plans cargo by `equivalent-weights.json` and lanes by `convoy-lanes.json` (56.1x–56.2x, lanes already in `convoy.rs` for 56.1x); arrival in phase D at ports within capacity (56.27); §58.1's abstract bombing (R-032, R-093, R-127) and 58.3's fuel loss on landing (R-090) | 56.x; `convoy.rs`'s lanes and bombing are reused, its cargo is not |
| Coastal and tactical shipping | Axis coastal counters (`axis-coastal-ships.json`) sail in phase H (R-025); Commonwealth port-to-port transfers in the tactical-shipping segment, limited only by port capacity | 56.3x, R-025 |
| Commonwealth base | Cairo holds unlimited supply of every kind (57.0, R-026); Alexandria too in the Italian Campaign; Graziani's 60.44 names both | R-026 |
| Railway | Each rail run carries units or supplies, never both (R-071), by `equivalent-weights.json`'s stacking points (R-070); Axis rolling stock (54.4x, R-072) in the scenarios that give it | 54.3x–54.4x; `rail.rs` keeps its §32 branch |
| Trucks under §58 | 58.41's smaller starting fleets (R-091), monthly truck losses (58.42, R-092, `motorisation-losses.json`), trucks withheld on arrival (58.43) | §58 stands in for the Air Game |
| Rulings | Each logistics ruling whose options a probe can tell apart gets a `Ruleset` switch with the ruling's option numbers and its accepted Decision as default: R-020, R-021, R-022, R-040, R-041, R-042, R-043. The rest are rewritten in `NOT_SIMULATED.md` as "as decided" with their module, in the slice that builds them | AGENTS.md: every ruling a switch or a row; R-001's precedent for switches a probe needs |
| Probes | Water: R-020 (water drawn per well per stage by stack size, options 1–2), R-022 (a stack's assault strength dry, by option). Fuel: R-040 and R-042 (fuel left in a Graziani dump and a tank after six turns, by option), R-041 and R-043 (fuel burned per stage by segment pattern and CPA). `probes/R-020.json` and so on, in the shape of `probes/R-018.json` | AUTOPILOT's "the water and fuel probes"; probes are why rulings are switches |
| UI | Each slice adds its orders to the existing panels: fuel source and declared CP in `MovePanel`; points fired in the combat panels; the phase C segments as tabs of the organisation panel; a truck convoy panel beside `MarchPanel`; dumps and stocks in the stack panel, the owner's only; stage IV and the convoy plan as a panel like `FleetPanel` | "Shows up in the UI's panels", as in Mission 5 |

## Crates touched

- `cna-data`: `Abstractions::air` applied at set-up; loaders for
  `ammunition-consumption`, `fuel-evaporation`, `water-availability`,
  `well-poisoning`, `truck-characteristics`, `supply-dump-capacity`,
  `supply-dump-demolition`, `port-capacity`, `axis-convoy-level`,
  `axis-convoy-capacity`, `axis-coastal-ships`, `equivalent-weights`,
  `logistics-sequence`; dump rows' `stores` and `water` (already parsed).
- `cna-rules`: pure helpers (fuel owed per segment, capacity, ammunition
  owed, water owed, stores owed, evaporation) and the seven ruling switches.
- `cna-game`: a `logistics` module tree (`stock`, `fuel`, `ammo`, `water`,
  `stores`, `trucks`, `dumps`, `ports`, `tonnage`), `Modules::logistics_game`
  and `supply_mode()`, the clock's stage IV and segments; `Order` and `Event`
  gain variants.
- `cna-probe`: the water and fuel probes.
- `cna-play`: `--logistics`.
- `cna-web` and `web/`: the orders, panels and the new-game checkbox.

## Testing

- **Rule tests per case**, failing first, each refusal naming its case.
- **Determinism and replay.** Every slice adds a property test (random
  Graziani Logistics games from a seed, as `determinism.rs` does) that two
  replays of the same log match byte for byte; the old goldens
  (`tests/games/graziani-m5.json`, `graziani-short.json`) and every
  Playwright fixture stay pinned and must not change: that is the proof the
  module is off for them.
- **Conservation.** A property test that, across any stage, supply points
  only leave the books by a named event (spent, evaporated, captured,
  sunk, blown, abandoned): the sum before plus arrivals equals the sum after
  plus losses, per kind and side.
- **Golden game.** `tests/games/graziani-logistics.json`, pinned by
  `.sha256`, extended per slice; `regenerate_` rewrites it, per AGENTS.md's
  bump steps.
- **Playwright.** Each slice's panel driven by clicks to the pinned hash,
  as in Mission 5; one new spec, `logistics.spec.ts`.
- **Probes.** `cargo run -p cna-probe -- check` covers the new probes; each
  new switch must change at least one probe's output.
- **Rulings.** The existing test that every ruling in `vendor/cna/rulings/`
  has a switch or a row keeps passing at every slice.

## Slices

One PR each, in order; each gets its own plan at its start except slice 1,
whose plan is `docs/plans/2026-11-06-logistics-game.md` together with the
task list for the rest. Each leaves a playable game.

1. **The switch, set-up, fuel and ammunition for Graziani's Offensive.**
   `supply_mode()`; `logistics_game` exclusive with `abstract_supply`;
   set-up from the base sides and `abstractions.air`; stocks in units,
   first-line trucks and dumps; 49.13–49.16 fuel (R-041, R-043);
   50.12–50.15 ammunition and R-050; capture of dumps; `--logistics` and the
   browser checkbox; the fuel probes for R-041 and R-043. Playable: units
   move and fight on what they carry and what stands in their hex. Water,
   stores and convoys are not yet charged or delivered, logged as such.
   Needs from cna: units' starting stocks (Request 1).
2. **Trucks and dumps.** Second- and third-line convoys in phase H, extended
   CPA, loading costs, no leapfrogging, phase C's supply-distribution
   segment, dumps built, blown and at capacity, abandoned vehicles, trucks
   breaking down. Needs nothing new.
3. **Water.** Wells, draws, depletion, poisoning and sweetening, oases,
   usage per stage, lack of water, phase C's water-distribution segment;
   the water probes (R-020, R-022) and switches R-020–R-022. Needs nothing
   new for Graziani.
4. **Stores, evaporation and attrition.** Stage IV, stores and prisoners,
   disorganisation and attrition, half rations, pasta, evaporation;
   phase C's attrition segment; the fuel probes for R-040 and R-042.
   Needs nothing new.
5. **Ports and Axis convoys.** Port efficiency and capacity, tonnage
   convoys, cargo planning, arrival, §58's bombing and fuel loss,
   60.37's pre-play convoys by tonnage, the Commonwealth base, §58.4x
   truck losses. Needs nothing new for Graziani.
6. **Shipping and the railway.** Axis coastal ships, tactical shipping,
   blocking and clearing harbours, mines in ports, the railway under R-071
   and R-070, Axis rail where a scenario gives it, air lift under the
   Logistics Game. After this slice Graziani's Logistics Game is whole and
   the module joins `Modules::ALL` if Brian says so (question 1).
7. **The other scenarios.** Rommel's Arrival, Operation Crusader and The
   Last Chance (and the campaigns that extend them) set up and play a
   Game-Turn with the module on: pipelines (52.2x, R-061), each scenario's
   logistics special rules, 64.7x's supply clause read natively. Needs from
   cna: pipelines as typed data (Request 2).
8. **Docs.** README, `NOT_SIMULATED.md` (the Systems row, every logistics
   ruling's row), this Status, the queue.

## Risks

- **Per-unit stocks are a lot of state.** A full Graziani set-up has a few
  hundred units with four numbers each, plus truck lots. `BTreeMap`s keep it
  deterministic; save size grows by tens of kilobytes. Acceptable; the
  server snapshots already compress.
- **Order verbosity.** Every move declaring CP and a fuel source would make
  scripted games and clicks tedious. The defaults (draw what the planned
  path needs, from tanks first) make the common move one click; the panel
  shows the draw before it is committed.
- **Half-built modules.** A mix of `logistics_game` and a supply-reading
  module whose Logistics branch is not built would play nonsense. The
  refusal in `with_modules` is the guard, removed module by module.
- **Golden churn.** The new golden changes every slice and is re-pinned in
  the slice's commit, as in Mission 5. The old goldens must never change;
  if one does, the slice has leaked into §32 mode and is wrong.
- **Rules cna has not typed.** Unit starting stocks (Request 1) and
  pipelines (Request 2) are prose or absent. Slice 1 starts on its default
  meanwhile; slice 7's pipelines wait.

## Requests for cna this mission makes

To be posted to `PROGRESS.md` **Requests for cna** with this PR:

1. **What units hold at the start of a Logistics Game.** 59.4–59.6 list
   dumps and trucks, and let trucks start loaded (59.4) and attached
   motorisation points carry a load under §32 (59.66), but no case says what
   a combat unit itself holds when play begins: fuel in its tanks (49.14),
   the one fire of ammunition a TOE point carries without trucks (50.15.2),
   water and stores. Blocks slice 1's set-up. Until answered: tanks full,
   one fire per TOE point, one stage's water, nothing else, as an open
   ruling with that as option 1.
2. **Pipelines as typed data.** The scenarios name pipelines only in notes
   ("along railroad hexes", Crusader's "from the railhead to Bir Khamsa");
   neither `data/map/` nor a scenario's `construction` block lists pipeline
   hexes. A typed list per scenario (and the map's printed pipeline, if any)
   would let 52.2x run. Blocks slice 7 only.
3. **Fuel rates for every fuel user.** 49.12 makes HQs with unbracketed TOE
   and every gun-class unit fuel users; `weapon-systems.json` gives
   `fuel_rate` per weapon system. Slice 1 checks that every Graziani unit
   that uses fuel resolves to a rate, and posts the gaps it finds as one
   request then. Not known to block.

## Brian's questions

At most three; each has a default the autopilot takes if Brian approves the
spec without answering.

1. **Should the Logistics Game become the default for new games once
   Graziani's is whole (after slice 6)?** *Recommended: no.* New games keep
   §32 and the Logistics Game is a tick-box at new game. The bookkeeping
   slows every move, and a group should choose it; the two modes are each
   other's regression test.
2. **Should the browser pick fuel and ammunition sources for the player, or
   ask every time?** *Recommended: pick, and show.* Each move and fire takes
   the default source (tanks and the unit's own trucks first, then the hex's
   dump) and the panel shows what was drawn, with a control to change it
   before committing. Asking every time is the paper game's bookkeeping
   moved onto the screen.
3. **Are cna's house-rule variants V-002 (no evaporation in tanks) and
   V-004 (stores in instalments) wanted as options?** *Recommended: no, not
   in this mission.* They are recorded in cna but not rulings; building them
   adds switches nobody has asked for. A later one-PR addition if a group
   wants them.
