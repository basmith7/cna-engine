# Mission 2, slice 3 (state, clock and orders) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `GameState` that walks the whole sequence of play for
Graziani's Offensive, takes orders only from the player whose turn it is,
logs them, replays exactly, and round-trips through JSON; plus the
`cna-play` binary.

**Architecture:** `cna-game` gains `state`, `clock`, `order`, `event`,
`log`, `rng`; slice 2's `setup` feeds `GameState::new`. Every phase exists;
phases this mission does not model emit `PhaseSkipped` and advance.
`cna-play` (new binary crate) does the file I/O.

**Spec:** `docs/designs/2026-10-08-game-state-design.md`
**Previous plans:** slices 1–2.

## Domain facts the implementer needs

- **Sequence** (5.1, 5.2, 7.1x; cna `rules/20-sequence-of-play.md`): a
  Game-Turn is I initiative determination, II naval convoy, then three
  OpStages, then end of turn. Each OpStage: joint phases A initiative
  declaration (the holder chooses to be Player A or B, every stage), B
  weather (the holder rolls), C organisation, D convoy arrival, E CW fleet;
  then Player A runs F reserve designation, G movement and combat, H truck
  convoy, J rail, K repair, L patrol; then Player B the same. There is no
  phase I. G is a repeatable cycle: movement (with reaction), breakdown,
  combat (six steps), reserve release.
- **Initiative:** the Axis holds it through Game-Turn 1 OpStage 3 (60.6,
  scenario `initiative`). From Game-Turn 2 each side rolls one die plus its
  rating (`initiative-ratings.json`: Commonwealth 3 to GT 42; Axis 1 with
  no German units), ties re-rolled.
- **Weather:** `weather.json` with errata E-025 applied, by season
  (`seasons.json`); the holder rolls two dice read sequentially (11–66).
  Graziani starts in mid-September 1940.
- **CP reset:** CP spent resets at the start of each OpStage, not at the
  half (6.14).
- **Ruleset defaults** become cna's accepted Decisions. Today only R-018
  changes (option 3 → 2). Regenerate `probes/R-018.json` and update the
  README's probe text if its finding changes.

## Tasks

### Task 1: clock

**Interfaces:** `Phase` (A–L, cna's `phase` enum names, serde
kebab-case), `Half { A, B }`, `Segment` (movement, breakdown,
position-determination, barrage, retreat-before-assault, force-assignment,
anti-armour, close-assault, reserve-release), `Clock { game_turn, stage,
phase, half, segment }`, `Clock::next(&self) -> Clock` for the fixed
sequence.

- [ ] **Step 1: Write the failing tests:** from GT1 stage 1 phase A,
  repeated `next` visits A–E once, then F–L for Player A, then F–L for
  Player B, then stage 2; after stage 3 comes the next Game-Turn's
  initiative; the 36 OpStage halves of Graziani (18 OpStages) are visited exactly once.
- [ ] **Step 2–5:** run (FAIL), implement, run (PASS), commit `Clock`.

### Task 2: state, orders, events, RNG

**Interfaces:**
- `GameState { scenario: String, rules_commit: String, ruleset: Ruleset,
  clock: Clock, initiative: Option<Side>, player_a: Option<Side>,
  weather: Weather, pieces: BTreeMap<UnitId, Piece>, pending: Option<Pending>,
  rng: GameRng, log: Vec<LoggedOrder> }`, `Serialize + Deserialize`.
- `GameRng { seed: u64, draws: u64 }` wrapping `ChaCha8Rng`; rebuilt from
  `(seed, draws)` on deserialise; `d6()`, `two_dice_sequential()`.
- `Order` (spec's list) and `LoggedOrder { player: Side, order: Order }`.
- `Event` enum: `PhaseStarted`, `PhaseSkipped`, `DieRolled`,
  `InitiativeDeclared`, `WeatherSet`, `Placed`, … (grown by later slices).
- `Illegal { case: &'static str, reason: String }`.
- `GameState::apply(&mut self, player: Side, order: Order) -> Result<Vec<Event>, Illegal>`.
- `Ruleset` gains `Serialize`/`Deserialize` as `{"R-001": 1, …}`.

- [ ] **Step 1: Write the failing tests:** an order from the wrong player
  is `Illegal` and leaves the state byte-identical (compare JSON); a
  state with pieces round-trips through JSON unchanged; `GameRng` after
  `n` draws equals a fresh one fast-forwarded `n`; ruleset JSON is
  `{"R-001":1,…}`.
- [ ] **Step 2–5:** as Task 1; commit `Game state, orders, RNG`.

### Task 3: set-up orders and the start of play

- [ ] **Step 1: Write the failing tests:** `GameState::new(scenario,
  seed)` leaves pending pieces and `pending = SetUp { side }`, the Axis first
  (our choice: §59 and §60 do not say who sets up first); a `Place` on a hex outside
  the placement area is `Illegal("59.2")`; a `Place` breaking the hex's
  stacking ceiling is `Illegal("9.14")`; `not_within` of an enemy unit is
  checked; once nothing is pending the clock is GT1 stage 1 phase A.
- [ ] **Step 2–5:** as Task 1; commit `Set-up orders`.

### Task 4: initiative, weather and skipped phases

- [ ] **Step 1: Write the failing tests:** in GT1 only the Axis may
  `DeclareInitiative`; in GT2 the roll uses ratings 1 and 3 and a tie
  re-rolls (seeded case found by search, pinned); weather is rolled in
  phase B and logged; phases C, D (except reinforcements, slice 4), E, H,
  J, K, L emit `PhaseSkipped` and advance; `Pass` ends a player's G phase.
- [ ] **Step 2–5:** as Task 1; commit `Initiative, weather, skipped phases`.

### Task 5: ruleset defaults follow cna

- [ ] **Step 1: Write the failing test:** for every switched ruling, the
  default variant's number equals the option in the ruling file's
  `## Decision` section ("Option n.").
- [ ] **Step 2–4:** run (FAIL on R-018), change the default, regenerate
  probes (`cargo run -p cna-probe -- run`), run (PASS). Commit
  `Ruleset defaults follow cna's decisions`.

### Task 6: log, replay and `cna-play`

**Interfaces:** `Log { scenario, rules_commit, ruleset, seed, orders }`;
`replay(&Log) -> Result<GameState>`; binary `cna-play` with `new
<scenario> --seed N -o state.json`, `apply state.json orders.json`,
`replay log.json`, `show state.json` (a text summary: clock, pending,
pieces per hex).

- [ ] **Step 1: Write the failing tests:** set-up orders for every
  pending piece (a fixture, `tests/games/setup.json`) then `Pass` through
  GT1 stage 1; `replay` of the log gives a state whose JSON equals the
  live one; loading a state with a different `rules_commit` fails.
- [ ] **Step 2–5:** as Task 1; commit `Log, replay, cna-play`.
