# Mission 4, slice 3 (accounts) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Real sign-in with Discord (OAuth2 + PKCE), sessions, open
sign-up with rate limits, CSRF protection, admins and suspension, all
testable offline against a fake Discord.

**Spec:** `docs/designs/2026-10-13-multiplayer-server-design.md`
**Depends on:** slice 2 merged.

## Rules for every task

- As slice 2 (stage your own files, paste output, stop what you start,
  Postgres from `deploy/compose.dev.yml`).
- No secret value in the repo, not even a test one that looks real:
  tests generate keys at run time.

## Tasks

### Task 1: the fake Discord

- [ ] Bin `fake-discord` in `cna-server` behind feature `fake`
  (`required-features`), port from `FAKE_DISCORD_BIND` (default 3241),
  users from `FAKE_DISCORD_USERS` (`id:name,…`, default two: `alice`,
  `bob`). Endpoints as the spec's *Fake Discord* row; the authorize page
  lists the users as links that redirect to `redirect_uri` with `code`
  and the given `state`; the token endpoint verifies the PKCE
  `code_verifier` against the stored challenge (so the server's PKCE is
  actually tested); `/_sent` and `/_members` expose what the bot did.
- [ ] Unit tests for the fake itself (PKCE mismatch refused). Commit.

### Task 2: OAuth2 sign-in

- [ ] Failing integration test: `GET /auth/discord` → 302 to the
  configured authorize URL with `client_id`, `scope=identify guilds.join`,
  `state`, `code_challenge`; following the fake's link to
  `/auth/discord/callback` → an `accounts` row, a session cookie
  (`HttpOnly`, `SameSite=Lax`, `Secure` unless `CNA_DEV=1`), 302 to `/`;
  `GET /api/me` → the account. A wrong `state` → 400 and no account.
- [ ] Implement with the `oauth2` crate; Discord base URLs
  (`CNA_DISCORD_API`, `CNA_DISCORD_AUTHORIZE`) default to Discord's and
  point at the fake in tests and compose. Sessions: `tower-sessions` with
  its Postgres store (its own migration), 30-day expiry, `cycle_id` at
  sign-in. `POST /auth/sign-out`. Commit.
- [ ] The `CurrentAccount` extractor reads the session; the
  `X-Cna-Test-Account` path stays only under `test-auth` for the
  server's own tests. Commit.

### Task 3: abuse limits and CSRF

- [ ] Failing tests: the 11th `GET /auth/discord` from one IP in a
  minute → 429; the 11th game created by one account in an hour → 429;
  a 21st open game → 409; a `POST` without `Content-Type:
  application/json` or with a foreign `Origin` → 403; a 65 KB body → 413.
- [ ] Implement: `tower_governor` keyed by peer IP (trust
  `X-Forwarded-For` only when `CNA_TRUST_PROXY=1`, as behind Pangolin),
  per-account counts from the database, `RequestBodyLimitLayer`, an
  `Origin` check middleware for state-changing methods. Commit.

### Task 4: admins and suspension

- [ ] Failing tests: an account whose Discord id is in `CNA_ADMINS` sees
  `GET /admin/accounts` (others 403); `POST
  /admin/accounts/{id}/suspend` deletes that account's sessions; its next
  request is 401 and its sign-in is refused with a page saying the
  account is suspended; `restore` undoes it. A suspended account's games
  stay as they are.
- [ ] Implement. Commit.

### Task 5: compose and PR

- [ ] `deploy/compose.dev.yml` gains `fake-discord` and `server` (built
  from the workspace with `cargo run`, or a dev Dockerfile stage; pick
  the simpler and say why), wired with the env above.
- [ ] Manual check pasted into the PR: sign in as `alice` through the
  fake in a browser (`curl` transcript acceptable), `/api/me` answers.
- [ ] All gates; PR ready.
