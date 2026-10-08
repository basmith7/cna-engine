# Mission 2, slice 6 (victory and the golden game) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** End Graziani's Offensive with a victory report, prove the whole
engine on a scripted game, and close Mission 2's docs.

**Spec:** `docs/designs/2026-10-08-game-state-design.md`
**Previous plans:** slices 1–5.

## Domain facts the implementer needs

- **End:** after Game-Turn 6 OpStage 3 (60.22).
- **Victory** (60.8, scenario `victory`: `kind: "levels"`): each side's
  highest level whose every `all_of` objective holds. `hold` is a
  placement (`place` or `hexes`, with `any_hex` meaning one of them
  suffices) occupied by that side's combat unit and not by the enemy's.
  Each level's `supply` clause is English until cna answers the request;
  report it as `unchecked`.

## Tasks

1. **Victory evaluation.** `victory::evaluate(&GameState, &Scenario) ->
   VictoryReport { axis: Option<Level>, cw: Option<Level>, checked,
   unchecked }`. Tests: hand-built states for each Axis and Commonwealth
   level; Alexandria counted by either of its two hexes.
2. **Game end.** After GT6 OS3 the state is `Over(report)` and every
   order is `Illegal("60.22")`.
3. **The golden game.** `tests/games/graziani-short.json`: set-up, then
   at least two OpStages with Italian moves along the coast road, a
   Commonwealth reaction, a barrage and a close assault on a real hex;
   then passes to the end. Pin the final state's SHA-256 and the victory
   report; replay it in the test.
4. **Determinism property test.** Random legal `Move` orders (from a
   seeded generator over each piece's legal steps) for one stage, then
   replay: equal states.
5. **Docs.** README (what the engine plays and how to run `cna-play`),
   `NOT_SIMULATED.md` (a *Systems* table: §32 supply, breakdown,
   organisation, engineering, repair, patrols, rail, air, logistics,
   limited intelligence), the spec's Status, the queue in `AUTOPILOT.md`
   (Mission 2 `done`, Mission 3 `next`), `MISSION 2 COMPLETE` in the
   journal and `PROGRESS.md`.
