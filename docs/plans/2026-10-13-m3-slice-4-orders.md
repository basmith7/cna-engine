# Mission 3, slice 4 (set-up and movement) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Play the non-combat part of Graziani by clicking: free set-up,
initiative, reserves, moving stacks along engine-computed legal paths,
ending segments and phases, with the hot-seat hand-off and a log.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Previous plans:** slices 1–3.

## Rules for every order the UI sends

- The player is always `view.acting_player`; the UI never picks a side.
- Every order goes through `engine.apply`; on `illegal`, show a toast
  "Illegal (8.1x): reason" and append it in red to the log panel; the view
  is not refreshed (nothing changed).
- On `ok`, append the events to the log panel and refresh the view.
- The UI shows choices from `reachable`, `placements`, `answers` only, but
  never assumes they are complete for correctness: `apply` decides.

## Tasks

1. **Selection.** Click a hex: stack panel with checkboxes for the acting
   player's pieces (all on by default); Escape clears. Vitest for the
   selection store. Commit `web: selection`.
2. **Set-up.** Clicking a piece in the "to place" tray highlights
   `placements(unit)` hexes; clicking one sends `Place`. The tray shows
   the deployment's placement text (`within 3 hexes of C0716`) and any
   unchecked `constraint` text M2 logs. Commit `web: set-up`.
3. **Reachable overlay.** With a selection in a movement segment, call
   `reachable`; shade hexes (a CSS class, opacity by cost band), print the
   CP in the hex, mark DP-costing and ZOC-stop hexes with distinct
   outlines; hovering shows the path as a polyline. Click sends `Move` with
   that path. Shift-click builds a custom path hex by hex (each hex adjacent to the
   last; the overlay then shows only the path). Enter sends it; the engine
   alone judges it, so a bad custom path is an `illegal` toast, not a UI
   check. Commit `web: movement`.
4. **Phase controls.** Buttons from `answers()` and the clock: "End
   segment", "Pass", "Declare initiative: be Player A / B", "Designate
   reserve" (with selection), "Release reserve". Disabled when the engine
   would refuse (not in `answers`) except Move. Commit `web: phase
   controls`.
5. **Hot-seat curtain.** When `acting_player` changes after an `apply`, a
   full-screen overlay "Commonwealth player: click to continue" hides the
   board. Vitest for the store transition. Commit `web: hot-seat`.
6. **Log panel.** `format.ts`: one English line per `Event` variant (a
   `switch` with `never` exhaustiveness check so a new M2 event fails
   `tsc`), grouped under clock headers. Vitest for each variant. Commit
   `web: log`.
7. **e2e.** Playwright: place every pending Axis then Commonwealth piece
   by clicking (fixture positions from M2's `tests/games/setup.json`),
   pass to the Axis movement segment, select the Sidi Barrani stack, assert
   a reachable hex shows the pinned CP, click it, assert the piece moved
   (`window.__cna.view()` in test builds); attempt an illegal move by
   shift-click into an escarpment and assert the toast case. Commit
   `web: e2e set-up and movement`.
