# Autopilot progress

Human-facing journal for the unattended runs (see `AUTOPILOT.md`). The
autopilot rewrites **Status** and **Next steps** at the end of every run and
the cron script appends to **Runs and quota**. The only section a human
writes is **Feedback**.

## Feedback


## Addressed

## Status

**MISSION 1 COMPLETE** (2026-09-26). All four slices merged: a Rust rules
core with switches for R-001, R-009, R-011, R-012, R-015 and R-018, and
eight probes in `probes/` that render on the decision board with
`python3 tools/decisions_page.py --probes <cna-engine>/probes`.

| Part | What | State | PR |
|---|---|---|---|
| 1 | Slice 1: close assault (R-011, R-012, oddities) | merged | #1 |
| 2 | Slice 2: barrage and anti-armour (R-009, R-015) | merged | #2 |
| 3 | Slice 3: cohesion (R-001) | merged | #3 |
| 4 | Slice 4: construction costs (R-018, chart vs text) | merged | #4 |

**Found (slice 1):** under R-012 option 2, fighting costs the defender less
than the 30 % buy-out on average in every column, but only by 0.6 points at
+17 (table percentage only). Under R-011 option 2 a 1:4 attacker's expected
loss goes from 10 % to 51 % of its strength. The defender +2 gap (34–36) is
8.3 % of rolls in that column; the attacker −2 "13-18" cell cannot occur.
No undeclared gaps or overlaps in the close-assault table.

**Found (slice 2):** under R-009 option 2 the reply barrage against phasing
infantry in a level-two fortification pins or destroys up to 39 points
more often (3-4 points: 39 % against 0 %). Under R-015, for armour
assaulting out of rough ground up a slope, option 1 cuts defensive
anti-armour damage by 12 % and option 3 by 24 % against option 2.

**Found (slice 3):** a unit alternating a 4-DP push with a stage of rest
never collapses under R-001 option 1 (level never below −4) but collapses
at stage 13 under option 2 with no reset rule. The Morale Modifier gap
(level −4, reading 56) is 2.8 % of rolls at that level. No undeclared gaps
in the morale table.

**Found (slice 4):** R-018's options differ only on a dummy dump's CP (3 or
2) and a real dump's stores (20 or 10); the engine defaults to option 3
(charts' CP, text's stores), following the ruling's Rationale. On the
guarded-dump raid check the chart's "at least" beats the text's "above" by
up to 16.7 points (defence 7). The text's temporary repair facility costs
3x the chart's fuel and time; its facility rebuild 3x the fuel.

## Next steps

Mission 1 is done; the autopilot does nothing further until a new mission
is issued. Candidates for Brian: (a) the spec's `cna`-side decision policy
(the autopilot writing Decisions for the probed rulings in `cna`), which
Mission 1 kept read-only; (b) R-001 already has a Decision in `cna` but its
status is still `proposed`.

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
