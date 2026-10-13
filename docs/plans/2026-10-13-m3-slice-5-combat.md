# Mission 3, slice 5 (combat and the end) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** All of combat entered through the UI, with each sealed answer
kept off screen from the other player, and the game's end shown as the
engine's victory report.

**Spec:** `docs/designs/2026-10-13-minimal-ui-design.md` (*Combat, as the
clock walks it*; *Sealed steps*; *The end*).
**Previous plans:** slices 1–4. Every engine name below is merged code on
`origin/main` (`21f36a7`).

**Proving screenshots:** `slice-5/barrage-entry.png`, the Axis barrage
panel mid-entry at golden order 68 (1st Libyan Artillery Regt, 6 points,
target 1 KRRC chosen); `slice-5/sealed-other-side.png`, the
Commonwealth's barrage panel after the curtain, showing none of the Axis
fires; `slice-5/end-dialog.png`, the report on the golden game's final
state.

## Domain facts

- Step order, who acts and how: the spec's table. Sealed steps wait on
  the phasing player, then the other (`sealed.waiting`); the close
  assault step is no pending (`clock.segment == "close-assault"`,
  `pending: null`, the phasing player to act).
- A sealed answer is checked alone (`check_gun_positions`,
  `check_barrage`, `check_assignments` in `combat::plots`) and returns
  only `sealed {side, step}`; the last answer's events reveal both.
- `combat.plots` and `combat.assignments` are keyed by side;
  `combat.forward` is one set for both sides.
- `Allocation.target` is a hex for the phasing side and left out for the
  non-phasing side (it defends its own hex, 14.12).
- A retreat path that meets an undeclared enemy ZOC is refused (13.2x,
  15.82, 10.36). `forced-retreat` `pass` stands and pays where R-012 lets
  it; for `must-attack` it surrenders the stack (an open M2 follow-up:
  the panel says so before sending).

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

1. **Sealed options, redaction, assaults to resolve.** Refactor the three
   plot checks so each per-item test (a gun-class unit; a firer with
   barrage points and an enemy target it may hit across that hexside; a
   unit with TOE to commit, its roles, and the hexes each may name) is a
   function the check and a new `GameState::sealed_options(&self, &Data,
   Side) -> SealedOptions` both call. Tests: every offered single item,
   sent alone with 1 point, is accepted by `apply` on a clone; every own
   on-map unit and enemy unit or hex not offered is refused; the golden
   game's answers at orders 66–72 use only offered items. `view` redacts,
   while `pending` is `sealed`, the current step's answers by sides
   other than `to_act`: `combat.plots[side]`, `combat.assignments[side]`,
   and for `gun-positions` that side's pieces in `combat.forward`; tests
   in the states before golden orders 67, 69 and 72 that no Axis answer is in the
   Commonwealth's view and that the full answers are there once revealed.
   `view` adds `assaults_to_resolve` (`unresolved_assaults` made
   `pub(crate)`) in the close assault step. Expose `sealed_options` in
   `cna-web`. Commit `Sealed options and a redacted view`.
2. **`reachable` for retreats.** `Purpose::RetreatBeforeAssault` (13.22's
   limit for a stack not next to an enemy combat unit; pinned,
   already-retreated and collapsed units give the `apply` case) and
   `Purpose::ForcedRetreat` (`plan_with` forced; `Reach` gains `short`,
   the hexes short of `hexes` from the nearest `from` hex; a short hex is
   returned only where `apply` accepts stopping short, so never for
   `must-attack`). Neither returns a hex past an undeclared ZOC. Proptest both ways against `apply` on a clone, per
   purpose. Commit `Reachable retreats`.
3. **Sealed panels.** `gun-positions` (tick forward units from the
   options), `barrage` (per firer: points and a target from the options,
   as `Fire`s; empty is "fire nothing"), `assign-forces` (per unit: split
   TOE between anti-armour and close assault, with target hexes for the
   phasing side, as `Assignment`s; an unlisted unit is withheld). Submit
   clears the panel's inputs, then curtain, then the other side. An
   illegal answer shows the case and keeps the inputs. Commit `web:
   sealed steps`.
4. **Retreats.** `retreat-before-assault` (pick a stack, then a
   `reachable(…, RetreatBeforeAssault)` hex, or Pass to end the step)
   and `forced-retreat` (the pieces, `reason`, `hexes`, `from` and `raw`
   shown; a `reachable(…, ForcedRetreat)` hex, or Pass with the stand or
   surrender consequence spelled out from `reason`). Commit `web:
   retreats`.
5. **Close assault step.** Hexes in `assaults_to_resolve` are
   highlighted; clicking one sends `assault {target}`; Pass ends the step
   (refused with 15.2x while one is left, shown as a toast). Commit `web:
   assault`.
6. **Results.** Combat events read well in the log (die rolls, barrage
   and CRT results, TOE losses, cohesion, retreats, prisoners, captures,
   combat CP and refunds); counters and pinned/forward markers update
   from the view. Commit `web: combat results`.
7. **End of game.** When `view.over` is not null: the dialog of the
   spec's *End of game* decision (each side's level, the `checked`
   table, `unchecked` prose, the final clock), every order control gone,
   the map still viewable. Fixture `game-over.json` is `cna-play replay
   tests/games/graziani-short.json`'s output, made by the fixture
   generator. Commit `web: end of game`.
8. **e2e fixtures.** Extend `regenerate_e2e_fixtures` with
   `combat-start.json` (`replay` of golden orders 0–65),
   `retreat-before-assault.json` (a stack that may retreat),
   `forced-retreat.json` (an assault-result retreat, the seed found by a
   bounded search in the generator and pinned) and `game-over.json`.
   Commit `web: combat fixtures`.
9. **e2e.** From `combat-start.json`, click golden orders 66–76 (both
   gun positions, the Axis barrage on 1 KRRC and the empty Commonwealth
   one, the retreat-before-assault pass, both assignments, the assault
   on C3922, the passes), asserting after each Axis sealed answer and
   curtain that no Axis choice is in the DOM, that the assault's losses
   are in the log, and at the end `__cna.save()` byte-equal to `cna-play
   replay` of orders 0–76. From the fixtures: a retreat before assault
   along a reachable path, a forced retreat, and the end dialog's levels
   and conditions against `view.over`. Capture the three proving
   screenshots, run screenshot-critique, fix what it finds. Commit `web:
   e2e combat`.
