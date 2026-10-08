# Mission 2, slice 2 (scenario and pieces) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load Graziani's Offensive with its OA sheets, unit
characteristics and reinforcement schedule, and expand its set-up into
*pieces* (one per OA unit), each on a hex, in a box, or pending a free
placement.

**Architecture:** Loaders in `cna-data` (`oa`, `characteristics`,
`schedule`, `scenario`); the expansion of deployments into pieces in a new
`cna-game::setup` module (the crate is created here, holding only `setup`
and `piece` until slice 3).

**Spec:** `docs/designs/2026-10-08-game-state-design.md`
**Previous plans:** slice 1 (map); Global Constraints of Mission 1 slice 1.

## Domain facts the implementer needs

- **Scenario** `vendor/cna/data/scenarios/grazianis-offensive.json`, schema
  `data/schema/scenario.schema.json`. Keys: `id`, `name`, `group`,
  `sources`, `notes`, `start`/`end` (`{game_turn, opstage}`: 1/1 to 6/3),
  `initiative` (`{side: "axis", through: {1, 3}}`), `construction`,
  `victory`, `sides.{axis,cw}`, `abstractions`. A child scenario has
  `extends: "scenario:<slug>"` and takes every top-level block it does not
  give (`italian-campaign.json`).
- **Deployment** `{placement, units[], trucks?, notes?}`. A unit entry has
  exactly one of `unit` (`unit:it:maletti-div-hq`: the counter plus what
  its OA sheet assigns to it) or `formation` (`it:bardia-garrison-hex-c4321`:
  every counter on that OA sheet), and optional indicators: `less`,
  `detached` (removed from the entry), `attached` (added to it), `assigned`,
  `consists_of` (the entry is exactly these), `alone: true` (the HQ only),
  `status` (`in-training` | `broken-down`), `toe` (an override).
  Axis: 26 deployments, 47 entries; CW: 16 deployments, 32 entries.
- **Placement** has one locator: `hexes[]`, `place`, `box` (`tripoli`,
  `tripolitania`, …), `within {of, hexes}`, `region` (`libya` | `egypt`),
  `sheets[]`, `air_facility`; optional `not_within {of: "enemy-unit",
  hexes}` and `constraint` (English; logged as unchecked). A list of
  `hexes` with more than one hex is a free choice among them.
- **Land Game only** (60.92): apply `abstractions.air_and_logistics`:
  each side block in `replaces` replaces that side's block of the same name
  (only `supply` here); `adds` appends. Supply units become pieces of kind
  `SupplyUnit` (inert in Mission 2). Ignore `air`, `fleet`, `trucks`,
  `repair`.
- **OA** `data/oa/{it,cw,de}.json`: `formations[]` with `id`, `name`,
  `basic_morale`, `attached[]`, `units[]`. Unit: `id`, `name`,
  `abbreviation`, `id_code` (or null), `parent`, `toe [{weapon, points}]`,
  `arrives` (`gameTime` | `"deployed"` | null), `toe_mark` (`N` means the
  ID code's `max_toe`), `morale` (overrides the formation's).
- **Characteristics** `data/tables/unit-characteristics.json`, keyed by
  `(nation, id_code)`: `unit_type`, `cpa`, `anti_air`, `barrage`,
  `anti_armour`, `vulnerability`, `armour_protection`,
  `close_assault_offence`, `close_assault_defence`, `max_toe`. Cells are
  `int | string | null`; strings carry printed marks (`"10+"` motorisable,
  `"30*"`, `"(1)"`). Parse into `Rating { value: Option<i32>, mark: Option<Mark> }`
  and keep the raw string.
- **Schedule** `data/tables/reinforcement-schedule.json`: rows `{side,
  game_turn, opstage, kind: arrives|withdraws, units[], trucks?}`. In
  Game-Turns 1–6 all rows are Commonwealth (Polish HQ 1/3; 2 RTR, 7 RTR,
  6 NZ Fld 2/3; 1 Fld 3/3; 3 Hus 4/3; 7 Ind HQ 5/1; 6 NZ HQ 6/3).
- **R-096:** a unit the set-up places starts on the map even if its OA
  gives a later arrival, and does not arrive again.

## Tasks

### Task 1: OA and characteristics loaders

**Files:** `crates/cna-data/src/oa.rs`, `characteristics.rs`.

**Interfaces:** `Oa::load_all() -> Result<Oa>`; `Oa::unit(&UnitId)`,
`Oa::children(&UnitId)`, `Oa::formation(&str)`, `Oa::morale(&UnitId)`;
`Characteristics::load()`, `Characteristics::row(nation, id_code)`;
`Rating`, `Mark { Motorisable, KeepsCpa, Parenthesised, Other(String) }`.

- [ ] **Step 1: Write the failing tests:** counts (it 436, cw 436, de 148
  units); every `parent` resolves; every non-null `id_code` has a
  characteristics row; `"10+"` parses to 10 with `Motorisable`; a unit
  with a per-unit `morale` overrides its formation's.
- [ ] **Step 2–5:** run (FAIL), implement, run (PASS), commit
  `OA sheets and unit characteristics`.

### Task 2: scenario and schedule loaders

**Files:** `crates/cna-data/src/scenario.rs`, `schedule.rs`.

**Interfaces:** `Scenario::load(slug) -> Result<Scenario>` (follows
`extends`); `Scenario::land_only(&self) -> Scenario` (applies
`abstractions.air_and_logistics`); `Placement` enum with the variants
above; `UnitEntry`; `Schedule::load()`, `Schedule::arrivals(side, turn, stage)`.

- [ ] **Step 1: Write the failing tests:** Graziani loads with 26 + 16
  deployments; the Italian Campaign loads through `extends` and keeps
  Graziani's sides; `land_only` swaps the Axis `supply` for the 10
  supply-unit blocks and adds the Commonwealth's 6; every `Placement`
  variant used in the file deserialises; the schedule has the eight GT 1–6
  rows above.
- [ ] **Step 2–5:** as Task 1; commit `Scenario and schedule loaders`.

### Task 3: deployments to pieces

**Files:** create `crates/cna-game/` (`Cargo.toml`, `src/lib.rs`,
`src/piece.rs`, `src/setup.rs`); add to the workspace.

**Interfaces:**
- `Piece { id: UnitId, side: Side, nation, id_code, parent, kind: PieceKind,
  toe: u32, cpa: Rating, morale: i32, status, location: Location }`;
  `PieceKind { Combat, Hq, SupplyUnit }`.
- `Location { Hex(HexId), Box(OffMapBox), Pending(PendingPlacement), Eliminated }`.
- `setup::expand(&Scenario, &Oa, &Characteristics, &Map) -> Result<Vec<Piece>>`.

- [ ] **Step 1: Write the failing tests:** the Axis and Commonwealth piece
  counts are pinned (compute once, check by hand against two formations,
  record in the test with the arithmetic in a comment); `unit:cw:6-aus-hq`
  is `in-training` and its `less` units are absent; a `detached` unit is
  not under its parent's entry; an `attached` unit is; every piece with a
  single hex or place is on it; every `region`/`within`/`sheets` piece is
  `Pending`; no unit is placed twice (R-094) and none appears in both the
  set-up and the GT 1–6 schedule (R-096).
- [ ] **Step 2–5:** as Task 1; commit `Set-up expands to pieces`.
- [ ] **Step 6:** if any Graziani `unit_type` does not map to a 9.4
  equivalent, add a bullet to **Requests for cna** on `main` and list the
  units in the PR.
