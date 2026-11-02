# Mission 5, slice 7: replacement points from repair (§20.4x, 19.6x)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 5 left
"repaired points wait in `GameState.repaired` until organisation"; slice
6's non-goals left replacements beyond that pool). Every decision here is
the autopilot's under Brian's delegation of 2026-09-26 and is reversible.

**Goal.** Field repair (phase K) already turns broken-down tanks and
recce vehicles back into TOE points, but they wait in a side-wide pool
forever. This slice puts them back into units: a repaired point stands in
the hex it was repaired in, and a unit there absorbs it in phase C's
reorganisation segment. A tank battalion that broke down crossing the
Sollum escarpment is whole again two stages later, which is what makes
field repair worth its fuel.

**Non-goals.** Replacement points from the Reinforcement Track, the Axis
Replacement Pool and Commonwealth production (none arrive in Graziani's
Offensive: no replacement points before November 1940, 60.4x); training
(20.4x's stage counts) for anything but arrivals; replacement counters
moving as units (20.42: CPA, the −3 morale, the truck convoy phase): a
repaired point waits where it was repaired until a unit comes to it;
rebuilding eliminated units and HQ revival (19.62–19.63); upgrading recce
(20.5x); the two Autoblinda points that may start in Tobruk as
replacements (60.31; they start attached).

## Decisions

| Topic | Decision | Why |
|---|---|---|
| State | `GameState.repaired` becomes by side, hex and weapon system (`BTreeMap<Side, BTreeMap<HexId, BTreeMap<String, u32>>>`). A save of the old shape reads its points as lost (logged at load): Mission 5 is local only and the golden games have none | Points stand somewhere (20.41: absorbed "in the unit's hex") |
| Absorb | `Order::Absorb { piece, weapon, points }` in phase C (both players, as Attach): the unit and the points share a hex; 1 CP per two points, rounded up, charged to the unit and its parent if the parent is in the hex (19.6x, CP chart) | 19.61, 20.41 |
| Which points fit | Tank points fit any unit with tank points; a recce vehicle class fits a recce unit of that class; the weapon is added to the unit's `weapons` list (a new entry where it had none) | 19.6x's "any tank type fits any tank battalion" |
| Ceiling | A unit never exceeds its OA TOE (the sheet's `toe` sum, or `max_toe` for `N`) | 19.61 |
| Zero | A unit at zero TOE is eliminated and absorbs nothing | 19.62 |
| UI | The stack panel lists a hex's repaired points (the owner's only, 3.61) with an Absorb button per eligible unit in phase C | One panel, as Attach |

## Tasks

### Task 1: state by hex

- [x] Write the failing test: a field repair leaves its points in the
  wrecks' hex. Move `repaired` to by-hex; the view's redaction follows.

### Task 2: Absorb

- [x] Write the failing tests: absorb in phase C at 1 CP per two; refused
  outside phase C, in another hex, over the OA ceiling, for a non-tank
  unit taking tank points, for a unit at zero. Then `Order::Absorb`.

### Task 3: UI, docs

- [x] Stack panel lines and the Absorb button; `replacements.spec.ts`
  from the set-up fixture; `NOT_SIMULATED.md`.
