# Mission 2, slice 4 (movement) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Legal movement in phase G and the reaction and reserve rules
around it: CP and CPA (§6), terrain and hexsides (§8), stacking (§9),
zones of control and contact (§10), reserves (§18), off-map boxes and
reinforcements.

**Architecture:** Pure cost and eligibility helpers in `cna-rules`
(`movement::step_cost`, `zoc::exerts`, `stacking::points`), state changes
in `cna-game::{movement, zoc, reserve}`. Terrain costs come from
`terrain-effects.json` (errata applied); `cna_data::terrain` grows the
`cp`, `breakdown` and `stacking` columns.

**Spec:** `docs/designs/2026-10-08-game-state-design.md`
**Previous plans:** slices 1–3.

## Domain facts the implementer needs

(cna `rules/30-capability-points.md`, `40-movement.md`,
`50-stacking-and-zoc.md`; case numbers are SPI's.)

- **CPA** from the piece's characteristics row; a printed 0 counts as 10
  except for movement (6.1x). A stack moving together uses its lowest CPA
  (6.15). CP spent comes from one pool per OpStage, including the enemy's
  half (6.14). Each CP over the CPA is 1 DP, applied to cohesion at once
  (6.21). A non-motorised unit (CPA 10 or less) may not voluntarily spend
  over 150 % of its CPA in its own half (8.1x).
- **Step cost** (8.3x, `terrain-effects.json`): the entered hex's
  `cp.non_mot` or `cp.mot`, plus the crossed hexside's (slope and
  escarpment by `up`, wadi, ridge, rivers). Along a connected road
  hexside the road rate applies and other terrain is ignored, except
  escarpments for vehicles; along a track, half the hex cost (E-031). An
  unfinished road is a track. Vehicles: never up an escarpment; down only
  by track; never across a major river except by road; no desert for light
  trucks and motorcycles; salt marsh by road or track only, except light
  trucks, motorcycles and recce (8.4x). A mixed formation pays the dearer
  cost and obeys both sets of prohibitions (R-004). The Commonwealth may
  never go west of Marble Arch A2109.
- **Weather** (§29): sandstorm doubles movement costs; rain makes roads
  tracks and wadis impassable except by road.
- **Stacking** (9.1x–9.3x): ceiling from the hex's terrain row (6; 3
  mountain; 8 major city); checked at the end of each friendly movement
  segment and at the end of the stage, never mid-move. Points by
  equivalent (9.4, `stacking-point-values.json`); an HQ with nothing
  attached is 0. The road limit (9.33) is off in a Land-only game (32.9).
- **ZOC** (10.1x–10.2x): a hex with more than 1 SP of combat units, not
  collapsed (−26), with at least 10 raw close-assault defence points,
  exerts a ZOC into adjacent hexes, not across sea, major river, lake or
  escarpment hexsides, nor into hexes it could not enter. The non-phasing
  player declares whether it is exerted (10.16): a `Pending::DeclareZoc`.
  Entering an enemy ZOC costs nothing and ends the move; no ZOC-to-ZOC; a
  friendly *combat* unit in the hex cancels it. A lone non-combat unit or bare HQ
  in an enemy ZOC is captured.
- **Contact** (8.6x, R-003): a unit is in contact if in an enemy ZOC at the
  start of either side's movement segment. Leaving costs 2 CP (break
  contact) or 4 (disengage, if Engaged).
- **Continuation** (8.2x): after the first movement segment of a G phase,
  only units within 2 hexes of an enemy combat unit, and released
  reserves, may move again.
- **Reaction** (8.5x, R-006, R-007, R-008): when a phasing unit moves
  adjacent, motorised non-phasing combat units not in an enemy ZOC, contact
  or Engaged may react: a `Pending::Reaction` offer. A unit may not react
  to an enemy whose CPA is 6 or more higher than its own and who declares
  an assault on it (pinning, 8.5x). Reaction moves never
  enter an enemy ZOC and spend CP only on movement.
- **Reserves** (§18): designated in phase F by the phasing player;
  Reserve I may shift one hex per movement segment (not into an enemy
  ZOC); at the first reserve-release segment it is released or becomes
  Reserve II. Released from I: may not exceed CPA, at most one assault.
  From II: half CPA (5 for leg infantry), at most one assault or probe,
  +1 DP for voluntary combat.
- **Off-map boxes** (8.8x, `off-map-distances.json`): Axis only; moving
  between a box and the map takes whole OpStages.
- **Reinforcements** (20.1x): in phase D of their stage; Commonwealth at
  Cairo, Axis at Tripoli; no CP to land or for the first hex. Stacking
  binds only returning withdrawn units (which choose Cairo or Alexandria)
  and the Benghazi diversion (out of scope).

## Tasks

Each task: write the failing tests, run (FAIL), implement, run (PASS),
commit. Every rejection test asserts the case number and an unchanged state.

1. **Terrain columns and step cost.** `cna_data::terrain` reads `cp`,
   `breakdown`, `stacking`; `cna_rules::movement::step_cost(mot, from,
   to, hexside, weather) -> Result<Cost, Prohibited>`. Tests: hand-priced
   steps on the real map: clear, rough, up and down a slope, down the
   Halfaya escarpment by track (C3922 side), along the coast road, a
   track halving rough, a vehicle refused up an escarpment (8.4x), light
   trucks refused desert, sandstorm doubling.
2. **Move orders.** `Order::Move { pieces, path }` in the movement
   segment: path adjacency, ownership, the stack's lowest CPA, CP charged,
   overspend DP, the 150 % cap, never into an enemy-occupied hex (8.1x),
   Marble Arch. Commit `Move orders`.
3. **Stacking.** `cna_rules::stacking::points`; checked at `EndSegment`
   (9.14); rejection names 9.1x. Requests for cna if any Graziani unit's
   equivalent is ambiguous.
4. **ZOC and contact.** Eligibility, reach, `DeclareZoc` pending,
   stop-on-entry, no ZOC-to-ZOC (10.2x), break contact and disengage
   costs, captures of lone non-combat units.
5. **Continuation.** The two-hex rule for later movement segments.
6. **Reaction.** `Pending::Reaction`, `Order::React`, the R-006/R-008
   exclusions, no ZOC entry.
7. **Reserves.** `DesignateReserve` in F, the Reserve I shift, release or
   flip at the first release segment, release limits.
8. **Off-map boxes and reinforcements.** Box-to-map moves by whole stages;
   schedule arrivals in phase D at Cairo / Tripoli.
9. **Cross-sheet movement.** If cna's join table has landed, load it in
   `Map` and un-ignore slice 1's seam test; otherwise leave the ignore and
   say so in the journal.
