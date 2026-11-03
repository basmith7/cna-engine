# Mission 5: rail stacks that change their load en route (8.77, R-005)

Spec: `docs/designs/2026-10-13-land-turn-design.md`. `#79`'s plan
(`docs/plans/2026-10-27-m5-rail-units.md`) built one stack each way a
stage, from one hex to another, and left picking up and setting down
along the line as a non-goal. R-005 (option 2) lets each direction's one
stack change its load along the line, provided the unit and supply limits
hold at every point. Every decision here is the autopilot's under
Brian's delegation of 2026-09-26 and is reversible.

**Goal.** The Commonwealth's one train each way can drop a battalion at
Mersa Matruh's approaches and take a supply unit on at El Daba in the
same run, as the printed game lets it. It closes the last rail item in
`NOT_SIMULATED.md`.

**Non-goals.** A second run the same way in a stage (still refused).
Reversing direction mid-run. Bombing or cutting the line (the Air Game).

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Order | `Order::Rail` gains `stops: Vec<RailStop>` (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`), each `{ at: HexId, off: Vec<PieceId>, on: Vec<PieceId> }`, in path order, strictly after the start hex; no stops is today's run, so old logs replay unchanged | One order keeps the run atomic; R-005 |
| Path | Judged as today for the stack that starts the run (`rail_reaches`); each stop's hex must be on that path | The line and its ZOC limits do not depend on the load |
| Getting off | Pieces aboard (an attached unit gets off with its parent, as `carried` expands) stay at the stop, marked `railed` | 8.77 |
| Getting on | Pieces at the stop that `may_rail` (no CP spent, not railed, not in an enemy ZOC), expanded by `carried`, ride on to the end or a later stop | 8.77, 8.7x |
| Limits | After each stop: at most 2 SP of units aboard; supply units riding any part of the run count toward 32.3x's two a stage, all one way; supply stacking checked at each hex where supply gets off and at the end | R-005: "the limits hold at every point" |
| Events | One `Railed { pieces, from, to }` per group that boarded and alighted at the same hexes, in boarding order | The log reads each leg; no new event kind |
| UI | Engine first (this plan's Task 1); the browser's stops come in Task 2: after a run's destination is armed, clicking a hex on the shaded path makes it a stop where the ticked pieces there get on, and unticking a rider at a stop lets it off | Keeps Task 1 one PR |

## Tasks

### Task 1: the engine

- [x] Failing tests in `rail.rs`: a battalion rides from Alexandria and
  gets off at a stop while a supply unit there gets on and rides to the
  end; a stop that would put 3 SP aboard is refused (8.7x); a stop off
  the path is refused; a run without stops is unchanged.
- [x] `RailStop`, `Order::Rail { stops }`, the leg walk in `rail`, the
  events.
- [x] `NOT_SIMULATED.md`: drop "picking up and setting down en route by
  rail (R-005)" from the abstract supply row's *Not yet*; R-005's row.

### Task 2: the browser

- [ ] Stops on the armed rail path (`MovePanel`/`App.svelte`), a
  Playwright test that drops a battalion and takes on a supply unit.
