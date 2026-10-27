# cna-engine: the rest of the Land Game turn

**Date:** 2026-10-13
**Mission:** 5 of `cna-engine` (`AUTOPILOT.md` queue), Part 0.
**Status:** built for Graziani's Offensive, bar the items below (2026-10-27).
Slices 1–9 merged as PRs #31–#60 (organisation, shells and the stack
panel, prisoners off map by R-132, guards, replacements, construction);
#59 adds formations to close assault's size table (15.55). Their plans
are in `docs/plans/`. #75 judges shells against the 19.3 chart's
counted components (R-141); #77 captures lone enemy supply entered
(32.1x); #78 makes reactions and every retreat pay fuel (32.22, R-120).
Still open for Graziani's: convoy lanes beyond Tripoli, combat units by
rail, substitution (waits on cna mapping counters to chart slots). The
mission's second half, other scenarios' land rules (*Non-goals*), has
begun: #61 smoke-tests every scenario (`cna-game::scenarios`), #63
scores points victories, #62 lets the browser start any of them. With
cna 803d5d5 (#65) Rommel is a commander piece (R-135) and replacement
counters are made at set-up (R-096), so The Last Chance joins; with cna
fc78330 (#67) the Italian divisions' hexes are listed and set up two
apart (R-136), so all nine set up and play a Game-Turn. #66 builds 31.0's initiative, recall
and morale, #68 his +5 CPA escort (R-137), reactions and free retreat
through enemy ZOC, #70 his capture alone in an enemy ZOC once his escape is spent (R-138), #74 his own retreat when an assault destroys his stack (R-140); #73 makes a reaction pin only on an enemy ZOC the unit stood in before the mover came (R-139); the LRDG raid (27.9x) remains, with the desert raiders. Mission 4 still comes first the moment its spec
(PR #25) is approved (see *Order against Mission 4*).

Written by the autopilot under Brian's delegation of 2026-09-26: every
decision below is Claude's, is cheap to reverse, and can be reversed with
one PR. The mission itself (organisation, breakdown, construction, the
special rules and abstract supply, each behind the existing phase skips
and shown in the UI) is Brian's, from the queue.

## Goal

Graziani's Offensive played with the Land Game's whole turn instead of
the slice Missions 2 and 3 built. Today the engine passes through phases
C, H, J, K and L and the G-phase breakdown segment with a
`PhaseSkipped` event, every unit counts as supplied with unlimited
ammunition and fuel, and the supply units placed at set-up are inert
counters. When this mission is done, supply is the constraint it is in
the printed game: units spend fuel to move and ammunition to fight, from
supply units they can reach; supply units carry finite loads, arrive by
the abstract tables, move on motorisation points and can be captured;
vehicles break down and are repaired; formations attach and detach in
the organisation phase.

How this moves **the goal** (two players, a browser, every rule enforced):
the game Mission 3 put in a browser is a combat-and-movement game with the
logistics switched off; CNA *is* its logistics. §32's abstract supply is
the rules SPI wrote for exactly this case (the Land Game without the Air
and Logistics Games, 32.0, which Graziani's Offensive applies by 60.92).
Every rule added here sits in `cna-game`'s state machine, so the network
engine of Mission 4 serves it unchanged.

## Non-goals

- **The Logistics Game (§48–58) and the Air Game (§33–47).** Later rows of
  the queue; §32 is their abstract stand-in and is what this mission
  builds. The `air_and_logistics` abstraction block stays the one applied.
- **Phase E (Commonwealth fleet, §30) and 32.7x (bombing the fleet).** The
  fleet does nothing in a Land-only Graziani game; it stays skipped.
- **Phase L (patrol).** Patrol points belong to the Air Game; skipped.
- **§27 raids and §31 (Rommel).** No unit in Graziani's Offensive can use
  them (Rommel arrives after Game-Turn 6).
- **Other scenarios.** Their land rules are the mission's second half,
  after Graziani's is whole: one slice per scenario that needs a land rule
  Graziani does not exercise (fortifications, minefields, engineers
  building, §27), each with its own plan written when it starts.
- **Limited intelligence beyond the view's redaction.** 3.61's table is
  honoured where a slice reveals something new (supply loads and
  motorisation points are secret, dummies look like real supply units);
  the full table is the server's (Mission 4).

## What "done" means

1. A Graziani game started with `cna-play new` carries every supply unit's
   ammunition and fuel (60.92), each side's motorisation points
   (60.92's conversion of medium trucks) and no AA points 32.8x removes.
2. A motorised unit that cannot reach a supply unit with fuel cannot move
   by land; a unit that cannot pay ammunition cannot barrage or assault;
   each refusal names its case (32.2x). Spending is logged per supply
   unit and replays exactly.
3. Phases C, D, H, J and K and the breakdown segment do what §19–22, §32
   and §21 say for this scenario; the only phases still skipped are E and
   L (and those the non-goals name).
4. Each new decision is an order, legal or refused, in the UI's panels;
   the golden game is extended through the new phases and the Playwright
   golden e2e still matches `replay` byte for byte.
5. `NOT_SIMULATED.md`'s *Systems* table no longer lists abstract supply,
   breakdown, organisation or repair for Graziani's Offensive.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Which § numbers | cna's `::: spi` badges, which are SPI's: §19 assignment and attachment, §20 reinforcements and replacements, §21 breakdown, §22 repair, §23–26 engineering, construction and fortifications, §28 prisoners, §32 abstract supply. The queue row's "§20 organisation, §19 supply" shorthand is corrected here | Errors cite SPI cases; the numbers must be SPI's |
| Order | Supply first (state, then spending, then movement and arrivals), then breakdown and repair, then organisation, then construction | Supply changes every stage of a Graziani game; breakdown every motorised move; organisation and construction rarely in six turns (60.7: no minefields, fortifications or pipeline) |
| Supply-unit state | `Piece` gains `load: Option<Load { ammo, fuel }>` for real supply units (dummies have none); a unit with both at zero is removed (32.1x). Loads come from the scenario's `supply` rows | One owner for a dump's contents; the piece already exists |
| Motorisation points | A per-hex pool per side, `motorisation: BTreeMap<Location, u32>`, plus points attached to a supply unit (30 real, 6 dummy, 32.3x) or carrying a unit (32.5x) recorded on that piece. Starting points from each deployment's `trucks.medium` (60.92's note; light and heavy give none) | Points are not counters; they are a count where trucks stand |
| Draw range | A unit draws from a friendly supply unit whose hex is within half its CPA (rounded as cna says) by `reach`'s medium-truck costs, foot infantry as infantry; never through impassable terrain or an enemy ZOC without a friendly combat unit; supply units never count as units in the trace (R-119) | `reach.rs` already computes exactly this kind of path; R-110's trace is a precedent |
| Fuel | Paid the first time a fuel-using unit moves by land in a stage, at the instant it moves, from one supply unit in range the mover names (default: the nearest with fuel, ties by piece id); paid again on exceeding CPA (32.22). `fuel_rate` from `weapon-systems.json` where cna gives one, else the 32.22 class rates | The mover chooses which dump, as at the table; a default keeps scripted orders short |
| Ammunition | Paid per battalion-equivalent (9.4's equivalent; a division by its components) at the instant of combat: non-phasing 1 to defend in an assault, 2 to barrage; phasing double. Pinned, retreated-in and withheld units pay none (R-013, which leaves `NOT_SIMULATED.md`) | 32.21 as restated |
| Unable to pay | Until cna answers the request below: a unit that cannot pay fuel may not move by land (refused, 32.22); one that cannot pay ammunition may not barrage or attack (refused, 32.21) and **defends at full strength**, logging `AmmunitionShort` | The narrowest reading that stops free movement without inventing a penalty; switchable once cna rules |
| Supply stacking | At most 5 supply units in a major city hex, 3 elsewhere, any in the Tripoli–Tunisia boxes (32.1x); road and track stacking limits ignored (32.9) | Data-free, from the restatement |
| Capture | An enemy combat unit entering a supply unit's hex captures it with its load, usable at once (32.1x); a dummy is removed when found | 32.1x |
| Arrivals | The turn gains the naval convoy stage (sequence II) in the clock: the planning roll on `simplified-supply.json` (Axis two turns ahead under the planning month's letter, Commonwealth four ahead under the arrival period, 32.4x), Axis convoys by lane (32.6x, `axis-convoy-*.json`), monthly motorisation losses (32.5x, `motorisation-losses.json`) | The clock should have every printed step; arrivals come through phase D like any reinforcement |
| Breakdown | `Segment::Breakdown` runs 21.3x: each vehicle class's accumulated points by terrain (`terrain-effects.json`'s `breakdown` column) and weather, the column shift by `bar` (`weapon-systems.json`) or class (`breakdown-adjustments.json`), two dice on `breakdown.json`; broken-down TOE points become a marker on the hex. Motorisation points never break down (32.5x) | All of it is data in cna today |
| Repair | Phase K: field repair (22.2x) from `vehicle-repair.json` in slice 5; towing (21.6x) and facility repair (22.3x, 22.4x, `destroyed-tank-repair.json`) with the repair facilities in the construction slice, since towing only goes toward one | Breakdown without repair only bleeds |
| Organisation | Phase C, reorganisation segment: attach and detach under §19 with `max-attachments.json` and `formation-organisation.json`; rearrange supply loads in a hex (32.1x); attach or detach motorisation points (32.3x); the once-a-turn air lift of a supply unit (32.3x) | One phase, one panel |
| Construction | Phase C, construction segment: 24.9 dumps only for Graziani (60.7 forbids the rest), with `cna-rules::construction::dump_cost` under R-018 | Graziani needs no more; the other scenarios' slices add the rest |
| Rail | Phase J: the Commonwealth moves two supply units per stage one direction on the railway, which ends at D3714 (60.7, 32.3x) | The only rail use in Graziani |
| Prisoners | §28's guards and the phase-H march, with `prisoners-captured.json` already loaded | Captures already happen; their prisoners should exist |
| Switch | One ruleset switch, `abstract_supply` (on by default for Graziani), turns slices 2's spending off so the M2/M3 golden game still replays under the old rules | A group may still play the supply-free game, and the old golden stays a regression test |
| UI | Each slice adds its orders to the existing panels (`GamePanel`, `MovePanel`, the combat panels) and its state to the stack panel: loads, points, broken-down markers. Secret values (loads, points) appear only in the owner's view | "Shows up in the UI's panels", per the queue |

## Order against Mission 4

Mission 4 (the multiplayer server) is the current mission and waits on
Brian's review. Under "never idle" this mission's spec and its first
slices go ahead meanwhile; the moment **Feedback** approves PR #25 the
autopilot finishes the slice in hand and switches to Mission 4. Nothing
here touches the server's seams: `GameState`, orders and the redacted
view keep their shapes, and each slice only adds variants and fields.

## Crates touched

- `cna-data`: `simplified-supply`, `motorisation-losses`,
  `axis-convoy-*`, `breakdown`, `breakdown-adjustments`, `vehicle-repair`,
  `destroyed-tank-repair`, `max-attachments`, `formation-organisation`
  loaders; `weapon-systems` exposes `class`, `fuel_rate`, `bar`.
- `cna-rules`: pure helpers (ammunition owed, fuel owed, breakdown column).
- `cna-game`: `supply` (loads, draw, spend), `motorisation`, `breakdown`,
  `repair`, `organisation`, `convoy`; `phases.rs` stops skipping each
  phase as its slice lands; `Order` and `Event` gain variants.
- `cna-web` and `web/`: the new orders and panels.

## Testing

- Rule tests per case, failing first, each refusal naming its case.
- Random walks both ways (as `reach.rs` does) for the draw range: every
  supply unit `in_range` returns is accepted by `apply`, and no other.
- The golden game is extended per slice and pinned by hash; the old golden
  replays with `abstract_supply` off.
- Playwright: each slice's new panel driven by clicks to the pinned hash.

## Slices

One PR each, in order; each has a plan in `docs/plans/`.

1. **Supply state** (`2026-10-13-m5-slice-1-supply-state.md`): loads,
   motorisation points, supply stacking, capture, 32.8x AA removal at
   set-up; the view and stack panel show them to their owner. No spending.
2. **Spending** (`…-slice-2-spending.md`): the draw range, fuel on the
   first land move and over CPA, ammunition at barrage and assault, the
   refusals, the `abstract_supply` switch, empty dumps removed.
3. **Supply movement** (`…-slice-3-supply-movement.md`): phase C's
   motorisation attach/detach and load rearranging, supply units moving in
   G (stacked with a combat unit) or H, phase J rail, the air lift,
   destroying one's own supply, dummies.
4. **Arrivals** (`…-slice-4-arrivals.md`): the naval convoy stage,
   simplified supply planning, Axis convoys, monthly motorisation
   losses. Convoy bombing followed once cna had the lanes (56.1):
   `…-convoy-bombing.md` bombs the Tripoli lane (lane 2) from the
   westernmost Commonwealth division HQ; lanes beyond Tripoli wait.
5. **Breakdown and repair** (`…-slice-5-breakdown.md`): the breakdown
   segment, markers, towing, phase K repair.
6. **Organisation, dumps and prisoners** (`…-slice-6-organisation.md`):
   §19 attach and detach, 24.9 dumps, §28 prisoners, docs (README,
   `NOT_SIMULATED.md`, this Status, the queue).

Slices 1 and 2 have full plans with this spec; 3–6 share one task list,
`2026-10-13-m5-slices-3-6.md`, and each gets its own plan at its start,
written against the code slices 1 and 2 leave behind.

## Risks

- **"Battalion-equivalent" for odd units.** A company pays as a fraction
  or not at all? 32.22 says company-equivalents mostly pay none; 32.21
  does not say. Slice 2 follows 9.4's equivalent and treats a company as
  zero for ammunition, logged; a request asks cna.
- **Golden-game churn.** Every slice changes the golden log's tail. Each
  slice re-pins by hash in one commit, as Mission 3 did.
- **Performance of the draw range.** One reach per mover per stage, from
  the supply unit outward (a reverse search) rather than per unit.

## Requests for cna this mission makes

Posted to `PROGRESS.md` **Requests for cna** with this PR:

1. **What a unit may do that cannot pay (32.21, 32.22).** Can a
   fuel-using unit with no fuel in range move at all (by land, by rail,
   reacting, retreating)? Does a defender with no ammunition fight at full
   strength, reduced, or not at all? Slice 2 refuses the move and the
   attack and lets the defence stand.
2. **Ammunition for company-equivalents (32.21).** Does a company pay a
   fraction, one, or nothing? Slice 2 charges nothing.
3. **Starting motorisation points as data (60.92).** The conversion is a
   note (`abstractions.air_and_logistics.motorisation`); a typed count per
   deployment or per side would let the engine stop reading `trucks`.
   Not blocking: slice 1 reads `trucks.medium`.
