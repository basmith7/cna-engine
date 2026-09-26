# Slice 3 (cohesion) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Model the cohesion level and its DP/RP bookkeeping, add a switch
for R-001, load the Morale Modifier Table with a coverage test, measure the
table's printed gap (level −4, reading 56) in the `chart-oddities` probe,
and write `probes/R-001.json`.

**Architecture:** As slices 1–2. `cna-data` gains a morale-modifier loader;
`cna-rules` gains `cohesion` and `morale` modules; `cna-probe` gains an
R-001 probe and extends `chart-oddities`.

**Spec:** `docs/designs/2026-09-26-engine-probes-design.md`
**Previous slices:** `docs/plans/2026-09-26-slice-1-close-assault.md` (its
Global Constraints apply unchanged), `…-slice-2-barrage-anti-armour.md`.

## Domain facts the implementer needs

- **Cohesion** (SPI 6.2–6.26). One signed integer per unit, 0 normal. Each
  DP lowers it by one at once; each RP raises it by one at once; it never
  exceeds +10. Rest (no CP spent in a whole Operations Stage) earns 5 RP but
  rest alone never lifts the level above 0. A victory (the defender wholly
  vacates as a direct result) earns 3 RP. A unit earns 1 DP per CP spent
  over its CPA in a stage, and 3 DP for losing 30 % or more in one close
  assault.
- **R-001** (SPI 6.26, 17.5). Option 1: a unit collapses (cannot move,
  attack or defend; surrenders to any adjacent enemy combat unit) at a
  cohesion **level** of −26 or worse. Option 2: it collapses on 26 or more
  **DP** accumulated. The ruling notes option 2 has no reset rule; the
  engine's option 2 counts every DP ever earned and never resets, and the
  probe says so. Default option 1 (the ruling's Decision and Rationale).
- **Morale Modifier Table** (`morale-modifier.json`, SPI 17.4). `rows[]`
  have `level` (+8 … −17; +8 means "+8 or better", −17 "−17 or worse") and
  `modifier`, a map from `+4 … -4` and `surrender` to a reading range
  `{from, to}` or null. `known_gaps` is `[{level, readings}]`; today
  `[{level: -4, readings: [56]}]`.

## Review Focus

1. The morale table's coverage test must report any reading in no cell or
   two cells except the declared gaps, as slice 1 does for close assault.
2. Rest never lifts the level above 0, and no RP lifts it above +10;
   rest on a level above 0 leaves it unchanged. Tested.
3. Option 2's tally counts DP only, never reduced by RP. Tested.

---

### Task 1: Morale Modifier Table

**Files:** Create `crates/cna-data/src/morale.rs`, `crates/cna-rules/src/morale.rs`.

**Interfaces:**
- `cna_data::morale::{MoraleModifier, MoraleRow, MoraleGap}`, `MoraleModifier::load()`, `MoraleModifier::row(level: i32) -> &MoraleRow` (clamped to +8 / −17).
- `cna_rules::morale::lookup(t, level, reading) -> Lookup` reusing `close_assault::Lookup`'s shape: `enum MoraleLookup { Modifier(String), Gap, Overlap(Vec<String>) }`.

- [ ] **Step 1: Write the failing tests:** 26 rows from +8 to −17; `row(12)` is the +8 row and `row(-40)` the −17 row; every (level, reading) resolves to exactly one cell except the declared gaps (compare with `known_gaps` as in slice 1); at level −17 every reading is `surrender`.
- [ ] **Step 2–4:** run (FAIL), implement, run (PASS). An undeclared gap or overlap is a `cna` data finding: record it, do not change the test.
- [ ] **Step 5: Commit.** `Morale Modifier Table`

### Task 2: Cohesion and the R-001 switch

**Files:** Create `crates/cna-rules/src/cohesion.rs`; modify `ruleset.rs`, `NOT_SIMULATED.md`.

**Interfaces:**
- `R001 { CohesionLevel = 1, DpTally = 2 }`, default 1; `Ruleset.r001`; `SWITCHED` gains `R-001`.
- `cohesion::Unit { level: i32, dp_tally: u32 }` with `Default`, `earn_dp(n)`, `rest()`, `victory()`, and `collapsed(&self, rules: &Ruleset) -> bool`.

- [ ] **Step 1: Write the failing tests:** rest from −1 gives 0, from −8 gives −3, from +2 stays +2; victory from +9 gives +10; `earn_dp` then `rest` lowers the level and raises it back but leaves `dp_tally` at the DP earned; a unit at level −26 has collapsed under option 1 and not under option 2 if its tally is 25 (and the reverse for level −3 with tally 26).
- [ ] **Step 2–4:** run (FAIL), implement, remove R-001 from `NOT_SIMULATED.md`, run (PASS).
- [ ] **Step 5: Commit.** `Cohesion; R-001 switch`

### Task 3: R-001 probe

**Files:** Create `crates/cna-probe/src/probes/r001.rs`.

- [ ] **Step 1: Write the failing test:** x is stages `1` … `24`; the option 1 series is minus the cohesion level after each stage and never exceeds 4; the option 2 series is the DP tally and first reaches 26 at stage 13. (Both are plotted as positive numbers: the board's chart has no negative axis.)
- [ ] **Step 3: Implement.** `kind: Line`. Scenario: a unit that alternately pushes 4 CP over its CPA (4 DP) and rests for a whole stage. Series: "Minus the cohesion level (option 1)" and "DP tally, never reset (option 2)". `finding`: the stage at which each option collapses the unit (or that option 1 never does) and the lowest level reached. `question`: "A unit alternating a 4-DP push with a stage of rest: when does it collapse?"
- [ ] **Step 4:** run, `cargo run -p cna-probe -- run`. **Step 5: Commit.** `R-001 probe`

### Task 4: Morale gap in chart-oddities

**Files:** Modify `crates/cna-probe/src/probes/oddities.rs`.

- [ ] **Step 1: Write the failing test:** the probe has an x label containing `morale -4` with value 100 × 1 / 36.
- [ ] **Step 3: Implement** from the morale table's `known_gaps` (label `"morale -4: no cell for 56"`), and drop the "not measured yet" sentence from the finding.
- [ ] **Step 4:** run, `cargo run -p cna-probe -- run`. **Step 5: Commit.** `chart-oddities: morale gap`

### Task 5: Slice done

- [ ] `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo run -p cna-probe -- check` clean.
- [ ] README: add R-001 to the probe table.
- [ ] Render check with `vendor/cna/tools/decisions_page.py --probes`.
- [ ] PR ready, CI green, merge (`gh pr merge --merge`).
- [ ] `docs/autopilot/PROGRESS.md`: slice 3 done; next: slice 4 plan.
