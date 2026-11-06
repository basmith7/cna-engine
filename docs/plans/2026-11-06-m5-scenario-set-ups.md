# Mission 5: the other scenarios' set-ups and special rules (§61–64)

Spec: `docs/designs/2026-10-13-land-turn-design.md` (*Non-goals*: "Other
scenarios. Their land rules are the mission's second half, after
Graziani's is whole: one slice per scenario that needs a land rule
Graziani does not exercise"). Graziani's Offensive is whole (#31–#128).
Every decision here is the autopilot's under Brian's delegation of
2026-09-26 and is reversible with one PR.

**Goal.** All nine scenarios set up and play a Game-Turn (#61, #67), but
past Graziani's they play on Graziani's rules: the scenario files'
`construction` block (forts, minefields, rail ends, roads built), the
reinforcement schedule's withdrawals, and the set-up special rules
(release rules, tanks starting broken down, printed motorisation points)
are read by nothing. Until they are, Crusader's Tobruk has no forts, the
Last Chance has no minefields and Rommel's Arrival's Commonwealth can
leave Cairo at once. Each task below makes one more scenario play as
printed. This serves **The goal**: two players, any scenario, every rule
enforced.

**Non-goals.** The Logistics Game's port efficiency, pipelines and water
(R-027; the abstraction leaves them out); production charts (the Short
Campaign's cuts, 64.53–64.54, need the Logistics Game); the Air Game;
per-unit tank strength and model upgrades that cna holds only as prose
(62.32, 62.42, 63.31, 63.32, 63.48): a request to cna first.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Child scenarios | `extends` merges `sides` side by side and block by block, the child's blocks winning (#130) | the schema: a child "takes every block it does not give"; The Long Retreat gives only reinforcements |
| Set-up works | the `construction` block's fortifications and minefields are placed at set-up as if built (levels, real fields, the owner's), before either side places a unit | 59.12's set-up blocks; the engine's construction state already holds both |
| Rail | the Commonwealth line's end is the scenario's `railroad_ends`, else D3714 (60.7) | one field, no new mechanics; both sides using it (63.72) is its own task |
| Withdrawals | a `withdraws` row removes the named unit at the start of its Game-Turn's phase D, wherever it is; a unit not in play is skipped and logged | 4.43; the schedule already loads (`Schedule::withdrawals`) |
| Release rules | Rommel's Arrival's 61.38 as an order check on moves out of Cairo and Alexandria: none in the first stage, then one battalion-equivalent a stage | the narrowest reading; a ruling request if cna's prose leaves the count open |

## Tasks, one PR each

### Task 1: The Long Retreat's set-up (#130)
- [x] Merge a child's `sides` block by block; the smoke test needs both
  sides' combat units on the map.

### Task 2: forts and minefields at set-up
- [x] Rows that fill every land hex next to a place, behind `set_up_works`
  (#132). Rows a player picks and the minefields wait on cna (PROGRESS,
  *Requests for cna*: who places them).
- [ ] Read `construction.fortifications` and `construction.minefields`
  (Crusader 62.5, Rommel's Arrival 61.6, The Last Chance 63.76–63.77)
  into typed rows in `cna-data`; place them in `GameState::with_modules`.
  A test per scenario that a listed hex starts at its level.

### Task 3: withdrawals (4.43)
Waits on cna (PROGRESS, *Requests for cna*: withdrawals as data).
- [ ] Remove `withdraws` rows' units in phase D of their Game-Turn
  (Crusader GT63, Rommel's Arrival GT26–38, The Long Retreat GT108 and
  GT110, the Italian Campaign GT13–15); the browser logs it.

### Task 4: rail ends and roads built
- [x] `railroad_ends` behind the `rail_ends` module (#131).
- [ ] `railroad_ends` sets the Commonwealth line's end (Crusader C3430,
  The Last Chance cut west of El Alamein); `roads_built` upgrades those
  hexes' unfinished roads. Both sides on the railway (63.72) if the rules
  prose is concrete enough, else a cna request.

### Task 5: Rommel's Arrival's special set-up
- [ ] The release rule (61.38), 7th Armoured's tanks starting broken
  down, the printed starting motorisation points (61.72).

### Task 6: tanks under repair at the start
- [ ] Crusader's Alexandria shops (62.32) and the Last Chance's Tobruk
  (63.42) as broken-down points in a facility at set-up.

### Task 7: the campaigns' victory
- [x] 64.76's ratio levels from cna's `victory.ratios`.
- [ ] 64.71–64.75: automatic wins, the end-of-game supply test,
  unused-replacement and withdrawal points.

### Task 8: NOT_SIMULATED
- [x] R-097 and R-099–R-109 rewritten (#132); R-097 built behind
  `pre_play_convoys` (#133).
- [ ] Rows R-097 and R-099–R-109 still say only Graziani's loads; rewrite
  them, and add rows for what Tasks 2–7 leave out.
