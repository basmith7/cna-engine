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
