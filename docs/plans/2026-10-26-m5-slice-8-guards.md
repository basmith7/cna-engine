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

Revised while building (the first draft had guards fight as 0/1 points
and be made in phase C only; a capture in phase G would then have had no
phase C before its phase H march).

| Topic | Decision | Why |
|---|---|---|
| Guards as state | `GameState.guards: BTreeMap<Side, BTreeMap<HexId, u32>>`, guard points by owner and hex, not counters, open information. They do not fight | 28.21's counter is a 0/1 infantry point; points keep the state flat like prisoners, and fighting guards waits for a combat slice |
| Making guards | `Order::DetailGuards { piece, points }`: a combat unit's infantry TOE points (its parts of class infantry) become guards in its hex; it keeps at least one TOE point. `Order::ReleaseGuards { piece, points }`: guards in the hex rejoin a combat unit there with an infantry part, up to printed strength (19.61). Either in phase C or the owner's phase H, free of CP | 28.21, 28.22 say nothing of timing or cost; phase H is where a phase G capture is first marched |
| Marching | A march takes one guard per five prisoners (rounded up) from the start hex to the end hex; short of guards, the march is refused (28.17) | 28.17 with E-029 |
| Phase H | Opens when the phasing side holds prisoners or guards on the map | Guards alone may still be released |
| Escape | At the end of the captor's phase H, a hex marched into this phase keeps five prisoners a guard; one standing still keeps all with a guard (a camp). The rest escape: within 8 CP of a combat unit of their side (a foot path that avoids 32.17's blocked hexes) they leave play to their side's `escaped` tally; farther, they wait as `escapees` in the hex | 28.23–28.25; the walk toward friends is flavour (28.2x) |
| Liberation and recapture | A combat unit moving or reacting into a hex where the enemy holds prisoners with no combat unit of its own frees them (`escaped`) and takes the guards as its prisoners; into enemy escapees' hex, it takes them prisoner | 28.24, 28.26; guards not fighting makes the first a capture |

## Tasks

### Task 1: guard points

- [x] Write the failing tests: detail and release; outside phase C/H
  refused; a march without guards is refused, and the guards follow it.

### Task 2: escape

- [x] Write the failing tests: unguarded prisoners within 8 CP of a
  friendly unit rejoin at the end of phase H; farther ones wait as
  escapees; a camp keeps all with one guard; a marched hex five a guard;
  an enemy entering frees unprotected prisoners and recaptures escapees.

### Task 3: golden, UI, docs

- [x] The module-on golden details 3 guards from the IX Libyan battalion
  at C4021 before the march; goldens and e2e fixtures re-pinned.
- [x] Stack panel guard and escapee lines, a Detail/Release row;
  `combat.spec.ts` and `golden.spec.ts` drive the order by clicks;
  `NOT_SIMULATED.md`.
