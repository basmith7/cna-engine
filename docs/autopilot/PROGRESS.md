# Autopilot progress

Human-facing journal for the unattended runs (see `AUTOPILOT.md`). The
autopilot rewrites **Status** and **Next steps** at the end of every run and
the cron script appends to **Runs and quota**. The only section a human
writes is **Feedback**.

## Feedback


## Addressed

- 2026-10-13 (Brian, interactive session) asked for a multiplayer server with real accounts and many games, ahead of the rest of the Land Game turn; chose Discord sign-in, open sign-up and turn alerts by Discord DM → queue row 4, land rules moved to row 5.
- 2026-10-12 (Brian) approved the Mission 2 spec, PR #5, in an interactive session → merged 2026-10-12; that session builds slices 1–6 and holds the autopilot lock while it does.

## Status

**MISSION 4, Part 0** (2026-10-13): the multiplayer server's spec and six
slice plans are PR #25, labelled `needs-brian`. Nothing for Mission 4 is
built until **Feedback** approves it. The spec asks three questions, each
with a default: how the bot reaches DMs (`guilds.join` and a small CNA
Discord server), host port 3240, and the image name
`ghcr.io/basmith7/cna`.

**MISSION 3 COMPLETE** (2026-10-13): a minimal browser UI for Graziani's
Offensive in `web/` on `cna-game` compiled to WebAssembly (`crates/cna-web`,
the `vendor/cna` data built in). Slices 1–6 merged as PRs #18–#23: the
engine in the browser, the map and counters, set-up, movement, combat
and the end, then save/load/autosave with the golden game played through
the UI by Playwright (`web/tests/golden.spec.ts`: 446 clicks' worth of
orders, a save, a reload and a load in the middle, the final state
byte-equal to `replay`). The spec is
`docs/designs/2026-10-13-minimal-ui-design.md` (Status: built). Local
only, nothing hosted; secrecy is hot-seat only (README).

**MISSION 2 COMPLETE** (2026-10-13): a headless game state for Graziani's
Offensive in `cna-game`, driven by `cna-play`. Slices 1–6 merged as PRs
#6, #8, #9, #11, #12 and #15: map, scenario and pieces, state and the
sequence of play, movement, combat, victory. Two scripted games in
`tests/games/` replay in CI; the short game's final state and victory
report are pinned. The spec is `docs/designs/2026-10-08-game-state-design.md`
(Status: built).

Nonblocking combat follow-ups from PR #12's review stay open for a later
slice (listed in the journal entry of 2026-10-12 and PR #15's body): rule
tests for 15.36, 6.26, 13.1, 15.1x and 13.2x; the retreat-short 10 % taken
after rounding rather than folded in (15.83); a must-attack `Pass`
surrendering a stack that has a path (10.36); attacking forward guns and
artillery in the anti-armour role (15.84, 14.14); a released unit's later
combat segments (18.2x); a malformed save with an empty sealed `waiting`
panics.

**MISSION 1 COMPLETE** (2026-09-26): rules core with switches for R-001,
R-009, R-011, R-012, R-015, R-018 and eight probes in `probes/` (PRs #1–#4).

## Next steps

- **Review the Mission 4 spec: PR #25.** Approve (or ask for changes)
  under **Feedback**, answering its three questions if the defaults are
  wrong. Building starts with slice 1 once approved.
- Mission 3's accepted residue, for a polish slice when it matters: the
  count badge touches the top counter's CPA; frontier stacks overlap at
  the fitted view; the 390 px layout is the slice 2 one (mobile is a
  non-goal); ticks are offered in a sealed step though nothing moves;
  selecting an uncapped motorised stack takes ~0.6 s to shade; at 390 px
  the end dialog's table clips its right-hand columns (it scrolls).
- From PR #23's review: hostile saves are now refused (PR #26 checks
  every piece and hex a save names). Still open: a Load or Continue opens with no curtain, so whoever loads sees
  `to_act`'s (redacted) view; consider dropping the curtain on load.
- Since Mission 2: #13, #14 (review follow-ups), #16 (14.34 anti-armour
  bans), #17 (seam `up`, R-117, R-118, R-119) merged; `vendor/cna` is at
  cna 4184683.

## Requests for cna

What this repo needs from cna, one bullet each: what, why, which mission it
blocks. cna's autopilot reads this every run and answers with a PR; delete
the bullet once it lands and `vendor/cna` is bumped.

- **Typed set-up constraints for Graziani's Offensive.** The scenario's
  free placements carry `constraint` sentences the engine cannot check
  and the UI can only show (`cna-game::place`, `web/src/SetupPanel.svelte`):
  "Libyan part of sheet C only" and "Libyan part of sheet C; all units of
  the group in one hex" (Axis supply, `sheets: ["C"]`, 59.2); "on sheet C
  or D" (Commonwealth supply, `region: egypt`); "any one hex of map D that
  lies west of Mersa Matruh (D3714); all three units in that one hex
  (60.92)" (Commonwealth supply, `sheets: ["D"]`); and "or Helwan (E1430)"
  on a fixed placement at Cairo, which the engine never offers. Wanted as
  data an engine can check: a country filter on an area (`sheets` and
  `region` together, or a `country` key), "the group in one hex"
  (`together: true` or a group id), "west of a hex" (a column bound), and
  an alternative to a fixed place (`place: ["cairo", "helwan"]` or a
  second locator). Until then the engine trusts the owner, the UI shows
  the prose as "also: …", and Helwan is never offered. Not blocking:
  Mission 3 slice 3 ships with the prose.
- **Which loss base governs for withheld defenders?** `rules/60-combat.md`
  under 15.1x says withheld TOE points count in the defender's loss base;
  under 15.83 (percentage losses) the defender adds withheld units only in
  an overrun. The engine follows 15.83 (`cna-game::combat::assault`). Not
  blocking; Mission 2's follow-ups.

## Runs and quota

| Started | Minutes | Weekly before→after | 5h before→after | Cost | Turns | Result |
|---|---|---|---|---|---|---|
| 2026-09-26 10:53 | 26 | 55→55 | 3→7 | $8.30 | 129 | Mission 1 is complete: all four slices are merged into `main` (PRs #1–#4), and I logged `MISSION 1 COMPLETE` in `PROGRESS.md` and the journal. On `main`, clippy |
| 2026-09-26 13:53 | 0 | 56→56 | 2→2 | $0.17 | 2 | Nothing to do this run: Mission 1 is complete and merged, and the standing orders say to do nothing further after that. - **Feedback:** the section in `PROGRESS |
| 2026-09-26 16:53 | 0 | 59→59 | 21→21 | $0.17 | 2 | I did nothing this run. Mission 1 is complete and merged (PRs #1–#4), and the standing orders say to stop once that happens. The **Feedback** section in `PROGRE |
| 2026-09-26 19:53 | 0 | 60→60 | 1→1 | $0.17 | 2 | I did nothing this run: Mission 1 is complete (PRs #1–#4 merged), and the standing orders say to do nothing more after that. The **Feedback** section in `PROGRE |
| 2026-09-26 22:53 | 0 | 62→62 | 15→15 | $0.17 | 2 | I did nothing this run. Mission 1 is complete: all four slices are merged (PRs #1–#4), and the standing orders say to stop once that happens. - **Feedback:** th |
| 2026-09-27 01:53 | 0 | 63→63 | 4→4 | $0.17 | 2 | I did nothing this run. Mission 1 is complete (PRs #1–#4 merged), and the standing orders say to do nothing more after that. The Feedback section in `PROGRESS.m |
| 2026-09-27 04:53 | 0 | 63→63 | 0→0 | $0.17 | 2 | I did nothing this run. Mission 1 is complete: all four slices are merged (PRs #1–#4), and the standing orders say to stop once that happens. The **Feedback** s |
| 2026-09-27 10:53 | 0 | 67→67 | 4→4 | $0.17 | 2 | I did nothing this run. Mission 1 is complete (PRs #1–#4 are merged), and the standing orders say to do nothing more after that. The **Feedback** section in `PR |
| 2026-09-27 13:53 | 0 | 70→70 | 18→18 | $0.17 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to stop once that happens. -  |
| 2026-09-27 16:53 | 0 | 70→70 | 0→0 | $0.17 | 2 | I did nothing this run: Mission 1 is complete (PRs #1–#4 merged), and the standing orders say to do nothing more after that. - **Feedback:** the section in `PRO |
| 2026-09-27 19:53 | 0 | 70→70 | 2→2 | $0.17 | 2 | I did nothing this run: Mission 1 is already complete, and the standing orders say to do nothing more after that. All four slices are merged (PRs #1–#4), there  |
| 2026-09-27 22:53 | 0 | 71→71 | 3→3 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete, and the standing orders say to stop once that happens. - **Feedback:** the Feedback section in `PROGRESS. |
| 2026-09-28 01:53 | 0 | 72→72 | 0→0 | $0.18 | 2 | I did nothing this run. Mission 1 is already complete: all four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after that. - * |
| 2026-09-28 04:53 | 0 | 72→72 | 0→0 | $0.18 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after t |
| 2026-09-28 07:53 | 0 | 73→73 | 8→8 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete, and the standing orders say to do nothing further after that. All four slices are merged (PRs #1–#4). - * |
| 2026-09-28 10:53 | 0 | 75→75 | 24→24 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-28 13:53 | 0 | 76→76 | 2→3 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-28 16:53 | 0 | 76→76 | 0→0 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-28 19:53 | 0 | 78→78 | 11→11 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-28 22:53 | 0 | 78→78 | 2→2 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-29 01:53 | 0 | 79→79 | 8→8 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-29 04:53 | 0 | 79→79 | 0→0 | $0.19 | 2 | Nothing to do this run. Mission 1 is already complete: all four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after that. The |
| 2026-09-29 07:53 | 0 | 79→79 | 0→0 | $0.18 | 2 | I did nothing this run. Mission 1 is already complete (PRs #1–#4 are merged), and the standing orders say to do nothing more after that. The Feedback section in |
| 2026-09-29 10:53 | 0 | 82→? | 22→? | $0.18 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after t |
| 2026-09-29 13:53 | 0 | 84→84 | 2→2 | $0.19 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-29 16:53 | 0 | 5→5 | 18→19 | $0.19 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The **Feedback** section |
| 2026-09-29 19:53 | 0 | 8→8 | 55→55 | $0.19 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-30 07:53 | 0 | 22→22 | 8→9 | $0.19 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged and `PROGRESS.md` already records `MISSION 1 COMPLETE`. The standing orders say to d |
| 2026-09-30 10:53 | 0 | 24→24 | 33→33 | $0.19 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and the standing orders say to stop once that happens. The Feedback section |
| 2026-09-30 13:53 | 0 | 25→25 | 5→5 | $0.19 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged and `PROGRESS.md` already records `MISSION 1 COMPLETE`. The standing orders say to d |
| 2026-09-30 16:53 | 0 | 28→28 | 39→39 | $0.19 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say to do  |
| 2026-09-30 19:53 | 0 | 4→4 | 4→4 | $0.18 | 2 | Nothing to do this run. Mission 1 finished on 2026-09-26: all four slices are merged as PRs #1–#4, and the standing orders say to do nothing further after that. |
| 2026-09-30 22:53 | 0 | 5→5 | 6→6 | $0.19 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after t |
| 2026-10-01 01:53 | 0 | 5→5 | 0→0 | $0.19 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged as PRs #1–#4, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The |
| 2026-10-01 04:53 | 0 | 5→5 | 0→0 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged as PRs #1–#4, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The |
| 2026-10-01 07:53 | 0 | 5→5 | 0→0 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 10:53 | 0 | 6→6 | 6→6 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 13:53 | 0 | 10→10 | 1→1 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 16:53 | 0 | 16→16 | 37→37 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 19:53 | 0 | 18→18 | 11→11 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 22:53 | 0 | 20→20 | 0→0 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders sa |
| 2026-10-02 01:53 | 0 | 21→21 | 6→6 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-02 04:53 | 0 | 23→23 | 13→13 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-02 07:53 | 0 | 23→23 | 14→14 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-02 10:53 | 0 | 24→24 | 4→4 | $0.18 | 2 | Mission 1 is already complete, so I did nothing this run, as the standing orders require. - **Feedback:** the Feedback section in `PROGRESS.md` is empty, so the |
| 2026-10-02 13:53 | 0 | 24→24 | 0→0 | $0.20 | 2 | I did nothing this run. Mission 1 finished on 2026-09-26 (PRs #1–#4 are merged), and the standing orders say to do nothing more once it is complete. The Feedbac |
| 2026-10-02 16:53 | 0 | 25→25 | 6→6 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders sa |
| 2026-10-02 19:53 | 0 | 26→26 | 4→4 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-02 22:53 | 0 | 28→28 | 13→13 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-03 01:53 | 0 | 28→28 | 3→3 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders sa |
| 2026-10-03 04:53 | 0 | 28→28 | 3→3 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 07:53 | 0 | 28→28 | 0→0 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders |
| 2026-10-03 10:53 | 0 | 29→29 | 2→2 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 13:53 | 0 | 33→33 | 25→25 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 16:53 | 0 | 38→38 | 2→2 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 19:53 | 0 | 43→43 | 31→31 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 22:53 | 0 | 53→53 | 31→31 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-04 01:53 | 0 | 55→55 | 43→43 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-04 04:53 | 0 | 55→55 | 0→0 | $0.23 | 2 | Nothing to do this run. Mission 1 is already complete: PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders say to do  |
| 2026-10-04 07:53 | 0 | 55→55 | 0→0 | $0.22 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders say to do  |
| 2026-10-04 10:53 | 0 | 63→63 | 51→51 | $0.22 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say to do n |
| 2026-10-04 13:53 | 0 | 67→67 | 0→0 | $0.19 | 2 | Feedback is empty, no open autopilot PRs, and Mission 1 is already complete, so I did nothing this run. The standing orders say to stop once all four parts are  |
| 2026-10-04 16:53 | 0 | 74→74 | 40→40 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, the Feedback section is em |
| 2026-10-04 19:53 | 0 | 74→74 | 0→0 | $0.18 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to stop once that's done. - * |
| 2026-10-04 22:53 | 0 | 76→76 | 17→17 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 01:53 | 0 | 3→3 | 30→30 | $0.24 | 2 | Mission 1 is already done, so this run did nothing. All four slices are merged (PRs #1–#4), and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing or |
| 2026-10-05 04:53 | 0 | 3→3 | 33→33 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 07:53 | 0 | 3→3 | 0→1 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 10:53 | 0 | 90→90 | 7→7 | $0.24 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 13:53 | 0 | 14→14 | 60→61 | $0.24 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 16:53 | 0 | 15→15 | 0→0 | $0.24 | 2 | Nothing to do this run. Mission 1 is already complete: PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say to do  |
| 2026-10-05 19:53 | 0 | 93→93 | 11→11 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-05 22:53 | 0 | 24→24 | 14→14 | $0.24 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-06 01:53 | 0 | 98→98 | 0→1 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, so the standing orders  |
| 2026-10-06 04:53 | 0 | 31→31 | 0→0 | $0.25 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, which the standing orde |
| 2026-10-06 07:53 | 0 | 37→37 | 100→100 | $0.00 | 1 | You've hit your session limit · resets 8:20am (America/Phoenix) |
| 2026-10-06 13:53 | 0 | 43→43 | 100→100 | $0.00 | 1 | You've hit your session limit · resets 2:10pm (America/Phoenix) |
| 2026-10-06 16:53 | 0 | 1→1 | 4→4 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, so the standing orders  |
| 2026-10-06 19:53 | 0 | 51→51 | 0→0 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The  |
| 2026-10-06 22:53 | 0 | 55→55 | 49→49 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the  |
| 2026-10-08 13:53 | 12 | 37→37 | 0→3 | $6.23 | 7 | The Mission 2 spec and its six slice plans are written and up for your review in **PR #5**, which is marked ready and labelled `needs-brian`. Per the standing o |
| 2026-10-08 19:53 | 3 | 40→40 | 0→1 | $1.60 | 45 | PR #5 (the Mission 2 spec) is updated with cna's answers to all six of our requests, and it is still waiting for your review. Nothing for Mission 2 has been bui |
| 2026-10-08 22:53 | 0 | 42→42 | 8→8 | $0.43 | 10 | Nothing was built this run. The Mission 2 spec (PR #5) is still waiting on your review, and the standing orders don't allow building until you approve it under  |
| 2026-10-09 01:53 | 1 | 43→43 | 8→8 | $0.43 | 11 | PR #5's CI was failing, and it's green again. Nothing else was built, because Mission 2 is still waiting on your review of the spec. - **Cause:** the previous r |
| 2026-10-09 04:53 | 0 | 43→43 | 0→0 | $0.32 | 6 | Nothing was built this run. Mission 2 is still waiting for your review of PR #5, and the standing orders don't let me build anything until you approve it under  |
| 2026-10-09 07:53 | 0 | 44→44 | 0→1 | $0.31 | 6 | Nothing was built this run. Mission 2 is still waiting on your review of the spec in PR #5, and the standing orders don't let me build anything for it until you |
| 2026-10-09 10:53 | 0 | 44→44 | 0→0 | $0.27 | 4 | I built nothing this run because PR #5, the Mission 2 spec, is still waiting on your review. The standing orders don't let me build anything for Mission 2 until |
| 2026-10-09 13:53 | 0 | 45→45 | 5→5 | $0.29 | 5 | Nothing was built this run. The Mission 2 spec in PR #5 is still waiting on your review, and the standing orders don't let me build anything for Mission 2 until |
| 2026-10-09 16:53 | 0 | 47→47 | 7→7 | $0.29 | 5 | Nothing was built this run. The Mission 2 spec (PR #5) is still waiting on your review, and the standing orders don't allow any Mission 2 work until you approve |
| 2026-10-09 19:53 | 0 | 47→47 | 7→8 | $0.29 | 5 | I didn't build anything this run: the Mission 2 spec (PR #5) is still waiting for your review, and nothing for Mission 2 can start until you approve it under ** |
| 2026-10-09 22:53 | 0 | 48→48 | 7→7 | $0.29 | 5 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting on your review, and the standing orders say nothing for Mission 2 gets built until you app |
| 2026-10-10 01:53 | 0 | 49→49 | 0→0 | $0.28 | 4 | Nothing got built this run. The Mission 2 spec in PR #5 is still waiting on your review, and the standing orders don't let any Mission 2 work start until you ap |
| 2026-10-10 04:53 | 0 | 49→49 | 0→0 | $0.28 | 4 | Nothing was built this run: the Mission 2 spec (PR #5) is still waiting for your review. The standing orders say nothing for Mission 2 gets built until you appr |
| 2026-10-10 07:53 | 0 | 49→49 | 0→0 | $0.29 | 5 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting for your review, and the standing orders don't let me build anything for Mission 2 until y |
| 2026-10-10 10:53 | 0 | 50→50 | 4→4 | $0.28 | 4 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting for your review, and the standing orders say nothing for Mission 2 gets built until you ap |
| 2026-10-10 13:53 | 0 | 50→50 | 0→0 | $0.28 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-10 16:53 | 0 | 51→51 | 4→5 | $0.30 | 5 | Nothing got built this run. The Mission 2 spec (PR #5) is still waiting for your review. The standing orders say no Mission 2 work starts until you approve it u |
| 2026-10-10 19:53 | 0 | 53→53 | 3→3 | $0.30 | 5 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-10 22:53 | 0 | 54→54 | 11→11 | $0.28 | 4 | I didn't build anything this run. The Mission 2 spec in PR #5 is still waiting for your review, and the standing orders don't allow any Mission 2 work until you |
| 2026-10-11 01:53 | 0 | 56→? | 10→? | $0.31 | 5 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 04:53 | 0 | 57→? | 1→? | $0.29 | 4 | I didn't build anything this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until y |
| 2026-10-11 07:53 | 0 | 57→? | 2→? | $0.24 | 5 | I built nothing this run, because Mission 2 is still waiting for your review of PR #5. The standing orders say to build nothing for that mission until you appro |
| 2026-10-11 10:53 | 0 | 57→57 | 2→2 | $0.28 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 13:53 | 0 | 58→58 | 6→7 | $0.29 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 19:53 | 0 | 58→? | 2→? | $0.30 | 5 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 22:53 | 0 | 61→61 | 18→18 | $0.31 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting on your review. The standing orders don't allow any Mission 2 work until you approv |
| 2026-10-12 01:53 | 0 | 61→? | 0→? | $0.29 | 5 | I built nothing this run. The Mission 2 spec (PR #5) still has no approval under **Feedback**, and the standing orders say nothing for Mission 2 gets built unti |
| 2026-10-12 04:53 | 0 | 61→? | 0→? | $0.27 | 4 | Brian, I built nothing this run. Mission 2 is still waiting on your review of its spec, PR #5, and the standing orders don't allow building it until you approve |
| 2026-10-12 07:53 | 0 | 63→? | 11→? | $0.25 | 5 | I didn't build anything this run. Mission 2 is still waiting for your review of the spec in PR #5, and the standing orders say nothing gets built until you appr |
| 2026-10-12 10:53 | 0 | 64→64 | 22→22 | $0.35 | 6 | Brian, nothing got built this run. The Mission 2 spec in PR #5 still has no approval under **Feedback**, and the standing orders don't allow any Mission 2 work  |
| 2026-10-12 13:53 | 0 | 65→65 | 3→3 | $0.32 | 4 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting for your review, and the standing orders don't allow any Mission 2 work until you approve  |
| 2026-10-12 16:53 | 0 | 66→? | 0→? | $0.31 | 4 | I built nothing this run: the Mission 2 spec (PR #5) still needs your review, and the standing orders don't allow any Mission 2 work until you approve it under  |
| 2026-10-13 08:53 | 29 | 1→4 | 10→68 | $4.03 | 3 | Brian, the Mission 4 spec is up as **PR #25**, labelled `needs-brian`, and nothing for Mission 4 will be built until you approve it under **Feedback**. **What's |
