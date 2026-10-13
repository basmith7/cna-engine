# cna-engine: a minimal browser UI for Graziani's Offensive

**Date:** 2026-10-13 (revision 3, against Mission 2 complete)
**Mission:** 3 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** proposed. Not marked for Brian's review in the queue, so it
merges once its gates pass, unless Brian says otherwise.

Written by the autopilot under Brian's delegation of 2026-09-26: every
decision below is Claude's, is cheap to reverse, and can be reversed with
one PR. The scope (a minimal UI on Mission 2's game state, local only) is
Brian's, from the *Path to a playable game* agreed on 2026-10-07.

**Checked against the code.** Every name, type, path and case number
below is the merged code on `origin/main` at `21f36a7`: Mission 2
complete (slices 1–6, PRs #10–#15, with the slice 4 and slice 5 review
follow-ups). Nothing here is planned against an unmerged branch.

## Goal

Two people at one computer play **Graziani's Offensive** to its end in a
browser: they see the map and every unit on it, set up their free
placements by clicking hexes, see where a selected stack can move and what
it costs in CP, give every order Mission 2 accepts by clicking, answer the
engine's interrupts (ZOC declarations, reactions, sealed combat steps,
retreats) when it asks, read the log and the victory report at the end,
and save and reload the game as the JSON state.

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
   with its CP cost; clicking one sends `move` along the engine's cheapest
   path; an illegal order shows the engine's case and reason and changes
   nothing.
4. Every `Pending` variant, and the close assault step (which is not a
   pending), is answered through a panel offering only what the engine
   computed, or, where the answer is a number, checked by `apply`.
5. Gun positions, barrage, force assignment, assault and retreats are
   entered by clicking; results show in the log from the engine's
   `Event`s.
6. When the game ends, a dialog shows the engine's `VictoryReport`: each
   side's level and every objective checked.
7. Save writes the state JSON; Load restores it and play continues
   identically (byte-equal state after the same further orders).
8. Playwright (headless Chromium, CI) plays M2's golden game
   `tests/games/graziani-short.json` (446 orders, seed 7) through UI
   clicks and ends with a `save()` byte-equal to `cna-play replay` of the
   same log, the end dialog open.

## The engine as it is (what M3 builds on)

**State and orders.**

- `cna_game::data::Data` has 18 pub fields: `rules_commit`, `scenario`,
  `oa`, `chars`, `weapons`, `map`, `terrain`, `stacking`, `weather`,
  `initiative`, `schedule`, `off_map`, and the combat tables `barrage`,
  `anti_armour`, `close_assault`, `morale`, `size_shifts`, `prisoners`.
  `Data::load(slug)` is native-only.
- `GameState::new(&Data, seed) -> anyhow::Result<GameState>`;
  `GameState::apply(&mut self, &Data, Side, Order) -> Result<Vec<Event>,
  Illegal>`; `Illegal { case: &'static str, reason: String }`. A rejected
  order changes nothing and is not logged.
- `GameState::to_act() -> Option<Side>`: the pending's side (for
  `sealed`, `waiting[0]`), else the initiative player in phase A, else
  Player A or B by `clock.half`; `None` once the game is over. This is the
  "acting player" the first draft planned to add.
- Save/load: `GameState::to_json()` (pretty JSON plus a newline) and
  `from_json(json, &Data)`, which checks scenario and `rules_commit` only.
  `to_log()` / `Log { scenario, rules_commit, ruleset, seed, orders }` /
  `log::replay(&Data, &Log)` give the order log and replay. A missing
  ruleset switch in a save takes its default.
- `Side` is `axis` / `cw` in JSON. Pieces are `PieceId` strings
  (`unit:it:…`, `supply:axis:16`, `supply:axis:dummy-1`). `Piece` carries
  `nation`, `id_code`, `kind` (`combat` / `hq` / `supply-unit` /
  `dummy-supply-unit`), `toe`, `weapons`, `cpa`, `morale`, `status`,
  `equivalent`, `cohesion`, `cp_spent`, `contact`, `engaged`, `reserve`,
  `released` and `location` (`hex` / `box` / `transit {from, to, done}` /
  `pending` / `eliminated`). The view needs only `unit_type` added (from
  `data.chars`).
- `Order` (kebab-case, externally tagged; `pass` and `end-segment` are
  bare strings): `place {piece, hex}`, `declare-initiative {player_a}`,
  `move {pieces, path, assault}`, `react {pieces, path}`, `declare-zoc
  {exerting}`, `move-off-map {pieces, to}`, `gun-positions {forward}`,
  `barrage {fires: [Fire {piece, points, target}]}`,
  `retreat-before-assault {pieces, path}`, `assign-forces {assignments:
  [Assignment {piece, anti_armour: [Allocation], close_assault:
  [Allocation]}]}` with `Allocation {target?: HexId, points}` (the
  non-phasing side leaves `target` out: it defends its own hex),
  `assault {target}`, `retreat {path}`, `designate-reserve {pieces}`,
  `release-reserve {pieces}`, `end-segment`, `pass`.
- `Pending` (kebab-case): `set-up {side}`, `declare-zoc {side, hexes,
  at_start}`, `reaction {side, eligible}`, `sealed {step, waiting}` with
  `step` one of `gun-positions` / `barrage` / `assign-forces`,
  `retreat-before-assault {side}`, `forced-retreat {side, pieces, hexes,
  from, reason, raw, overrun}` with `reason` one of `assault-result` /
  `withheld` / `must-attack`. Initiative, reserves and the close assault
  step are **not** pendings: they are ordinary orders with `pending:
  null`.
- `Event` has 38 kebab-case variants, among them the combat ones
  (`sealed`, `gun-positions-set`, `barrage-plotted`, `barrage`,
  `anti-armour`, `assault`, `losses`, `retreat-ordered`, …) and
  `game-over {report}`.

**Combat, as the clock walks it** (`combat::segment`). After `end-segment`
closes a movement segment the clock goes through the `Segment`s in order;
a step with nothing to do emits `phase-skipped` and is passed:

| Segment | Who acts | How |
|---|---|---|
| `breakdown` | nobody | always skipped |
| `position-determination` | both, sealed | `sealed {step: gun-positions}` if a gun-class unit of either side is next to the enemy; `gun-positions {forward}` |
| `barrage` | both, sealed | `sealed {step: barrage}` if a unit with barrage points is next to the enemy; `barrage {fires}` (empty to fire nothing) |
| `retreat-before-assault` | non-phasing | `retreat-before-assault {side}` whenever the sides are adjacent; `retreat-before-assault` orders, then `pass` |
| `force-assignment` | both, sealed | `sealed {step: assign-forces}`; `assign-forces {assignments}` |
| `anti-armour` | nobody | resolved from the assignments |
| `close-assault` | phasing | no pending; `assault {target}` per hex the phasing side assigned an assault against, then `pass` (refused with 15.2x while one is unresolved) |
| `reserve-release` | phasing | `release-reserve`, `end-segment` (another cycle) or `pass` |

`forced-retreat` interrupts any step that orders a retreat (15.82) and the
end of the close assault step (must-attack, 10.36); it is answered with
`retreat {path}` or `pass` (stand and pay, R-012).

**Sealed steps.** The phasing player answers first, then the other
(`waiting`). Each answer is checked alone by `check_gun_positions`,
`check_barrage` or `check_assignments` (`combat::plots`, all
`pub(crate)`), against the state as it stands and never against the other
answer, and `apply` returns only `Event::Sealed {side, step}` until the
last is in; then `gun-positions-set`, `barrage-plotted` or
`forces-assigned` reveal both, and the step resolves. Where the answers
wait: `combat.plots` and `combat.assignments` are maps **keyed by side**;
gun positions are **not**: `combat.forward` is one set holding both
sides' forward units, split by each piece's side only when revealed.

**The end.** `GameState.over: Option<VictoryReport>`; `is_over()`. The
report (`victory::VictoryReport`) is `{axis, cw}` (each an optional
`VictoryLevel`), `checked: [Condition {side, level, objective, hexes,
any_hex, held, unsupplied, met}]` and `unchecked: [String]` (conditions
the data states only in prose). The last order also emits
`game-over {report}`; afterwards `to_act()` is `None` and every order is
refused with 60.22. Graziani ends after Game-Turn 6 OpStage 3, the clock
at the start of GT7.

**Scripted games** (`tests/games/`, pinned by `cna-game` tests):

| File | Orders | Ends |
|---|---|---|
| `setup.json` | 76: the 59 free placements (36 Axis, 23 Commonwealth), initiative, and GT1 OpStage 1 passed through, its sealed barrage and assignment steps answered empty | GT1 OpStage 2, initiative declaration |
| `graziani-short.json` (the golden game) | 446, seed 7: set-up (orders 0–58); initiative, the Libyans' moves into Sollum (C4021) and Fort Capuzzo (C4020) with a Commonwealth reaction (59–65); gun positions, a barrage on 1 KRRC, retreat before assault passed, assignments and the assault on Halfaya (C3922) (66–76); a Commonwealth move and barrage (77–87); then every OpStage passed to the end | GT7, the report: neither side reaches a level, every condition checked; final state SHA-256 pinned in `cna-game::golden` |

The golden game never uses `declare-zoc`, `move-off-map`,
`designate-reserve`, `release-reserve`, a `retreat-before-assault` other
than `pass`, or `retreat`; those panels need their own e2e fixtures. It
does use `move` with `assault: true` once (order 61, the Libyans into
Sollum), which the slice 4 e2e covers. (Corrected after slice 4 review:
an earlier draft listed the assault flag among the gaps.)

**What the browser cannot use yet.**

- Data loading: every `cna-data` loader reads `vendor/cna` with `std::fs`
  via `cna_root()` (which `canonicalize().expect`s a path from
  `CARGO_MANIFEST_DIR`), `errata::load_table` lists `data/errata/`, and
  `rules_commit()` shells out to git. All of it compiles for wasm32 and
  fails at run time.
- Movement: `GameState::plan` / `plan_with(…, forced)` (crate-private)
  price a path step by step via `cna_rules::movement::step_cost`, report
  `ends_in_zoc`, and an `interrupt` where an *undeclared* enemy ZOC
  forces a `declare-zoc` pause mid-move (10.16, R-003). A retreat path
  that meets an undeclared ZOC is refused (13.2x, 15.82, 10.36), not
  paused. There is no legal-move query.
- Set-up: `state::test_support::candidates` (test-only) lists a pending
  piece's placement area, narrowed by two hand filters that only keep the
  scripted games tidy (Axis pieces stay in Libya; Commonwealth supply on
  sheet D goes next to Mersa Matruh). Scenario `constraint` sentences are
  prose (`Location::Pending` carries the `Placement` with its
  `constraint`); `setup::expand` lists them as `Unchecked` and the owner
  is trusted to honour them (59.2). Graziani's free placements carry
  "Libyan part of sheet C (only)", "…; all units of the group in one
  hex" and "any one hex of map D that lies west of Mersa Matruh (D3714);
  all three units in that one hex"; a fixed placement carries "or Helwan
  (E1430)", which the engine never offers (those units go to Cairo).
  There is no public placement query.
- Combat options: the plot checks are crate-private and the hexes still
  to assault (`unresolved_assaults`) are `pub(super)`; nothing tells a
  client which units may fire, at whom, or which hexes it must assault.
- Loading a hostile save: `from_json` does not validate invariants. A
  `sealed` pending with an empty `waiting` panics in `Pending::side` (so
  in `to_act()`), and clock invariants (stage 1–3, `half` only in F–L,
  `segment` only in G) are unchecked. A panic in wasm kills the page.

### What M3 adds to the engine, and what it no longer needs

| Item | Status |
|---|---|
| `acting_player()` | **Dropped**: `to_act()` exists. |
| Save/load/replay | **Dropped**: `to_json`, `from_json`, `to_log`, `replay` exist. |
| A victory report | **Dropped**: `over: Option<VictoryReport>` and `game-over` exist. |
| `answers()` for every pending | **Dropped** as a general API. Finite choices come from the pending itself (`declare-zoc.hexes`, `reaction.eligible`, `forced-retreat.from`) or from the queries below; numbers (points, splits) are typed in and judged by `apply`. |
| `geom` in `cna-game` | **Dropped**: geometry is a rendering concern, ported to TypeScript in `web/` (see Decisions). The engine stays rules-only. |
| Data from strings (`Source` seam in `cna-data`) | **Needed**, slice 1. |
| A `view` (the state minus `log` and `rng`, plus `to_act` and `unit_type`) | **Needed**, slice 1; slice 5 adds sealed redaction and `assaults_to_resolve`. |
| `placements(piece)` | **Needed**, slice 3. Enforcing the prose constraints is not M3's: it needs cna to type them (see *Requests*). |
| `reachable(side, pieces, purpose)` | **Needed**: slice 4 for `move` and `react`, slice 5 adds `retreat-before-assault` and `forced-retreat`. |
| `sealed_options(side)` | **Needed**, slice 5: M2 keeps the plot checks crate-private. |
| `from_json` invariant checks | **Needed**, slice 6, before Load accepts a file. |

## Decisions

| Topic | Decision | Why |
|---|---|---|
| How the browser gets the engine | `cna-game` compiled to **WebAssembly** through a new thin crate **`cna-web`** (`cdylib` + `rlib`) using `wasm-bindgen`, built with **`wasm-pack build --target web`**; no server process | Local play needs no server; the same Rust validates every order with zero duplication in TypeScript |
| Engine boundary | `cna-web` exposes one `Game` class with string-JSON methods: `new_game(seed)`, `load(json)`, `save()`, `view()`, `apply(side, order_json)`, then `placements(piece)` (slice 3), `reachable(side, pieces_json, purpose)` (slices 4–5) and `sealed_options()` (slice 5). Orders are M2's `Order` serde JSON verbatim. Every method is a plain Rust fn returning `serde_json::Value`, natively tested; the `#[wasm_bindgen]` layer only converts | A narrow JSON seam is testable from both sides and is the shape a server will carry. Plain JSON strings (not `serde-wasm-bindgen`) so the native tests and the browser see byte-identical output |
| Data loading | **`cna-data` gets a `Source` seam**: `trait Source { fn read(&self, rel: &str) -> Result<String>; fn list(&self, dir: &str) -> Result<Vec<String>>; }`, an `Fs` impl over `cna_root()`, and every loader becomes `load_from(&dyn Source)` with `load()` calling it with `Fs`. `Data::load_from(slug, &dyn Source, rules_commit)` beside `Data::load`. One seam instead of a `from_str` per type, and it covers the scenario `extends` chain and the errata directory listing, which per-type `from_str` would not | Matches the existing doc comment on `Data` ("another host fills the same struct by other means") with the least code; keeps errata application identical on both hosts |
| Embedded data | `cna-web`'s `build.rs` walks the `vendor/cna/data` directories `Data` reads (`map/`, `tables/`, `oa/`, `scenarios/`, `errata/`), generates an `Embedded` `Source` of `include_str!`s, and records `git -C vendor/cna rev-parse HEAD` as `RULES_COMMIT`. Raw size about 7.4 MB (6.2 MB of it the map), compressing well | One artefact, no fetch logic, and a save's `rules_commit` always matches the data compiled in. The generated list cannot drift from the files |
| Map drawing data | The web app imports the four map files it draws (`sheets`, `hexes`, `hexsides`, `places`) straight from `vendor/cna/data/map` as Vite JSON imports. They are also inside the wasm: a known, accepted duplicate of a few MB on localhost | Drawing is not rules; keeps the wasm API free of a geometry export |
| Geometry | A TypeScript port of `vendor/cna/tools/map_geom.py` (pointy-top, size 20, its row shift from `sheets.json` `odd_rows_shift`), sheets A–E side by side with sheet *k* offset by 33 columns (`seams.json`: column 33 meets the next sheet's column 01; E's column 00 is not a hex), Malta (M) as an inset. Pinned by a Vitest against polygon points parsed from cna's committed `site/public/map/C.svg` and a seam pair's centres one hex width apart | cna's renderer is the agreed look; pinning to its output stops drift without putting a UI concern in the engine |
| Front-end stack | **Vite + TypeScript + Svelte 5**, no component library or CSS framework | Standard toolchain; Svelte compiles to small plain JS |
| Map rendering and performance | One inline SVG. The **base layer** (7 073 hex polygons, hexsides, settlements, labels) is rendered **once** from the static JSON, outside any reactive state, with a single delegated click handler on the `<svg>` reading `data-hex`. Overlays (reachable shading, placement highlights, selection, counters) are separate `<g>` layers that hold only the hexes they mark. Budget, measured in slice 2 by Playwright: first paint of map + counters under 3 s, a pan or zoom step under 100 ms on Brian's desktop. Fallback if missed, in order: hide hexsides and labels below a zoom level, `shape-rendering: optimizeSpeed`, then a `<canvas>` base layer with SVG overlays | 7k polygons are fine in SVG *if* nothing re-renders them on each state change; that, not the count, is the real risk |
| Pan and zoom | `viewBox` pan (drag) and zoom (wheel, about the cursor), a "go to hex" box, start view fitted to sheets B–E | Simplest thing that works at this size |
| Counters | SVG drawn from `view()`: nation colour band, a NATO frame with a branch glyph chosen from `unit_type` by a small table in `web/src/counters.ts`, size mark from `equivalent`, `id_code`, TOE and CPA. Supply and dummy supply have their own glyph (they are not combat units). Small markers for `reserve`, `contact` / `engaged`, and in combat `pinned` and `forward`. Stacks show the top counter and a count badge; the stack panel lists all | Ours and data-derived; one obvious table to extend |
| Off-map pieces | A side strip of labelled boxes (`OffMapBox`: Tripoli, Tunisia, Tripolitania, Malta, Italy-Sicily, Crete), an "in transit" list (`Location::Transit`), and a per-side "to place" tray (`Location::Pending`) | They exist in the state and must be visible and selectable (`move-off-map`) |
| State sync | The engine state is the **only** game state. After every accepted `apply` the UI replaces its whole `view` from `Game.view()`; it never patches pieces locally. UI-only state is limited to selection, hover, the open panel's unsent inputs and the curtain. In test builds `window.__cna` exposes `view()` and `save()`; every e2e step asserts the rendered `data-unit` positions equal the view's | Divergence between a UI copy and the engine is the classic bug here; not having a copy removes it |
| Legal moves | `reachable(side, pieces, purpose) -> Vec<Reach { hex, cp, path, dp, ends_in_zoc, may_be_interrupted }>` with `purpose` `move` / `react` (slice 4) / `retreat-before-assault` / `forced-retreat` (slice 5): a Dijkstra over the same step pricing `apply` uses (refactor `plan_with` so both call one per-step function, `forced` for a forced retreat). A hex whose step ends in a known enemy ZOC is returned but not expanded; for `move`, a hex past an *undeclared* ZOC is returned flagged `may_be_interrupted`; for the other purposes it is not returned, since `apply` refuses such a path. `react` never enters an enemy ZOC; `retreat-before-assault` keeps 13.22's limit; `forced-retreat` keeps only hexes at least `hexes` from every `from` hex (all of them for `must-attack`). Proptest per purpose: every returned path is accepted by `apply` on a clone at the returned cost, and every random accepted path ends in a returned hex at no lower cost | Showing a move the engine would reject, or hiding one it would accept, is the worst bug a rules UI can have |
| Placements | `placements(piece) -> Vec<HexId>`: the placement area (the locator half of `candidates`, promoted out of `test_support`) filtered by a dry-run `place` on a clone; **not** narrowed by `candidates`' two hand filters, which stay test-only so the scripted games' pins do not move. The tray shows the piece's `constraint` prose under it as "also: …", the owner's to honour, as in M2 | Exactly the hexes `apply` accepts, by construction. Hand-coding prose in the query would make the UI stricter than the engine, a second copy of a rule nobody typed |
| Sealed options | `sealed_options(side) -> SealedOptions` for the pending step: gun-class units that may stand forward; per unit with barrage points, the enemy units it may target; per unit with TOE to commit, the roles it has (anti-armour, close assault) and, for the phasing side, the hexes each role may name. Built from per-item predicates that `check_gun_positions`, `check_barrage` and `check_assignments` are refactored to call, so one owner decides both. Points are typed in and judged by `apply` | The options are needed to draw the panels; refactoring the checks keeps them from becoming a second copy of 12.x–15.x |
| Moving | Click a hex: the stack panel lists its pieces, own pieces ticked. In a movement segment the reachable overlay shows; clicking a hex sends `move` with that path. An "assault" toggle sets `move.assault`. Shift-click builds a custom path hex by hex; Enter sends it, `apply` alone judges it | The engine prices moves; the UI never prices a step |
| Interrupts | One side panel (not a blocking modal: the map stays clickable) per `Pending` variant, switched exhaustively in TypeScript (`never` check, so a new variant fails `tsc`): `declare-zoc` ticks per hex; `reaction` picks units from `eligible` then a hex from `reachable(react)`, or Pass; the three sealed steps, the two retreats, and the close assault step (detected as `clock.segment == "close-assault"` with no pending) get panels in slice 5. Until then slice 4 shows those variants as a named placeholder, so the switch is exhaustive from the start | Each interrupt becomes one explicit question built from the pending's own data |
| Phase controls | Buttons for `end-segment`, `pass`, `declare-initiative` (A/B), `designate-reserve` / `release-reserve` (with a selection), shown by clock phase. They are not pre-filtered: an illegal click is an `Illegal` toast | Pre-filtering would be a second copy of the rules |
| Whose turn | A top bar shows clock (`Game-Turn`, OpStage, phase, half, segment), weather, initiative, Player A and **who must act** (`to_act()`). Every order is sent as `to_act()`; the UI never chooses a side | The engine, not the UI, decides who is at the keyboard |
| Secrecy (hot-seat) | A full-screen curtain ("Commonwealth to play — click to continue") whenever `to_act()` changes. While `pending` is `sealed`, `view()` **redacts** the step's answers by sides other than `to_act()`: their entry in `combat.plots` or `combat.assignments`, and for `gun-positions` their pieces in the shared `combat.forward` set (filtered by piece side). A sealed answer's `apply` result is only `sealed {side, step}`, safe to log. Each sealed panel clears its inputs on submit. The UI never renders `save()` JSON. Accepted limits, stated in the README: the autosave in `localStorage`, a downloaded save and `window.__cna` (test builds only) hold everything | Fair enough at one screen; real secrecy needs the server, which redacts the same way |
| Log | A side panel of one English line per `Event` (TS formatter, exhaustive `switch` with `never` over all 38 variants), grouped under clock headers; illegal attempts in red, not part of the game log | Readable history without the engine owning prose |
| Save and load | Save downloads `graziani-gt<N>-os<M>.json` (`save()`); Load accepts a file, errors from `GameState::from_json` shown and the current game kept; autosave to `localStorage` after each accepted order, offered as "Continue" on start | The state already is the save format |
| End of game | When `view.over` is not null, a dialog renders the `VictoryReport`: each side's level (or "no level"), a table of `checked` conditions (side, level, objective, held hexes, unsupplied holders, met), and `unchecked` prose if any; the final clock. The map stays viewable behind it; every order button is gone | The engine already decides the result; the UI only reads it |
| Repo layout | `crates/cna-web` (Rust) and `web/` (Vite app, `package.json`, Vitest, Playwright); `web/src/pkg` is wasm-pack output, git-ignored, rebuilt by `npm run wasm` | Keeps Node out of the Cargo workspace root |
| Tooling | Node 22 LTS, npm with a committed lockfile; CI installs the `wasm32-unknown-unknown` target and a pinned `wasm-pack` (0.15). Both are already installed on Brian's machine (checked 2026-10-12) | Reproducible CI, nothing new to install locally |

## WebAssembly: what is proven and what is not

`cargo check -p cna-game --target wasm32-unknown-unknown` **passes** on
`21f36a7` (re-run 2026-10-13, Mission 2 complete). That proves less than it
looks: `std::fs` and `std::process` compile for that target and fail at
run time, and `cna_data::cna_root()` would panic. The check stays in CI
(it catches threads, `getrandom` and native-only crates; the engine's RNG
is `rand_chacha` seeded from the state, and `sha2` is a dev-dependency, so
nothing needs `getrandom` today), but the real proof is slice 1's
`wasm-pack test --node` that builds a Graziani `GameState` from the
embedded `Source` and applies an order.

## Testing

- **Rust, natively:** `Source` parity (every `Data` field equal loaded
  via `Fs` and via `Embedded`); `placements`, `reachable` (per purpose)
  and `sealed_options` against `apply` on a clone, with hand-pinned cases
  on the real map (coast road from Sidi Barrani, Halfaya only by the
  track, a mixed stack paying the dearer cost, a ZOC stop, the 150 % foot
  cap, a vehicle going into DP); `view` redaction for each sealed step;
  the `Game` JSON methods (illegal order returns
  `{"illegal":{"case","reason"}}` and leaves `save()` unchanged;
  `load(save())` round-trips byte for byte); `from_json` refusing the
  malformed saves listed above.
- **wasm:** `wasm-pack test --node crates/cna-web`: new game, apply a
  legal and an illegal order, save/load round trip.
- **TypeScript (Vitest):** geometry against `C.svg`, event formatter (one
  case per variant), counter table (every `unit_type` in Graziani has a
  glyph), pending-panel exhaustiveness (compile-time), the curtain store.
- **Playwright (headless Chromium):** per slice, below, with `data-hex` /
  `data-unit` selectors and `window.__cna` assertions. Slices 3–5 drive
  consecutive stretches of `tests/games/graziani-short.json` by clicks
  (orders 0–58, 59–65, 66–76); slice 6 plays all 446. Fixture states for
  the panels the golden game never opens are generated by an ignored
  `cna-web` test and checked in under `web/tests/fixtures/`.
- **Screenshots:** each UI slice names the screenshot that proves it.
  Captured by Playwright at 1440×900 into `~/Projects/cna-m3-shots/slice-N/`
  and saved as a CI artefact (not pixel-compared), plus one 390×844
  capture per slice to confirm the page stays usable (scrolls, nothing
  out of reach); a mobile layout stays a non-goal. The implementer runs
  the **screenshot-critique** skill on every visual change before calling
  a slice done, fixes what it finds, and pastes the verdict into the PR.
- Gates: the Rust gates as before (`cargo clippy --all-targets -- -D
  warnings`, `cargo test`, `cargo run -p cna-probe -- check`), plus
  `cargo check -p cna-game --target wasm32-unknown-unknown`, `wasm-pack
  test --node crates/cna-web`, `npm run check` (svelte-check, tsc), `npm
  test`, `npm run build`, `npx playwright test`.

## Slices

One PR each, in order; each has a plan in `docs/plans/` and ends with
something visible in a browser and a named proving screenshot. Every slice
needs only merged code: Mission 2 is complete.

1. **Engine in the browser** (`2026-10-13-m3-slice-1-engine.md`):
   `cna-data` `Source`, `cna-web` with embedded data, `Game` (`new_game`,
   `load`, `save`, `view`, `apply`), the `web/` scaffold drawing a plain
   hex outline grid and the top bar, CI. *Screenshot:*
   `slice-1/grid.png`, the theatre's 7 073 hex outlines with the top bar
   reading GT1 set-up, Axis to act.
2. **Map and counters** (`…-slice-2-map.md`): cna's palette, hexsides,
   settlements, labels, counters, stacks, off-map strip and tray,
   pan/zoom, the performance budget. *Screenshots:* `slice-2/start.png`
   (the start position fitted to B–E) and `slice-2/sidi-barrani.png`
   (zoomed, stacked counters, open stack panel).
3. **Set-up** (`…-slice-3-setup.md`): `placements`, the tray with each
   piece's constraint prose, click to place, the curtain, illegal toast,
   log panel, the cna request below. *Screenshots:* `slice-3/placements.png` (a tray
   piece selected with its legal hexes highlighted) and
   `slice-3/curtain.png`.
4. **Movement and the non-combat turn** (`…-slice-4-movement.md`):
   `reachable` for `move` and `react`, the overlay, `move` (with assault
   flag), custom paths, `move-off-map`, initiative, reserves, end
   segment, pass, the `declare-zoc` and `reaction` panels, placeholders
   for the combat pendings. *Screenshot:* `slice-4/reachable.png`, the
   overlay with CP labels around the 1st Libyan Division's stack.
5. **Combat and the end** (`…-slice-5-combat.md`): `sealed_options`,
   sealed redaction, `assaults_to_resolve`, `reachable` for retreats; the
   gun position, barrage and force assignment panels, retreat before
   assault, close assault, forced retreat, results, end-of-game dialog.
   *Screenshots:* `slice-5/barrage-entry.png` (the Axis barrage panel
   mid-entry), `slice-5/sealed-other-side.png` (the Commonwealth's panel
   after the curtain, showing none of the Axis fires) and
   `slice-5/end-dialog.png` (the report on the golden game's final state).
6. **Save, load and the golden game** (`…-slice-6-e2e.md`): `from_json`
   invariant checks, save, load, autosave, the golden game through the
   UI, docs, `MISSION 3 COMPLETE`. *Screenshots:*
   `slice-6/golden-end.png` (the end dialog after the golden game played
   by clicks) and `slice-6/load-error.png`.

## Risks

- **Undeclared ZOC mid-move.** A `move` can stop early for a
  `declare-zoc` interrupt; the overlay can only flag it. The UI must
  handle `apply` returning with `pending: declare-zoc` mid-path and, after
  the answer, not resend the rest of the path (the engine resumes or not;
  the UI just refreshes the view).
- **Gun positions share one set.** `combat.forward` mixes both sides, so
  redaction filters by piece side; a later change to per-side storage in
  the engine must keep `view`'s redaction test passing.
- **Golden game length in a browser.** 446 orders by clicks, most of them
  sealed steps and passes, with a curtain at each change of side. If
  Playwright takes too long, the driver keeps clicking every order but
  the per-order DOM check is sampled (every order of 0–87, then every
  OpStage boundary); it never skips an order.
- **Coverage gaps in the golden game.** Off-map moves, reserves, ZOC
  declarations and retreats are never in it; their e2e
  rests on generated fixtures, and a forced retreat depends on a die roll,
  so its fixture is found by seed search and pinned.
- **Malformed saves.** Until slice 6's `from_json` checks land, Load must
  not be offered; slices 1–5 only load what `save()` wrote.
- **Shared `CARGO_TARGET_DIR`.** Worktrees sharing one target dir can
  run each other's stale test binaries (journal, 2026-10-12; seen again
  2026-10-13, when a `cna-rules` test listed rulings this tree's
  `vendor/cna` does not have and passed on a re-run). A gate that fails
  on paths or data it should not see is re-run before being believed.
- **SVG performance** on lower-end machines is unmeasured beyond Brian's
  desktop; the fallbacks above are ordered but not built.

## Requests for cna this mission makes

- **Typed set-up constraints** (slice 3 posts it): Graziani's
  `constraint` sentences (above) as data an engine can check: a country
  filter on an area, "the group in one hex", "west of a hex", and an
  alternative to a fixed place (Helwan for Cairo). Until then the engine
  trusts the owner, the UI shows the prose, and Helwan is never offered.
- If counters need a field cna lacks (for example a short display name
  per OA unit), slice 2 falls back to `id_code` and posts a request rather
  than inventing data.
