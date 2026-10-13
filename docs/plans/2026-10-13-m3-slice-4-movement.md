# Mission 3, slice 4 (movement and the non-combat turn) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Play every non-combat order of a Graziani OpStage by clicking:
initiative, reserves, moves along engine-computed paths, off-map moves,
end segment, pass, and the two movement interrupts (ZOC declaration and
reaction). The slice ends where combat begins: `end-segment` opens the
sealed gun-positions step, shown as a placeholder until slice 5.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Previous plans:** slices 1–3.

**Proving screenshot:** `slice-4/reachable.png`: the 1st Libyan
Division's stack selected as golden order 61 is about to be given, with
the reachable overlay, CP numbers in hexes, DP and ZOC-stop outlines.

## Rules for every task

- Stage only your own files (`git add <paths>`, never `git add -A`).
- Paste the output of each verification command into the PR body; do not
  summarise it as "passes".
- Stop any dev or preview server you start (`npm run dev`, `npm run
  preview`) before finishing; Playwright's `webServer` stops its own.
- On any visual change, capture the slice's proving screenshots with
  Playwright at 1440×900 (plus one at 390×844) into
  `~/Projects/cna-m3-shots/slice-N/` under the names below, run the
  **screenshot-critique** skill on them, fix what it finds, and paste its
  verdict in the PR.
- `export CARGO_TARGET_DIR=$HOME/.cache/cna-engine-target`; a gate that
  fails on data this tree does not have is re-run before being believed
  (spec, *Risks*).

## Tasks

1. **`reachable` in `cna-game`.** Refactor `GameState::plan_with` so the
   per-step pricing and refusals are one function both it and the query
   call. `GameState::reachable(&self, &Data, Side, &[PieceId], Purpose)
   -> Result<Vec<Reach>, Illegal>` with `Purpose::{Move, React}` (slice 5
   adds the two retreats) and `Reach { hex, cp, path, dp, ends_in_zoc,
   may_be_interrupted }`; `Err` with the case `apply` would give when the
   stack cannot move now. Dijkstra;
   known-ZOC hexes are returned and not expanded; for `Move`, hexes past
   an undeclared ZOC are flagged `may_be_interrupted`; `React` never
   enters an enemy ZOC (8.5x). Tests: hand-pinned real-map cases (coast
   road, Halfaya by the track only, mixed stack, ZOC stop, 150 % foot
   cap, vehicle into DP, Marble Arch, golden order 61's path among the
   results at its cost); proptest per purpose, both ways, against
   `apply` on a clone. Expose in `cna-web`. Commit `Reachable hexes with
   CP cost`.
2. **Selection and overlay.** Stack panel with own pieces ticked; in a
   movement segment call `reachable`, shade hexes, CP labels, distinct
   outlines for DP / ZOC stop / may-be-interrupted; hover draws the path.
   Click sends `move`; an "assault" toggle sets `assault`. Shift-click
   builds a custom path, Enter sends it. Commit `web: movement`.
3. **Off-map moves.** Selecting a stack in a box, in transit or at
   Nofilia offers the `OffMapPlace` destinations as buttons →
   `move-off-map`. Commit `web: off-map moves`.
4. **Phase controls.** End segment, Pass, Declare initiative (A/B),
   Designate reserve / Release reserve (with selection), shown by clock
   phase, not pre-filtered. Commit `web: phase controls`.
5. **Interrupt panels.** `Pending.svelte`, exhaustive over every
   `Pending` variant on main: `declare-zoc` (tick each of `hexes`; the
   panel says whether it is `at_start`), `reaction` (pick from
   `eligible`, then a `reachable(…, React)` hex, or Pass); `sealed`,
   `retreat-before-assault` and `forced-retreat` render a named
   placeholder ("combat: slice 5") so the `never` check holds now. When a
   `move` returns with `declare-zoc` pending, the UI only refreshes; it
   never resends the rest of the path. Commit `web: ZOC and reaction`.
6. **e2e fixtures.** An ignored `cna-web` test,
   `regenerate_e2e_fixtures`, writes `web/tests/fixtures/<name>.json`
   (state JSON) for the panels the golden game never opens, each built
   from a `replay` of a golden prefix plus piece moves or orders through
   the public API, and checked to open the panel it is named for:
   `declare-zoc.json`, `off-map.json` (an Axis stack in a box),
   `reserve.json` (phase F, then a reserve release segment). A
   non-ignored test fails if a checked-in fixture differs from what the
   generator builds. Commit `web: e2e fixtures`.
7. **e2e.** From the slice 3 end state, click golden orders 59–65
   (initiative, pass, the Libyans into Sollum, the Commonwealth's
   reaction through the panel and its pass, the move into Fort Capuzzo,
   end segment); assert a pinned CP on order 61's hex before clicking it,
   the pieces' DOM positions against `__cna.view()` after each order, and
   at the end the `sealed` placeholder with the Axis to act and
   `__cna.save()` byte-equal to `cna-play replay` of orders 0–65. A
   shift-click path up the escarpment gives the expected case. From the
   fixtures: a `declare-zoc` answer, a `move-off-map`, a designate and a
   release. Capture `slice-4/reachable.png`, run screenshot-critique, fix
   what it finds. Commit `web: e2e movement`.
