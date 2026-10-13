# Mission 4, slice 2 (the server, headless) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `cna-server` holds games in Postgres and memory, takes orders
over HTTP from the seat holders, pushes each side its own view by SSE,
and survives a restart by replaying. Sign-in is a test-only header in
this slice; Discord comes in slice 3.

**Spec:** `docs/designs/2026-10-13-multiplayer-server-design.md`
**Depends on:** slice 1 merged (`cna-embed`, `cna-game::view`).

## Rules for every task

- Stage only your own files; paste verification output into the PR.
- Stop any server or compose stack you start (`docker compose down`).
- Integration tests need Postgres: `docker compose -f
  deploy/compose.dev.yml up -d db` and `DATABASE_URL=postgres://cna:cna@localhost:5433/cna`.
  Tests use `#[sqlx::test]` (a fresh database per test).
- `export CARGO_TARGET_DIR=$HOME/.cache/cna-engine-target`.

## Tasks

### Task 1: crate, config, health

- [ ] `crates/cna-server` (lib + bin `cna-server`), deps per the spec's
  *Server crate* row (axum, tokio, sqlx with `postgres`, `runtime-tokio`,
  `tls-rustls`, `migrate`; tower-http; tracing; serde; anyhow;
  `cna-game`, `cna-embed`). `Config::from_env()` (`CNA_BIND`, default
  `0.0.0.0:3240`; `DATABASE_URL`; `CNA_PUBLIC_URL`; `CNA_WEB_DIST`,
  optional). `GET /healthz` → 200 `ok` after `SELECT 1`, 503 otherwise.
- [ ] `deploy/compose.dev.yml`: `db` (postgres:18, user/db `cna`,
  host port 5433, a named volume). Failing test first:
  `healthz_is_ok_with_a_database`. Commit.

### Task 2: schema

- [ ] `migrations/0001_games.sql`: `accounts` (id bigserial, discord_id
  text unique, name, avatar, is_admin derived at runtime not stored,
  suspended_at, alerts bool default true, alerts_unreachable_at,
  created_at), `games` (with `alias_key bytea` from `OsRng`), `seats`, `orders`, `snapshots` exactly as the
  spec's *Persistence* row, with `orders` primary key `(game_id, seq)`
  and `seats` unique `(game_id, side)` and `(game_id, account_id)`.
  `sqlx::migrate!()` at start. Test: migrating twice is a no-op. Commit.

### Task 3: the registry and applying an order

- [ ] Failing tests on a `Games` value with a pool: create a game
  (seed from `OsRng`, `games.rules_commit = RULES_COMMIT`), seat two
  accounts, apply the golden game's first order as the Axis → stored as
  seq 0; the same order as the Commonwealth → 403-equivalent
  `NotYourSeat`; an illegal order → the engine's `Illegal`, nothing
  stored; a stale `seq` → `Conflict`.
- [ ] Implement `Games::{create, get, apply}` per the spec's *Applying an
  order* row: per-game `tokio::sync::Mutex`, `unalias_order`, apply on a
  clone in `spawn_blocking` under `catch_unwind`, one transaction for the
  order row and `games.to_act/status/updated_at`, then swap and
  `broadcast`. Snapshot every 50 orders in the same transaction. Commit.

### Task 4: load and restart

- [ ] Failing tests: after 120 golden orders, drop the registry and
  `get` the game again → the state hashes equal; with snapshots deleted
  → equal by full replay; with `games.rules_commit` changed to another
  value → replayed under this data and moved (every order accepted);
  with an order row corrupted to an illegal one → `stale`, not a panic.
- [ ] Implement load = latest matching snapshot + tail, else `replay`.
  Idle eviction after 30 min (a tokio interval; test with a short
  duration). Commit.

### Task 5: the HTTP API

- [ ] Routes from the spec's *HTTP API* row, except sign-in and admin:
  a `CurrentAccount` extractor that, in this slice, reads
  `X-Cna-Test-Account: <discord_id>` and is compiled only with the
  `test-auth` feature (slice 3 replaces it). Every game route resolves
  the caller's seat; queries (`placements`, `reachable`,
  `sealed-options`, `preview`) run as that seat and un-alias inputs;
  responses use `view_for`/`events_for` with the game's aliases and open
  flag, refusals `refusal_for`; `placements` for a piece not the
  caller's is 403.
  `GET /save` refuses (409) until `over`.
- [ ] Tests per route: shape, seat checks, the Commonwealth never
  receiving an Axis secret (`cna_game::view::leaks` on every response body). The `test-auth`
  module starts with `#[cfg(all(feature = "test-auth",
  not(debug_assertions)))] compile_error!(…)`. Commit.

### Task 6: SSE

- [ ] `GET /games/{id}/stream`: `axum::response::sse` with keep-alive;
  each message `id: <seq>`, data `{seq, events, view}` for the caller's
  side; on connect with `Last-Event-ID` (or `?after=`) the server first
  sends one catch-up message (the events since, from the stored orders,
  and the current view). Caps: 4 streams per account per game, 32 per
  account (429 beyond); a game with an open stream is never evicted. Test with a raw HTTP client: two streams, one
  per side, receive their own redacted messages for an order. Commit.

### Task 7: the golden game over HTTP

- [ ] Test `golden_game_over_http`: spawn the router on an ephemeral
  port, two test accounts seated Axis and Commonwealth, post all 446
  orders of `tests/games/graziani-short.json` as the side each names
  (the game created with the golden seed through a `test-auth`-only
  `POST /test/games {seed}`), then `GET /save` and assert its SHA-256 is
  the hash in `tests/games/graziani-short.sha256` (moved there in
  slice 1). Commit.

### Task 8: CI and PR

- [ ] `ci.yml`: a `server` job with a `postgres:18` service,
  `DATABASE_URL` set, `cargo test -p cna-server --features test-auth`.
  The `test` job's `cargo test` must not need Postgres: gate
  `cna-server`'s database tests on `DATABASE_URL` being set via
  `#[sqlx::test]`'s own skip, or exclude the crate there and say so.
- [ ] All gates; PR ready.
