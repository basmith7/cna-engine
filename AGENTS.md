# AGENTS.md: invariants and gotchas

What the code and the other docs don't tell you. Missions, gates and limits are in
`AUTOPILOT.md` (and the shared protocol in cna's); setup and commands in `README.md`; what the
engine leaves out in `NOT_SIMULATED.md`.

## Rules

- Rules live in `cna-game` and `cna-rules`. `web/` draws what the engine's API returns and
  sends orders back; `apply` alone judges them.
- Every cna ruling needs a home: a `Ruleset` switch (the `switches!` list in
  `crates/cna-rules/src/ruleset.rs`) or a row in `NOT_SIMULATED.md`. A test reads
  `vendor/cna/rulings/` and fails on a ruling with neither, so a bump that brings a new
  ruling fails until it has one. A switch's option numbers are the ruling's, and its default
  is the ruling's accepted Decision.
- No front end picks a ruleset: `GameState::with_modules` sets `Ruleset::default()`. Other
  options are reached only by tests, probes or a hand-edited log.
- `NOT_SIMULATED.md` lags in places: its combat row says no ammunition or fuel is deducted,
  its Rail row says combat units don't ride, and its Organisation row predates replacements
  and R-141's chart-based shells. Where it disagrees with the Status in
  `docs/designs/2026-10-13-land-turn-design.md`, trust the spec.

## Determinism and saves

- The same scenario, ruleset, modules, seed, orders and `vendor/cna` commit give the same
  state, byte for byte. Draw randomness only from the game's RNG (`rng.rs`), and keep state
  in `BTreeMap`/`BTreeSet` so iteration order is fixed.
- New rules go behind a `Modules` flag (`#[serde(default)]`) or a `Ruleset` switch, so saves
  and logs made before them replay unchanged.
- `GameState::new` starts with every module off, as old saves play. `cna-play new` (unless
  given `--no-*` flags) and the browser turn them all on, so a test built on
  `GameState::new` exercises no supply, breakdown or construction.
- `rules_commit` is `git rev-parse HEAD` in `vendor/cna`: uncommitted edits there play under
  the old commit's name.

## Bumping `vendor/cna`

Saves, logs, probes and test fixtures record the `vendor/cna` commit, and the scripted games
record the ruleset defaults too, so a bump or a changed default fails their tests until
they're regenerated. In one commit:

1. Move the submodule (`git -C vendor/cna checkout <sha>`) and stage `vendor/cna`.
2. `cargo run -p cna-probe -- run`.
3. `cargo test -p cna-game -- --ignored regenerate_ --nocapture`, then paste the two hashes
   it prints into `FINAL_STATE_SHA256` and `M5_FINAL_STATE_SHA256` in
   `crates/cna-game/src/golden.rs`.
4. `cargo test -p cna-web -- --ignored regenerate_e2e_fixtures` rewrites the Playwright
   fixtures and their `.sha256` pins in `web/tests/fixtures/`.
5. Run the gates and the `web/` checks, with `cargo test --no-fail-fast` (plain
   `cargo test` stops at the first failing crate and skips the rest). A failure left after
   this is a real change in cna, such as a new ruling with no home, not a stale pin.
