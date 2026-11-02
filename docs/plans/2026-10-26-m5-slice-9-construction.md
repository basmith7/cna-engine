# Mission 5, slice 9: fortifications and construction (§24–26)

Spec: `docs/designs/2026-10-13-land-turn-design.md`. The spec's slices
1–6 are built except what waits on cna (attachment limits, 24.9 dumps,
convoy bombing); the queue row names engineering and construction
(§24–26) too, and Graziani's Axis sets up seven engineer battalions, so
construction is live in this scenario although it starts with none
(60.7). Every decision here is the autopilot's under Brian's delegation
of 2026-09-26 and is reversible.

**Goal.** Cities defend like the fortresses they are, and engineers dig:
the Axis can fortify the frontier or mine the coast road, and an assault
on Tobruk meets its level-2 fortification instead of the bare city
terrain.

**Non-goals.** Roads, railways, air facilities and repair facilities
(24.5x–24.8x: no unfinished roads or air game in Graziani's); bombing
(Air Game); 22.3x facility repair; barrage against facilities (12.5x,
needs the Air Bombardment Table); the Logistics Game's stores and water
(32.11 ignores them).

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Fortification state | `GameState.fortifications: BTreeMap<HexId, u8>`, built or reduced levels; a major city with no entry is level 2, Alexandria and Cairo 3 (25.11, TEC footnote 4, E-031) | Cities are data; only changes need state |
| Combat shift | A hex's in-hex shift is the best of its terrain's and its fortification level's TEC row, not their sum; a major city's terrain row is "see fortifications", so its level's row. Armour targets in anti-armour fire take a fortification's benefit only in a major city (TEC footnote 12) | 12.33, 14.3x: "best single benefit"; footnote 12 |
| Level-3 cities | Units in a level-3 city ignore pinned results (25.24) | |
| Construction segment | Phase C opens for a side with an engineer that could start or is running a project. `Order::Build { item, hex, builders }` starts one; a project completes at the start of a later phase C after the chart's stages; builders spending CP, leaving, or being pinned that stage lose it (24.1x, 24.2x) | 24.11–24.16 |
| Costs under §32 | Stores and water are ignored (32.11); a real minefield's 15 ammunition comes from a supply unit in the hex | 32.11 ignores stores and water only; a supply unit carries ammunition. Request on `main` asks cna to confirm |
| Fortifications | One engineer-capable unit plus an infantry battalion of 3+ TOE, three stages, one level at a time to 2 (cities back to their own level), never in salt marsh, delta, major city, mountain or desert, never begun in enemy ZOC unless rebuilding (24.4x, the chart) | |
| Minefields | An engineer battalion or company (or a CW E-HQ) lays a real or dummy field in one stage, one per hex, in the chart's terrain, never in a major city or enemy ZOC; owner-secret real/dummy, flipped when an enemy enters; TEC entry costs, engineer escorts at +4 CP, the 5–6 mine roll for unescorted vehicles, the combat shift, clearing and dummy removal (24.3x, 26.x) | |
| Chart vs text | Minefield terrain follows the chart (clear, sand-gravel, salt marsh) where 24.3x's text says rough instead of salt marsh | Data governs, as R-018 does for dumps; noted for cna |

## Slices

- **9a** fortification levels in combat and the level-3 pin immunity.
- **9b** the construction segment and building fortifications.
- **9c** minefields: laying, entry, revealing, mine damage, clearing.

## Tasks

### Task 1 (9a): fortification levels in combat

- [x] Write the failing tests: a unit in Tobruk is barraged and
  assaulted on the fortification-2 row; a built level 1 in rough takes
  rough's better shift where it is better; an armour target outside a
  city gets no fortification benefit in anti-armour fire; a level-3
  city ignores pinned.
- [x] `fortification_level`, the shift in `combat/terrain.rs`, the pin
  check (PR #48).

### Task 2 (9b): building fortifications

- [x] Write the failing tests: Build refused without an engineer and a
  3-TOE infantry battalion, in a forbidden terrain or enemy ZOC, or a
  second project; the level goes down at the fourth phase C; a builder
  spending CP loses the stage.
- [x] `construction.rs`, a `construction` module that opens phase C,
  the order, the stack panel's Fortify control (`construction.spec.ts`),
  `NOT_SIMULATED.md`. As built: the level goes down when the third idle
  stage is counted at the OpStage's end, not at the next phase C (no
  combat falls between); pinned builders are not yet checked.

### Task 3 (9c): minefields

- [x] Laying: `Item::RealMinefield`/`DummyMinefield` on `Order::Build`,
  `GameState.minefields`, 24.33's ammunition, the view's secret, the
  stack panel's Lay controls (`construction.spec.ts`).
- [x] Entry: TEC costs with engineer escorts, revealing on enemy entry,
  dummy removal at the end of the movement phase, the 5–6 roll per
  unescorted motorised combat unit, the combat shift (26.26).
- [x] Clearing (26.11): `clear-minefield` on `Order::Build`, a one-stage
  project for one engineer (a choice, so an order), friend or foe.
- [ ] Later: the trucks' roll, and mine costs on a step into enemy ZOC.
