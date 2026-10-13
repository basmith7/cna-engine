# Mission 5, slice 1 (supply state) Implementation Plan

> **For agentic workers:** implement this plan task by task with the
> **implement-spec** skill. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Graziani's supply units carry their ammunition and fuel, each
side has its motorisation points where its trucks started, supply
stacking is enforced, and 32.8x's AA points are gone at set-up. Nothing
is spent yet; the golden game replays unchanged except where supply
stacking or AA removal touches it (re-pinned in one commit).

**Spec:** `docs/designs/2026-10-13-land-turn-design.md`
**Depends on:** Mission 3, complete on `origin/main`.

## Rules for every task

- Stage only your own files (`git add <paths>`, never `git add -A`).
- Paste each verification command's output into the PR body.
- `export CARGO_TARGET_DIR=$HOME/.cache/cna-engine-target`.
- Failing test first, every task; cite the case in each test name or
  comment, never the rule text.

## Tasks

### Task 1: loads on supply units (32.1x, 60.92)

- [ ] Read `SupplyBlock`'s `ammo`/`fuel` notes in cna (and its schema):
  is the figure per unit or per block? If cna does not say, follow per
  unit (every Graziani figure is within 40/60) and add a request.
- [ ] Failing test in `setup.rs`: every Graziani real supply unit has
  `load == Some(Load { ammo, fuel })` from its block (Derna's printed dash
  is 0 ammo); dummies have `None`; Tobruk's three have 15/50 each.
- [ ] `piece::Load { ammo: u32, fuel: u32 }`, `Piece.load:
  Option<Load>` (`serde(default, skip_serializing_if = "Option::is_none")`
  so older saves load), filled in `setup.rs`'s supply loop.
- [ ] `check_invariants`: a real supply unit has a load within 40/60 and
  a dummy none (failing-first test with a hostile save).

### Task 2: motorisation points (32.5x, 60.92)

- [ ] Failing test: after set-up the Axis has `sum(trucks.medium)` over
  its deployments in motorisation points, held in the hex (or box) of
  each deployment; the Commonwealth likewise; light and heavy give none.
- [ ] `GameState.motorisation: BTreeMap<Side, BTreeMap<Location key,
  u32>>` (a hex id or box name as the key, so it serialises as a JSON
  object). Pending placements: the points follow the deployment's pieces
  when its `Place` order lands (test with a free-placement deployment
  that has trucks, or a synthetic one if Graziani has none).
- [ ] Points move with nothing yet (slice 3); they are captured with a
  hex only when slice 3 attaches them.

### Task 3: supply stacking and road stacking (32.1x, 32.9)

- [ ] Failing tests: a `Place` or `Move` that would put a fourth supply
  unit in a non-city hex, or a sixth in a major city, is refused citing
  32.1x; Tripoli–Tunisia boxes take any number. Dummies count (they look
  real). Whether set-up data already breaks the cap is a load error.
- [ ] Failing test: road and track stacking limits are not applied with
  §32 on (32.9): check what `movement.rs` does today; if it has no road
  stacking limit, the test pins that and the task is a comment.

### Task 4: AA removal at set-up (32.8x)

- [ ] Find which Graziani pieces carry AA: Commonwealth AA units and
  light/heavy AA TOE entries; German *z*/*bb* units and light AA entries;
  Italian *ss*/*tt*/*uu* units, AA entries of *pp* units, heavy AA on *kk*
  artillery. Use `weapon-systems.json`'s `class` to tell AA weapons.
- [ ] Failing test listing the removed pieces and the reduced TOE of
  the stripped ones; implement in `setup.rs` behind the `abstract_supply`
  module flag (Task 5). If the OA cannot tell light from heavy AA for a
  weapon, add a request for cna and remove only the unambiguous ones.

### Task 5: the module flag

- [ ] `Modules { abstract_supply: bool }` on `GameState` and `Log`,
  `serde(default)` (false) so pre-M5 logs and saves replay as they were;
  `cna-play new` and `cna-web`'s new game set it true. The old golden log
  keeps its hash; a new golden (`tests/games/graziani-m5.json`, the old
  orders with the module on) is pinned.
- [ ] `--no-abstract-supply` on `cna-play new`.

### Task 6: the view and the stack panel

- [ ] `view()` redacts `load` and the other side's `motorisation` for
  pieces not of `to_act`'s side (3.61: loads and truck numbers are
  secret); test in `api.rs`.
- [ ] ~~`StackPanel` shows a supply unit's load and the hex's
  motorisation points for the owner~~: moved to slice 2, Task 6. The
  browser's new game stays module-off in this slice (the M3 golden e2e's
  set-up stacks supply past 32.1x's cap), so there is nothing to show yet.
- [ ] Commit; PR; gates (`cargo clippy --all-targets -- -D warnings`,
  `cargo test`, `cargo run -p cna-probe -- check`, `npm test`,
  Playwright).
