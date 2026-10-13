# Mission 3, slice 5 (interrupts and combat) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every decision the engine asks of the non-acting player, and
all of combat, entered through the UI; the game's end shown.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md`
**Previous plans:** slices 1–4; needs M2 slice 5 (combat) merged, and
M2 slice 6 for the victory report.

## Tasks

1. **Pending modal framework.** `Pending.svelte` switches on
   `view.pending`'s variant (exhaustive, `never` check); each variant is a
   component that builds its order from `answers()` plus map clicks
   (the map stays interactive under the modal; only hexes and units the
   answers mention are clickable). The curtain from slice 4 precedes it
   when the player changes. Commit `web: pending modal`.
2. **ZOC declaration (10.16) and reaction (8.51).** ZOC: yes/no per
   listed unit. Reaction: choose reacting units from the list, then a hex
   from `reachable` for them, or decline. Commit `web: ZOC and reaction`.
3. **Barrage and assault orders.** In the combat segment the selection
   panel shows "Barrage" / "Assault" for a target hex the attackers can
   reach in M2's terms; probe vs full assault as M2's `Assault` fields.
   Commit `web: combat orders`.
4. **Retreat before assault.** The non-phasing player chooses to stay or
   picks a retreat path (hexes from `answers()` or `reachable`). Commit
   `web: retreat`.
5. **Sealed force assignment.** First player assigns pieces to roles;
   submit; curtain; second player assigns; the engine resolves when both
   are in. The first entry is never displayed to the second. Commit
   `web: sealed plots`.
6. **Results.** Combat events in the log (rolls, results, TOE losses,
   cohesion, retreats, captures); counters update from the view; a brief
   highlight on hexes whose pieces changed. Commit `web: combat results`.
7. **Victory.** When the view is over, a dialog with each side's level
   and the checked conditions; the board stays viewable. Commit `web:
   victory`.
8. **e2e.** Playwright from a saved mid-game fixture (built by
   `cna-play` from M2's golden game prefix): a Commonwealth reaction, a
   barrage, a close assault with force assignment from both sides, and a
   loss shown in the log. Commit `web: e2e combat`.
