# Mission 6: the Logistics Game, Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/designs/2026-11-06-logistics-game-design.md`. Where this
plan and the spec disagree, follow the spec and say so in the journal.
**Depends on:** Brian's approval of the spec in **Feedback** (Mission 6 is
under his review). Nothing below is built until then.

Every decision here is the autopilot's under Brian's delegation of
2026-09-26 and is reversible with one PR.

Slice 1 has its full task list here. Slices 2–8 have a task list each;
each gets its own plan (`docs/plans/YYYY-MM-DD-m6-slice-n-<slug>.md`) at its
start, written against the code the slices before it leave behind.

## Rules for every task

- Stage only your own files (`git add <paths>`, never `git add -A`).
- Paste each verification command's output into the PR body.
- `CARGO_TARGET_DIR` on disk under `work/targets/`, not `/tmp` (tmpfs).
- Failing test first, every task; cite the case in each test name or
  comment, never the rule text.
- The old goldens (`tests/games/graziani-m5.json`, `graziani-short.json`)
  and every fixture in `web/tests/fixtures/` must replay to their pinned
  hashes with no regeneration. A slice that changes one has leaked into §32
  mode: fix the slice, do not re-pin.
- Gates for `gh pr ready`: `cargo clippy --all-targets -- -D warnings`,
  `cargo test --no-fail-fast`, `cargo run -p cna-probe -- check`, the `web/`
  checks and Playwright.

## Slice 1: the switch, set-up, fuel and ammunition (Graziani's Offensive)

**Goal:** `cna-play new --logistics` starts Graziani's Offensive on its full
set-up (base `sides` plus `abstractions.air`, 59.6), with stocks in units,
first-line trucks and dumps. Units burn fuel per block of five CP from their
own hex (49.13–49.16) and spend ammunition per TOE point that fires
(50.12–50.15); out of ammunition is R-050. The browser can start and play
such a game. Water, stores, convoys and second-line trucks are logged as
not yet charged.

### Task 1: `supply_mode()`, behaviour-neutral

- [ ] Bump `vendor/cna` to e9a90b3 or later (the spec was read there),
  regenerating what AGENTS.md's *Bumping `vendor/cna`* lists, in its own
  commit.
- [ ] `Modules::supply_mode() -> SupplyMode { None, Abstract, Logistics }`
  from the two bools; every `modules.abstract_supply` read in `cna-game`
  becomes a match on it. No new behaviour.
- [ ] Gates green with every golden and fixture at its pinned hash
  (paste `cargo test -p cna-game golden` output).

### Task 2: the `logistics_game` module and its guard (48.0)

- [ ] Failing tests: a `Modules` JSON without the field reads as `false`; a
  set with `logistics_game` and `abstract_supply` both on is refused by
  `with_modules` citing 48.0; so is `logistics_game` with any module whose
  Logistics branch is not built (list them in one const, emptied slice by
  slice: in slice 1, `pre_play_convoys`, `campaign_victory`, `breakdown`,
  `towing`, `destroyed_tanks`, `withdrawals` only if it reads supply, and
  any other module that reads `abstract_supply`).
- [ ] `Modules.logistics_game`, `#[serde(default, skip_serializing_if =
  "std::ops::Not::not")]`, not in `Modules::ALL`.
- [ ] `cna-play new --logistics` turns it on and `abstract_supply` (and the
  guarded modules) off; `cna-play` prints which modules a game has.

### Task 3: the full set-up (59.4–59.6, 59.61)

- [ ] Read `scenario.schema.json` for `Abstraction` with `air`: confirm
  `replaces`/`adds` semantics match the `air_and_logistics` code path in
  `setup.rs` and reuse it.
- [ ] Failing tests: with the module on, Graziani has no supply units and no
  motorisation points; it has a dump at Tobruk holding 200 ammunition, 2000
  fuel, 500 stores; the Tripoli box holds its listed supply and is not a
  dump (R-073); no air-facility trucks or supplies (59.61); every
  deployment's trucks are first-line on its units by class.
- [ ] `Stock { fuel, ammo, stores, water }` (`u32` each); `PieceKind::Dump`
  and `DummyDump`; `GameState.stocks: BTreeMap<PieceId, Stock>` for units,
  dumps and truck lots; `GameState.trucks: BTreeMap<PieceId, Trucks>` for
  first-line lots (`light`, `medium`, `heavy`).
- [ ] Units' own starting stocks by the default in the spec's Request 1
  (tanks full by R-043's capacity, one fire per TOE point, one stage's
  water). Write the open question into the PR body and post Request 1 if
  Part 0 has not.
- [ ] 59.4's free loading of trucks at set-up: a `LoadTrucks` set-up order
  (any supply kind from the side's listed supplies is *extra*, not taken
  from a dump), refused past `truck-characteristics.json`'s capacity.
- [ ] `check_invariants`: no negative stock (unsigned), a truck lot's load
  within capacity, no supply unit or motorisation point in Logistics mode.

### Task 4: fuel (49.12–49.16, R-041, R-043)

- [ ] `cna-rules::logistics::fuel_owed(cp, rate)` (blocks of five, part
  block whole, R-041) and `capacity(cpa, rate)` (R-043), with the switches
  `R-041` and `R-043` in `ruleset.rs`; defaults are the accepted options.
- [ ] Failing tests: who uses fuel (49.12: vehicles, guns, unbracketed HQs;
  not motorcycles or towed tanks); a move draws at the start of each
  movement segment from tanks, first-line trucks or a dump in the start hex,
  the order naming the source or the default (tanks, trucks, dump, ties by
  id); a draw past capacity tops up the tanks only to capacity; a move with
  no fuel anywhere is refused citing 49.15; three 3-CP segments cost three
  blocks (R-041's example).
- [ ] Every fuel user resolves to a rate from `weapon-systems.json` or
  `truck-characteristics.json`; list any Graziani unit that does not and
  post the gaps as one request (spec, Request 3).
- [ ] Reactions and retreats pay too (as R-120 did under §32): check 49.15
  for what a retreat with no fuel does; follow cna or open a request.

### Task 5: ammunition (50.12–50.15, R-013, R-050)

- [ ] Failing tests: barrage pays 4 per TOE point firing, anti-armour 3,
  close assault 2 (`ammunition-consumption.json`'s Logistics half); a fire
  order may name fewer points than the unit has (50.14.3); the source is the
  unit, its first-line trucks or a dump in its hex (50.15); pinned,
  retreated-in and withheld units pay none (R-013).
- [ ] R-050 as decided: a unit with no ammunition in it or its hex cannot
  fire or defend, has no ZOC, surrenders when barraged or assaulted or at the
  end of an enemy phase in an enemy ZOC, and may not enter an enemy ZOC
  unless a friend with ammunition or a dump is there. `NOT_SIMULATED.md`'s
  R-050 row becomes "as decided, with `logistics_game` on".

### Task 6: capture (49.11, 50.16, 51.15, R-051)

- [ ] Failing test: an enemy combat unit entering a dump's hex captures it
  with every point; a dummy dump is removed when found; captured shares
  round up (R-051).

### Task 7: the browser

- [ ] New-game screen: a "Logistics Game" checkbox, off by default.
- [ ] Stack panel: each unit's, truck lot's and dump's stock, to its owner
  only (`view_for` redacts the enemy's); `MovePanel` shows the fuel the
  planned path draws and from where, with a source picker; the combat panels
  show ammunition owed and a points-fired control.
- [ ] Playwright `logistics.spec.ts`: start Graziani with the box ticked,
  move one Italian division, barrage once, match the pinned hash.

### Task 8: golden, probes, docs

- [ ] `tests/games/graziani-logistics.json` and its `.sha256`, with a
  `regenerate_` test, through Game-Turn 1.
- [ ] Probes `probes/R-041.json` (fuel per stage by segment pattern) and
  `probes/R-043.json` (capacity by CPA 20–30 and rate) in `cna-probe`.
- [ ] `NOT_SIMULATED.md`: the Logistics Game row says what slice 1 does;
  R-041, R-043 leave the table for switches; README's module list gains
  `--logistics`.

## Slice 2: trucks and dumps (53.x, 54.1x)

- [ ] Detach first-line trucks into convoys in phase C (53.11, not at
  cohesion −5 or worse, 53.23); convoys as `PieceKind::Convoy`.
- [ ] Convoys move in phase H on the extended CPA, breaking down past it
  (53.22); loading and unloading at 53.24's costs, free in phase C's
  supply-distribution segment; a load's CP spent travels with it (53.25).
- [ ] Phase C's supply-distribution segment: share supplies in a hex up to
  each holder's capacity.
- [ ] Dumps: built at R-018's chart cost, capacity (54.13, R-073), blown
  (54.14, 54.17, E-032); abandoned vehicles (53.31–53.32).
- [ ] Trucks break down (21.x with `truck-characteristics.json`'s BAR);
  `breakdown` and `towing` leave the guard list.
- [ ] UI: a truck convoy panel beside `MarchPanel`; dumps on the map.

## Slice 3: water (52.x)

- [ ] Wells at `village`, `bir` and `oasis` settlements in `hexes.json`;
  draws once a stage (R-020) on `water-availability.json` with the secret
  depletion die (52.13–52.15).
- [ ] Poisoning and sweetening (52.16–52.17, R-021 as decided).
- [ ] Usage per stage (52.41–52.42, R-062), hot weather doubled (52.43),
  evaporation hook for slice 4.
- [ ] Lack of water (52.51–52.53, R-022); phase C's water-distribution
  segment.
- [ ] Switches and probes R-020 and R-022.
- [ ] UI: water in the stack panel; a well draw order; dry units marked.

## Slice 4: stores, evaporation and attrition (49.3, 51.x, 52.6)

- [ ] Clock: stage IV once a Game-Turn, module on only.
- [ ] Stores paid per 51.11; prisoners and guards (51.12, 51.17, R-053);
  half rations (51.23); disorganisation and attrition (51.21–51.22, R-052);
  phase C's attrition segment (R-030).
- [ ] Evaporation of fuel and water (49.3, 52.44, R-040, R-042); switches
  and probes R-040 and R-042.
- [ ] Pasta (52.6, R-060).
- [ ] UI: a stage IV panel.

## Slice 5: ports and Axis convoys (55.1x, 56.x, 57.0, 58.x)

- [ ] Ports: efficiency and capacity per OpStage (55.1x, R-080, R-081),
  starting levels from `construction.ports` (R-027).
- [ ] Tonnage: `axis-convoy-level.json` and `axis-convoy-capacity.json` for
  next turn; cargo planning by `equivalent-weights.json`; lanes by
  `convoy-lanes.json` (reuse `convoy.rs`'s); arrival in phase D at ports
  within capacity (56.27–56.28); replacements by convoy (56.23).
- [ ] §58: the abstract convoy attack on cargo (58.1, R-032, R-093, R-127),
  fuel lost on landing (58.3, R-090), smaller fleets and monthly truck
  losses (58.41–58.43, R-091, R-092); 60.37's pre-play convoys by tonnage
  (`pre_play_convoys` leaves the guard list).
- [ ] Commonwealth base: Cairo unlimited (57.0, R-026), Alexandria too
  where the scenario says (60.44).
- [ ] UI: a convoy-planning panel in the naval convoy stage.

## Slice 6: shipping and the railway (54.3x–54.5, 55.2x, 56.3x)

- [ ] Axis coastal ships sail in phase H (56.31–56.35, R-025); Commonwealth
  tactical shipping in phase C's segment.
- [ ] Blocking, clearing and mines (55.21–55.28, R-082, R-083).
- [ ] The railway: units or supplies per run (R-071) by equivalent weights
  (54.5, R-070); Axis rolling stock (54.4x, R-072) where a scenario gives it.
- [ ] Air lift of supplies under the Logistics Game.
- [ ] Graziani's Logistics Game whole: the guard list is empty for
  Graziani's modules; the golden runs six Game-Turns. Ask Brian's question 1
  outcome before adding the module to `Modules::ALL`.

## Slice 7: the other scenarios

- [ ] Rommel's Arrival, Operation Crusader, The Last Chance and their
  extending campaigns set up and play a Game-Turn with the module on
  (extend `cna-game::scenarios`'s smoke test).
- [ ] Pipelines (52.21–52.25, R-061) once cna types them (Request 2).
- [ ] Each scenario's logistics special rules; 64.7x's supply clause read
  natively in `campaign.rs` (`campaign_victory` leaves the guard list).

## Slice 8: docs

- [ ] README, `NOT_SIMULATED.md` (Systems row; every logistics ruling row
  reads "as decided" or is a switch), the spec's Status, the queue row.
- [ ] `MISSION 6 COMPLETE` in the journal and `PROGRESS.md`.
