# Mission 3, slice 4 (movement and the non-combat turn) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Play every non-combat order of a Graziani OpStage by clicking:
initiative, reserves, moves along engine-computed paths, off-map moves,
end segment, pass, and the two merged interrupts (ZOC declaration and
reaction).

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Previous plans:** slices 1–3.

**Proving screenshot:** a selected Italian stack near Sidi Barrani with
the reachable overlay, CP numbers in hexes, DP and ZOC-stop outlines.

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

1. **`reachable` in `cna-game`.** Refactor `GameState::plan` so the
   per-step pricing and refusals are one function both `plan` and the
   query call. `GameState::reachable(&self, &Data, Side, &[PieceId]) ->
   Result<Vec<Reach>, Illegal>` with `Reach { hex, cp, path, dp,
   ends_in_zoc, may_be_interrupted }`; `Err` with the case `apply` would
   give when the stack cannot move now. Dijkstra; known-ZOC hexes are
   returned and not expanded; hexes past an undeclared ZOC are flagged.
   Also used for `react` (never into an enemy ZOC) and, in slice 5,
   retreats. Tests: hand-pinned real-map cases (coast road, Halfaya by
   the track only, mixed stack, ZOC stop, 150 % foot cap, vehicle into
   DP, Marble Arch); proptest both ways against `apply` on a clone.
   Expose in `cna-web`. Commit `Reachable hexes with CP cost`.
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
5. **Interrupt panels.** `Pending.svelte`, exhaustive over the merged
   variants: `declare-zoc` (tick each of `hexes`), `reaction` (pick from
   `eligible`, then a `reachable` hex, or Pass). When a `move` returns
   with `declare-zoc` pending, the UI only refreshes; it never resends the
   rest of the path. Commit `web: ZOC and reaction`.
6. **e2e.** From the slice 3 end state: initiative, pass to an Axis
   movement segment, select the Sidi Barrani stack, assert a pinned CP on
   a reachable hex, click it, assert the piece moved in both DOM and
   `__cna.view()`; a shift-click path over the escarpment gives the
   expected case; a move that triggers a Commonwealth reaction is
   answered through the panel. Commit `web: e2e movement`.
