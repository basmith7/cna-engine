# Mission 3, slice 1 (engine in the browser) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Graziani game created and driven by the real engine inside a
browser page: `cna-data` loads from any `Source`, `cna-web` embeds the
data, and `web/` shows the hex grid and the clock from `Game.view()`.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Depends on:** M2 slices 1–4 (merged).

**Proving screenshot:** the theatre's 7 073 hex outlines (no styling yet)
with the top bar reading "Game-Turn 1 · set-up · Axis to act".

## Rules for every task

- Stage only your own files (`git add <paths>`, never `git add -A`).
- Paste the output of each verification command into the PR body; do not
  summarise it as "passes".
- Stop any dev or preview server you start (`npm run dev`, `npm run
  preview`) before finishing; Playwright's `webServer` stops its own.
- On any visual change, capture the slice's proving screenshot and run
  the **screenshot-critique** skill on it before calling the slice done;
  paste its verdict in the PR.

## Tasks

### Task 1: the `Source` seam in `cna-data`

- [ ] Add `cna_data::source` with `trait Source { fn read(&self, rel:
  &str) -> Result<String>; fn list(&self, dir: &str) -> Result<Vec<String>>; }`
  (paths relative to `vendor/cna/data`, `/`-separated) and `Fs` over
  `cna_root()`.
- [ ] Every loader (`Map`, `Scenario` incl. `extends`, `Oa::load_all`,
  `Characteristics`, `Terrain`, `StackingPoints`, `Weather`,
  `InitiativeRatings`, `Schedule`, `OffMapDistances`, the combat tables,
  `errata::load_table`) becomes `load_from(src: &dyn Source)`; `load()`
  calls it with `Fs`. No behaviour change: existing tests pass untouched.
- [ ] `Data::load_from(slug, &dyn Source, rules_commit: String)` in
  `cna-game::data`; `Data::load` calls it.
- [ ] Test: a `MemSource` (a `BTreeMap` filled from `Fs`) gives
  `Data` equal field by field to `Data::load` (derive `PartialEq` where
  missing, or compare serde JSON). Commit `data: load from any Source`.

### Task 2: `cna-web` crate

- [ ] `crates/cna-web` (`cdylib`, `rlib`), deps `cna-data`, `cna-game`,
  `serde_json`, `wasm-bindgen`. `build.rs` walks `vendor/cna/data/{map,
  tables,oa,scenarios,errata}`, writes `$OUT_DIR/embedded.rs` (a sorted
  `&[(&str, &str)]` of `include_str!`s) and `RULES_COMMIT` from `git -C
  vendor/cna rev-parse HEAD`; `rerun-if-changed` on those dirs.
- [ ] `Embedded: Source`; `static DATA: OnceLock<Data>` built on first
  use for `grazianis-offensive`.
- [ ] `api.rs`: plain fns returning `serde_json::Value`: `new_game(seed)`,
  `load(json)` (`GameState::from_json`), `save`, `view`, `apply(side,
  order_json)` → `{"ok": [Event]}` or `{"illegal": {"case", "reason"}}`.
  `view` = the state JSON minus `log` and `rng`, plus `to_act` and
  `unit_type` per piece. `lib.rs`: a `#[wasm_bindgen] Game` whose methods
  take and return JSON strings.
- [ ] Native tests: embedded `Data` equals `Data::load`; new game pending
  `set-up` for `axis`; an illegal `place` returns case `59.2` and leaves
  `save()` unchanged; `load(save())` round-trips byte for byte; a save
  with another `rules_commit` errors. Commit `cna-web: Game API with
  embedded data`.
- [ ] `wasm-bindgen-test` (node): the same new/apply/save/load through
  JS strings; `wasm-pack test --node crates/cna-web`. Record the release
  `.wasm` size raw and gzipped in the PR. Commit `cna-web: wasm tests`.

### Task 3: `web/` scaffold with the grid

- [ ] `npm create vite@latest web -- --template svelte-ts`, trimmed;
  `npm run wasm` = `wasm-pack build ../crates/cna-web --target web
  --out-dir ../../web/src/pkg`; `.gitignore` `node_modules`, `src/pkg`,
  `test-results`, `playwright-report`. Vitest, svelte-check, Playwright
  (Chromium only).
- [ ] `src/engine.ts` (init wasm, typed wrappers), `src/types.ts`
  (hand-written types for `View`, `Order`, `Event`, `Pending`;
  generation via `tsify`/`ts-rs` is a later option if they drift).
- [ ] `src/geom.ts`: port of `vendor/cna/tools/map_geom.py` plus the
  theatre offset (sheet *k* of A–E adds `k·33·w`; M as an inset).
  Vitest: three hexes' corners equal the `points` in
  `vendor/cna/site/public/map/C.svg` minus C's offset to 0.01; seam pair
  C0633–D0701 centres one `w` apart; no two centres coincide.
- [ ] `Map.svelte` draws outlines only, once, with `data-hex`; top bar
  from `view`. Commit `web: grid and clock from the engine`.

### Task 4: smoke e2e and CI

- [ ] Playwright with `npm run build && npm run preview` as `webServer`:
  7 073 `[data-hex]`, top bar text, `window.__cna.view().to_act ===
  "axis"` (`__cna` only when `import.meta.env.MODE === "test"`). Save the
  proving screenshot as an artefact.
- [ ] CI: `cargo check -p cna-game --target wasm32-unknown-unknown`,
  pinned `wasm-pack`, `wasm-pack test --node crates/cna-web`, job `web`
  (Node 22, `npm ci`, `npm run wasm`, `npm run check`, `npm test`,
  `npx playwright install --with-deps chromium`, `npx playwright test`).
  Commit `CI: wasm and web`.
