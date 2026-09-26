# Slice 2 (barrage and anti-armour) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load the Barrage Results Table, the Anti-Armour Combat Results
Table and the Terrain Effects Chart (errata applied), resolve barrage and
anti-armour fire as exact dice distributions, add switches for R-009 and
R-015, and write `probes/R-009.json` and `probes/R-015.json`.

**Architecture:** As slice 1. New loaders in `cna-data`, new pure modules in
`cna-rules`, two new probes in `cna-probe`. Nothing in slice 1 changes
except `Ruleset`, `SWITCHED`, `NOT_SIMULATED.md` and `probes::Ctx`.

**Spec:** `docs/designs/2026-09-26-engine-probes-design.md`
**Previous slice:** `docs/plans/2026-09-26-slice-1-close-assault.md` (its
Global Constraints apply unchanged).

## Domain facts the implementer needs

- **Barrage table** (`barrage-results.json`, SPI 12.6). Nine point bands
  (`columns`: `1-2 3-4 5-6 7-8 9-10 11-12 13-14 15-16 17+`, each with
  `min`/`max`, `max` null on `17+`). `rows` is a flat list of cells
  `{class, column, result, dice: {from, to}}`; `class` is `infantry`,
  `armor`, `gun` or `truck`; `result` is `no-effect`, `pinned`, `lose-1` or
  `lose-2`. Dice are the sequential reading (11–66). Per the table notes,
  `lose-n` on infantry or armour also pins.
- **Barrage terrain** (SPI 12.33). The target's terrain shifts the *band*
  down by the chart's `shifts.barrage` (negative = toward the defender);
  the best single in-hex benefit applies, not cumulative. Below `1-2` means
  no effect. 12 points against a level-two fortification (shift −2) reads
  `7-8`.
- **Anti-armour table** (`anti-armour-results.json`, SPI 14.6). Columns
  `0 1 … 15 16+` (actual points). `rows` are 18 dice pairs: row `"11"`
  covers readings 11–12, `"13"` covers 13–14, `"15"` 15–16, `"21"` 21–22 …
  `"65"` 65–66. `damage[column]` is damage points or `null` (none). The
  phasing player reads one row lower than rolled; row `"11"` stays put.
- **Anti-armour terrain** (SPI 14.3). Terrain shifts the points column
  toward the defender by `shifts.anti_armour`; in-hex effects are not
  cumulative (take the best), a hexside effect adds to the in-hex one. A
  shift below column 1 uses column 0. `"prohibited"` (up-escarpment) means
  no fire at all.
- **Terrain chart** (`terrain-effects.json`, SPI 8.37). `rows[]` have
  `kind` (`hex`, `hexside`, `fortification`, `minefield`), `terrain` and
  `shifts` (`barrage`, `anti_armour`, `close_assault`), where a shift is an
  integer, `"prohibited"`, a string such as `"see-fortifications"`, or the
  key is absent. Errata E-031 applies to this table.
- **R-009** (SPI 12.33, 14.0). Option 1: every barrage is shifted by the
  target hex's terrain, whichever side fires. Option 2: only barrages fired
  by the phasing player are shifted; the non-phasing reply is not. Default
  option 1 (the ruling's Rationale; the restated rule already does it).
- **R-015** (SPI 14.0, 14.32, 14.33). Two fires in one exchange: phasing
  fire at the defending armour in the assaulted hex, and non-phasing fire
  at the assaulting armour in its own hex.
  - Phasing fire: defended-hex shift plus the hexside crossed, under every
    option.
  - Non-phasing fire: option 1 — the assaulting armour's own hex shift, no
    hexside; option 2 — no shift at all; option 3 — own hex shift plus the
    hexside. Default option 2 (the ruling's Rationale: what the text says).

## Review Focus

1. A terrain row with a non-integer shift must not be read as 0 silently:
   `"prohibited"` is its own case, anything else fails the loader test.
2. Band and column shifts below the table: barrage gives no effect,
   anti-armour uses column 0. Tested.
3. The phasing row-lower rule clamps at row `"11"`. Tested.
4. Every barrage (class, band) and every anti-armour row covers the 36
   readings exactly once. Tested like slice 1's `only_declared_gaps_exist`;
   an undeclared gap or overlap is a `cna` data bug to report, not to paper
   over.

---

### Task 1: Load the three tables

**Files:**
- Create: `crates/cna-data/src/barrage.rs`, `crates/cna-data/src/anti_armour.rs`, `crates/cna-data/src/terrain.rs`
- Modify: `crates/cna-data/src/lib.rs`

**Interfaces:**
- `barrage::{Barrage, Band, Cell}`, `Barrage::load()`, `Barrage::band_index(points: u32) -> Option<usize>` (None for 0 points).
- `anti_armour::{AntiArmour, Row}`, `AntiArmour::load()`, `AntiArmour::row_index(reading: u8) -> usize`, `AntiArmour::damage(row: usize, column: usize) -> u32` (null reads 0).
- `terrain::{Terrain, TerrainRow, Shift}` with `enum Shift { Cols(i32), Prohibited, Other(String) }`; `Terrain::load()`, `Terrain::shifts(kind, name) -> Option<&Shifts>` (none for a row that refers elsewhere, such as major city).

- [ ] **Step 1: Write the failing tests:** band of 12 points is `11-12` and of 40 is `17+`; reading 12 is row 0 and 66 row 17; `damage(row "11", column "16+") == 22`; `rough` has barrage shift `Cols(-1)` and `up-escarpment` anti-armour shift `Prohibited`; loading `terrain-effects` applies E-031 without error.
- [ ] **Step 2: Run them.** `cargo test -p cna-data`. Expected: FAIL.
- [ ] **Step 3: Implement** with serde, reading only via `errata::load_table`. `Shift` deserialises from either an integer or a string (`#[serde(untagged)]`).
- [ ] **Step 4: Run them.** Expected: PASS.
- [ ] **Step 5: Commit.** `cna-data: barrage, anti-armour and terrain tables`

### Task 2: Barrage distributions

**Files:** Create `crates/cna-rules/src/barrage.rs`.

**Interfaces:**
- `barrage::outcome(t: &Barrage, class: &str, points: u32, shift: i32) -> BarrageOutcome { p_effect: f64, expected_loss: f64, unresolved: f64 }`. `p_effect` is the chance of `pinned` or a loss; `expected_loss` in TOE points.

- [ ] **Step 1: Write the failing tests:** every (class, band) covers the 36 readings exactly once; infantry at 9–10 points unshifted: `p_effect` and `expected_loss` hand-computed from the JSON (write the arithmetic in a comment); 12 points with shift −2 equals 7 points unshifted; 2 points with shift −1 is no effect (`p_effect == 0`).
- [ ] **Step 2–4:** run (FAIL), implement, run (PASS).
- [ ] **Step 5: Commit.** `cna-rules: exact barrage distributions`

### Task 3: Anti-armour distributions

**Files:** Create `crates/cna-rules/src/anti_armour.rs`.

**Interfaces:**
- `anti_armour::expected_damage(t: &AntiArmour, points: u32, shift: i32, phasing: bool) -> f64`: column = `min(points, 16)` shifted and clamped at 0 and 16; row from the reading, one lower for the phasing player (clamped at row 0).

- [ ] **Step 1: Write the failing tests:** 16 points, non-phasing, unshifted equals the mean over the 36 readings of the `16+` column (hand-computed); the phasing player's expected damage is never above the non-phasing one's at the same column; a shift of −20 reads column 0.
- [ ] **Step 2–4:** run (FAIL), implement, run (PASS).
- [ ] **Step 5: Commit.** `cna-rules: exact anti-armour damage`

### Task 4: Switches R-009, R-015

**Files:** Modify `crates/cna-rules/src/ruleset.rs`, `NOT_SIMULATED.md`; add to `barrage.rs` and `anti_armour.rs`.

**Interfaces:**
- `R009 { EitherSide = 1, NonPhasingOnly = 2 }` (default 1), `R015 { OwnHexBoth = 1, PhasingOnly = 2, HexAndHexsideBoth = 3 }` (default 2). `Ruleset` gains `r009`, `r015`; `SWITCHED` gains both.
- `barrage::terrain_shift(rules, target_shift: i32, firer_phasing: bool) -> i32`.
- `anti_armour::terrain_shift(rules, firer_phasing: bool, target_hex_shift: i32, hexside_shift: i32) -> i32`, where for phasing fire the target hex is the defended hex, and for non-phasing fire it is the assaulting armour's own hex.

- [ ] **Step 1: Write the failing tests:** a table test over every (option, firer) pair for both functions, matching the Domain facts.
- [ ] **Step 2–4:** run (FAIL), implement, remove R-009 and R-015 from `NOT_SIMULATED.md`, run (PASS; the coverage test still passes).
- [ ] **Step 5: Commit.** `Ruleset switches R-009, R-015`

### Task 5: R-009 probe

**Files:** Create `crates/cna-probe/src/probes/r009.rs`; `Ctx` gains `barrage`, `anti_armour`, `terrain`.

- [ ] **Step 1: Write the failing test:** x is the nine barrage bands (use each band's `min`, and 17 for `17+`); under option 2 the non-phasing reply against phasing infantry in a level-two fortification equals the unshifted `outcome`, and under option 1 it equals `outcome` with shift −2 (read from the terrain table, not hardcoded).
- [ ] **Step 3: Implement.** `kind: Line`. Scenario: the non-phasing player's reply barrage against a phasing infantry battalion that has stopped in a level-two fortification. Series: option 1 and option 2, y = `p_effect` × 100 ("% of barrages that pin or destroy"). `finding`: the largest gap in percentage points and at which band; also how much the expected loss changes. `question`: "Reply barrage against phasing infantry in a level-two fortification: how much does the terrain shift matter?"
- [ ] **Step 4:** run, `cargo run -p cna-probe -- run`. **Step 5: Commit.** `R-009 probe`

### Task 6: R-015 probe

**Files:** Create `crates/cna-probe/src/probes/r015.rs`.

- [ ] **Step 1: Write the failing test:** x is actual anti-armour points `1`…`16`; option 2's series equals the unshifted non-phasing `expected_damage`; option 3 is never above option 1, which is never above option 2.
- [ ] **Step 3: Implement.** `kind: Line`. Scenario: the non-phasing player's anti-armour fire at armour that assaults out of a rough hex (in-hex shift from the chart) across an up-slope hexside (hexside shift from the chart). Three series, one per option; y = expected damage points. `finding`: the average and largest reduction of options 1 and 3 against option 2, in damage points and per cent. `question`: "Defensive anti-armour fire at armour assaulting out of rough ground, up a slope".
- [ ] **Step 4:** run, `cargo run -p cna-probe -- run`. **Step 5: Commit.** `R-015 probe`

### Task 7: Slice done

- [ ] `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo run -p cna-probe -- check` clean.
- [ ] README: add both probes to the table.
- [ ] Check both render: `python3 vendor/cna/tools/decisions_page.py /tmp/d.html --probes probes` (run from `vendor/cna`, pointing at `../../probes`).
- [ ] PR ready, CI green, merge (`gh pr merge --merge`).
- [ ] `docs/autopilot/PROGRESS.md`: slice 2 done; next: slice 3 plan.
