# Mission 2, slice 5 (combat) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The combat segment of phase G on real pieces: the six steps of
5.2 (G.3) wired to Mission 1's rules core, with losses, retreats,
cohesion and the must-attack rule.

**Architecture:** `cna-game::combat` builds each step's inputs from the
pieces and the map and calls `cna_rules::{barrage, anti_armour,
close_assault, cohesion, morale}`. Where the core returns a distribution,
the game samples one outcome with its RNG and logs the roll. New pure
helpers go to `cna-rules`; the core gains *sampling* entry points next to
its exact ones, each tested to agree with the distribution.

**Spec:** `docs/designs/2026-10-08-game-state-design.md`
**Previous plans:** slices 1–4; Mission 1 slices 1–3 for the tables.

## Domain facts the implementer needs

Read cna `rules/60-combat.md` before starting; this plan names the steps,
and the case numbers there govern.

- **Steps** (5.2 G.3; cna `rules/60-combat.md`, which governs where
  `20-sequence-of-play.md` words it differently): 1 gun positions: each
  player marks every gun-class unit adjacent to the enemy *forward* or
  *back* for the whole segment; 2 barrage: both players secretly pick
  targets in adjacent enemy hexes, then all barrages roll, and hits and
  pins apply after the last (§12); 3 retreat before assault: any unpinned
  **non-phasing** unit may pull back; 4 assault assignment, secret: both
  players split TOE points between anti-armour and close assault (a point
  is never split); unpinned armour and gun units may be withheld; the
  phasing player decides privately which assaults are probes; 5
  anti-armour fire, both sides at once, all losses land before step 6
  (§14); 6 close assaults by the **phasing** player, in the order they
  choose, revealing after each whether it was a probe (§15). (The
  sequence file's "Player A / Player B" wording is a request to cna.)
- **Strength** (`60-combat.md`, *Raw and actual strength*): raw points
  are TOE points × the rating on the unit's characteristics row
  (`barrage`, `anti_armour`, `close_assault_offence`/`defence`,
  `armour_protection`, `vulnerability`), summed over the units first;
  actual strength is raw ÷ 10 rounded to nearest (.5 up), 4 raw or less is
  zero, with the exception there for both sides under 10. Tanks and guns rate by weapon
  system (4.47–4.49), not yet in cna (**Requests for cna**); until it
  lands they use their ID-code row and `NOT_SIMULATED.md` says so.
- **CP** (6.3; `60-combat.md` CP table, `cp-costs.json`): phasing unit
  that barrages, fires anti-armour or close assaults 5; probes only 2; is
  only barraged 3. Non-phasing unit that barrages, is assaulted or takes a
  holding-off barrage 3; is probed 2; defends a full (non-probe) close
  assault at a final differential of −4 or worse 1. Ceilings per combat
  segment: phasing 5, non-phasing 3. The charge falls on **every unit in
  the hex**, taking part or not. (`cp-costs.json` also gives the refund
  for a probe; that disagreement is a request to cna; follow the rules
  text.)
- **Losses** are TOE points; a 30 % or greater close-assault loss is 3 DP;
  a successful assault that clears the hex gives 3 RP (6.2x).
- **Must attack** (10.3x): every enemy hex putting a ZOC on a phasing
  combat unit is barraged or assaulted (a probe at −4 or better, or a
  holding-off barrage of at least the hex's non-gun battalion
  equivalents), or the unit retreats 3 hexes, spends all its CP and takes
  3 DP. Hexes holding only artillery, anti-tank, anti-aircraft or
  non-combat units are exempt (`50-stacking-and-zoc.md`, *Must attack*).
- **Rulings already switched** apply through the `Ruleset`: R-001, R-009,
  R-011, R-012, R-015.

## Tasks

Each task: failing tests first (hand-built positions on the real map,
seeded rolls pinned), implement, pass, commit.

1. **Strengths.** `combat::raw(piece, kind)` and
   `combat::actual(raw_sum)` with the rounding, the 4-or-less rule and the
   both-under-10 exception; tests on three Graziani units by hand.
   CP charging: every unit in the hex, with the per-segment ceilings.
2. **Sampling entry points** in `cna-rules` for barrage, anti-armour and
   close assault; a test per table that 10 000 seeded samples match the
   exact distribution within tolerance, and that one seeded sample is
   pinned.
3. **Gun positions and sealed plots.** `Order::GunPositions`; `Order::Barrage` and
   `Order::AssignForces` collected from both players under
   `Pending::Sealed`; neither is revealed (in the log) until both are in.
4. **Barrage resolution** and its CP; pins and losses.
5. **Retreat before assault** (non-phasing), path legality as movement.
6. **Anti-armour, then close assault** in the phasing player's order; losses,
   retreats (R-012's buy-out), advance after combat, captures, cohesion.
7. **Must-attack** check at the end of the combat segment.
8. **Reserve release** segment closing the cycle; a full G cycle test.
