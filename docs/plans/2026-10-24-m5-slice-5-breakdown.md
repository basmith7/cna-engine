# Mission 5, slice 5: breakdown and repair (segment G-breakdown, phase K)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 5). Every
decision here is the autopilot's under Brian's delegation of 2026-09-26
and is reversible. Taken ahead of slice 4, which waits on a Game-Turn
calendar from cna (the order of slices 4 and 5 does not matter to
either).

**Goal.** Distance costs vehicles: tanks, armoured cars and trucks that
drive far break down (§21), so a desert offensive outruns itself as it
did in 1940, and broken-down vehicles become something to tow, repair
and capture. It un-skips the breakdown segment and, with repair, phase K.

**Non-goals.** Repair facilities beyond field repair, tank delivery
squadrons (§24–26, engineering); the Logistics Game's trucks (breakdown
of motorisation points is excluded by 32.5x anyway); air strikes on
broken-down vehicles.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Rules core | `cna-rules::breakdown`: column (BAR + weather, 71+ caps, below 4–10 nothing), percent, losses (fractions up, one point ignores 10 %), must-check (a higher unshifted column than the last check) | 21.2x, 21.3x |
| What accumulates | Per piece, `breakdown: f64` this OpStage, the terrain's `breakdown` value per hex entered (and wadi hexsides), in every kind of move; reset at the stage's start | 21.2x: points last through both halves |
| Which TOE points | Tank, SP-gun, armoured-car and armoured-recce weapon systems on the piece; BAR per model from the characteristics where cna gives it, else the type's base | 21.1x |
| When | At each stop (end of an order) a unit over 3 points in a higher column checks, one reading per vehicle group | 21.2x |
| Broken-down points | A `BrokenDown` marker piece in the hex (no CPA, no combat), holding the points by weapon system; half left in the start hex when the move began near the enemy (21.41) | 21.4x |
| Capture, towing, repair | Capture by a battalion-plus moving adjacent (21.5x); field repair on `vehicle-repair.json` (22.2x), one order per type per hex, a fuel a point from a supply unit in the hex or in range of a friendly unit there; repaired points wait in `GameState.repaired` for organisation (22.5). Towing (21.6x) goes only toward a repair facility and facility repair (22.3x) needs one, so both move to the construction slice with the facilities | 21.5x, 21.6x, §22 |

## Tasks

### Task 1: rules core

- [x] `cna-data::breakdown` (both tables) and `cna-rules::breakdown`,
  with 21.3x's worked example as a test.

### Task 2: accumulating and checking

- [x] A `breakdown` module (on in new games) so old logs replay; motorised
  units collect step points in moves, reactions and retreats
  (`Piece.breakdown_halves`, whole and half points only), reset at the
  stage's start.
- [x] At each stop a check per vehicle weapon system in a higher column
  (`Piece.breakdown_checked`), BAR by model from `weapon-systems.json`,
  hot weather one right; losses leave the TOE.
- [ ] Moved out: armoured cars, recce and trucks inside a unit's TOE (cna R-125 types recce; take it with the bump). The sandstorm
  shift is done, read per order rather than per phase.

### Task 3: markers, capture, towing, repair

- [x] Broken-down points kept per hex and side, by weapon system (`GameState.broken_down`): a map of wrecks rather than marker pieces, so no piece-kind match changes.
- [x] Capture (21.5x) by a battalion or brigade moving next to unguarded enemy wrecks out of enemy ZOC; they change sides where they stand.
- [x] 21.41: a move begun next to an enemy brigade-equivalent, or two hexes from one through an open hex (no friendly unit, no friendly ZOC, not sea), leaves at least half of what breaks down in its start hex.
- [x] Phase K field repair (22.2x): `Order::Repair`, `cna-data::repair`, K skipped when nothing may be tried; wrecks marked on the map and listed in the stack panel.
- [ ] Moved out: capture's three-hex placement and passable-terrain test; towing (21.6x) and facility repair, with the facilities (construction slice).

### Task 4: UI, golden, docs

- [x] Stack panel shows points accumulated and wrecks; the log names
  checks, losses, captures and repair rolls; the golden re-pinned;
  `NOT_SIMULATED.md`. Playwright: `supply.spec.ts` phase K.
