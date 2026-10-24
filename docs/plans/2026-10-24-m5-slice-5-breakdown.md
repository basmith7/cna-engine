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
| Capture, towing, repair | Capture by a battalion-plus moving adjacent (21.5x); towing 10 CP in phase K (21.6x); field repair on `vehicle-repair.json` (§22) | 21.5x, 21.6x, §22 |

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
- [ ] Armoured cars, recce and trucks inside a unit's TOE; the sandstorm
  shift (needs the CP spent under sandstorm per move).

### Task 3: markers, capture, towing, repair

- [ ] Broken-down marker pieces; capture; phase K towing and field repair.

### Task 4: UI, golden, docs

- [ ] Stack panel shows points accumulated and markers; the log names
  checks and losses; the golden re-pinned; `NOT_SIMULATED.md`.
