# Slice 4 (construction costs) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load the Construction Chart, add a switch for R-018 with the
supply-dump cost it decides, and write `probes/R-018.json` and
`probes/chart-vs-text.json` (the board item listing the other
chart-versus-text disagreements).

**Architecture:** As slices 1–3. `cna-data` gains a construction loader and
a desert-raider-raids loader; `cna-rules` gains `construction` and a
`dice::sums()` helper; `cna-probe` gains two probes.

**Spec:** `docs/designs/2026-09-26-engine-probes-design.md`
**Previous slices:** `docs/plans/2026-09-26-slice-1-close-assault.md` (its
Global Constraints apply unchanged) and slices 2–3.

## Domain facts the implementer needs

- **Construction Chart** (`construction.json`, SPI 24.17). `rows[]` have
  `item`, `situation` (`build`, `rebuild-level`, …), optional `supplies`
  (`fuel`, `stores`, …), optional `cp`, optional `stages`. The chart's dump
  rows: `real-supply-dump` 3 CP + 10 stores; `fake-supply-dump` 2 CP, with
  `fake_ratio_max` 0.5.
- **R-018** (SPI 24.9, 24.17, 6.3). The text of 24.9 gives a real dump as
  3 CP + 20 stores and a dummy as 3 CP; the chart (and 6.3's CP summary)
  give 3 CP + 10 stores and 2 CP. Option 1: text. Option 2: charts.
  Option 3: CP from the charts, stores from the text (dummy 2 CP, real 3 CP
  + 20 stores). **Default: option 3.** The ruling's Rationale finds the
  charts' CP better attested (two charts agree) and the stores tied, and the
  restated rules already follow 24.9 for stores; option 3 is the reading
  that changes least while following the Rationale. Text figures are
  numbers, not rule text: encode them as constants citing SPI 24.9.
- **Chart vs text** (board item `chart-vs-text`).
  - Temporary repair facility: chart 50 fuel + 250 stores in 1 stage;
    SPI 24.82 150 fuel + 250 stores over 3 stages.
  - Repair facility one-level rebuild: chart 10 fuel + 50 stores; SPI 24.84
    30 fuel + 50 stores.
  - Guarded supply dump raid (SPI 27.91 chart; 27.5x text): the raider
    survives the guards on a two-dice **sum** at least equal to (chart) or
    above (text) the guards' raw close-assault defence. The two differ by
    exactly the chance that the sum equals the defence.

## Review Focus

1. Chart values must be read from `construction.json`, never retyped.
2. `dice::sums()` must give the 11 sums 2–12 with probabilities that sum to 1.
3. The guard-check difference is P(sum = defence), zero for defence
   outside 2–12. Tested.

---

### Task 1: Construction Chart and dice sums

**Files:** Create `crates/cna-data/src/construction.rs`; modify `crates/cna-rules/src/dice.rs`.

**Interfaces:**
- `cna_data::construction::{Construction, ConstructionRow, Supplies}`, `Construction::load()`, `Construction::row(item, situation) -> Option<&ConstructionRow>`.
- `cna_rules::dice::sums() -> impl Iterator<Item = (u8, f64)>`: (sum, probability) for 2–12.

- [ ] **Step 1: Write the failing tests:** the real dump row has 3 CP and 10 stores; the fake dump row 2 CP; the temporary repair facility 50 fuel, 250 stores, 1 stage; `sums()` has 11 entries, probabilities sum to 1, P(7) = 6/36.
- [ ] **Step 2–4:** run (FAIL), implement, run (PASS).
- [ ] **Step 5: Commit.** `Construction Chart; dice sums`

### Task 2: R-018 switch and dump cost

**Files:** Create `crates/cna-rules/src/construction.rs`; modify `ruleset.rs`, `NOT_SIMULATED.md`.

**Interfaces:**
- `R018 { Text = 1, Charts = 2, CpChartsStoresText = 3 }`, default 3; `Ruleset.r018`; `SWITCHED` gains `R-018`.
- `construction::DumpCost { cp: u32, stores: u32 }`; `construction::dump_cost(rules, t: &Construction, dummy: bool) -> DumpCost`.

- [ ] **Step 1: Write the failing test:** a table over the three options × real/dummy matching the Domain facts.
- [ ] **Step 2–4:** run (FAIL), implement, remove R-018 from `NOT_SIMULATED.md`, run (PASS).
- [ ] **Step 5: Commit.** `R-018 switch; supply-dump cost`

### Task 3: R-018 probe

**Files:** Create `crates/cna-probe/src/probes/r018.rs`; `Ctx` gains `construction`.

- [ ] **Step 1: Write the failing test:** x is `["Real dump: CP", "Real dump: stores", "Dummy dump: CP"]`; each option's series equals `dump_cost` under that option.
- [ ] **Step 3: Implement.** `kind: Bar`, one series per option. `finding`: what differs (dummy CP, real stores) and the CP for a ten-dump network at the chart's maximum share of fakes, per option. `question`: "What a supply dump costs under each option".
- [ ] **Step 4:** run, `cargo run -p cna-probe -- run`. **Step 5: Commit.** `R-018 probe`

### Task 4: chart-vs-text probe

**Files:** Create `crates/cna-probe/src/probes/chart_vs_text.rs`.

- [ ] **Step 1: Write the failing test:** x is guards' raw defence `2` … `12`; the chart series minus the text series equals 100 × P(sum = defence) at every point (e.g. 16.7 at 7); the finding names both repair-facility gaps with the chart values read from the table.
- [ ] **Step 3: Implement.** `kind: Line`. Series "Chart (27.91): sum at least the defence" and "Text (27.5x): sum above the defence", y = % of raids where the raider survives the guards. No `option` (this is a board item, not a ruling). `finding`: the largest gap and where, then the two repair-facility gaps ("the text costs 3x the fuel …"). `question`: "Chart against text: how much the printed disagreements matter".
- [ ] **Step 4:** run, `cargo run -p cna-probe -- run`. **Step 5: Commit.** `chart-vs-text probe`

### Task 5: Slice done

- [ ] `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo run -p cna-probe -- check` clean.
- [ ] README: both probes in the table.
- [ ] Render check with `vendor/cna/tools/decisions_page.py --probes`.
- [ ] PR ready, CI green, merge (`gh pr merge --merge`).
- [ ] `docs/autopilot/PROGRESS.md` and the journal: `MISSION 1 COMPLETE`.
