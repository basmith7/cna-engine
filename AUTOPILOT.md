# AUTOPILOT.md — standing orders for unattended runs

Read by `~/Scripts/claude-autopilot.sh`, which cron launches every few hours
when the Claude plan has headroom. Each run is a fresh session with no memory
of the last one; this file plus the journal is the only continuity. Read `ARCHITECTURE.md` to orient in the code.

## First: the shared protocol in cna

Before anything else, read cna's `AUTOPILOT.md` from its `main`:

```
gh api repos/basmith7/cna/contents/AUTOPILOT.md --jq .content | base64 -d
```

Its **The goal**, **Picking up where the last run left off**, **Working
rules** and **Handing off** bind this repo too; the rest of it is cna's own.
This file adds cna-engine's missions and specifics, and wins where the two
differ.

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
| 2 | **Game state, headless, for one small scenario.** Load `vendor/cna/data/map/`; place the scenario's units as typed state; enforce the turn sequence (§5, §7) and movement (CP and CPA §6, terrain §8, stacking and ZOC §9–10, §18); combat from the existing rules core. Scripted orders in, illegal ones rejected, a replayable log out; the state serialises to JSON and back. No AI, no UI. **Brian reviews the spec before any building** (below) | one small scenario: set-up, OA, victory conditions (cna Mission 6, Part 1) | done |
| 3 | **A minimal UI** on top of Mission 2's game state: the map, the units, legal moves shown from the rules core, orders entered by clicking. Local only | | done |
| 4 | **A multiplayer server** (Brian, 2026-10-13): real accounts and many games at once, played at a distance. A Rust server (axum or similar) runs `cna-game` natively and is the only holder of the full state; each client gets its own side's redacted view over HTTP plus a live channel (WebSocket or SSE), so sealed orders are secret for real, not hot-seat. Accounts: **Discord sign-in** (OAuth2, its own Discord application, not the Pikachu one shared with arcs), **open sign-up** (so rate limits on sign-up and game creation, and an admin who can suspend an account). A lobby: create a game, invite or open it, pick a side, a list of my games showing whose move it is. **Turn alerts by Discord DM** from a bot when it becomes your move (opt-out per account). Postgres for accounts, games and the order log (a game is its scenario plus its orders, replayed; snapshots for speed). The `web/` UI gains sign-in, the lobby and a network engine beside the local wasm one, which stays for hot-seat. Ships the way Brian's own apps do (`Vault-13/Infrastructure.md`, *App deployments*): a Docker image on a tag, Tower, Pangolin at `cna.basmith.net`. The autopilot builds and tests everything locally (compose with Postgres, a fake OAuth provider); the outside steps (the Discord application and bot, secrets on Tower, the Pangolin resource, DNS) are written up in the spec as Brian's checklist and not done unattended. **Brian reviews the spec before any building** | | next |
| 5 | **The rest of the Land Game turn** for Graziani's Offensive and then every scenario's land rules: organisation (§20: attachment, detachment, replacements), breakdown (§21), engineering and construction (§24–26), the special rules (§28–31), abstract supply (§19, §32) with supply units that matter. Each lands in `cna-game` behind the existing phase and segment skips and shows up in the UI's panels, so the game stops passing through phases C, D, H, J, K and L. The spec names the order and what each needs from cna | the Land Game sections restated in cna; check each one and post requests for the gaps | |

**Later, in no fixed order.** When Mission 5 is done, take the one that
moves the game closest to **The goal**, add it as a row with its own spec,
and go on: the Logistics Game (§48–58) as a module behind one switch, with
the water and fuel probes; the Air Game (§33–47), once cna has restated it;
the remaining scenarios and the campaign games; AI players. A mission that
Brian would call large gets the same review as Mission 2.

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

## Specifics for this repo

- **Start of run:** `git submodule update --init` first; the clone does not
  fetch `vendor/cna`.
- **Gates** for `gh pr ready`: `cargo clippy --all-targets -- -D warnings`,
  `cargo test` and `cargo run -p cna-probe -- check`.
- **The `cna` repo is read-only from here**: ask for changes under
  **Requests for cna**. Bumping `vendor/cna` to a newer `cna` main is
  allowed at the start of a part; regenerate `probes/` in the same commit.
