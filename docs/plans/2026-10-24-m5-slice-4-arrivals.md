# Mission 5, slice 4: arrivals (the naval convoy stage, phase D)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (slice 4). Every
decision here is the autopilot's under Brian's delegation of 2026-09-26
and is reversible.

**Goal.** Supply stops being a fixed stock: each turn both sides plan
supply units that arrive later (32.4x), Axis convoys run the gauntlet of
the Commonwealth's bombers (32.6x), and motorisation points wear away
month by month (32.5x). With slice 3's movement this closes the abstract
supply loop, so a long scenario can be played without running dry by
construction.

**Non-goals.** Replacement points and the replacement pool (§20, slice
6); the Logistics Game's ships, ports and cargo (§48–58); the air lift
(needs HQ echelons from cna); tank and gun losses at sea (no tank or gun
replacements travel yet).

**Unblocked (2026-10-25)**: cna's `data/tables/calendar.json` maps every
Game-Turn to a year, month and week (four weeks a month); `cna-data::calendar`
reads it. Branched from the cna bump (PR #36), which brings it in.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Where the naval convoy stage sits | Run by the engine as the clock enters each Game-Turn's first OpStage, before phase A, module-on only; no clock change, since nobody acts in it yet (a real stage when lanes give players choices) | 5.2's stage II; old logs must replay unchanged |
| Who acts in it | Nobody yet: planning is two die rolls the engine makes, arrivals are placed by the engine | Slice 4 has no choices that matter until lanes do |
| Planning | At each turn's stage II, except the scenario's first turn: the Axis rolls under the tonnage letter of the planning month and gets that many supply units two turns later; the Commonwealth rolls under the arrival turn's period and gets them four turns later (32.4x) | 32.4x |
| Tonnage letter | `axis-convoy-level.json` for the month planned in (32.4x says the planning month; 56.21 uses the sailing month for the full game) | 32.4x names the planning month |
| Commonwealth arrivals | Phase D of the arrival turn's first OpStage: in Cairo, one in Alexandria per three received (32.4x), loaded 40/60, supply stacking checked (a full city sends the rest to Cairo… or a request for cna if neither fits) | 32.4x, 32.1x |
| Axis arrivals | Phase D: by convoy to the lane's port, at most three supply units and twenty motorisation points per lane (32.6x); one lane (Tripoli) until lanes are data the engine can read | Graziani's Axis supply comes through Tripoli and Benghazi |
| Convoy bombing | The lane's column of `axis-convoy-bombing.json` against the westernmost full Commonwealth division, two dice on `air-bombardment.json`; 10 % hits points only, 20 %+ everything, at least one of each (32.6x) | 32.6x |
| Dummies | One per three real units received (32.1x) | 32.1x |
| Monthly motorisation losses | The first stage II of each month: `motorisation-losses.json` for the previous month, of all the side's points (boxes included), removed from map E (Commonwealth) or map A and the boxes (Axis) (32.5x) | 32.5x |

## Tasks

### Task 1: the stage and the planning rolls

- [ ] Failing test: a module-on game opens each Game-Turn after the
  first with a planning event per side; module off, nothing changes.
- [ ] `GameState.planned: Vec<Planned { side, turn, units }>`; events
  `SupplyPlanned`.

### Task 2: Commonwealth arrivals

- [ ] Phase D places the turn's Commonwealth units in Cairo and
  Alexandria (one in three), with their dummies.

### Task 3: Axis convoys and bombing

- [ ] One lane, its cap, the bombing chart and the bombardment table.

### Task 4: motorisation losses

- [ ] The monthly chart, rounded down per hex; the event names the loss.

### Task 5: UI, golden, docs

- [ ] The log names planning, arrivals and losses; the stack panel shows
  arrived units; the golden re-pinned; `NOT_SIMULATED.md`.
