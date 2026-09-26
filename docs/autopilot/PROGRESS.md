# Autopilot progress

Human-facing journal for the unattended runs (see `AUTOPILOT.md`). The
autopilot rewrites **Status** and **Next steps** at the end of every run and
the cron script appends to **Runs and quota**. The only section a human
writes is **Feedback**.

## Feedback


## Addressed

## Status

Slice 1 merged 2026-09-26: Rust workspace, close-assault table with errata,
exact dice distributions, switches for R-011 and R-012, and probes
`R-011`, `R-012` and `chart-oddities` (they render on the board with
`--probes`). Slice 2 in progress.

| Part | What | State | PR |
|---|---|---|---|
| 1 | Slice 1: close assault (R-011, R-012, oddities) | merged | #1 |
| 2 | Slice 2: barrage and anti-armour (R-009, R-015) | in progress | |
| 3 | Slice 3: cohesion (R-001) | not started | |
| 4 | Slice 4: construction costs (R-018) | not started | |

**Found (slice 1):** under R-012 option 2, fighting costs the defender less
than the 30 % buy-out on average in every column, but only by 0.6 points at
+17 (table percentage only). Under R-011 option 2 a 1:4 attacker's expected
loss goes from 10 % to 51 % of its strength. The defender +2 gap (34–36) is
8.3 % of rolls in that column; the attacker −2 "13-18" cell cannot occur.
No undeclared gaps or overlaps in the close-assault table.

## Next steps

For the autopilot: continue slice 2 on `autopilot/slice-2` (plan
`docs/plans/2026-09-26-slice-2-barrage-anti-armour.md`).

## Runs and quota

| Started | Minutes | Weekly before→after | 5h before→after | Cost | Turns | Result |
|---|---|---|---|---|---|---|
