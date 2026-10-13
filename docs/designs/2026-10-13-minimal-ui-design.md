# cna-engine: a minimal browser UI for Graziani's Offensive

**Date:** 2026-10-13
**Mission:** 3 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** proposed (draft PR). Not marked for Brian's review in the
queue, so it merges once its gates pass, unless Brian says otherwise.

Written by the autopilot under Brian's delegation of 2026-09-26: every
decision below is Claude's, is cheap to reverse, and can be reversed with
one PR. The scope (a minimal UI on Mission 2's game state, local only) is
Brian's, from the *Path to a playable game* agreed on 2026-10-07.

**Built against a moving target.** Mission 2 is being built while this is
written; no `cna-game` code had merged on `origin/main` at `ac71cfc`. The
names below are the M2 spec's and slice plans' (`GameState`, `Order`,
`Event`, `Pending`, `Illegal`, `apply(&mut self, player, order) ->
Result<Vec<Event>, Illegal>`). Slice 1 of this mission starts by checking
them against the merged code and renaming here, in the same PR, wherever
M2 shipped something different.

## Goal

Two people at one computer play **Graziani's Offensive** to its end in a
browser: they see the map and every unit on it, set up their free
placements by clicking hexes, see where a selected stack can move and what
each hex costs in CP, give every order Mission 2 accepts by clicking,
answer the opponent's interrupts (ZOC declarations, reactions, retreats,
sealed plots) when the engine asks, read the log and the victory report,
and save and reload the game as the JSON state.

How this moves **the goal** (two players, a browser, every rule enforced):
it is the first time a person can *play* the engine. Everything that
decides legality stays in Rust and runs in the browser as WebAssembly, so
the UI holds no rules; the later two-player server sends the same `Order`
JSON to the same engine, and this client becomes its front end with the
hot-seat hand-off replaced by a network. Nothing here is throwaway except
the hot-seat hand-off screen.

## Non-goals

- **A server, accounts, play at a distance.** A later mission. Nothing is
  hosted: `npm run dev` / `npm run preview` on localhost only (AUTOPILOT
  *Limits*).
- **Hidden information.** Hot-seat players share a screen. Sealed plots
  are entered behind a "hand over to the other player" screen so neither
  sees the other's choice before both are in, but the state still holds
  everything (as in Mission 2).
- **SPI art of any kind.** No SPI map, no counter scans, no VASSAL images,
  ever. The map is cna's own schematic redraw style generated from
  `data/map/`; counters are drawn from data (NATO symbol, nation colour,
  numbers).
- **Polish.** No animation, sound, themes, mobile layout, undo beyond
  reloading a save, or tutorial. A desktop browser at 1280 px or wider.
- **Other scenarios.** The UI loads whatever `cna-game` can start, but
  only Graziani's Offensive is tested.
- **AI opponents.**

## What "done" means

1. `cd web && npm install && npm run dev` serves the game on localhost; a
   new Graziani's Offensive game opens with the whole theatre map and
   every placed unit as a counter.
2. Each side's free placements are made by clicking a pending unit and
   then a highlighted legal hex; play starts when none are left.
3. Selecting a stack in the movement segment shades every reachable hex
   with its CP cost; clicking one moves the stack along the engine's
   cheapest path; an illegal order shows the engine's case number and
   reason and changes nothing.
4. Every `Pending` decision M2 defines is asked of the right player with
   a dialog listing exactly the legal answers.
5. Barrage, assault and retreat are entered on the map; results show in
   the log as the engine's events.
6. Save writes the state JSON; Load restores it and the game continues
   identically.
7. A Playwright test plays a short scripted game through the UI (set-up,
   a move, an illegal move rejected, a reaction, an assault, save, load)
   in headless Chromium in CI.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| How the browser gets the engine | `cna-game` compiled to **WebAssembly** through a new thin crate **`cna-web`** using `wasm-bindgen`, built with **`wasm-pack build --target web`**; no server process | The M2 spec already requires `cna-game` to build for wasm, local-only play needs no server, and the same Rust validates every order with zero duplication in TypeScript |
| Engine boundary | `cna-web` exposes one `Game` class with JSON-in/JSON-out methods (`new_game`, `load`, `save`, `view`, `apply`, `reachable`, `placements`, `answers`, `geometry`) using `serde-wasm-bindgen`; orders are M2's `Order` serde JSON verbatim | A narrow string/JSON seam is trivially testable from both sides and is exactly the shape a network server will carry later |
| Static data in the browser | `cna-web` embeds the JSON files Graziani needs from `vendor/cna/data` with `include_str!` at build time (map, scenario, OA, tables, roughly 3 MB before gzip) and parses them through `cna-data`'s from-string loaders | One artefact, no fetch logic, and the `rules_commit` in a save always matches the data compiled in |
| Data loaders | `cna-data` loaders gain `from_str` / `from_value` entry points beside the file-path ones (if M2 did not already add them) | wasm has no file system; parsing must not care where the bytes came from |
| Front-end stack | **Vite + TypeScript + Svelte 5**, no component library, no CSS framework; `vite-plugin-wasm` not needed (wasm-pack's `--target web` output is an ES module) | Vite and TypeScript are the standard toolchain; Svelte compiles to small plain JS and its reactive markup suits an SVG of 7 000 hexes better than hand-rolled DOM updates, with less runtime than React |
| Map rendering | One inline **SVG** of the theatre, drawn in the browser from geometry `cna-web` returns: the hex polygons, hexside segments, settlements and labels follow `vendor/cna/tools/map_render.py` (`map_geom.py` pointy-top geometry, size 20, its `PALETTE` and `STROKES` classes copied as CSS) | cna's renderer is the project's agreed look and its CC0 schematic style; drawing it live lets each `<polygon id="C3922">` be a click target and lets overlays share coordinates |
| Theatre layout | Sheets A–E side by side west to east, sheet *k* offset by 33 columns, same rows (`seams.json` note: column 33 meets the next sheet's column 01; E's column 00 is not a hex); Malta as a small inset | Matches cna's seam data, so cross-sheet neighbours sit next to each other on screen |
| Geometry source | A Rust port of `map_geom.py` (`centre`, `corners`, sheet offset) in `cna-game::geom`, tested against the polygon points in cna's committed `site/public/map/*.svg` | One geometry for engine and UI, pinned to cna's own output so the two cannot drift |
| Pan and zoom | SVG `viewBox` pan/zoom with mouse drag and wheel, a "go to hex" box, start view fitted to the scenario's hexes (sheets B–E) | 7 000 polygons render fine; no tiling or canvas needed at this size |
| Counters | SVG `<symbol>`s drawn from data: nation colour band (Italian, British, Indian, Australian, …), a NATO frame and branch symbol chosen from the characteristics row's `unit_type` via a small mapping table in the web app, size marks from `equivalent`, the `id_code` / short name, and TOE points and CPA as numbers; stacks show the top counter plus a count badge, expanded on click | Counters are ours and data-derived, never SPI art, and a mapping table is one obvious place to extend |
| Selection and moves | Click a hex: a stack panel lists its pieces with checkboxes (all own pieces selected by default). With a selection in the movement segment, `reachable(pieces)` returns every hex the stack can end in this order with its cheapest CP cost, the path, and flags (ends in enemy ZOC, would exceed CPA → DP, would exceed 150 %); hexes are shaded and labelled with the CP. Clicking one sends `Move { pieces, path }` with the returned path; shift-click builds a custom path hex by hex, re-querying from its end | The engine computes the moves; the UI never prices a step itself |
| Legal-move query | New in `cna-game` (Mission 2 provides only `apply`): `GameState::reachable(&self, data, &[UnitId]) -> Vec<Reach>`, a Dijkstra over the same step-cost and legality checks `apply` uses, stopping where movement must stop (entering enemy ZOC, the 150 % cap); each `Reach` is re-validated by a test that `apply` accepts its path | Showing a move the engine would reject, or hiding one it would accept, is the worst bug a rules UI can have; one function used by both closes it |
| Other legal-answer queries | Also new: `placements(unit) -> Vec<HexId>` for a pending set-up piece, and `answers(&self) -> Vec<Order>` listing the legal answers to the current `Pending` (ZOC yes/no, reacting units and hexes, retreat hexes, barrage targets) where the set is finite | The UI offers only what the engine accepts; anything outside these lists is still checked by `apply` |
| Whose turn | A top bar shows the clock (Game-Turn, OpStage, phase, half, segment), weather, initiative, and **who must act**: `pending`'s player if set, else the phasing player. Every order is sent as that player | Hot-seat means the engine, not the UI, decides who is at the keyboard |
| Hot-seat hand-off | When the acting player changes, a full-screen "Commonwealth to play — click to continue" curtain hides the board; for sealed plots (force assignment) the first player's entry is hidden before the second plays | Minimal fairness at one screen, and the curtain is the only part the network later removes |
| Pending decisions | A modal per `Pending` variant (set-up, ZOC declaration, reaction offer, retreat before assault, sealed force assignment, initiative declaration, reserve designation) built from `answers()`; map clicks fill hex or unit choices inside it | Each SPI interrupt becomes one explicit question with only legal choices |
| Combat entry | In the combat segment, select attacking pieces, then click a target hex: a menu offers Barrage or Assault per M2's orders; force assignment is the sealed modal; results are shown from `Event`s (rolls, losses, retreats, cohesion) in the log and as counter changes | Combat stays the M2 order set; the UI only collects arguments |
| Log | A side panel lists events in plain English from a TypeScript formatter over `Event`'s serde JSON, grouped by clock, with case numbers where the event has one; illegal attempts appear in red but are not part of the game log | Readable history without the engine owning prose |
| Save and load | Save downloads `graziani-<clock>.json` (the `GameState` JSON) and keeps an autosave in `localStorage` after every accepted order; Load accepts a file or the autosave; `load` rejects a state from another `rules_commit` with M2's error | The state already is the save format; nothing new to version |
| Victory | When the state is `Over`, a dialog shows M2's victory report (each side's level and the conditions checked) | The game must visibly end |
| Repo layout | `crates/cna-web` (Rust) and `web/` (the Vite app, its `package.json`, `tests/`, Playwright config); `web/src/pkg` is the wasm-pack output, git-ignored, rebuilt by `npm run wasm` | Keeps the Node toolchain out of the Cargo workspace root; standard layout for wasm-pack + Vite |
| Tooling pins | Node 22 LTS, npm (lockfile committed), `wasm32-unknown-unknown` target and `wasm-pack` installed in CI; locally `rustup target add wasm32-unknown-unknown` and `cargo install wasm-pack` (asked of Brian before installing on his machine) | Standard tools, reproducible CI |

## Engine API Mission 2 does not provide

Built first, in slice 1, all pure Rust in `cna-game` and tested natively:

1. `reachable` (above), sharing the step-cost and refusal code with
   `apply`'s `Move` validation.
2. `placements(unit)` for free set-up pieces.
3. `answers()` for the current `Pending`.
4. `acting_player()` (the `pending` player or the phasing player).
5. `geom`: hex centre, corners, hexside endpoints in theatre coordinates.
6. `cna-data` from-string loaders, if M2 has only path loaders.
7. Confirmation that `cna-game` builds for `wasm32-unknown-unknown`
   (no `std::fs`, no `getrandom` without its `js` feature, no threads).

If M2 ends up providing any of these, slice 1 uses M2's instead and drops
the item.

## Crates and directories touched

- `cna-data`: from-string loaders (if missing).
- `cna-game`: `query` (`reachable`, `placements`, `answers`,
  `acting_player`), `geom`.
- `cna-web` (new, `cdylib`): the `Game` wasm-bindgen class, embedded data.
- `web/` (new): Vite + Svelte + TypeScript app, Vitest unit tests,
  Playwright e2e.
- `.github/workflows/ci.yml`: a `web` job (wasm-pack build, `npm ci`,
  `npm run check`, `npm test`, Playwright).
- README, `NOT_SIMULATED.md` (unchanged rules coverage; a line that the UI
  adds none), this Status, the queue.

## Testing

- **Rust, natively:** for `reachable`, hand-built states on the real map
  (the coast road from Sidi Barrani, Halfaya by track, a stack whose
  lowest CPA limits it, ZOC entry stopping it, the 150 % cap) with pinned
  costs; a property test that every returned path is accepted by `apply`
  at exactly the returned cost, and that random paths `apply` accepts end
  in a returned hex at no lower cost. `geom` pinned against points parsed
  from cna's `site/public/map/C.svg`.
- **wasm:** `wasm-pack test --node` on `cna-web`: new game, view, apply a
  legal and an illegal order (JSON `{case, reason}`), save/load round
  trip equals byte for byte.
- **TypeScript (Vitest):** event formatter, counter symbol mapping (every
  `unit_type` in Graziani has a symbol), hex hit-testing from geometry,
  the store that tracks the acting player and the curtain.
- **End to end (Playwright, headless Chromium):** a scripted short game
  through the UI with `data-hex` / `data-unit` attributes as selectors,
  asserting on the engine's state JSON exposed on `window.__cna` in test
  builds; plus a screenshot of the start position checked for non-empty
  map and counters (not pixel-pinned).
- Gates: Rust gates as before, plus `npm run check` (svelte-check, tsc),
  `npm test`, `npx playwright test` in CI.

## Slices

One PR each, in order; each has a plan in `docs/plans/`. Slice 1 needs M2
slice 4 (movement) merged; slices 2–3 need only M2 slice 3 (state) and can
start earlier if the queue reaches them first.

1. **Engine queries** (`2026-10-13-m3-slice-1-queries.md`): `reachable`,
   `placements`, `answers`, `acting_player`, `geom`, from-string loaders,
   wasm build check in CI.
2. **wasm facade** (`…-slice-2-wasm.md`): `cna-web`, embedded data, the
   `Game` class, `wasm-pack test`.
3. **Map and counters** (`…-slice-3-map.md`): the `web/` app, theatre SVG
   in cna's style, pan/zoom, counters and stacks, top bar, Playwright
   smoke test.
4. **Set-up and movement** (`…-slice-4-orders.md`): placement, selection,
   reachable overlay, moves, end segment, pass, initiative, reserves,
   illegal-order display, hot-seat curtain, log panel.
5. **Interrupts and combat** (`…-slice-5-combat.md`): every `Pending`
   modal, barrage, assault, sealed force assignment, retreats, victory
   dialog.
6. **Save, load and the e2e game** (`…-slice-6-e2e.md`): save, load,
   autosave, the scripted Playwright game, CI job, docs, `MISSION 3
   COMPLETE`.

## Requests for cna this mission makes

None expected. If counters need a field cna lacks (for example a short
display name per OA unit), slice 3 falls back to `id_code` and posts a
request rather than inventing data.
