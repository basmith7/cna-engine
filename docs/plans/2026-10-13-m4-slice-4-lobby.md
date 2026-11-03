# Mission 4, slice 4 (lobby and the network client) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two people sign in, meet in the lobby, start a game and play
it in their own browsers, each seeing only their side, while hot-seat
still works with no server.

**Spec:** `docs/designs/2026-10-13-multiplayer-server-design.md`
**Depends on:** slice 3 merged.

**Proving screenshots:** `slice-4/lobby.png` (my games with whose move,
open games, a create form) and `slice-4/two-players.png` (the two
contexts side by side mid-barrage: the Commonwealth panel shows none of
the Axis fires and the Axis counters' strengths as "?").

## Rules for every task

- As slice 3, plus Mission 3's screenshot rules: Playwright at 1440×900
  (and one 390×844) into `~/Projects/cna-m4-shots/slice-4/`, run the
  **screenshot-critique** skill on visual changes and paste its verdict.

## Tasks

### Task 1: the lobby API

- [ ] Failing tests: `POST /api/games {side: "axis" | "cw" | "either",
  open: bool, open_information: bool}` → a game in `open` with the
  creator seated (either = the creator gets Axis when someone joins as
  Commonwealth and vice versa; decided at join) and an invite token
  (shown once, stored hashed); `GET /api/games?open` lists open public
  games without invite tokens; `POST /api/games/{id}/join {side,
  invite?}` seats the caller, `active` once both seats are taken; joining
  your own game, a taken side, or a private game without the token is
  refused; `GET /api/games?mine` returns each game with `my_side`,
  `to_act`, `my_move: bool`, the clock, `status`, the opponent's name;
  `POST /api/games/{id}/leave` → `abandoned`; the daily sweep marks a
  60-day-idle game `abandoned` (test with an injected clock).
- [ ] Implement. Commit.

### Task 2: the `Engine` interface

- [ ] Refactor `web/src/engine.ts`: `interface Engine` with the current
  methods made `async` (`view`, `apply`, `placements`, `reachable`,
  `sealedOptions`, `preview`, `save`), `LocalEngine` wrapping wasm, and
  every call site awaiting. Mission 3's Vitest and Playwright suites pass
  unchanged (the hot-seat golden game included). Commit.
- [ ] Fields the enemy view lacks render as "?" in the stack panel, the
  counter (strength) and the combat panels; types in `types.ts` make them
  optional; a Vitest per component on an enemy piece without `toe`.
  Commit.

### Task 3: `NetworkEngine`

- [ ] `NetworkEngine(gameId)`: `fetch` against the API (JSON,
  `credentials: "same-origin"`), an `EventSource` on `/stream` that
  replaces the view whole on each message and appends its events to the
  log; `apply` sends `seq` and, on 409, refetches and reports "the game
  moved on". No curtain in network games; the top bar says "Your move"
  or "Waiting for <name>". Vitest with a mocked fetch/EventSource.
  Commit.

### Task 4: screens

- [ ] Start screen: "Play online" (sign in with Discord, or the lobby if
  signed in) and "Hot-seat on this computer" (Mission 3's start screen).
  Lobby: my games (whose move first), open games with Join, Create
  (side, public/private, open information), invite link copy. Game page
  at `/game/{id}`; invite links `/join/{id}?invite=…`. Settings: alerts
  on/off (wired in slice 5; disabled with a note until then). Admin:
  the account list with suspend/restore, shown only to admins.
- [ ] The server serves `web/dist` (`CNA_WEB_DIST`) with an SPA fallback
  for these paths. Commit.

### Task 5: Playwright, two players

- [ ] A Playwright project `multiplayer` whose `webServer` is the
  compose stack (Postgres, fake Discord, server serving a fresh
  `web/dist`), `globalSetup` waiting on `/healthz`.
- [ ] Spec `two-players.spec.ts`: contexts `alice` and `bob` sign in
  through the fake; alice creates a private game as Axis with the golden
  seed (a `test-auth` route seeds it), bob joins by invite; they play
  golden orders 0–76 by clicks, each order in the context of the side it
  names (reuse `web/tests/helpers.ts`' drivers, parameterised by page);
  every response and SSE message bob's context receives is checked by
  the leak predicate (port it to TS or ask the server's
  `test-auth`-only `/test/leaks` endpoint); at the end the server's state
  hash for those 77 orders (from a `test-auth`-only
  `GET /test/games/{id}/hash`) equals the hash `replay` gives (pinned as a
  fixture by a `cna-server` test).
- [ ] CI: the `web` job gets a Postgres service and runs the
  `multiplayer` project after the hot-seat one. Commit; screenshots and
  critique; PR ready.
