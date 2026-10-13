# Mission 4, slice 5 (turn alerts) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When it becomes your move, a bot DMs you on Discord with a
link, once, reliably, unless you turned alerts off.

**Spec:** `docs/designs/2026-10-13-multiplayer-server-design.md`
(*Turn alerts*, question 1)
**Depends on:** slice 4 merged.

## Rules for every task

- As slice 4. The bot token in tests is generated at run time; the fake
  Discord checks it is sent as `Authorization: Bot <token>`.

## Tasks

### Task 1: joining the CNA server

- [ ] Failing test: at sign-in with alerts on, the server calls `PUT
  /guilds/{CNA_DISCORD_GUILD_ID}/members/{user}` with the user's access
  token and the bot token (the fake records it); with alerts off it does
  not; a failure there does not fail sign-in. Turning alerts off later
  removes nothing (Discord's member list is the user's to leave) but
  stops all DMs; the settings page says so. If Brian's review picked
  `identify` only, skip this task and say so in the PR.
- [ ] Implement. Commit.

### Task 2: the outbox

- [ ] Migration `alerts` (id, account_id, game_id, seq, created_at,
  sent_at, attempts, next_attempt_at, last_error). Failing tests on
  `Games::apply`: an order that changes `to_act` to a seat with alerts on
  inserts one row in the same transaction; an illegal order or a rolled
  back transaction inserts none; a change to the same account within 15
  minutes of an unsent or sent alert for that game inserts none; none
  while that account has an open SSE stream on the game (the registry
  counts streams per seat).
- [ ] Implement. Commit.

### Task 3: the sender

- [ ] A worker task: every 5 s, take due rows (`FOR UPDATE SKIP
  LOCKED`), open the DM channel (`POST /users/@me/channels {recipient_id}`),
  post the message ("Your move in Graziani's Offensive against <name>,
  Game-Turn <n> OpStage <m>: <public_url>/game/<id>"), set `sent_at`;
  on 429 respect `retry_after`; on other errors back off (30 s doubling,
  give up after 24 h); on 403 (DMs closed) set the account's
  `alerts_unreachable_at` and stop retrying that row.
- [ ] Tests against the fake (it can be told to answer 403 or 429 for a
  user). Commit.

### Task 4: the UI and e2e

- [ ] Settings: the alerts toggle (`PATCH /api/me {alerts}`); the lobby
  shows "Discord alerts can't reach you: allow DMs from server members"
  when `alerts_unreachable_at` is set (cleared by the next successful
  DM or by toggling).
- [ ] Playwright: in `two-players.spec.ts`, after alice's last set-up
  placement with bob's page closed, the fake's `/_sent` holds one DM to
  bob with the game link; with bob's alerts off, none. Commit; PR ready.
