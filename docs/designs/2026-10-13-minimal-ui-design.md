# cna-engine: a minimal browser UI for Graziani's Offensive

**Date:** 2026-10-13 (revised 2026-10-12 against merged Mission 2 code)
**Mission:** 3 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** proposed (draft PR). Not marked for Brian's review in the
queue, so it merges once its gates pass, unless Brian says otherwise.

Written by the autopilot under Brian's delegation of 2026-09-26: every
decision below is Claude's, is cheap to reverse, and can be reversed with
one PR. The scope (a minimal UI on Mission 2's game state, local only) is
Brian's, from the *Path to a playable game* agreed on 2026-10-07.

**Checked against the code.** Names, types and paths below are those on
`origin/main` at `4d419d1` (M2 slices 1–4 and PR #10, `Place.hexes`).
Combat names (`Pending::Sealed`, `Order::Barrage`, …) are from the
unmerged M2 slice 5 branch `autopilot/m2-slice-5-combat` at `08dbf3d` and
are marked *(slice 5 branch)*; victory names are from the M2 slice 6
*plan* and marked *(slice 6 plan)*. M3 slice 5 re-checks both when they
merge and renames here in its first commit.

## Goal

Two people at one computer play **Graziani's Offensive** to its end in a
browser: they see the map and every unit on it, set up their free
placements by clicking hexes, see where a selected stack can move and what
it costs in CP, give every order Mission 2 accepts by clicking, answer the
engine's interrupts (ZOC declarations, reactions, sealed combat steps,
retreats) when it asks, read the log and the end of the game, and save and
reload the game as the JSON state.

How this moves **the goal** (two players, a browser, every rule enforced):
it is the first time a person can *play* the engine. Everything that
decides legality stays in Rust and runs in the browser as WebAssembly, so
the UI holds no rules; the later two-player server sends the same `Order`
JSON to the same engine, and this client becomes its front end with the
hot-seat hand-off replaced by a network. Nothing here is throwaway except
the hot-seat curtain.

## Non-goals

- **A server, accounts, play at a distance.** A later mission. Nothing is
  hosted: `npm run dev` / `npm run preview` on localhost only.
- **Real hidden information.** Hot-seat players share a screen and the
  state JSON holds everything (as in Mission 2). The UI only avoids
  *showing* the other side's sealed answers (see *Secrecy*).
- **SPI art of any kind.** No SPI map, no counter scans, no VASSAL images,
  ever. The map is cna's own schematic style drawn from `data/map/`;
  counters are drawn from data.
- **Polish.** No animation, sound, themes, mobile layout, undo beyond
  reloading a save, tutorial, or accessibility beyond keyboard Escape and
  readable contrast. A desktop browser at 1280 px or wider, Chromium-tested.
- **Other scenarios.** Only Graziani's Offensive is embedded and tested.
- **AI opponents.**

## What "done" means

1. `cd web && npm ci && npm run wasm && npm run dev` serves the game on
   localhost; a new game opens with the theatre map and every placed unit
   as a counter, in under 3 s on Brian's desktop (measured, slice 2).
2. Free placements are made by clicking a piece in the "to place" tray and
   then a highlighted hex; play starts when none are left.
3. Selecting a stack in a movement segment shades every hex it can end in
   with its CP cost; clicking one sends `Move` along the engine's cheapest
   path; an illegal order shows the engine's case and reason and changes
   nothing.
4. Every `Pending` variant the merged engine has is answered through a
   panel offering only what the engine computed (or, where the answer set
   is not finite, checked by `apply`).
5. Barrage, force assignment, assault and retreats are entered by clicking;
   results show in the log from the engine's `Event`s.
6. Save writes the state JSON; Load restores it and play continues
   identically (byte-equal state after the same further orders).
7. Playwright (headless Chromium, CI) plays M2's golden game through UI
   clicks and ends with a `save()` byte-equal to `cna-play replay` of the
   same log.

## The engine as it is (what M3 builds on)

- `cna_game::data::Data` (pub fields: `rules_commit`, `scenario`, `oa`,
  `chars`, `map`, `terrain`, `stacking`, `weather`, `initiative`,
  `schedule`, `off_map`); `Data::load(slug)` is native-only.
- `GameState::new(&Data, seed) -> anyhow::Result<GameState>`;
  `GameState::apply(&mut self, &Data, Side, Order) -> Result<Vec<Event>,
  Illegal>`; `Illegal { case: &'static str, reason: String }`;
  `GameState::to_act() -> Option<Side>` (the pending side, else the
  phasing/initiative player; `None` when over). This is the "acting
  player" the old spec planned to add: **already provided**.
- Save/load: the state's serde JSON. `GameState::from_json(json, &Data)`
  checks scenario and `rules_commit`; `GameState::to_log()` / `Log` /
  `log::replay(&Data, &Log)` give the order log and replay: **already
  provided**.
- `Side` is `axis` / `cw` in JSON; pieces are `PieceId` (strings, not
  `UnitId`), `Piece` already carries `nation`, `id_code`, `kind`
  (`combat`/`hq`/`supply-unit`/`dummy-supply-unit`), `toe`, `cpa`,
  `morale`, `equivalent`, `cohesion`, `cp_spent`, `reserve`, `location`
  (`hex` / `box` / `transit` / `pending` / `eliminated`). The view needs
  only `unit_type` added (from `data.chars.row(nation, id_code)`).
- `Order` (kebab-case, externally tagged): `place`, `declare-initiative`,
  `move {pieces, path, assault}`, `react`, `declare-zoc {exerting}`,
  `move-off-map {pieces, to}`, `designate-reserve`, `release-reserve`,
  `end-segment`, `pass`; *(slice 5 branch)* `gun-positions {forward}`,
  `barrage {fires: [Fire]}`, `retreat-before-assault {pieces, path}`,
  `assign-forces {assignments: [Assignment]}`, `assault {target}`,
  `retreat {path}`.
- `Pending`: `set-up {side}`, `declare-zoc {side, hexes, at_start}`,
  `reaction {side, eligible}`; *(slice 5 branch)* `sealed {step:
  gun-positions|barrage|assign-forces, waiting}`,
  `retreat-before-assault {side}`, `forced-retreat {side, pieces, hexes,
  from, reason, raw}`. Initiative and reserves are **not** pendings: they
  are ordinary orders with `pending: None`.
- End of game: today `GameState.over: bool` and `Event::GameOver`; the
  victory report is *(slice 6 plan)* `VictoryReport { axis, cw, checked,
  unchecked }`.
- Data loading: every `cna-data` loader reads `vendor/cna` with
  `std::fs` via `cna_root()` (which `canonicalize().expect`s a path from
  `CARGO_MANIFEST_DIR`), `errata::load_table` lists `data/errata/`, and
  `rules_commit()` shells out to git. None of this works in a browser.
- Movement: `GameState::plan` (crate-private) prices a path step by step
  via `cna_rules::movement::step_cost`, reports `ends_in_zoc`, and an
  `interrupt` where an *undeclared* enemy ZOC forces a `DeclareZoc` pause
  mid-move (10.16, R-003). There is no legal-move query.
- Set-up: `state::test_support::candidates` (test-only) lists a pending
  piece's placement area; stacking and `not_within` are checked only by
  `apply`. There is no public placement query.

### What M3 adds to the engine, and what it no longer needs

| Item | Status |
|---|---|
| `acting_player()` | **Dropped**: `to_act()` exists. |
| Save/load/replay | **Dropped**: `from_json`, `to_log`, `replay` exist. |
| `answers()` for every pending | **Dropped** as a general API. Finite choices come from the pending itself (`declare-zoc.hexes`, `reaction.eligible`, `forced-retreat.from`) or from the two queries below; everything else is offered as a button and judged by `apply`. |
| `geom` in `cna-game` | **Dropped**: geometry is a rendering concern; ported to TypeScript in `web/` (see Decisions). The engine stays rules-only. |
| Data from strings (`Source` seam in `cna-data`) | **Needed** (slice 1). |
| `placements(piece)` | **Needed** (slice 3): `candidates` promoted out of `test_support`, filtered by a dry-run `apply` on a clone. |
| `reachable(side, pieces)` | **Needed** (slice 4): a Dijkstra over `plan`'s step pricing. |
| Sealed-step options (who may fire at whom, who may be assigned) | **Needed** (slice 5) if M2 slice 5 does not expose `combat::plots` checks publicly. |
| A redacted `view` | **Needed** (slice 1, extended in slice 5). |

## Decisions

| Topic | Decision | Why |
|---|---|---|
| How the browser gets the engine | `cna-game` compiled to **WebAssembly** through a new thin crate **`cna-web`** (`cdylib` + `rlib`) using `wasm-bindgen`, built with **`wasm-pack build --target web`**; no server process | Local play needs no server; the same Rust validates every order with zero duplication in TypeScript |
| Engine boundary | `cna-web` exposes one `Game` class with string-JSON methods: `new_game(seed)`, `load(json)`, `save()`, `view()`, `apply(side, order_json)`, `placements(piece)`, `reachable(side, pieces_json)`, and later `sealed_options()`. Orders are M2's `Order` serde JSON verbatim. Every method is a plain Rust fn returning `serde_json::Value`, natively tested; the `#[wasm_bindgen]` layer only converts | A narrow JSON seam is testable from both sides and is the shape a server will carry. Plain JSON strings (not `serde-wasm-bindgen`) so the native tests and the browser see byte-identical output |
| Data loading | **`cna-data` gets a `Source` seam**: `trait Source { fn read(&self, rel: &str) -> Result<String>; fn list(&self, dir: &str) -> Result<Vec<String>>; }`, an `Fs` impl over `cna_root()`, and every loader becomes `load_from(&dyn Source)` with `load()` calling it with `Fs`. `Data::load_from(slug, &dyn Source, rules_commit)` beside `Data::load`. One seam instead of a `from_str` per type, and it covers the scenario `extends` chain and the errata directory listing, which per-type `from_str` would not | Matches the existing doc comment on `Data` ("another host fills the same struct by other means") with the least code; keeps errata application identical on both hosts |
| Embedded data | `cna-web`'s `build.rs` walks the `vendor/cna/data` files Graziani needs (`map/`, `tables/`, `oa/`, `scenarios/`, `errata/`), generates an `Embedded` `Source` of `include_str!`s, and records `git -C vendor/cna rev-parse HEAD` as `RULES_COMMIT`. Raw size about 7.4 MB (6.2 MB of it the map), compressing well | One artefact, no fetch logic, and a save's `rules_commit` always matches the data compiled in. The generated list cannot drift from the files |
| Map drawing data | The web app imports the four map files it draws (`sheets`, `hexes`, `hexsides`, `places`) straight from `vendor/cna/data/map` as Vite JSON imports. They are also inside the wasm: a known, accepted duplicate of a few MB on localhost | Drawing is not rules; keeps the wasm API free of a geometry export |
| Geometry | A TypeScript port of `vendor/cna/tools/map_geom.py` (pointy-top, size 20, its row shift from `sheets.json` `odd_rows_shift`), sheets A–E side by side with sheet *k* offset by 33 columns (`seams.json`: column 33 meets the next sheet's column 01; E's column 00 is not a hex), Malta (M) as an inset. Pinned by a Vitest against polygon points parsed from cna's committed `site/public/map/C.svg` and a seam pair's centres one hex width apart | cna's renderer is the agreed look; pinning to its output stops drift without putting a UI concern in the engine |
| Front-end stack | **Vite + TypeScript + Svelte 5**, no component library or CSS framework | Standard toolchain; Svelte compiles to small plain JS |
| Map rendering and performance | One inline SVG. The **base layer** (7 073 hex polygons, hexsides, settlements, labels) is rendered **once** from the static JSON, outside any reactive state, with a single delegated click handler on the `<svg>` reading `data-hex`. Overlays (reachable shading, placement highlights, selection, counters) are separate `<g>` layers that hold only the hexes they mark. Budget, measured in slice 2 by Playwright: first paint of map + counters under 3 s, a pan or zoom step under 100 ms on Brian's desktop. Fallback if missed, in order: hide hexsides and labels below a zoom level, `shape-rendering: optimizeSpeed`, then a `<canvas>` base layer with SVG overlays | 7k polygons are fine in SVG *if* nothing re-renders them on each state change; that, not the count, is the real risk |
| Pan and zoom | `viewBox` pan (drag) and zoom (wheel, about the cursor), a "go to hex" box, start view fitted to sheets B–E | Simplest thing that works at this size |
| Counters | SVG drawn from `view()`: nation colour band, a NATO frame with a branch glyph chosen from `unit_type` by a small table in `web/src/counters.ts`, size mark from `equivalent`, `id_code`, TOE and CPA. Supply and dummy supply have their own glyph (they are not combat units). Stacks show the top counter and a count badge; the stack panel lists all | Ours and data-derived; one obvious table to extend |
| Off-map pieces | A side strip of labelled boxes (`OffMapBox`: Tripoli, Tunisia, Tripolitania, Malta, Italy-Sicily, Crete), a "in transit" list (`Location::Transit`), and a per-side "to place" tray (`Location::Pending`) | They exist in the state and must be visible and selectable (`move-off-map`) |
| State sync | The engine state is the **only** game state. After every accepted `apply` the UI replaces its whole `view` from `Game.view()`; it never patches pieces locally. UI-only state is limited to selection, hover, the open panel and the curtain. In test builds `window.__cna` exposes `view()` and `save()`; every e2e step asserts the rendered `data-unit` positions equal the view's | Divergence between a UI copy and the engine is the classic bug here; not having a copy removes it |
| Legal moves | `reachable(side, pieces) -> Vec<Reach { hex, cp, path, dp, ends_in_zoc, may_be_interrupted }>`: Dijkstra over the same step pricing `apply`'s `Move` uses (refactor `plan` so both call one per-step function); a hex whose step ends in a known enemy ZOC is returned but not expanded; a hex past an *undeclared* ZOC is returned flagged `may_be_interrupted` (the move may stop early for a `declare-zoc`). Proptest: every returned path is accepted by `apply` on a clone at the returned cost, and every random accepted path ends in a returned hex at no lower cost | Showing a move the engine would reject, or hiding one it would accept, is the worst bug a rules UI can have |
| Placements | `placements(piece) -> Vec<HexId>`: `candidates` (promoted from `test_support`, including its two hand-honoured `constraint` filters) filtered by a dry-run `Place` on a clone | Exactly the hexes `apply` accepts, by construction |
| Moving | Click a hex: the stack panel lists its pieces, own pieces ticked. In a movement segment the reachable overlay shows; clicking a hex sends `move` with that path. An "assault" toggle sets `move.assault`. Shift-click builds a custom path hex by hex; Enter sends it, `apply` alone judges it | The engine prices moves; the UI never prices a step |
| Interrupts | One side panel (not a blocking modal: the map stays clickable) per `Pending` variant, switched exhaustively in TypeScript (`never` check, so a new variant fails `tsc`): `declare-zoc` ticks per hex; `reaction` picks units from `eligible` then a hex from `reachable`, or Pass; combat panels in slice 5 | Each interrupt becomes one explicit question built from the pending's own data |
| Phase controls | Buttons for `end-segment`, `pass`, `declare-initiative` (A/B), `designate-reserve` / `release-reserve` (with a selection), shown by clock phase. They are not pre-filtered: an illegal click is an `Illegal` toast | Pre-filtering would be a second copy of the rules |
| Whose turn | A top bar shows clock (`Game-Turn`, OpStage, phase, half, segment), weather, initiative, Player A and **who must act** (`to_act()`). Every order is sent as `to_act()`; the UI never chooses a side | The engine, not the UI, decides who is at the keyboard |
| Secrecy (hot-seat) | A full-screen curtain ("Commonwealth to play — click to continue") whenever `to_act()` changes. `view()` **redacts** sealed answers not belonging to the side to act: `combat.plots`, `combat.assignments`, and gun positions while `pending` is `sealed` *(slice 5 branch)*. The UI never renders `save()` JSON. Accepted limits, stated in the README: the autosave in `localStorage`, a downloaded save and `window.__cna` (test builds only) hold everything | Fair enough at one screen; real secrecy needs the server, which redacts the same way |
| Log | A side panel of one English line per `Event` (TS formatter, exhaustive `switch` with `never`), grouped under clock headers; illegal attempts in red, not part of the game log | Readable history without the engine owning prose |
| Save and load | Save downloads `graziani-gt<N>-os<M>.json` (`save()`); Load accepts a file, errors from `GameState::from_json` shown and the current game kept; autosave to `localStorage` after each accepted order, offered as "Continue" on start | The state already is the save format |
| End of game | When `over`, a dialog shows the result: M2 slice 6's victory report if it is in the state or derivable from the engine, else "Game over" and the final clock | The game must visibly end, whatever slice 6 ships |
| Repo layout | `crates/cna-web` (Rust) and `web/` (Vite app, `package.json`, Vitest, Playwright); `web/src/pkg` is wasm-pack output, git-ignored, rebuilt by `npm run wasm` | Keeps Node out of the Cargo workspace root |
| Tooling | Node 22 LTS, npm with a committed lockfile; CI installs the `wasm32-unknown-unknown` target and a pinned `wasm-pack`. Both are already installed on Brian's machine (checked 2026-10-12) | Reproducible CI, nothing new to install locally |

## WebAssembly: what is proven and what is not

`cargo check -p cna-game --target wasm32-unknown-unknown` **passes** on
`4d419d1` (2026-10-12). That proves less than it looks: `std::fs` and
`std::process` compile for that target and fail at run time, and
`cna_data::cna_root()` would panic. The check therefore stays in CI (it
catches threads, `getrandom` and native-only crates; the engine's RNG is
`rand_chacha` seeded from the state, so no `getrandom` today), but the
real proof is slice 1's `wasm-pack test --node` that builds a Graziani
`GameState` from the embedded `Source` and applies an order.

## Testing

- **Rust, natively:** `Source` parity (every `Data` field equal loaded
  via `Fs` and via `Embedded`); `placements` and `reachable` as above,
  including hand-pinned cases on the real map (coast road from Sidi
  Barrani, Halfaya only by the track, a mixed stack paying the dearer
  cost, a ZOC stop, the 150 % foot cap, a vehicle going into DP); `view`
  redaction; the `Game` JSON methods (illegal order returns
  `{"illegal":{"case","reason"}}` and leaves `save()` unchanged;
  `load(save())` round-trips byte for byte).
- **wasm:** `wasm-pack test --node crates/cna-web`: new game, apply a
  legal and an illegal order, save/load round trip.
- **TypeScript (Vitest):** geometry against `C.svg`, event formatter (one
  case per variant), counter table (every `unit_type` in Graziani has a
  glyph), pending-panel exhaustiveness (compile-time), the curtain store.
- **Playwright (headless Chromium):** per slice, below, with `data-hex` /
  `data-unit` selectors and `window.__cna` assertions; slice 6 plays the
  golden game.
- **Screenshots:** each UI slice names the screenshot that proves it,
  saved as a CI artefact (not pixel-compared). The implementer runs the
  **screenshot-critique** skill on every visual change before calling a
  slice done, and pastes the verdict into the PR.
- Gates: the Rust gates as before, plus `cargo check -p cna-game --target
  wasm32-unknown-unknown`, `wasm-pack test --node crates/cna-web`, `npm
  run check` (svelte-check, tsc), `npm test`, `npx playwright test`.

## Slices

One PR each, in order; each has a plan in `docs/plans/`. Every slice ends
with something visible in a browser, except where noted. Slices 1–4 need
only merged M2 code; slice 5 needs M2 slice 5 merged; slice 6 needs M2
slice 6's golden game.

1. **Engine in the browser** (`2026-10-13-m3-slice-1-engine.md`): `cna-data` `Source`, `cna-web` with embedded data, `Game`
   (`new_game`, `load`, `save`, `view`, `apply`), the `web/` scaffold
   drawing a plain hex outline grid and the top bar, CI. *Screenshot:* the
   theatre's 7 073 hex outlines with the top bar reading GT1 set-up, Axis
   to act.
2. **Map and counters** (`…-slice-2-map.md`): cna's palette,
   hexsides, settlements, labels, counters, stacks, off-map strip and
   tray, pan/zoom, the performance budget. *Screenshot:* the start
   position fitted to B–E, and zoomed on Sidi Barrani showing stacked
   counters.
3. **Set-up** (`…-slice-3-setup.md`): `placements`, the tray, click to
   place, the curtain, illegal toast, log panel. *Screenshot:* a tray
   piece selected with its legal hexes highlighted; the curtain.
4. **Movement and the non-combat turn** (`…-slice-4-movement.md`):
   `reachable`, the overlay, `move` (with assault flag), custom paths,
   `move-off-map`, initiative, reserves, end segment, pass, the
   `declare-zoc` and `reaction` panels. *Screenshot:* the reachable
   overlay with CP labels around a selected Italian stack.
5. **Combat and the end** (`…-slice-5-combat.md`): sealed gun positions,
   barrage and force assignment, retreat before assault, assault, forced
   retreat, results, end-of-game dialog. *Screenshot:* the barrage panel
   mid-entry, and the second player's sealed panel showing none of the
   first's choices.
6. **Save, load and the golden game** (`…-slice-6-e2e.md`): save, load,
   autosave, the golden game through the UI, docs, `MISSION 3 COMPLETE`.
   *Screenshot:* the end-of-game dialog after the golden game.

## Risks

- **Undeclared ZOC mid-move.** A `move` can stop early for a
  `declare-zoc` interrupt; the overlay can only flag it. The UI must
  handle `apply` returning with `pending: declare-zoc` mid-path and, after
  the answer, not resend the rest of the path blindly (the engine resumes
  or not; the UI just refreshes the view).
- **Combat API not merged.** Slice 5 is planned against a branch with
  uncommitted work; expect renames.
- **Sealed answers in shared state.** `view` redaction depends on how M2
  stores answers; gun positions currently land in a shared
  `combat.forward` set (slice 5 branch), which needs care to redact.
- **Victory report shape.** M2 slice 6 plans `Over(report)` but the state
  has `over: bool`; the end dialog degrades to "Game over" if no report
  is reachable.
- **SVG performance** on lower-end machines is unmeasured beyond Brian's
  desktop; the fallbacks above are ordered but not built.

## Requests for cna this mission makes

None expected. If counters need a field cna lacks (for example a short
display name per OA unit), slice 2 falls back to `id_code` and posts a
request rather than inventing data.
