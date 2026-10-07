# AUTOPILOT.md — standing orders for unattended runs

Read by `~/Scripts/claude-autopilot.sh`, which cron launches every few hours
when the Claude plan has headroom. Each run is a fresh session with no memory
of the last one; this file plus the journal is the only continuity.

## The goal

This repo and `basmith7/cna` share one goal, and every mission in either
repo serves it:

> **An open-source, self-hostable digital *Campaign for North Africa*.**
> Two players play a scenario, and in the end the full campaign, in a
> browser, against each other, with the engine enforcing every rule. Each
> ruling is a switch, so a group picks its ruleset.

The two repos split the work:

- **cna** is the rulebook: every SPI rule restated in our own
  words, every chart, map hex, scenario and OA sheet as data, and every
  ambiguity decided as a ruling. It is done when an engine can build the
  whole game from it without opening the SPI books. No engine code there.
- **cna-engine (this repo)** builds the game from cna: rules core, game
  state, turn sequence, then a server and browser client. It reads cna as a
  submodule and never copies its prose.

This repo is the critical path to the goal. Each mission should leave the
game measurably closer to playable.

## Mission

The autopilot works through a **queue** of missions (issued 2026-10-07).
Mission 1 took one day and then left the autopilot idle for over a week, so
finishing a mission now means starting the next one in the same run. A
`COMPLETE` entry in the journal or `PROGRESS.md` is never a reason to idle.

Brian's delegation of 2026-09-26 stands: decide anything cheap to reverse
yourself (architecture, crate layout, libraries, UI design). Anything Brian
writes in **Feedback** wins.

### Queue

This follows the *Path to a playable game* Brian agreed on 2026-10-07: one
small scenario in cna, a headless game state for it here, then a minimal UI.

| # | Mission | Needs from cna | State |
|---|---|---|---|
| 1 | Rules core and ruling probes (combat, cohesion, construction) | | done |
| 2 | **Game state, headless, for one small scenario.** Load `vendor/cna/data/map/`; place the scenario's units as typed state; enforce the turn sequence (§5, §7) and movement (CP and CPA §6, terrain §8, stacking and ZOC §9–10, §18); combat from the existing rules core. Scripted orders in, illegal ones rejected, a replayable log out; the state serialises to JSON and back. No AI, no UI. **Brian reviews the spec before any building** (below) | one small scenario: set-up, OA, victory conditions (cna Mission 6, Part 1) | next |
| 3 | **A minimal UI** on top of Mission 2's game state: the map, the units, legal moves shown from the rules core, orders entered by clicking. Local only | | queued |

**Later, in no fixed order.** When Mission 3 is done, take the one that
moves the game closest to **The goal**, add it as a row with its own spec,
and go on: the rest of the Land Game turn (organisation, engineering,
special rules, abstract supply §19–32); the Logistics Game (§48–58) as a
module behind one switch, with the water and fuel probes; the Air Game
(§33–47), once cna has restated it; the remaining scenarios and the
campaign games; a server for two players at a distance; AI players. A
mission that Brian would call large gets the same review as Mission 2.

The current mission is the first row not marked `done`. If it needs cna
data that is not there yet, post a request (below); Mission 2's spec and
its map and state slices do not need the scenario and can start now. When a
mission's last part merges: log `MISSION n COMPLETE` in the journal and
`PROGRESS.md`, set its row to `done` and the next to `next`, and go straight
on to the next mission's Part 0.

### Every mission's parts

0. **Spec and plan**, one PR, branch `autopilot/m2-spec` (and so on). Write
   `docs/designs/YYYY-MM-DD-slug-design.md` and, per slice,
   `docs/plans/YYYY-MM-DD-slug.md`, in the shape of
   `docs/designs/2026-09-26-engine-probes-design.md` and the slice plans:
   goal (how it moves **The goal** forward), non-goals, a decisions table,
   crates touched, testing, slices one PR each. Decisions are yours under
   the delegation and must be reversible; say so in the spec's header.
   Then follow the plans task by task. Where a plan and its spec disagree,
   follow the spec and say so in the journal.
   - **Brian's review (Mission 2, and any mission marked for it).** Mark the
     spec PR ready, label it `needs-brian`, and do not merge it. Put one line
     in `PROGRESS.md` **Next steps**: "Review the Mission N spec: PR #n".
     Merge only when Brian's **Feedback** approves it; if he asks for
     changes, make them on the same PR and wait again. Until then, build
     nothing for that mission; post any **Requests for cna** it needs, then
     stop. GitHub approval cannot work here (the autopilot pushes as Brian),
     so Feedback is the only signal.
   - Every other spec: merge it once the gates pass.
1. **Slices**, one PR each, TDD, until the mission's spec is met.
2. **Docs**: README, `NOT_SIMULATED.md`, design Status, this queue.

### Requests for cna

The rules repo works through its own queue, but takes requests from here
first. When this repo needs something from cna (a missing table, data in
the wrong shape, a rule too vague to implement), add one bullet to the
**Requests for cna** section of `docs/autopilot/PROGRESS.md` on `main`:
what is needed, why, and which mission it blocks. cna's autopilot reads
that section every run and answers with a PR on cna. When it lands, bump
`vendor/cna` and delete the bullet. Do not edit cna's rules yourself.

### Limits

Stay inside this repo. Do not deploy or host anything: hosting is Brian's
call.

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
   (`gh pr merge --merge`) and move on, unless it is labelled
   `needs-brian` and **Feedback** has not approved it.
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
- **The `cna` repo is read-only from here**: ask for changes under
  **Requests for cna**. Bumping `vendor/cna` to a newer `cna` main is allowed at the start of a
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
