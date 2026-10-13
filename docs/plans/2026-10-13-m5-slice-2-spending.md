# Mission 5, slice 2 (spending) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** with `abstract_supply` on, a fuel-using unit pays fuel from a
supply unit in range the first time it moves by land in a stage and again
past its CPA; units pay ammunition to barrage and to assault or defend;
the orders that cannot pay are refused with their case; an empty supply
unit is removed. R-013 is implemented as decided.

**Spec:** `docs/designs/2026-10-13-land-turn-design.md`
**Depends on:** slice 1.

## Rules for every task

As slice 1.

## Tasks

### Task 1: rates (cna-rules)

- [ ] `cna_rules::supply::fuel_owed(class, equivalent, carrying) -> u32`
  and `ammo_owed(equivalent, role, phasing) -> u32` (32.21, 32.22), pure,
  with a table test per row of the restatement (tank battalion 2, tank
  company 1, gun-class and recce battalion 1, camel recce 0, company
  doing anything else 0; ammunition 1 defend, 2 barrage, doubled when
  phasing; a division pays by its components).
- [ ] Where `weapon-systems.json` gives `fuel_rate` for a unit's
  weapons, compare it with the class rate in a test; if they disagree,
  follow `fuel_rate` and say so in the PR.

### Task 2: the draw range

- [ ] `GameState::supply_in_range(piece) -> Vec<PieceId>`: friendly real
  supply units with fuel or ammunition whose hex is within half the
  piece's CPA, traced at medium-truck costs (foot infantry as infantry),
  never through prohibited terrain or an enemy ZOC hex without a friendly
  combat unit, supply units ignored as blockers (R-119). Reuse `reach.rs`.
- [ ] Random-walk test both ways (as `reach.rs`): every unit returned is
  accepted as a source by `apply`, and a source not returned is refused.

### Task 3: fuel

- [ ] `Piece.fuelled: bool` (this stage) and `Piece.fuelled_over:
  bool`, reset in `start_stage`.
- [ ] `Order::Move` gains `fuel_from: Option<PieceId>` (serde default);
  default is the nearest in range with fuel, ties by id.
- [ ] Failing tests: a tank battalion with no supply in range is refused
  (32.22); with one in range its first move debits 2 fuel and logs
  `FuelSpent`; a second move that stage debits nothing; a move past CPA
  debits again; rail and sea moves pay nothing; a foot unit pays nothing.
  React, retreat before assault and forced retreat: follow the cna answer
  to the request; until then they pay if they can and move anyway.

### Task 4: ammunition

- [ ] Barrage plots and assault force assignment check ammunition in
  range per battalion-equivalent at the instant of combat; failing tests
  for the refusal (32.21), the debit, R-013 (pinned, retreated-in and
  withheld pay none), and a defender with none fighting on with
  `AmmunitionShort` logged.

### Task 5: empty dumps, the switch, NOT_SIMULATED

- [ ] A supply unit at 0/0 is removed (32.1x), test.
- [ ] With `abstract_supply` off none of this runs (test on the old
  golden).
- [ ] R-013 leaves `NOT_SIMULATED.md` for the "Implemented as decided"
  table; the *Systems* row for §32 is rewritten.

### Task 6: UI

- [ ] The move panel shows the fuel source and cost and offers the other
  sources in range; the barrage and assign panels show ammunition owed;
  refusals appear in the panel as today. Playwright for one paid move and
  one refused one; the golden e2e re-pinned.
