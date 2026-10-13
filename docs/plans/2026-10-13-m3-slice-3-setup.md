# Mission 3, slice 3 (set-up) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Both sides make their free placements by clicking, with a
hot-seat curtain, illegal-order toasts and the log panel; the game reaches
GT1 phase A.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Previous plans:** slices 1–2.

**Proving screenshots:** `slice-3/placements.png`, an Axis tray piece
selected with its legal hexes highlighted and its constraint prose shown;
`slice-3/curtain.png`, the curtain "Commonwealth to play".

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

## Rules for every order the UI sends

- The side is always `view.to_act`; the UI never picks one.
- On `illegal`: toast "Illegal (case): reason", a red line in the log
  panel, view not refreshed.
- On `ok`: events to the log panel, view replaced from `Game.view()`.

## Tasks

1. **`placements` in `cna-game`.** Split `state::test_support::candidates`
   into a public locator half (the placement area of a pending piece's
   `Placement`) and the two test-only hand filters, which stay in
   `test_support` so `auto_place` and the scripted games' pins do not
   change. `GameState::placements(&self, &Data, &PieceId) -> Vec<HexId>`
   = the area, kept where a `place` on a clone is accepted. Tests: for
   every Graziani pending piece of a new game and of the state after
   order 35 of `tests/games/graziani-short.json` (the Axis done), every
   returned hex is accepted and every omitted hex in the area is refused
   (stacking, `not_within`); a piece that is not pending gives none.
   Expose in `cna-web`. Commit `Placements query`.
2. **Tray and placing.** Click a tray piece → highlight `placements`;
   click any hex → `place` (a hex outside the highlight is sent too;
   the engine refuses it). The tray shows the piece's locator in words and
   its `constraint` prose (from `location.pending`) as "also: …", the
   owner's to honour. Commit `web: set-up`.
3. **cna request.** Post the spec's *Typed set-up constraints* request
   (`AUTOPILOT.md`, requests to cna) with the sentences and the cases
   they come from. Commit with the docs.
4. **Curtain.** On a `to_act` change after `apply`, full-screen overlay
   until clicked. Vitest for the store. Commit `web: hot-seat curtain`.
5. **Log panel and toasts.** `format.ts` with one line per `Event`
   variant, all 38 on main including combat and `game-over` (exhaustive
   `never`), clock headers. Vitest per variant. Commit `web: log`.
6. **e2e.** Click orders 0–58 of `tests/games/graziani-short.json` (the
   59 placements, identical to `setup.json`'s first 59: the Axis's 36,
   the curtain, the Commonwealth's 23); one deliberate illegal click
   (a hex outside the highlight: the UI sends it anyway and the engine
   judges) asserts the toast case `59.2` and an unchanged `save()`; end in GT1 phase A with the
   Axis to act and `window.__cna.save()` byte-equal to `cna-play replay`
   of a log holding those 59 orders. Capture the proving screenshots,
   run screenshot-critique, fix what it finds. Commit `web: e2e
   set-up`.
