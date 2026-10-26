# Mission 5, slice 8: prisoner guards and escape (28.2x)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 6, *Prisoners*);
slice 6b (`2026-10-26-m5-slice-6b-prisoners.md`) built prisoner points and
the march and left guards and escape out. Every decision here is the
autopilot's under Brian's delegation of 2026-09-26 and is reversible.

**Goal.** Prisoners cost their captor something: infantry points turned
into guards, or men who walk home. Today the Axis marches Halfaya's 12
west for free; with guards it gives up three infantry points to do it.

**Non-goals.** Escapees walking toward friends beyond 8 CP (28.2x says the
rule is for flavour; they stay put and are recaptured by an enemy entering
their hex); the replacement-point return a month later (20.4x arrivals are
not built); detention camps beyond "five or more standing still need one
guard"; trucks.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Guards as state | `prisoners` gains guards: `GameState.guards: BTreeMap<Side, BTreeMap<HexId, u32>>`, guard points by hex, not counters. A guard point fights as one infantry TOE point rated 0/1 in the hex's defence only (no attack) | 28.21: a one-point infantry unit 0/1; points keep the state flat like prisoners |
| Making guards | `Order::DetailGuards { piece, points }` in phase C: the unit loses that many infantry TOE points, the hex gains guard points; `Order::ReleaseGuards { hex, points, to }` gives them back to a unit in the hex | 28.21, 28.22 |
| Marching | A march takes one guard per five prisoners (rounded up) along with it; short of guards, the march is refused | 28.17 with E-029 |
| Escape | At the end of the captor's phase H, prisoners beyond their hex's ratio (one guard per five on the move, one per camp of six or more standing still) escape: within 8 CP of a friendly unit (foot infantry, no enemy ZOC or units on the path) they leave play for their own side (`escaped` tally); farther, they become escapees in the hex | 28.23–28.26 |
| Recapture | An enemy combat unit entering escapees' hex makes them its prisoners again | 28.26 |

## Tasks

### Task 1: guard points

- [ ] Write the failing tests: detail and release in phase C; a march
  without guards is refused; guards defend a hex alone at 1.

### Task 2: escape

- [ ] Write the failing tests: unguarded prisoners within 8 CP of a
  friendly unit leave at the end of phase H; farther ones wait as
  escapees; an enemy entering recaptures them.

### Task 3: golden, UI, docs

- [ ] The module-on golden details 3 guards at C4021 before the march;
  stack panel lines; `NOT_SIMULATED.md`.
