# Mission 3, slice 3 (set-up) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Both sides make their free placements by clicking, with a
hot-seat curtain, illegal-order toasts and the log panel; the game reaches
GT1 phase A.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Previous plans:** slices 1–2.

**Proving screenshots:** an Axis tray piece selected with its legal hexes
highlighted; the curtain "Commonwealth to play".

## Rules for every task

- Stage only your own files (`git add <paths>`, never `git add -A`).
- Paste the output of each verification command into the PR body; do not
  summarise it as "passes".
- Stop any dev or preview server you start (`npm run dev`, `npm run
  preview`) before finishing; Playwright's `webServer` stops its own.
- On any visual change, capture the slice's proving screenshot and run
  the **screenshot-critique** skill on it before calling the slice done;
  paste its verdict in the PR.

## Rules for every order the UI sends

- The side is always `view.to_act`; the UI never picks one.
- On `illegal`: toast "Illegal (case): reason", a red line in the log
  panel, view not refreshed.
- On `ok`: events to the log panel, view replaced from `Game.view()`.

## Tasks

1. **`placements` in `cna-game`.** Move `candidates` out of
   `state::test_support` into a public `GameState::placements(&self,
   &Data, &PieceId) -> Vec<HexId>` that keeps its two `constraint`
   filters and keeps only hexes a `Place` on a clone accepts. Tests: for
   every Graziani pending piece, every returned hex is accepted and a
   sample of omitted hexes inside the area is refused (stacking,
   `not_within`); `auto_place` uses `placements`. Expose in `cna-web`.
   Commit `Placements query`.
2. **Tray and placing.** Click a tray piece → highlight `placements`;
   click a hex → `place`. The tray shows the deployment's locator text
   and any `Unchecked` constraint text from `setup::expand`. Commit
   `web: set-up`.
3. **Curtain.** On a `to_act` change after `apply`, full-screen overlay
   until clicked. Vitest for the store. Commit `web: hot-seat curtain`.
4. **Log panel and toasts.** `format.ts` with one line per `Event`
   variant (exhaustive `never`), clock headers. Vitest per variant.
   Commit `web: log`.
5. **e2e.** Place every pending piece by clicking, positions from
   `tests/games/setup.json` (the Axis then the Commonwealth, curtain
   between); one deliberate illegal click asserts the toast case; end
   with `window.__cna.save()` equal to `cna-play` applying the same
   orders. Commit `web: e2e set-up`.
