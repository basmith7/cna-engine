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

| # | Mission | Needs from cna | State |
|---|---|---|---|
| 1 | Rules core and ruling probes (combat, cohesion, construction) | | done |
| 2 | **Map and game state**: load `vendor/cna/data/map/` (hexes, hexsides, places); units, formations and markers as typed state; a whole game state that serialises to JSON and back | map (done) | next |
| 3 | **Movement**: CP and CPA (§6), movement and terrain costs (§8), stacking and ZOC (§9–10, §18), legal-move generation on the real map. Switches for R-002–R-005 and R-010 where they change a number or a legal move | | queued |
| 4 | **One Land Game turn**: the sequence of play (§5, §7) as a state machine; combat (§11–16) wired to the rules core; organisation, engineering and special rules (§19–31); abstract supply (§32). A whole Game-Turn runs from scripted orders, rejects illegal ones, and writes a replayable log. Plan it as several slices | | queued |
| 5 | **First playable scenario, headless**: load the smallest scenario from cna's data, place every unit, play it to its victory check from scripted orders; save and load | scenarios and OA (cna Mission 6) | queued |
| 6 | **Server and browser client**: one self-hosted server (one binary or one container) and a browser client, so two people play that scenario hotseat or turn by turn, with legal moves shown from the rules core | | queued |
| 7 | **The Logistics Game** (§48–58) as a module behind one switch; the water and fuel probes; switches for the logistics rulings | logistics (done) | queued |
| 8 | **The Air Game** (§33–47) as a module behind one switch | Air Game (cna Mission 7) | queued |
| 9 | **The full campaign**: every scenario and campaign game set-up, and a whole campaign played start to finish from scripted orders | | queued |

The current mission is the first row not marked `done`. If it needs cna
data that is not there yet, post a request (below) and take the next row
that can start; come back when the data lands. When a mission's last part
merges: log `MISSION n COMPLETE` in the journal and `PROGRESS.md`, set its
row to `done` and the next to `next`, and go straight on to the next
mission's Part 0.

### Every mission's parts

0. **Spec and plan**, one PR, branch `autopilot/m2-spec` (and so on). Write
   `docs/designs/YYYY-MM-DD-slug-design.md` and, per slice,
   `docs/plans/YYYY-MM-DD-slug.md`, in the shape of
   `docs/designs/2026-09-26-engine-probes-design.md` and the slice plans:
   goal (how it moves **The goal** forward), non-goals, a decisions table,
   crates touched, testing, slices one PR each. Every decision is yours under
   the delegation and must be reversible; say so in the spec's header.
   Merge it once the gates pass, then follow the plans task by task. Where a
   plan and its spec disagree, follow the spec and say so in the journal.
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

### After the queue

When Mission 9 is complete the game is playable end to end. Until then, if
the queue runs dry because the rows above turn out smaller than expected,
add the next mission yourself: the largest gap left between this repo and
**The goal** (a rule that is not enforced, a part of play the client cannot
do), one row and one spec. Stay inside this repo; do not deploy or host
anything; hosting is Brian's call.

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
