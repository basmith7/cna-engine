# Mission 5: combat units by rail (8.7x, R-005)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (*Supply movement*
built the railway for supply units, `cna-game::rail`). `NOT_SIMULATED.md`
lists "combat units by rail" as not yet. Every decision here is the
autopilot's under Brian's delegation of 2026-09-26 and is reversible.

**Goal.** The Commonwealth can shuttle troops between Alexandria and
Mersa Matruh as the printed game lets it: one stack each way a stage, in
phase J, free of CP and fuel. In Graziani's Offensive that is how the
Matruh garrison is fed and drained.

**Non-goals.** Picking up and setting down en route (R-005 allows it): a
run here is one stack from one hex to another, and a second run the same
way in a stage is refused; recorded in `NOT_SIMULATED.md`. Rail with the
abstract-supply module off: phase J stays skipped there, so older logs
replay unchanged. Bombing and repairing the line (the Air Game).

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Who rides | Commonwealth combat units and HQs, with or without supply units, in one `Order::Rail` stack | 8.72, R-005: units and/or supplies |
| Limits | Each piece on the open line, no CP spent this stage, not in an enemy ZOC (the existing `may_rail` checks, widened past supply); the stack at most 2 SP (`cna_rules::stacking::points` per piece); never into an enemy hex or ZOC (existing) | 8.7x |
| Runs a stage | One stack eastward and one westward (`rail_runs: BTreeSet<RailWay>` beside `railed`), supply riding in them; supply units keep 32.3x's two a stage, all one way (`rail_supply_way`) | 8.74 and R-005 for stacks, 32.3x for supplies |
| Cost | No CP, no fuel, no breakdown points; the pieces are marked `railed` so they cannot run again | 8.7x, 32.2x |
| Phase J opens | When the module is on and the Commonwealth has a piece that `may_rail` | As today for supply |
| UI | `Purpose::Rail` already shades the line for ticked supply; it works for any ticked stack `rail_start` accepts | No new panel |

## Tasks

### Task 1: combat units ride

- [x] Write the failing tests: a Commonwealth battalion at Alexandria
  with no CP spent runs west in phase J and spends nothing; one that
  spent CP, or a stack over 2 SP, is refused (8.7x). (Built with the
  tests at Alexandria, where `at_rail` already opens phase J.)
- [x] Widen `may_rail` and `rail_start` past supply units; stacking
  points checked on the stack; the 32.3x count is of supply only.
  Phase J still opens only for supply (`rail_to_move`), so the goldens
  are unchanged until Task 3.
- [x] Carry attached units along (`carried`, 19.4x), as a move does.

### Task 2: one stack each way

- [x] Write the failing tests: a second eastward run in the stage is
  refused, a westward one allowed; supply still two a stage, one way.
- [x] `railed_runs`, cleared where `railed` is (phases.rs). Built as
  `rail_runs: BTreeSet<RailWay>` and `rail_supply_way` in place of
  `rail_east`. Deviation from the Decisions table: supply rides *in*
  the way's one stack (R-005: a stack is units and/or supplies), so two
  supply units going one way in two runs is refused; the second rides
  with the first or not at all.

### Task 3: UI, docs

- [ ] `rail_to_move` opens phase J for a rail-eligible combat unit; a
  Playwright test rails a unit from the `rail` fixture by clicks. This
  opens phase J in module-on golden stages it skipped (Commonwealth units
  at Matruh with no CP spent), adding Passes: re-script and re-pin as #78
  did (`golden.rs` hash, `graziani-m5.json`, `cna-web` order indices and
  `HASHED`, e2e fixtures, `golden.spec.ts`'s order count and
  `SAVE_AFTER`).
- [ ] `NOT_SIMULATED.md`: combat units by rail built; en-route pick-ups
  not. Design Status.
