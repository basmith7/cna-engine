# cna-engine: a multiplayer server

**Date:** 2026-10-13 (revision 1)
**Mission:** 4 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** spec, **awaiting Brian's review** (the queue marks this
mission for it). Nothing below is built until **Feedback** in
`docs/autopilot/PROGRESS.md` approves it.

Written by the autopilot under Brian's delegation of 2026-09-26: every
technical decision below is Claude's, is cheap to reverse, and can be
reversed with one PR. The scope is Brian's, from his direction of
2026-10-13 (queue row 4): a Rust server holding the only full state,
redacted per-side views over HTTP plus a live channel, Discord sign-in
with open sign-up, rate limits and an admin who can suspend, a lobby,
turn alerts by Discord DM (opt-out), Postgres with the game as its
scenario plus its orders, the `web/` UI gaining a network engine beside
the hot-seat one, and the ship path of Brian's own apps, with every
outside step left to Brian's checklist.

**Checked against the code.** Every name, type and path below is the
merged code on `origin/main` at `cec9776` (Missions 2 and 3 complete).

**Questions for Brian** (the spec picks an answer for each; say if you
want the other):

1. **How the bot reaches a player's DMs.** Discord only lets a bot DM a
   user it shares a server with (and the user may block DMs from server
   members). The spec asks for the `guilds.join` scope at sign-in and
   adds each player to a small "CNA" Discord server the bot lives in;
   the sign-in page says so, and turning alerts off leaves the server.
   The alternative is `identify` only and alerts that work only for
   people already in a server with the bot (for example Brian's own).
2. **Host port 3240** on Tower (free in `Infrastructure.md` today).
3. **The image name** `ghcr.io/basmith7/cna` (public: the repo is
   open source and the image holds no secrets).

## Goal

Two people on two computers, each signed in with Discord, find each
other in a lobby, start Graziani's Offensive, and play it to the end,
each seeing only what their side may see, with a Discord DM when it
becomes their move. Many games and many players at once; anyone may
sign up.

How this moves **the goal** (two players, a browser, against each other,
every rule enforced): Mission 3 made the game playable at one screen with
the opponent asked to look away. This makes it playable at a distance
with secrecy that holds: the server is the only holder of the full state,
so sealed orders, dummy supply units and the contents of enemy counters
(3.61) are secret for real. It is also the first piece that can be
hosted, so it is when "self-hostable" becomes true: one image, one
Postgres, a Discord application.

## Non-goals

- **Deploying or hosting anything.** The autopilot builds and tests
  locally (compose with Postgres and a fake Discord). The Discord
  application and bot, Tower, Pangolin and DNS are Brian's checklist
  (below), done by hand.
- **Other scenarios, ruleset pickers, the rest of the land rules.**
  Graziani's Offensive with the default ruleset is the only game a lobby
  creates (row 5 and later add the rest). The `scenario` and `ruleset`
  columns exist so that is a data change later, not a migration.
- **Spectators, chat, teams, time controls, ratings, rematch, undo.**
  Players talk on Discord. A game with no move for 60 days is marked
  `abandoned` (below); there is no clock.
- **Sign-in other than Discord**, email, passwords.
- **Simultaneous sealed answers.** The engine takes the phasing side's
  sealed answer first (`waiting`); the server keeps that order, and the
  secrecy is real because the other side's view never holds the answer.
- **Horizontal scaling.** One server process; per-game state in memory
  behind a lock, Postgres as the record. Enough for a hobby game by
  orders of magnitude; the design notes where a second process would
  need a change.
- **Mobile layout** (as in Mission 3).

## What "done" means

1. `docker compose -f deploy/compose.dev.yml up` starts Postgres, the
   fake Discord and `cna-server`; `http://localhost:3240` serves the UI.
2. Signing in goes through the (fake) Discord OAuth2 flow, creates an
   account on first sign-in, and sets a session cookie.
3. The lobby creates a game (pick a side or "either"), shares an invite
   link or lists it as open, lets a second account join and take the
   other side, and lists "my games" with whose move it is.
4. Each player's browser gets only its own side's view (`view_for`) and
   its own side's events, by HTTP and then live by SSE. A test proves the
   Commonwealth's HTTP and SSE traffic never holds an Axis sealed answer,
   an Axis dummy's identity, or an Axis counter's TOE.
5. Orders are posted over HTTP, applied by `cna-game` on the server,
   stored as an order log in Postgres; a server restart replays every
   game to the same state (from a snapshot plus the orders after it).
6. When a game's `to_act` becomes a player who has alerts on, the bot
   sends that player one DM with a link (the fake Discord records it).
7. Sign-up and game creation are rate-limited; an admin can suspend and
   restore an account, and a suspended account can neither sign in nor
   act.
8. The hot-seat game of Mission 3 still works with no server (the
   start screen offers both), and its Playwright suite still passes.
9. Playwright drives two browser contexts, one per side, through the
   golden game's set-up, movement and combat stretches (orders 0–76)
   against the compose stack, and the server's final state for those
   orders hashes to the same value `replay` gives.
10. A tag `v*` builds and publishes the image in GitHub Actions; the
    image, run with compose and Postgres, passes `/healthz` locally.
    Nothing is pushed to Tower.

## The engine as it is (what M4 builds on)

- `cna-web::api::Game` (native Rust over JSON, wasm through
  `lib.rs`): `new(seed)`, `load(json)`, `save()`, `view()`,
  `placements(piece)`, `reachable(side, pieces, purpose)`,
  `sealed_options(side)`, `apply(side, order)`, `preview(side, order)`.
  `view()` redacts sealed answers relative to `to_act()`, not to a
  viewer: right for hot-seat, wrong for a server.
- `view()` sends every piece in full: `toe`, `weapons`, `cohesion`,
  `cp_spent`, `morale`, `status`, `kind`, and the id. A dummy supply
  unit's id is `supply:<side>:dummy-<n>` and its kind
  `dummy-supply-unit`, so the id alone gives it away.
- `preview()` applies on a clone **with dice**: over a network it would
  tell a player tomorrow's die rolls (the RNG is in the state).
- `GameState::to_log()` / `log::replay(&Data, &Log)` rebuild a game from
  its seed and orders; `from_json` checks scenario, `rules_commit` and
  `check_invariants`. `Log` carries `ruleset`.
- Data on the web side is `cna-web`'s `embedded::data()`, built by its
  `build.rs` from `vendor/cna/data` with `RULES_COMMIT`.
- `Event` has 38 variants; several carry numbers or pieces a side may not
  learn under 3.61 (losses by piece, TOE behind a total, a dummy's id).
- `web/src/engine.ts` wraps the wasm `Game` synchronously; the Svelte
  app calls it directly.

## What the rules say a side may know

cna's restatement of 3.61–3.62 (*Limited intelligence*,
`rules/95-abstract-logistics-and-air.md`): a player is entitled to
nothing about an enemy counter's status, composition or attributes
except what a rule grants. Any stack may be examined (which counters are
in it), not what the counters contain; in land combat only totals are
revealed, and the anti-armour and close assault steps reveal unit types
that took part, not numbers. The sealed steps are secret until revealed
(`combat::plots`).

The server applies this by default. A game created with **open
information** (a per-game option, off by default) shows each side the
enemy's counters in full, for groups that play that way. It is a game
option, not a `Ruleset` switch: `Ruleset` holds cna's rulings
(`R-nnn`), and what a player is shown changes no rule the engine
applies, so replay and the golden hashes do not see it.

| What | Own side | Enemy (default) |
|---|---|---|
| Piece on the map or in a box | everything | the counter face: `nation`, `id_code`, `unit_type`, `name`, `abbreviation`, `equivalent`, `cpa`, `location`, `kind` with real and dummy supply both shown as `supply-unit`, markers the board shows (`reserve`, `contact`/`engaged`, `pinned`, `forward` once revealed) |
| `toe`, `weapons`, `cohesion`, `cp_spent`, `morale`, `status`, `parent` | shown | removed |
| Supply unit ids | real | an alias `supply:<side>:s<k>`, `k` from a per-game permutation of that side's real and dummy supply units keyed by a per-game secret (`games.alias_key`, 32 random bytes, never the seed), stable for the game. Aliasing is viewer-aware: a side's own ids are always real, only the enemy's are aliased, in every id the view or an event carries |
| An enemy piece still to be placed | everything | `location: pending` without its `Placement` (a dummy block's own placement would give it away); `placements` is refused for a piece not the caller's |
| Per-piece maps in the state and in `combat` (`moved`, `continuing`, `shifted`, `combat.charged`, `barrage_involved`, `probing`, `pinned`, `retreated`, `barraged`) | everything | enemy ids aliased; enemy numbers (`charged`) and judgments made on enemy TOE (`probing`) removed; what the board shows (`pinned`, `retreated`) kept. Slice 1 classifies every such field, so the list is checked, not trusted |
| `pending` | everything | id lists aliased; `forced-retreat.raw` (a function of TOE) removed for the side not retreating |
| Refusals (`Illegal.reason`) | as the engine says | through the alias layer; slice 1 audits every reason that interpolates pieces or numbers and rewrites any that would name an enemy internal |
| Sealed answers in `combat` | shown | removed until the reveal event (as `view()` does today, but by viewer) |
| `log`, `rng`, `seed` | never sent | never sent |
| Events | all | per variant: totals and unit types kept, per-piece losses of the enemy reduced to "losses in hex", dummy ids aliased; the 38-variant match is exhaustive so a new variant fails to compile until classified. Known to need reducing: `losses` (remaining `toe`), `disorganised` (`dp`, `level`), `combat-cp` and `combat-cp-refunded`, `moved` (`cost`: it makes `cp_spent` derivable), `barrage` (`points`), `assault` (`attacker_raw`), `prisoners` (`points`); `captured`, `surrendered`, `eliminated` and `barrage.target` are public facts on the board but carry ids to alias |

The exact per-variant table is slice 1's to write, as a test per
variant; where the restated rule is unclear the slice posts a request
for cna and takes the stricter reading. With open information, the enemy
columns equal the own-side ones, except the sealed answers (always
secret until revealed) and `log`/`rng`/`seed` (never sent).

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Where per-side knowledge lives | **`cna-game::view`**: `view_for(&GameState, &Data, viewer: Option<Side>, open: bool) -> Value` and `events_for(&GameState, &[Event], viewer, open) -> Vec<Value>`, plus `Aliases` (built from the seed and the side's supply pieces) with `alias`/`unalias` for orders. `cna-web::api::view()` becomes `view_for(state, data, to_act(), true)` with the old behaviour pinned by its existing tests | What a side may know is a rule (3.61), so it belongs to the engine, once, for both hosts. One owner, not a copy in the server |
| Preview over the network | `preview` returns the events only if the clone's RNG did not advance; otherwise `{"hidden": "dice"}`. The UI's one use (Pass in a forced retreat) draws no dice | Keeps the feature without leaking future rolls |
| Data on the server | Extract `cna-web`'s `build.rs` and `embedded` into **`crates/cna-embed`** (`data()`, `RULES_COMMIT`); `cna-web` and `cna-server` both depend on it. `build.rs` takes `CNA_RULES_COMMIT` from the environment when set, so the Docker build needs no `.git` | One artefact per host with the data compiled in; a running image always knows its `rules_commit`; no `vendor/cna` mount at run time |
| Server crate | **`crates/cna-server`**: lib + `cna-server` binary. **axum 0.8** on **tokio**, **sqlx** (Postgres, compile-time-unchecked queries so CI needs no live DB to build; migrations in `crates/cna-server/migrations/`, run at start), **tower-http** (static files, compression, request size limit, tracing), **tower_governor** (per-IP rate limits), **oauth2** (Discord, PKCE + state), **tower-sessions** with its Postgres store, **reqwest** (rustls) for Discord's REST API, **tracing**. Config from env (`CNA_` prefix) | The common axum stack; each piece is the standard crate for its job, none hand-rolled |
| The game in memory | A `Games` registry: `HashMap<GameId, Arc<tokio::sync::Mutex<Live>>>` loaded on first use; `Live` holds the `GameState`, the next `seq`, and a `tokio::sync::broadcast` sender for SSE. Evicted after 30 min idle with no open stream | One lock per game serialises its orders; games are independent |
| Applying an order | Under the game's lock: check the caller holds the seat for `side` (whose turn it is stays the engine's to judge: it refuses an out-of-turn order with its own case), un-alias piece ids, `apply` **on a clone** inside `spawn_blocking` with `catch_unwind`; on `Ok` insert `(game, seq, side, order)` and update `games.to_act`/`over` in **one transaction**, then swap the clone in and broadcast; on `Illegal` return it; on a panic return 500 and keep the old state | The order log is the record; a crash between commit and swap only loses the cache, which replays. A clone keeps a panicking order from leaving half a mutation |
| Persistence | Tables: `accounts`, `sessions` (tower-sessions), `games` (id, scenario, ruleset JSON, open-information flag, seed, rules_commit, created_by, status `open`/`active`/`over`/`abandoned`/`stale`, to_act, invite token hash, created/updated), `seats` (game, side, account), `orders` (game, seq, side, order JSON, at; primary key `(game, seq)`), `snapshots` (game, seq, state JSON), `alerts` (outbox, below). A snapshot is written every 50 orders; loading = latest snapshot whose `rules_commit` matches + the orders after it, else `replay` from the seed | "A game is its scenario plus its orders" (Brian); snapshots only for speed and always reproducible |
| New data, old games | On start the server compares each active game's `rules_commit` with its own. A mismatch is replayed under the new data; if every order is accepted the game moves to the new commit (logged), else it is marked `stale` and shown as such in the lobby | A `vendor/cna` bump must not silently fork a game in progress |
| Seeds | From `getrandom` on the server (`rand_core::OsRng`), stored in `games.seed`, never sent; the alias key is a separate random value, so knowing the dice tells nothing about aliases | The RNG is in the state; a client that knew the seed would know every roll |
| HTTP API | JSON under `/api`: `GET /me`, `PATCH /me` (alerts), `GET /games?mine` / `?open`, `POST /games` `{side}` , `POST /games/{id}/join` `{side, invite?}`, `GET /games/{id}` (my seat, my `view_for`, `seq`), `GET /games/{id}/events?after=seq`, `POST /games/{id}/orders` `{side, order, seq}` → `{ok: events}`/`{illegal}`/409 on a stale `seq`, `GET /games/{id}/placements?piece=`, `GET /games/{id}/reachable?pieces=&purpose=`, `GET /games/{id}/sealed-options`, `POST /games/{id}/preview`, `GET /games/{id}/save` (only once the game is over: the full state then hides nothing). Admin: `GET /admin/accounts`, `POST /admin/accounts/{id}/suspend` / `restore`. Every query runs for the caller's seat only and un-aliases its input | Mirrors the wasm `Game` method for method, so the client swaps engines without reshaping data |
| Live channel | **SSE**, `GET /games/{id}/stream`: on each accepted order the server sends `{seq, events: events_for(me), view: view_for(me)}` to each seat's open streams; the client reconnects with `Last-Event-ID` and catches up from `orders` | One-way push is all the game needs (orders go by POST); SSE rides plain HTTP through Pangolin and Traefik with no upgrade, and reconnects itself |
| Sign-in | Discord OAuth2 authorization code with PKCE and `state`; scopes `identify` and `guilds.join` (question 1). First sign-in creates an `accounts` row keyed by the Discord user id (display name and avatar refreshed at each sign-in). Session: tower-sessions cookie `cna_session`, `HttpOnly`, `Secure` (off only in dev), `SameSite=Lax`, 30-day expiry, the id rotated at sign-in | Standard flow; Discord ids are stable |
| Fake Discord | `cna-server`'s second binary **`fake-discord`** (behind a `fake` cargo feature, never in the release image): `/oauth2/authorize` (a page listing test users, click to consent), `/api/oauth2/token`, `/api/users/@me`, `PUT /api/guilds/{g}/members/{u}`, `POST /api/users/@me/channels`, `POST /api/channels/{c}/messages`, and `GET /_sent` returning the DMs it took. The server's Discord base URLs come from config | Lets compose, integration tests and Playwright run the real flow offline |
| Open sign-up and abuse | Per-IP limits (tower_governor): 10 sign-in starts per minute. Per-account limits (a governor keyed by account id): 60 orders per minute, 10 game creations per hour per account and at most 20 open games per account; request bodies capped at 64 KB. CSRF: state-changing routes need `Content-Type: application/json` and an `Origin` equal to the configured public URL (a missing `Origin` is refused; sign-out is a POST). At most 4 open streams per account per game and 32 per account. Admins are the Discord ids in `CNA_ADMINS`; a suspended account's sessions are deleted, its open streams closed, and sign-in is refused with a page saying so | Open sign-up on the internet needs these from day one; config, not code, holds the admin list |
| Turn alerts | A transactional **outbox**: the order transaction inserts an `alerts` row when `to_act` changes to a seat whose account has alerts on; a worker task sends it (open the DM channel, post "Your move in <game> (GT2 OpStage 1): <link>"), retries with backoff, gives up after 24 h, and marks a 403 (DMs closed) on the account so the lobby says "alerts can't reach you". At most one alert per game per account per 15 minutes, and none while that account has the game's stream open | An alert is never lost to a crash and never sent for an order that rolled back |
| Abandoned games | A daily task marks an active game with no order for 60 days `abandoned`; either player can also leave a game (`POST /games/{id}/leave`), which marks it abandoned. No winner is declared | No clock in scope, but the lobby must not fill with dead games |
| Web client | `web/src/engine.ts` becomes an `Engine` interface with two implementations: `LocalEngine` (today's wasm, synchronous wrapped in promises) and `NetworkEngine` (fetch + `EventSource`). The app's calls become `await`s; the view is replaced whole on each SSE message, as on each local apply today. New screens: sign-in, lobby, my games, settings (alerts), and an admin list for admins. The curtain stays for hot-seat only | One app, two engines; the "view replaced whole" rule of Mission 3 carries over |
| Serving the UI | `cna-server` serves `web/dist` (built by Vite) from the same origin with tower-http `ServeDir` and an SPA fallback; the wasm stays in the bundle for hot-seat | One origin means no CORS and cookie-only auth |
| Repo layout | `crates/cna-embed`, `crates/cna-server` (`src/`, `migrations/`, `tests/`), `deploy/compose.dev.yml`, `deploy/compose.prod.yml`, `Dockerfile`, `.github/workflows/docker-publish.yml`, `scripts/deploy-now.sh` (written, never run by the autopilot) | The shape Brian's other apps use (`Infrastructure.md`, *App deployments*) |
| Image | Multi-stage `Dockerfile`: a Node 22 + Rust stage builds the wasm and `web/dist`; a Rust stage builds `cna-server --release`; the runtime is `debian:trixie-slim` with the binary, `web/dist` and a non-root user; `HEALTHCHECK` on `/healthz` (which checks the database). Published to `ghcr.io/basmith7/cna` on a `v*` tag by `docker-publish.yml` after the tests | Brian's pattern: tag → image → `deploy-now.sh` |
| Tooling | Postgres 18 in compose and as a GitHub Actions service container; `sqlx-cli` is not needed (migrations are embedded with `sqlx::migrate!`). Docker is already on Brian's desktop (checked in slice 6, not installed by the autopilot) | Nothing new to install on Brian's machines |

## Security notes

- The full state never leaves the server while a game runs: no endpoint
  returns `save()` before `over`, `view_for` never holds `log`, `rng` or
  `seed`, and the test in *done* 4 checks every byte the Commonwealth
  receives for Axis-only facts.
- The test-only sign-in and seeded-game routes (`test-auth` feature)
  refuse to compile in a release build (`compile_error!` without
  `debug_assertions`), and the image is built without the feature.
- Every request that names a side is checked against the caller's seat;
  an order for the other side is 403, not `Illegal`.
- The engine is never trusted to survive hostile input: orders are
  parsed by serde into `Order`, applied on a clone under `catch_unwind`,
  and request bodies are size-capped. Saves are not uploadable to the
  server.
- Secrets (`CNA_DISCORD_CLIENT_SECRET`, `CNA_DISCORD_BOT_TOKEN`,
  `CNA_SESSION_KEY`, the Postgres password) come only from env; the repo
  holds a `deploy/.env.example` with names and no values.

## Testing

- The leak predicate (does this JSON hold anything of side X's that side
  Y may not know, given the full state?) is public as
  `cna_game::view::leaks`, so the engine's, the server's and the e2e
  tests share one definition. The golden game's final hash moves from a
  private test constant to `tests/games/graziani-short.sha256`, read by
  `cna-game` and `cna-server` alike.
- **`cna-game::view`, natively:** per field of the table above, own vs
  enemy, default and open information; per `Event` variant, own vs enemy; aliases
  are a bijection, stable across a save/load, and an order naming an
  alias un-aliases to the real id; the existing hot-seat `view()` tests
  unchanged. A **leak test**: for every prefix of the golden game, the
  Commonwealth's `view_for` and `events_for` serialised contain no Axis
  dummy id, no Axis `toe` value outside a revealed total, and no Axis
  plot before its reveal event (and the mirror).
- **`cna-server`, integration (`#[sqlx::test]`, a Postgres per test):**
  the API with a signed-in client per seat via the fake Discord; orders
  out of seat refused; illegal orders return the engine's case;
  duplicate `seq` → 409; restart (drop the registry) and replay to the
  same state hash; snapshot + tail = full replay; `rules_commit` mismatch
  → migrate or `stale`; rate limits; suspension; the outbox sends one DM
  per change of `to_act`, none for a rolled-back order, a 403 marks the
  account; the **golden game over HTTP**: two clients post the 446 orders
  of `tests/games/graziani-short.json` and the final state hashes to the
  pinned golden hash.
- **Playwright** against `deploy/compose.dev.yml` (CI service
  containers): sign-in as two fake users in two contexts, create and join
  by invite, play orders 0–76 by clicks (reusing Mission 3's drivers),
  the Commonwealth context's network log checked for Axis secrets, a DM
  recorded by the fake Discord, the lobby showing whose move. Mission 3's
  hot-seat suite runs unchanged.
- **Image:** slice 6 builds the image locally, starts it with
  `compose.prod.yml` plus the fake Discord, and checks `/healthz`, sign-in
  and one game created.
- Gates: Mission 3's, plus `cargo test -p cna-server` with
  `DATABASE_URL` set (CI: a Postgres service), and the Playwright
  multiplayer suite.

## Slices

One PR each, in order; each has a plan in `docs/plans/`.

1. **Per-side knowledge in the engine**
   (`2026-10-13-m4-slice-1-view.md`): `cna-embed`; `cna-game::view`
   (`view_for`, `events_for`, `Aliases`), open information,
   dice-safe preview; `cna-web` moved onto both with its tests
   unchanged; the leak test.
2. **The server, headless** (`…-slice-2-server.md`): `cna-server` with
   config, migrations, the registry, the game API and SSE, behind a
   test-only header sign-in; the golden game over HTTP; restart and
   snapshots; `deploy/compose.dev.yml` with Postgres; CI with a Postgres
   service.
3. **Accounts** (`…-slice-3-accounts.md`): Discord OAuth2 with the fake
   Discord, sessions, rate limits, CSRF and `Origin`, admins and
   suspension; the header sign-in removed outside tests.
4. **Lobby and the network client** (`…-slice-4-lobby.md`): the
   `Engine` interface, `NetworkEngine`, the sign-in, lobby, my-games and
   settings screens, invite links, abandonment; Playwright with two
   contexts through orders 0–76 and the leak check. *Screenshots:*
   `slice-4/lobby.png`, `slice-4/two-players.png`.
5. **Turn alerts** (`…-slice-5-alerts.md`): `guilds.join`, the outbox
   worker, debouncing, the opt-out, the "can't reach you" state, the
   Playwright check of a recorded DM.
6. **Ship** (`…-slice-6-ship.md`): `Dockerfile`, `compose.prod.yml`,
   `docker-publish.yml`, `scripts/deploy-now.sh`, `.env.example`, the
   image smoke test, README and `NOT_SIMULATED.md`, Brian's checklist
   final, `MISSION 4 COMPLETE`.

## Brian's checklist (outside steps, never done unattended)

Done once, after slice 6 merges; slice 6 rewrites this list with the
exact values.

1. **Discord application** "Campaign for North Africa" at
   discord.com/developers (its own, not Pikachu): OAuth2 redirect
   `https://cna.basmith.net/auth/discord/callback`; note the client id
   and secret. Add a **bot** to it; note the token; no privileged
   intents needed.
2. **A Discord server "CNA"** (question 1) with the bot invited with
   the *Create Instant Invite* permission, which Discord's add-member
   call needs alongside the user's `guilds.join` grant; note the guild
   id.
3. **Tower:** compose project `cna` in compose.manager (autostart) from
   `deploy/compose.prod.yml`: `cna-db` (postgres:18, data
   `/mnt/cache/appdata/cna/pg`) and `cna` (host port 3240, `/healthz`),
   the Unraid icon labels; `.env` root 600 with `CNA_PUBLIC_URL`,
   `CNA_DISCORD_CLIENT_ID`, `CNA_DISCORD_CLIENT_SECRET`,
   `CNA_DISCORD_BOT_TOKEN`, `CNA_DISCORD_GUILD_ID`, `CNA_SESSION_KEY`
   (64 random bytes, base64), `CNA_ADMINS` (Brian's Discord id),
   `POSTGRES_PASSWORD`.
4. **Pangolin:** an HTTP resource `cna.basmith.net` → `192.168.1.149:3240`;
   set `sso:false` (the API-created-resource gotcha); health check
   `/healthz`. DNS: the `*.basmith.net` wildcard already points at the
   VPS; wait 90 s before the first HTTPS hit.
5. **Backups:** add the `cna` Postgres to `tower-backup` (a `pg_dump`
   into the stage directory, as SMARTDraft's nightly dump).
6. **Ship:** tag `v0.1.0`; `scripts/deploy-now.sh`; sign in; log the
   change in `Infrastructure.md`'s Change Log and add `cna` to *App
   deployments*.

## Risks

- **Discord DM reachability** (question 1): users who block DMs from
  server members never get alerts; the account shows it. Discord may
  rate-limit DM channel creation; the outbox backs off.
- **Limited intelligence is a large redaction.** Hiding enemy TOE affects every panel
  that shows enemy numbers (the stack panel, the combat panels); slice 4
  must show "?" for removed fields rather than crash on a missing one.
  The leak test is what keeps it honest.
- **Event redaction may not match the restated rule exactly.** 3.61's
  table is prose; slice 1 posts requests for cna where it is unclear and
  is strict meanwhile.
- **Long games in memory.** A Graziani state is a few hundred KB; 30 min
  eviction bounds memory. A full replay of a long game on load is slow
  only before the first snapshot.
- **Shared `CARGO_TARGET_DIR`** (as in Mission 3): a gate that fails on
  data it should not see is re-run before being believed.

## Requests for cna this mission makes

- None needed to start. Slice 1 may post one for 3.61's per-event
  details (what an anti-armour or close assault result reveals, as
  fields), if the prose does not settle a variant.
