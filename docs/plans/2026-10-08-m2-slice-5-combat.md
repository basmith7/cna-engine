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

- **Steps** (5.2 G.3): 1 position determination (who attacks which hex);
  2 barrage, plotted secretly by both sides (§12); 3 retreat before
  assault, Player B only; 4 force assignment, secret: each assaulting and
  defending unit's points go to anti-armour or close assault, probes and
  withheld points chosen privately (§13–15); 5 anti-armour fire,
  simultaneous (§14); 6 close assaults in the order Player A chooses (§15).
- **Strength:** TOE points × the rating on the unit's characteristics row
  (`barrage`, `anti_armour`, `close_assault_offence`/`defence`,
  `armour_protection`, `vulnerability`). Tanks and guns rate by weapon
  system (4.47–4.49), not yet in cna (**Requests for cna**); until it
  lands they use their ID-code row and `NOT_SIMULATED.md` says so.
- **CP:** phasing barrage or assault 5, probe 2; non-phasing barrage,
  being barraged or defending 3, a probe 2; defenders get 2 back at a
  final differential of −4 or worse (6.3, `cp-costs.json`).
- **Losses** are TOE points; a 30 % or greater close-assault loss is 3 DP;
  a successful assault that clears the hex gives 3 RP (6.2x).
- **Must attack** (10.3x): every enemy hex putting a ZOC on a phasing
  combat unit is barraged or assaulted (a probe at −4 or better, or a
  holding-off barrage of at least the hex's non-gun battalion
  equivalents), or the unit retreats 3 hexes, spends all its CP and takes
  3 DP.
- **Rulings already switched** apply through the `Ruleset`: R-001, R-009,
  R-011, R-012, R-015.

## Tasks

Each task: failing tests first (hand-built positions on the real map,
seeded rolls pinned), implement, pass, commit.

1. **Strengths.** `combat::strength(piece, kind)` from TOE and ratings;
   tests on three Graziani units by hand.
2. **Sampling entry points** in `cna-rules` for barrage, anti-armour and
   close assault; a test per table that 10 000 seeded samples match the
   exact distribution within tolerance, and that one seeded sample is
   pinned.
3. **Position determination and sealed plots.** `Order::Barrage` and
   `Order::AssignForces` collected from both players under
   `Pending::Sealed`; neither is revealed (in the log) until both are in.
4. **Barrage resolution** and its CP; pins and losses.
5. **Retreat before assault** (Player B), path legality as movement.
6. **Anti-armour, then close assault** in Player A's order; losses,
   retreats (R-012's buy-out), advance after combat, captures, cohesion.
7. **Must-attack** check at the end of the combat segment.
8. **Reserve release** segment closing the cycle; a full G cycle test.
