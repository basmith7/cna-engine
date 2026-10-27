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

**MISSION 5** (the rest of the Land Game turn, started ahead of Mission 4
on 2026-10-13 while Brian reviews #25) is built for Graziani's Offensive
bar a short list (`docs/designs/2026-10-13-land-turn-design.md`, Status).
Merged through 2026-10-26: supply, spending, supply movement, arrivals,
the air lift, breakdown and repair, attachment, prisoners, guards,
replacements, construction, convoy bombing, gun losses, shells, the
stack panel's organisation, formations in the size table, every
scenario smoke-tested, points victories, the browser's scenario picker
(#31–#64), Rommel as a commander piece and replacement counters at
set-up (#65), 31.0's initiative, recall and morale (#66). **On
2026-10-26, fourth run:** cna answered both requests (R-136, R-137).
#67 bumps `vendor/cna` to fc78330 and places `apart` blocks, so
Rommel's Arrival and both campaigns set up: all nine scenarios now set
up and play a Game-Turn, and the browser offers every one. #68 builds
the rest of 31.0 bar the LRDG raid: the +5 CPA escort (R-137), his
reactions, and his free retreat through enemy ZOC (merged). #69
(merged) lets a side take back a piece during its set-up. **Fifth
run:** cna answered the lone-Rommel request (R-138); #70 (merged) bumps
`vendor/cna` to fcf8ea7 and builds it: the mover's own ZOC no longer
bars his reaction, and alone in an enemy ZOC with no escape left he is
captured. New requests: whose ZOC 8.53 means for every unit, which
19.3 chart components count toward a shell, and Rommel after his stack
is destroyed. #71 (merged) adds the last assignment tests; #72 (merged)
makes the reach search 3x faster under wasm (Mission 3's slow shading).
**Sixth run:** cna answered all three requests (R-139, R-140, R-141).
#73 bumps `vendor/cna` to eafa8a3 and builds R-139: a reaction is barred
only by an enemy ZOC the unit stood in before the mover arrived, so the
mover's own ZOC no longer pins. #74 (R-140): Rommel whose stack an
assault destroys retreats alone at once, captured if he stops short of
safety. #75 (R-141): shells are judged against the 19.3 chart's counted
components; in Graziani's Offensive the 2nd NZ Division and the
Trivioli tank regiment now start as shells.
Mission 4 resumes first the moment **Feedback** approves #25.

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

PR #12's review follow-ups are all closed: 15.84/14.14 gun losses (#54),
18.2x's released units (#53 and earlier), 10.36's must-attack Pass (#27),
the rule tests (15.36, 6.26, 13.1, 12.44 and 12.1x already had them;
#71, merged, adds 15.1x and 13.28).

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
  selecting an uncapped motorised stack took ~0.6 s to shade (#72 cut
  the engine's share 3x; not yet re-timed in a browser); at 390 px
  the end dialog's table clips its right-hand columns (it scrolls).
- Mission 5: merge #74 (R-140) and #75 (R-141) on green CI if this
  run did not. Then, for Graziani's: substitution (19.x, the chart's
  slots filled by like units), or Axis convoy lanes beyond Tripoli
  (ports as destinations, 56.15, 55.17, a new player order); or the
  desert raiders (§27) that the raid on Rommel needs, which want
  hidden movement and so Mission 4's per-side views.
- Shading after #72, timed 2026-10-26 in a Playwright test build:
  click to shaded ~0.5 s for the LXII tank battalion (631, 513, 491,
  494 ms), of which the engine is ~141 ms. The rest is the UI's; profile
  the overlay render before touching the engine again.
- Look at #75's two new starting shells (2nd NZ Division, the Trivioli
  tank regiment at Bardia): they follow the OA and the 19.3 chart, but
  change Graziani's opening.
- PR #23's review follow-ups are all merged: hostile saves refused
  (#26), the curtain on Load and Continue (#28), and 10.36's must-attack
  Pass (#27); #30 fixed the flaky wheel-zoom budget in `map.spec.ts`.
- Since Mission 2: #13, #14 (review follow-ups), #16 (14.34 anti-armour
  bans), #17 (seam `up`, R-117, R-118, R-119) merged; `vendor/cna` is at
  cna fcf8ea7.

## Requests for cna

What this repo needs from cna, one bullet each: what, why, which mission it
blocks. cna's autopilot reads this every run and answers with a PR; delete
the bullet once it lands and `vendor/cna` is bumped.

None open.

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
| 2026-10-13 12:53 | 11 | 6→7 | 93→100 | $2.51 | 1 | You've hit your session limit · resets 1:30pm (America/Phoenix) |
| 2026-10-24 08:53 | 67 | 20→21 | 0→4 | $15.17 | 246 | Mission 5 slice 2 is merged (PR #32). Slice 3 (supply movement) is about half built on draft PR #34. Everything is pushed and the gates pass locally: clippy, `c |
| 2026-10-24 10:53 | 75 | 21→22 | 4→11 | $18.97 | 3 | Rule 21.41 is done and pushed on PR #35 (fd815bb). If a unit's move began next to an enemy brigade-sized unit, or two hexes from one across an open hex, half of |
| 2026-10-25 16:53 | 69 | 27→29 | 0→10 | $16.44 | 3 | All three steps were already done in the last turn, about 20 minutes before the 01:23Z deadline: - **Playwright on #37:** `/tmp/pw-s4.txt` shows 33 passed (8.7m |
| 2026-10-25 20:53 | 71 | 30→31 | 18→2 | $13.68 | 2 | Both CI runs for #44's last commit have now finished green (test and web in each). That covers the duplicate run that was still pending when I merged, so the me |
| 2026-10-25 22:53 | 66 | 31→33 | 3→13 | $12.34 | 197 | Two Mission 5 slices merged this run, #45 and #46. The third, slice 8, is planned on draft PR #47 but nothing is built for it yet. - **#45, slice 6b (prisoners, |
| 2026-10-26 00:53 | 82 | 33→34 | 15→18 | $13.53 | 209 | Four PRs merged this run: slice 8 (prisoner guards and escape) and all three parts of slice 9 (fortifications and minefields). Mission 5 keeps moving while Miss |
| 2026-10-26 02:53 | 85 | 34→35 | 0→7 | $14.67 | 201 | Ten PRs merged this run (#51–#58, #60). Main passes the full Rust suite after the last merge. The only open autopilot PR besides the Mission 4 spec is draft #59 |
| 2026-10-26 04:53 | 82 | 35→35 | 7→12 | $9.19 | 157 | I merged five PRs this run (#59, #61, #62, #63, #64) and nothing is open at handoff. On the merged `main`, clippy and 511 tests passed locally; #62's later chan |
| 2026-10-26 06:53 | 68 | 35→? | 12→? | $9.88 | 4 | The Last Chance now sets up and plays in the engine and the browser, which makes six of the nine scenarios playable. Rommel is in the game with his initiative,  |
| 2026-10-26 09:53 | 74 | 36→37 | 4→11 | $10.94 | 3 | This run merged three Mission 5 PRs (#67, #68, #69). All nine scenarios now set up and play a Game-Turn, and the browser offers every one of them. - **#67:** cn |
| 2026-10-26 11:53 | 61 | 38→39 | 16→5 | $11.62 | 206 | I merged three PRs this run: the new Rommel ruling, two missing rule tests, and a 3× faster move search. Mission 4 is still waiting on your review of the spec,  |
