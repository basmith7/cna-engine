# Slice 1 (close assault) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Rust workspace that loads the `cna` close-assault table (errata
applied), resolves close-assault outcomes as exact dice distributions under a
ruleset of ruling switches, and writes probe results for R-011, R-012 and the
close-assault chart oddities to `probes/*.json`.

**Architecture:** Three crates. `cna-data` does I/O and errata only.
`cna-rules` holds pure functions and `Ruleset`. `cna-probe` is the binary that
runs probes and checks that the committed output is fresh. `cna` is a git
submodule at `vendor/cna`.

**Tech Stack:** Rust stable (edition 2024), serde, serde_json, json-patch,
anyhow, clap.

**Spec:** `docs/designs/2026-09-26-engine-probes-design.md`

## Global Constraints

- The licence is MIT. Every probe output records `rules_commit` (the
  `vendor/cna` HEAD) and `engine_commit`.
- Table data is read only from `vendor/cna/data/tables/*.json`, with errata
  from `vendor/cna/data/errata/*.json` applied at load. Never copy a table
  into this repo.
- Randomness is exact enumeration by default. Monte Carlo is allowed only
  with a seeded `ChaCha8Rng`.
- Ruling switches: one enum per ruling. Variant numbers equal the option
  numbers in `vendor/cna/rulings/R-nnn.md`, and the default is the option the
  printed text supports.
- Every ruling in `vendor/cna/rulings/` has a switch or a line in
  `NOT_SIMULATED.md`.
- No SPI rule text anywhere in this repo. Code comments cite case numbers
  (`SPI 15.79`) and never quote.

## Domain facts the implementer needs

- **Dice.** Close assault rolls two dice read *sequentially*. The two dice
  are physically different, and "large die first" means the larger *die*,
  not the higher number: the rules' example reads 2 and 5 as **25**. So the
  36 readings 11–16, 21–26, …, 61–66 each have probability 1/36. The same
  roll *summed* (2–12) is read against the `sums` lines.
- **Columns.** The table has 18 differential columns. Their ids, in order,
  are `-11 -8 -6 -4 -3 -2 -1 0 +1 +2 +3 +4 +5 +7 +9 +11 +14 +17`. Each has
  `min`/`max` (null at the open ends) and `overrun` (true for +11 and
  above).
- **Losses.** `losses[side][pct][column]` is `{from, to}`, an inclusive range
  of sequential readings, or `null` when that row cannot occur in that
  column. `side` is `attacker` or `defender`. `pct` is one of
  `50 40 30 25 20 15 10 5 0`. A reading should fall in exactly one row per
  side and column. Where it falls in none, that is a printed gap
  (`known_gaps` lists them).
- **Sums.** `sums[line][column]` is a list of dice sums, or `null`. The lines
  are `capture_attacker`, `engaged`, `capture_defender`, `retreat_1`,
  `retreat_2` and `retreat_3`.
- **Errata.** `data/errata/E-*.json` has `table` and `patches`, which are RFC
  6902 operations (`add`, `remove`, `replace`) against that table's JSON.
- **R-012.** A defender who commits nothing retreats 3 hexes with 3 DP
  (SPI 15.29). Under option 2 he may stay put and pay 10 % per hex not
  retreated (SPI 15.82), which is 30 %. Under options 1 and 3 he must
  retreat.
- **R-011.** The percentage loss is taken of each side's own raw points
  (option 1) or of both sides' combined raw points (option 2).
- **Double raw strength** (SPI 15.51). A side with at least twice the
  other's raw points shifts two columns its way.

## Review Focus

1. A patch path that does not resolve (an errata file edited in `cna`) must
   fail loudly, naming the errata id. It must not silently skip. Tested in
   Task 2.
2. A reading in no row, or in two rows, must be reported rather than counted
   as a 0 % loss. The distribution tracks `unresolved` probability
   separately. Tested in Task 3.
3. A `vendor/cna` bump that adds a ruling must fail the coverage test until
   the ruling has a switch or a `NOT_SIMULATED.md` line. Tested in Task 4.
4. `cna-probe check` must fail when `vendor/cna` has moved but `probes/`
   was not regenerated. Tested in Task 6.
5. Column shifts must clamp at the end columns (`-11`, `+17`): a shift past
   the end stays on the end column. Tested in Task 3.

---

### Task 1: Workspace, submodule, CI

**Files:**
- Create: `Cargo.toml`, `crates/cna-data/Cargo.toml`, `crates/cna-data/src/lib.rs`, `crates/cna-rules/Cargo.toml`, `crates/cna-rules/src/lib.rs`, `crates/cna-probe/Cargo.toml`, `crates/cna-probe/src/main.rs`, `LICENSE`, `README.md`, `.gitignore`, `.github/workflows/ci.yml`, `rust-toolchain.toml`
- Submodule: `vendor/cna` → `https://github.com/basmith7/cna.git`

**Interfaces:**
- Produces: `cna_data::repo_root() -> PathBuf` (the workspace root, from `CARGO_MANIFEST_DIR`), `cna_data::cna_root() -> PathBuf` (`<root>/vendor/cna`).

- [ ] **Step 1: Add the submodule and workspace**

```bash
git submodule add https://github.com/basmith7/cna.git vendor/cna
```

`Cargo.toml`:
```toml
[workspace]
resolver = "3"
members = ["crates/cna-data", "crates/cna-rules", "crates/cna-probe"]

[workspace.package]
edition = "2024"
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
json-patch = "4"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
```

`rust-toolchain.toml`:
```toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
```

`crates/cna-data/Cargo.toml`:
```toml
[package]
name = "cna-data"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
json-patch.workspace = true
anyhow.workspace = true
```

`crates/cna-rules/Cargo.toml` depends on `cna-data = { path = "../cna-data" }`, `serde`, `anyhow`. `crates/cna-probe/Cargo.toml` depends on `cna-data`, `cna-rules`, `serde`, `serde_json`, `anyhow`, `clap`.

- [ ] **Step 2: Write the failing test** in `crates/cna-data/src/lib.rs`

```rust
use std::path::PathBuf;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("workspace root")
}

pub fn cna_root() -> PathBuf {
    repo_root().join("vendor/cna")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submodule_is_checked_out() {
        assert!(cna_root().join("data/tables/close-assault-results.json").is_file(),
            "run: git submodule update --init");
    }
}
```

- [ ] **Step 3: Run it.** `cargo test -p cna-data`. Expected: PASS once the submodule is present. Temporarily move `vendor/cna` aside to see the failure message, then move it back.

- [ ] **Step 4: CI** (`.github/workflows/ci.yml`)

```yaml
name: ci
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { submodules: true }
      - uses: dtolnay/rust-toolchain@stable
        with: { components: clippy }
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test
      - run: cargo run -p cna-probe -- check
```

`cna-probe check` does not exist yet. Until Task 6, `crates/cna-probe/src/main.rs` is `fn main() {}`, which exits 0 for any argument.

`README.md`: two paragraphs saying what the repo is (point at the spec), plus `git clone --recursive`, `cargo test`, `cargo run -p cna-probe -- run`. `LICENSE`: MIT, copyright "2026 cna-engine contributors". `.gitignore`: `target/`.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Workspace, cna submodule, CI"`

### Task 2: Load the close-assault table with errata

**Files:**
- Create: `crates/cna-data/src/errata.rs`, `crates/cna-data/src/close_assault.rs`
- Modify: `crates/cna-data/src/lib.rs` (add `pub mod errata; pub mod close_assault;`)

**Interfaces:**
- Produces:
  - `cna_data::errata::load_table(name: &str) -> anyhow::Result<serde_json::Value>`: reads `data/tables/<name>.json` and applies every errata file whose `table == name`, in id order.
  - `cna_data::close_assault::{CloseAssault, Column, Range, Side}` and `CloseAssault::load() -> anyhow::Result<CloseAssault>`.
  - `Side` is an enum `{Attacker, Defender}` with `fn key(self) -> &'static str` returning `"attacker"`/`"defender"`.
  - `CloseAssault` has `columns: Vec<Column>`, `losses: BTreeMap<String, BTreeMap<String, BTreeMap<String, Option<Range>>>>`, `sums: BTreeMap<String, BTreeMap<String, Option<Vec<u8>>>>` and `known_gaps: Vec<KnownGap>` (`KnownGap { side: String, column: String, readings: Vec<u8> }`).
  - `CloseAssault::column_index(&self, id: &str) -> Option<usize>`.

- [ ] **Step 1: Write the failing tests** (`close_assault.rs`, bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e008_is_applied() {
        let t = CloseAssault::load().unwrap();
        let r = t.losses["defender"]["10"]["+4"].unwrap();
        assert_eq!((r.from, r.to), (34, 45)); // printed 24-45, errata E-008
    }

    #[test]
    fn eighteen_columns_in_order() {
        let t = CloseAssault::load().unwrap();
        let ids: Vec<_> = t.columns.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["-11","-8","-6","-4","-3","-2","-1","0","+1","+2","+3","+4","+5","+7","+9","+11","+14","+17"]);
        assert!(t.columns[15].overrun && !t.columns[14].overrun);
    }
}
```

And in `errata.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_patch_path_names_the_errata() {
        let mut table = serde_json::json!({"a": 1});
        let e = serde_json::json!({"id": "E-999", "table": "x",
            "patches": [{"op": "replace", "path": "/missing/deep", "value": 2}]});
        let err = apply_errata(&mut table, &e).unwrap_err().to_string();
        assert!(err.contains("E-999"), "{err}");
    }
}
```

- [ ] **Step 2: Run them.** `cargo test -p cna-data`. Expected: FAIL (types and functions not defined).

- [ ] **Step 3: Implement**

`errata.rs`:
```rust
use anyhow::{Context, Result};
use serde_json::Value;
use crate::cna_root;

pub fn apply_errata(table: &mut Value, errata: &Value) -> Result<()> {
    let id = errata["id"].as_str().unwrap_or("?");
    let patch: json_patch::Patch = serde_json::from_value(errata["patches"].clone())
        .with_context(|| format!("{id}: patches are not RFC 6902"))?;
    json_patch::patch(table, &patch).with_context(|| format!("{id}: patch does not apply"))?;
    Ok(())
}

pub fn load_table(name: &str) -> Result<Value> {
    let root = cna_root().join("data");
    let path = root.join("tables").join(format!("{name}.json"));
    let mut table: Value = serde_json::from_str(&std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?)?;
    let mut files: Vec<_> = std::fs::read_dir(root.join("errata"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    for f in files {
        let e: Value = serde_json::from_str(&std::fs::read_to_string(&f)?)?;
        if e["table"] == name {
            apply_errata(&mut table, &e)?;
        }
    }
    Ok(table)
}
```

`close_assault.rs`:
```rust
use std::collections::BTreeMap;
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Attacker, Defender }

impl Side {
    pub fn key(self) -> &'static str {
        match self { Side::Attacker => "attacker", Side::Defender => "defender" }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Column { pub id: String, pub min: Option<i32>, pub max: Option<i32>, pub et: bool, pub overrun: bool }

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Range { pub from: u8, pub to: u8 }

impl Range {
    pub fn contains(self, reading: u8) -> bool { (self.from..=self.to).contains(&reading) }
}

#[derive(Debug, Clone, Deserialize)]
pub struct KnownGap { pub side: String, pub column: String, pub readings: Vec<u8> }

#[derive(Debug, Clone, Deserialize)]
pub struct CloseAssault {
    pub columns: Vec<Column>,
    pub losses: BTreeMap<String, BTreeMap<String, BTreeMap<String, Option<Range>>>>,
    pub sums: BTreeMap<String, BTreeMap<String, Option<Vec<u8>>>>,
    #[serde(default)]
    pub known_gaps: Vec<KnownGap>,
}

impl CloseAssault {
    pub fn load() -> Result<Self> {
        Ok(serde_json::from_value(crate::errata::load_table("close-assault-results")?)?)
    }
    pub fn column_index(&self, id: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.id == id)
    }
}
```

- [ ] **Step 4: Run them.** `cargo test -p cna-data`. Expected: PASS.

- [ ] **Step 5: Commit.** `git commit -am "cna-data: close-assault table with errata"` (after `git add crates/`).

### Task 3: Exact close-assault distributions

**Files:**
- Create: `crates/cna-rules/src/dice.rs`, `crates/cna-rules/src/close_assault.rs`
- Modify: `crates/cna-rules/src/lib.rs` (`pub mod dice; pub mod close_assault;`)

**Interfaces:**
- Consumes: `cna_data::close_assault::{CloseAssault, Side}`.
- Produces:
  - `dice::rolls() -> impl Iterator<Item = (u8, u8)>`: all 36 ordered `(first, second)` pairs, each with probability `1.0 / 36.0`.
  - `close_assault::loss_pct(t: &CloseAssault, side: Side, column: &str, reading: u8) -> Lookup`, where `enum Lookup { Pct(u8), Gap, Overlap(Vec<u8>) }`.
  - `close_assault::SideOutcome { expected_pct: f64, unresolved: f64, expected_retreat_hexes: f64 }` and `outcome(t, side, column) -> SideOutcome`. Retreat applies only to the defender; it is 0 for the attacker.
  - `close_assault::shift(t: &CloseAssault, column: &str, by: i32) -> String`: moves `by` columns (positive toward `+17`) and clamps at the ends.

- [ ] **Step 1: Write the failing tests** (`close_assault.rs`, bottom)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use cna_data::close_assault::CloseAssault;

    fn t() -> CloseAssault { CloseAssault::load().unwrap() }

    #[test]
    fn rolls_are_36() {
        assert_eq!(crate::dice::rolls().count(), 36);
    }

    #[test]
    fn only_declared_gaps_exist() {
        // Every (side, column, reading) resolves to exactly one row, except known_gaps.
        let t = t();
        let mut gaps = vec![];
        for side in [Side::Attacker, Side::Defender] {
            for c in &t.columns {
                for (a, b) in crate::dice::rolls() {
                    match loss_pct(&t, side, &c.id, a * 10 + b) {
                        Lookup::Pct(_) => {}
                        Lookup::Gap => gaps.push((side.key().to_string(), c.id.clone(), a * 10 + b)),
                        Lookup::Overlap(rows) => panic!("{} {} {}: in rows {rows:?}", side.key(), c.id, a * 10 + b),
                    }
                }
            }
        }
        let declared: Vec<_> = t.known_gaps.iter()
            .flat_map(|g| g.readings.iter().map(move |r| (g.side.clone(), g.column.clone(), *r)))
            .collect();
        assert_eq!(gaps, declared);
    }

    #[test]
    fn gap_probability_is_reported_not_zero_loss() {
        let o = outcome(&t(), Side::Defender, "+2");
        assert!((o.unresolved - 3.0 / 36.0).abs() < 1e-12);
    }

    #[test]
    fn shift_clamps_at_ends() {
        let t = t();
        assert_eq!(shift(&t, "+14", 5), "+17");
        assert_eq!(shift(&t, "-8", -3), "-11");
        assert_eq!(shift(&t, "0", 2), "+2");
    }

    #[test]
    fn hand_computed_attacker_minus_11() {
        // Read the -11 attacker column from the JSON by hand and put the value here.
        // Expected: sum over the 36 readings of pct/36.
        let o = outcome(&t(), Side::Attacker, "-11");
        assert!(o.expected_pct > 30.0, "{o:?}");
    }
}
```

Before Step 3, open `vendor/cna/data/tables/close-assault-results.json`, compute the attacker `-11` expected loss by hand (a spreadsheet is fine), and replace `> 30.0` with an equality to 1e-9. The rules summary says the attacker loses 50 % on any 11–15 at −11 and "never escapes unhurt", so the value is well above 30.

- [ ] **Step 2: Run them.** `cargo test -p cna-rules`. Expected: FAIL (nothing defined).

- [ ] **Step 3: Implement**

`dice.rs`:
```rust
/// Two distinguishable dice, read first-then-second (SPI 11.4; the rules'
/// example reads 2 and 5 as 25). All 36 ordered pairs are equally likely.
pub fn rolls() -> impl Iterator<Item = (u8, u8)> {
    (1..=6u8).flat_map(|a| (1..=6u8).map(move |b| (a, b)))
}
pub const P: f64 = 1.0 / 36.0;
```

`close_assault.rs`:
```rust
use cna_data::close_assault::{CloseAssault, Side};
use crate::dice::{rolls, P};

#[derive(Debug, PartialEq)]
pub enum Lookup { Pct(u8), Gap, Overlap(Vec<u8>) }

pub fn loss_pct(t: &CloseAssault, side: Side, column: &str, reading: u8) -> Lookup {
    let hits: Vec<u8> = t.losses[side.key()].iter()
        .filter_map(|(pct, cols)| match cols.get(column).copied().flatten() {
            Some(r) if r.contains(reading) => Some(pct.parse().unwrap()),
            _ => None,
        })
        .collect();
    match hits.as_slice() {
        [] => Lookup::Gap,
        [p] => Lookup::Pct(*p),
        _ => Lookup::Overlap(hits),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SideOutcome { pub expected_pct: f64, pub unresolved: f64, pub expected_retreat_hexes: f64 }

pub fn outcome(t: &CloseAssault, side: Side, column: &str) -> SideOutcome {
    let mut o = SideOutcome { expected_pct: 0.0, unresolved: 0.0, expected_retreat_hexes: 0.0 };
    for (a, b) in rolls() {
        match loss_pct(t, side, column, a * 10 + b) {
            Lookup::Pct(p) => o.expected_pct += P * p as f64,
            _ => o.unresolved += P,
        }
        if side == Side::Defender {
            for (hexes, line) in [(1.0, "retreat_1"), (2.0, "retreat_2"), (3.0, "retreat_3")] {
                if let Some(Some(sums)) = t.sums[line].get(column) {
                    if sums.contains(&(a + b)) { o.expected_retreat_hexes += P * hexes; }
                }
            }
        }
    }
    o
}

pub fn shift(t: &CloseAssault, column: &str, by: i32) -> String {
    let i = t.column_index(column).expect("known column") as i32;
    let j = (i + by).clamp(0, t.columns.len() as i32 - 1) as usize;
    t.columns[j].id.clone()
}
```

`cna-rules/Cargo.toml` needs `cna-data`. If `only_declared_gaps_exist` fails with extra gaps or an overlap, **do not change the test to match**. Each extra item is a printed oddity nobody has declared yet. Add it to `cna`'s `known_gaps` with a note, in a `cna` PR titled "close-assault: undeclared gap/overlap found by cna-engine". Then bump the submodule here. Record it in `docs/autopilot/PROGRESS.md` **Found**.

- [ ] **Step 4: Run them.** `cargo test -p cna-rules`. Expected: PASS.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "cna-rules: exact close-assault distributions"`

### Task 4: Ruleset and coverage

**Files:**
- Create: `crates/cna-rules/src/ruleset.rs`, `NOT_SIMULATED.md`
- Modify: `crates/cna-rules/src/lib.rs` (`pub mod ruleset;`)

**Interfaces:**
- Produces:
  - `ruleset::Ruleset { pub r011: R011, pub r012: R012 }`, with `Default`.
  - `R011 { OwnRaw = 1, CombinedRaw = 2 }` (default `OwnRaw`, the printed example).
  - `R012 { MustRetreat = 1, BuyOut = 2, NoPathNoWithhold = 3 }` (default `BuyOut`, which the ruling's Rationale says the text supports).
  - `ruleset::SWITCHED: &[&str] = &["R-011", "R-012"]`.

- [ ] **Step 1: Write the failing test** (`ruleset.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ruling_is_switched_or_listed() {
        let dir = cna_data::cna_root().join("rulings");
        let listed = std::fs::read_to_string(cna_data::repo_root().join("NOT_SIMULATED.md")).unwrap();
        let mut missing = vec![];
        for e in std::fs::read_dir(dir).unwrap() {
            let name = e.unwrap().file_name().into_string().unwrap();
            let Some(id) = name.strip_suffix(".md").filter(|s| s.starts_with("R-")) else { continue };
            if !SWITCHED.contains(&id) && !listed.contains(&format!("| {id} |")) {
                missing.push(id.to_string());
            }
        }
        missing.sort();
        assert!(missing.is_empty(), "no switch and not in NOT_SIMULATED.md: {missing:?}");
    }
}
```

- [ ] **Step 2: Run it.** `cargo test -p cna-rules ruleset`. Expected: FAIL.

- [ ] **Step 3: Implement** the enums with `#[repr(u8)]` and `#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]`. Write `NOT_SIMULATED.md` as a table `| Ruling | Why not (yet) |`, with one row per remaining ruling in `vendor/cna/rulings/`:
  - R-001, R-009, R-015, R-018: "planned: slice 3 / 2 / 2 / 4".
  - R-013: "the difference is linear: ammunition scales with the pinned share; no probe adds information".
  - R-020, R-021, R-022: "needs the Logistics Game (§47–58) restated in cna".
  - R-002–R-008, R-010, R-014, R-016, R-017, R-019: "interpretive: a question of what the text means, not of numbers".

  Read each ruling's title so every reason is accurate. If `vendor/cna` has rulings beyond R-022, classify them the same way.

- [ ] **Step 4: Run it.** Expected: PASS.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Ruleset switches R-011, R-012; NOT_SIMULATED.md"`

### Task 5: Probe output format and the R-012 probe

**Files:**
- Create: `crates/cna-probe/src/output.rs`, `crates/cna-probe/src/probes/mod.rs`, `crates/cna-probe/src/probes/r012.rs`

**Interfaces:**
- Consumes: `cna_rules::close_assault::{outcome, SideOutcome}`, `cna_data::close_assault::CloseAssault`.
- Produces:
  - `output::Probe { id: String, question: String, rules_commit: String, engine_commit: String, kind: Kind, x: Axis, y: Axis, series: Vec<Series>, finding: String }`, where `Kind { Line, Bar }` serialises as `"line"`/`"bar"`, `Axis { label: String, values: Vec<String> }` (`values` is empty for y), and `Series { option: Option<u8>, label: String, values: Vec<f64> }`. It derives `Serialize, Deserialize, PartialEq`.
  - `probes::Ctx { table: CloseAssault }`.
  - `probes::all() -> Vec<(&'static str, fn(&Ctx) -> Probe)>`. The commits are filled in by the caller (Task 6), so probe functions leave them empty.

- [ ] **Step 1: Write the failing test** (`r012.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_out_is_flat_30_and_fight_matches_outcome() {
        let ctx = Ctx { table: CloseAssault::load().unwrap() };
        let p = probe(&ctx);
        assert_eq!(p.x.values.len(), 18);
        let buy = p.series.iter().find(|s| s.option == Some(2)).unwrap();
        assert!(buy.values.iter().all(|v| *v == 30.0));
        let fight = p.series.iter().find(|s| s.option.is_none()).unwrap();
        let o = cna_rules::close_assault::outcome(&ctx.table, Side::Defender, "+11");
        assert!((fight.values[15] - o.expected_pct).abs() < 1e-9);
        assert!(p.finding.contains("column"));
    }
}
```

- [ ] **Step 2: Run it.** `cargo test -p cna-probe`. Expected: FAIL.

- [ ] **Step 3: Implement**

```rust
use cna_data::close_assault::{CloseAssault, Side};
use cna_rules::close_assault::outcome;
use crate::output::{Axis, Kind, Probe, Series};
use super::Ctx;

pub fn probe(ctx: &Ctx) -> Probe {
    let cols: Vec<String> = ctx.table.columns.iter().map(|c| c.id.clone()).collect();
    let fight: Vec<f64> = cols.iter().map(|c| outcome(&ctx.table, Side::Defender, c).expected_pct).collect();
    let cheaper: Vec<&str> = cols.iter().zip(&fight).filter(|(_, f)| **f > 30.0).map(|(c, _)| c.as_str()).collect();
    let finding = if cheaper.is_empty() {
        "Fighting never costs the defender more than 30 % on average, so the 15.82 buy-out is never a bargain.".to_string()
    } else {
        format!("Under option 2 a defender facing column {} or worse loses less by withholding everything and paying 30 % than by fighting (expected loss when fighting: {:.0} % at {}).",
            cheaper[0], fight[cols.iter().position(|c| c == cheaper[0]).unwrap()], cheaper[0])
    };
    Probe {
        id: "R-012".into(),
        question: "Defender's expected loss when fighting, against withholding everything".into(),
        rules_commit: String::new(), engine_commit: String::new(),
        kind: Kind::Line,
        x: Axis { label: "Final close-assault column".into(), values: cols.clone() },
        y: Axis { label: "Defender loss, % of raw points".into(), values: vec![] },
        series: vec![
            Series { option: None, label: "Fight (expected)".into(), values: fight },
            Series { option: Some(1), label: "Withhold: retreat 3 hexes, 0 %".into(), values: vec![0.0; cols.len()] },
            Series { option: Some(2), label: "Withhold: stay, pay 30 %".into(), values: vec![30.0; cols.len()] },
        ],
        finding,
    }
}
```

Both withhold options also cost 3 DP; say so in the series label if the board's legend has room. Option 3 equals option 1 whenever a retreat path exists, so it is not plotted.

- [ ] **Step 4: Run it.** Expected: PASS.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "Probe format; R-012 probe"`

### Task 6: `cna-probe run` / `check`

**Files:**
- Modify: `crates/cna-probe/src/main.rs`
- Create: `probes/` (generated output, committed)

**Interfaces:**
- Consumes: `probes::all()`, `output::Probe`.
- Produces: the CLI `cna-probe run` (writes `probes/<id>.json`, pretty JSON with a trailing newline) and `cna-probe check` (exit 1 listing each stale or missing file).

- [ ] **Step 1: Write the failing test.** In `main.rs`, add `fn render_all(rules: &str, engine: &str) -> Vec<(String, String)>`, which returns (file name, contents), and test:

```rust
#[test]
fn staleness_follows_rules_commit() {
    let a = render_all("aaa", "e");
    let b = render_all("bbb", "e");
    assert_ne!(a, b);
    assert!(a.iter().any(|(n, _)| n == "R-012.json"));
}
```

- [ ] **Step 2: Run it.** Expected: FAIL.

- [ ] **Step 3: Implement.** `rules_commit` is `git -C vendor/cna rev-parse HEAD`. `engine_commit` is the literal `"see git log"`; do not embed HEAD, because that would make every commit stale its own output. `check` compares `render_all(..)` with the files on disk, ignoring `engine_commit`, and fails listing each difference. `run` writes the files. Use `clap` subcommands `Run` and `Check`.

- [ ] **Step 4: Run it.** `cargo test`, then `cargo run -p cna-probe -- run`, then `cargo run -p cna-probe -- check` (expected exit 0). Then `git -C vendor/cna checkout HEAD~1 && cargo run -p cna-probe -- check`: expected exit 1, naming R-012. Finish with `git -C vendor/cna checkout -`.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "cna-probe run/check; probes/R-012.json"`

### Task 7: R-011 probe

**Files:**
- Create: `crates/cna-probe/src/probes/r011.rs`; register it in `probes/mod.rs`.

**Interfaces:**
- Consumes: `outcome`, `shift`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn combined_base_multiplies_small_side_losses() {
    let ctx = Ctx { table: CloseAssault::load().unwrap() };
    let p = probe(&ctx);
    assert_eq!(p.x.values, ["1:4", "1:3", "1:2", "1:1", "2:1", "3:1", "4:1"]);
    let own = p.series.iter().find(|s| s.label == "Attacker, own raw (option 1)").unwrap();
    let comb = p.series.iter().find(|s| s.label == "Attacker, combined raw (option 2)").unwrap();
    // At 1:4 the combined base is 5x the attacker's own.
    assert!((comb.values[0] - own.values[0] * 5.0).abs() < 1e-9);
}
```

- [ ] **Step 2: Run it.** Expected: FAIL.

- [ ] **Step 3: Implement.** Take ratios A:D in `[(1,4),(1,3),(1,2),(1,1),(2,1),(3,1),(4,1)]`. The column starts at `"0"` and is shifted `+2` when A ≥ 2D, or `-2` when D ≥ 2A (double raw strength, SPI 15.51). Four series of y = raw points lost as a % of *that side's own* raw strength:
  - Attacker, option 1: `outcome(Attacker).expected_pct`.
  - Attacker, option 2: that × (A+D)/A.
  - Defender, option 1: `outcome(Defender).expected_pct`.
  - Defender, option 2: that × (A+D)/D.

  Cap values at 100 (a side cannot lose more than it has) and say so in `finding`, which should name the worst multiplier. `question`: "Loss as a share of each side's own strength, by size ratio (column 0 before the double-strength shift)".

- [ ] **Step 4: Run it**, then `cargo run -p cna-probe -- run`. Expected: PASS, and `probes/R-011.json` written.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "R-011 probe"`

### Task 8: Chart-oddities probe

**Files:**
- Create: `crates/cna-probe/src/probes/oddities.rs`; register it with id `chart-oddities` (the board item id in `cna/docs/autopilot/decisions.json`).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_13_18_cell_cannot_be_rolled_past_16_and_the_gap_is_3_in_36() {
    let p = probe(&Ctx { table: CloseAssault::load().unwrap() });
    let s = &p.series[0];
    let i = p.x.values.iter().position(|x| x.contains("13-18")).unwrap();
    let j = p.x.values.iter().position(|x| x.contains("34-36")).unwrap();
    assert_eq!(s.values[i], 0.0); // readings 17 and 18 do not exist
    assert!((s.values[j] - 100.0 * 3.0 / 36.0).abs() < 1e-9);
}
```

- [ ] **Step 2: Run it.** Expected: FAIL.

- [ ] **Step 3: Implement.** `kind: Bar`, one series, "% of rolls that land on the odd cell". For each `known_gaps` entry, the x label is `"<side> <column>: no row for <from>-<to>"` and the value is 100 × (gap readings among the 36) / 36. Add the attacker `-2` 20 % cell by hand, with the label `"attacker -2: 20 % row reads 13-18"` and the value 100 × (readings 17 and 18 that exist, which is 0) / 36. `finding`: say which oddities matter in play (value > 0) and which cannot occur. `question`: "How often does a roll land on a printed oddity?"

- [ ] **Step 4: Run it**, then `cargo run -p cna-probe -- run`. Expected: PASS.

- [ ] **Step 5: Commit.** `git add -A && git commit -m "chart-oddities probe"`

### Task 9: Slice done

- [ ] `cargo clippy --all-targets -- -D warnings`, `cargo test` and `cargo run -p cna-probe -- check` all clean.
- [ ] README: list the probes and how to view them on the board (`python3 tools/decisions_page.py --probes ~/path/to/cna-engine/probes` in `cna`).
- [ ] PR, CI green, merge (`gh pr merge --merge`).
- [ ] `docs/autopilot/PROGRESS.md`: mark slice 1 done. **Next steps** for the next run: write `docs/plans/<date>-slice-2-barrage-anti-armour.md` in the same shape as this plan, then execute it.
