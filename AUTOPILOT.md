# AUTOPILOT.md — standing orders for unattended runs

Read by `~/Scripts/claude-autopilot.sh`, which cron launches every few hours
when the Claude plan has headroom. Each run is a fresh session with no memory
of the last one; this file plus the journal is the only continuity.

## Mission

**Mission 1, issued 2026-09-26: the rules core and ruling probes.**

- **Spec:** `docs/designs/2026-09-26-engine-probes-design.md` — read it first.
- **Plan:** `docs/plans/2026-09-26-slice-1-close-assault.md` — follow it task
  by task (superpowers:executing-plans if available). Where the plan and the
  spec disagree, follow the spec and say so in the journal.

Parts, in order, one PR each:

1. **Slice 1 — close assault** (plan Tasks 1–9). Branch `autopilot/slice-1`.
2. **Slice 2 — barrage and anti-armour** (R-009, R-015). First write
   `docs/plans/<date>-slice-2-barrage-anti-armour.md` in the same shape as
   the slice 1 plan (read `vendor/cna/rules/60-combat.md` and the tables it
   names), commit it, then execute it. Branch `autopilot/slice-2`.
3. **Slice 3 — cohesion** (R-001, the Morale Modifier "no cell for 56"
   oddity). Same procedure. Branch `autopilot/slice-3`.
4. **Slice 4 — construction costs** (R-018 and the chart-vs-text cost
   disagreements listed as `chart-vs-text` in
   `vendor/cna/docs/autopilot/decisions.json`). Same procedure.

When all four are merged: log `MISSION 1 COMPLETE` in the journal and
`PROGRESS.md` and do nothing further.

## Picking up where the last run left off

You are in a dedicated clone owned by the autopilot, on `main`, freshly reset
to `origin/main`. In order:

1. `git submodule update --init` (the clone does not fetch `vendor/cna`).
2. Read `docs/autopilot/PROGRESS.md`. Its **Feedback** section is Brian's
   voice: act on every item first, then move it to **Addressed** with a
   one-line reply. If an item changes the mission, it wins over this file.
3. `gh pr list --state open --label autopilot` — if an autopilot PR is open,
   check out its branch, read the last entry of `docs/autopilot/JOURNAL.md`
   on it, and continue. If its gates and CI already pass, merge it
   (`gh pr merge --merge`) and move on.
4. Otherwise start the next unfinished part, branching from `origin/main`.

## Working rules

- Keep going until the wall-clock budget in the prompt is nearly spent.
- TDD: every step in the plan that says "write the failing test" means it.
- Commit small, push after every commit. Draft the PR
  (`gh pr create --draft --label autopilot`) after the first commit; `gh pr
  ready` only when `cargo clippy --all-targets -- -D warnings`, `cargo test`
  and `cargo run -p cna-probe -- check` pass locally.
- Never touch `main` directly except `docs/autopilot/PROGRESS.md`; never
  force-push; never rewrite history.
- **The `cna` repo is read-only from here**, with one exception the plan
  names (an undeclared close-assault gap or overlap): that is a small PR on
  `basmith7/cna` labelled `autopilot`, and the submodule bump follows it.
  Bumping `vendor/cna` to a newer `cna` main is allowed at the start of a
  part; regenerate `probes/` in the same commit.
- No SPI rule text in this repo, ever; cite case numbers.
- Commit messages end with the attribution trailer the harness gives you.

## Handing off

Before the budget runs out:

1. **Update `docs/autopilot/PROGRESS.md` on `main`** (commit directly, it is
   a doc): rewrite **Status** and **Next steps**; file replies under
   **Addressed**. Do not touch **Runs and quota**; the cron script fills it.
2. **Append to `docs/autopilot/JOURNAL.md`** on the working branch:

```
## <timestamp, local time> — <branch>
Done: <what landed, with commit shas>
In flight: <what is half-done and where>
Next: <the first concrete thing the next run should do>
Blocked: <anything only Brian can resolve, or "none">
```

Keep entries under ten lines.
