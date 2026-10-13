# Mission 3, slice 1 (engine queries) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `cna-game` the read-only queries a UI needs and Mission 2
does not provide (legal moves with CP cost, legal placements, legal
answers to a pending decision, who must act, map geometry), and prove the
crate builds for WebAssembly.

**Architecture:** A new `cna-game::query` module whose functions reuse the
exact validation code `apply` runs (no second copy of a rule), and a
`cna-game::geom` module porting cna's `tools/map_geom.py`. Nothing here
changes state.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Depends on:** Mission 2 slices 1–4 merged (map, scenario, state,
movement). Slices 5–6 of M2 are needed only for `answers()` on combat
pendings; add those variants when they land (Task 4 says how).

## Before Task 1

- [ ] Read the merged `crates/cna-game/src` on `origin/main`: record the
  real names of `GameState`, `Order`, `Pending`, `Illegal`, the
  movement validator, the static-data handle `apply` takes, and the
  loaders. Where they differ from the spec, update the spec and these
  plans in this PR's first commit.

## Domain facts the implementer needs

- **Movement stops** (M2 slice 4): entering an enemy ZOC ends the move
  (10.2x); a non-motorised unit may not voluntarily exceed 150 % of its CPA
  (8.1x); CP over the CPA costs DP (6.21) but is legal; a stack uses its
  lowest CPA (6.15); never into an enemy-occupied hex.
- **Geometry** (`vendor/cna/tools/map_geom.py`): pointy-top, size 20,
  `w = √3·size`, row pitch `1.5·size`; rows grow northward so `y` grows as
  the row falls; a row's half-hex shift from `sheets.json`
  `odd_rows_shift`. Theatre: sheet A–E index *k* adds `k·33·w` to `x`, and
  every sheet uses the tallest sheet's top row for `y` (`seams.json` note:
  column 33 meets the next sheet's column 01 on the same row; E's column
  00 is not a hex). Malta (M) is drawn as an inset south-west of A.

## Tasks

### Task 1: `geom`

**Interfaces:** `geom::centre(&Map, &HexId) -> (f64, f64)` (theatre
coordinates), `geom::corners(c) -> [(f64, f64); 6]`,
`geom::hexside(&Map, a, b) -> ((f64,f64),(f64,f64))`, `geom::bounds(&Map)`.

- [ ] **Step 1: Write the failing tests:** parse the polygon `points` of
  three hexes from `vendor/cna/site/public/map/C.svg` and check
  `corners(centre(...))` minus C's theatre offset equals them to 0.01; a
  seam pair from `seams.json` (C0633–D0701) has centres one hex width
  apart (±0.01); no two hexes' centres coincide.
- [ ] **Step 2: Run** `cargo test -p cna-game geom` (FAIL).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** (PASS). **Step 5: Commit** `Theatre geometry`.

### Task 2: `reachable`

**Interfaces:** `Reach { hex: HexId, cp: u32, path: Vec<HexId>, dp: u32,
stops_in_zoc: bool }`; `GameState::reachable(&self, data, pieces:
&[UnitId]) -> Result<Vec<Reach>, Illegal>` (`Err` when the pieces cannot
move at all now, with the same case `apply` would give).

Refactor first if needed: M2's `Move` validation must expose a per-step
function `step(&self, data, stack, from, to, spent) -> Result<StepOutcome,
Illegal>` that both `apply` and `reachable` call. Dijkstra over
`(hex, cp_spent)` with that step; a hex where the move must stop is not
expanded.

- [ ] **Step 1: Write the failing tests:** on the real map with a
  hand-built state: an Italian infantry stack at Sidi Barrani reaches
  along the coast road at road cost (pinned numbers, hand-computed in the
  test comment with the case cited); Halfaya escarpment refused except by
  the track hexside; a mixed stack is limited by its lowest CPA; a hex
  adjacent to a Commonwealth unit is reachable but not expanded; the
  150 % cap bounds a foot unit; a motorised unit may exceed with `dp > 0`;
  nothing is returned for a piece not owned by the acting player.
- [ ] **Step 2: Property test** (`proptest`, already used by M2 or add it
  as a dev-dependency): for random pieces in the golden start state,
  every `Reach` path is accepted by `apply(Move)` on a clone with exactly
  `cp` spent; for random accepted single-path moves, the end hex is in
  `reachable` with `cp` no higher.
- [ ] **Step 3–5:** run (FAIL), implement, run (PASS), commit
  `Reachable hexes with CP cost`.

### Task 3: `placements` and `acting_player`

- [ ] **Step 1: Write the failing tests:** for each pending Graziani
  deployment, every hex `placements(unit)` returns is accepted by
  `Place` on a clone, and a sample of hexes it omits are refused (59.2,
  9.14, `not_within`); `acting_player()` is the pending player when set,
  else the phasing player, and `apply` from the other side is `Illegal`.
- [ ] **Step 2–5:** as Task 1; commit `Placements and acting player`.

### Task 4: `answers`

**Interfaces:** `GameState::answers(&self, data) -> Vec<Order>`, the
finite legal answers to `pending` (and `DeclareInitiative` choices in
phase A). Variants whose answer set is not finite (force assignment
splits) return a template order and the UI builds the rest; document
which in the doc comment.

- [ ] **Step 1: Write the failing tests:** for each `Pending` variant
  M2 has merged, a state that raises it; every returned order is accepted
  by `apply` on a clone. A variant added later by M2 must add a case
  here: a test enumerates `Pending` variants via a `match` with no
  wildcard so the compiler flags a missing one.
- [ ] **Step 2–5:** as Task 1; commit `Legal answers to pending decisions`.

### Task 5: from-string loaders and the wasm build

- [ ] **Step 1:** if `cna-data` loaders take only paths, add
  `from_str` variants and make the path ones call them; test that both
  give equal values for the map and Graziani.
- [ ] **Step 2:** `rustup target add wasm32-unknown-unknown` (CI only
  unless Brian agrees locally); `cargo build -p cna-game --target
  wasm32-unknown-unknown` must pass. Fix any `std::fs` use behind the
  data handle and enable `getrandom`'s `js`/`wasm_js` feature only for
  that target.
- [ ] **Step 3:** CI: add the target and that build step to `ci.yml`.
- [ ] **Step 4:** run the gates; commit `cna-game builds for wasm`.
