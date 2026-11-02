# Mission 5, slice 6b: prisoners (§28)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 6, *Prisoners*).
Slice 6's plan (`2026-10-26-m5-slice-6-organisation.md`) Task 4 split in
two: this plan is §28; 24.9 dumps wait on cna (below). Every decision
here is the autopilot's under Brian's delegation of 2026-09-26 and is
reversible.

**Goal.** Captures already happen (15.88 and R-120 surrenders, 6.26,
10.36, 15.89's prisoner rolls) but the men vanish. Prisoner points make
them a thing on the map the captor has to walk to the rear, so a
surrender at Halfaya leaves a column marching on Sollum, and a later
victory or morale rule can count them.

**Non-goals.** Guards and escape (28.2x): no guard counters yet, so
nothing escapes; the 28.1x shift of up to three hexes at the instant of
surrender; detention camps; the store cost of feeding (Logistics Game
only); trucks carrying prisoners. Each is listed in `NOT_SIMULATED.md`.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| State | `GameState.prisoners: BTreeMap<Side, BTreeMap<HexId, u32>>`, the captor's points by hex, and `prisoners_out: BTreeMap<Side, u32>`, the points each captor has sent home. Old saves read both empty | Points, not counters: 28.1x counts points; one owner, like `motorisation` |
| Surrender | Each surrendering piece yields its **infantry** TOE points (its rated parts of class infantry); an HQ with none yields one (3.36) | 28.11: guns and tanks are captured equipment |
| 15.89 rolls | The table's points (the share of the losses just taken) are all prisoner points | The table already decides "prisoners rather than dead"; reversible if cna rules otherwise |
| Where | Prisoners stand with their captor: in the hex the captor's unit entered (6.26), the assaulting stack's hex (15.88, R-120, the defender's 15.89 losses), the assaulted hex (the attacker's 15.89 losses); a 10.36 surrender with no captor near leaves them where the stack stood | 28.12 has the captor take them; the shift is a non-goal |
| March | `Order::MarchPrisoners { from, path, points }` in the captor's phase H: 8 CP as foot infantry by the Terrain Effects Chart; never into an enemy-occupied hex or one an enemy ZOC reaches unless a friendly combat unit stands there; at most 40 at the end. A hex prisoners marched into this phase H marches no further, its earlier points included (they merge into one group); a march may not come back to its start | 28.13–28.15; points are fungible, so "the hex is done" stands in for per-point CP |
| Phase H | Opens when the phasing side holds prisoners on the map, as it does for supply units to move | 28.13: they move in the convoy phase |
| Departure | Points leave play on entering, or being taken in, a departure hex: Alexandria or Cairo for the Commonwealth; for the Axis, A2802, the hex units leave by for the Tripolitania box (8.8x), since cna has no Sirte box (request on `main`) | 28.15 |
| Display | `prisoners` and `prisoners_out` are open information; the stack panel and the map show a hex's prisoners | Prisoners are counters on the map |

## Crates touched

- `cna-game`: `prisoners.rs` (state, capture, march, departure), the
  capture sites in `combat/` and `movement.rs`, phase H's stop, the
  order.
- `cna-web`: the prisoners in the view; the stack panel's line and the
  march control.

## Testing

Unit tests in `prisoners.rs` for each row above; the module-on golden
game re-recorded (Halfaya's surrender now leaves prisoners, and phase H
opens for them), the e2e fixtures regenerated.

## Tasks

### Task 1: state and capture

- [x] Write the failing tests: a 6.26 surrender leaves the unit's
  infantry points as the captor's prisoners in the captor's hex; a
  15.89 roll's points join the captor; an HQ yields one; a capture in a
  departure hex goes straight out.
- [x] `prisoners.rs`: `capture`, the five call sites, serde defaults.

### Task 2: the march

- [x] Write the failing tests: march 8 CP and no further; refused into
  an enemy hex, into enemy ZOC, past 40, outside the captor's phase H,
  out of a hex marched into this phase; leaving play at the departure
  hex; phase H opens for prisoners alone.
- [x] `Order::MarchPrisoners`; phase H's stop.

### Task 3: golden, UI, docs

- [x] Re-record `tests/games/graziani-m5.json`, re-pin, regenerate the
  e2e fixtures.
- [x] The stack panel's prisoners line, a march panel in phase H (`combat.spec.ts` marches Halfaya's prisoners by clicks); `NOT_SIMULATED.md`.
