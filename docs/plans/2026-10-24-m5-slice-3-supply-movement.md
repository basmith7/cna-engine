# Mission 5, slice 3: supply movement

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 3; decisions
*Motorisation points*, *Organisation*, *Rail*). Every decision here is
the autopilot's under Brian's delegation of 2026-09-26 and is reversible.

**Goal.** Supply units stop being fixed where set-up put them: they gain
motorisation points in phase C, move with the army in G or alone in H,
ride the railway in J and fly once a turn, so the fuel and ammunition
slice 2 made scarce can follow the front. This is what turns abstract
supply from a tax into a logistics game, and it un-skips phases C, H
and J for module-on games.

**Non-goals.** Arrivals and convoys (slice 4); motorising foot
infantry with points (32.5x: how many points per unit is a request for
cna, below); §19 attachment (slice 6); sea movement of supply (32.3x:
needs the naval convoy stage, slice 4).

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Phase C's turn order | Joint in the rules (5.0); the engine asks Player A, then Player B, each answering with organisation orders and `pass` | One actor at a time keeps the log a sequence; nothing in C is secret from the other side but loads, which the view already hides |
| Where attached points live | `Piece.motorised: u32` on the supply unit; detached points go back to the hex's pool in `motorisation` | One owner per point; the pool already exists |
| How many | Exactly 30 for a real supply unit, 6 for a dummy (32.31); anything else is refused | The rule gives fixed figures |
| G movement | A supply unit with its points may be in a `Move` order's stack only with a friendly combat unit that starts and ends with it; CPA 15 for the stack's supply units (32.5x); a supply unit that moved in G may not move in H, and the reverse (32.31) | The restatement's "starts and stays stacked" read as moving together |
| H movement | Phase H stops being skipped for the module: each side's supply units with points move alone, CPA 15, then `pass` | 32.31 |
| Rail | Phase J: the Commonwealth moves up to two supply units, one direction, along the railway (to D3714, 60.7) | 32.3x, 60.7 |
| Air lift | One per side per turn in phase C, 12 fuel from a supply unit in the origin hex, between major cities, villages and boxes, with the 32.3x limits | 32.3x |
| Destroying own supply | An order in the phasing player's movement segment, its CP and die roll as cna gives them | 32.1x |
| Capture of points | Points attached to a captured supply unit go to the captor (32.5x) | 32.5x |

## Tasks

### Task 1: points on supply units, phase C

- [x] Failing test: a module-on game reaches phase C and stops for
  Player A (not `PhaseSkipped`); module off, C is still skipped.
- [x] `Order::AttachMotorisation { piece }` and `DetachMotorisation {
  piece }`: 30 or 6 points from or to the hex's pool; refused outside C
  (32.31), for a piece not a supply unit, with too few points in the
  hex, or when already attached; `MotorisationAttached` /
  `MotorisationDetached` events.
- [x] `Order::TransferLoad { from, to, ammo, fuel }` between supply units
  in one hex (32.1x), within 40/60.
- [x] View and redaction: `motorised` is the owner's only (3.61).

### Task 2: moving in G and H

- [x] A supply unit without points in a move is refused (32.31).
- [x] With points, only alongside a friendly combat unit, whole path;
  CPA 15 caps the stack's supply units (32.5x); `Piece.escorted` marks a
  supply unit that moved in G, which may not move in H.
- [x] Phase H un-skipped for the module; supply units move alone; `pass`.
  Deviation: H waits only when the phasing player has a motorised supply
  unit that did not move in G, and is skipped otherwise (as phase D is
  when nothing arrives), so the golden game does not churn.
- [x] Capture carries points to the captor: a real supply unit captured alone in an enemy ZOC (10.29) changes sides with its load and points (32.1x, 32.5x); dummies are still removed. Capture by entering a hex of lone enemy supply waits: entering an enemy hex is refused (8.1x).
- [x] The escort is one the supply unit started the segment with (or has moved with), from the review of #34.

### Task 3: rail and air

- [x] Phase J: Commonwealth rail moves, two per stage, one direction.
  The railway is the shortest finished line from Mersa Matruh to
  Alexandria's west hex (the Delta lines are not part of it; extension
  by construction waits for engineering). Outside G no ZOC declaration
  is asked: any Axis hex that could exert one counts (8.7x). J waits only
  when the Commonwealth has a supply unit that may ride, so the golden
  passes J once (495 orders). Combat units do not ride yet (R-005's
  units half).
- [ ] Air lift in C with its limits and 12 fuel. **Moved out of this slice**: its limits name division and brigade HQs and "the farthest hex row held by a division", and the OA data has no echelon; a request for cna is on `main`. It lands with §20 organisation (slice 6), which needs echelons too.
- [x] Destroying one's own supply: `DestroySupply { hex }` before anything moves in the segment, half CPA from every unit there, a die per supply unit (1–3, +1 at 1 SP or less), once per unit per segment.

### Task 4: UI and the golden game

- [x] Panels: phase C's controls (attach, detach, transfer), supply
  units tickable in G and H, rail in the move panel for J.
- [x] `tests/games/graziani-m5.json` re-recorded through the new phases
  (C and J passes, 495 orders); hashes and e2e fixtures re-pinned;
  Playwright drives one attach with a supply move in G, and one rail
  run. Deviation: the golden game itself passes C and J rather than
  attaching and moving supply, so the pinned fight is unchanged; the
  specs in `web/tests/supply.spec.ts` carry those orders.
- [ ] `NOT_SIMULATED.md`, journal; gates.

## Requests for cna

- How many motorisation points motorise a foot infantry unit (32.5x
  says "as medium trucks"; the truck capacity rules are in the Logistics
  Game). Blocks motorising infantry, not this slice.
