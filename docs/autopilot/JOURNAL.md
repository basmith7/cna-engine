# Autopilot journal

Machine-to-machine handoff between runs; see AUTOPILOT.md.

## 2026-09-26 11:02 MST — autopilot/slice-1
Done: plan Tasks 1–9 (PR #1); review fixes (errata with a bad `table` now fails; R-011/R-012 findings state their assumptions).
Plan vs ruling: R-012 option 3 keeps the 15.82 buy-out when a path exists, so it equals option 2 there (the plan said option 1); followed the ruling. Attacker −11 expected loss is 930/36 ≈ 25.8 %, not the plan's "> 30".
Added `loss_base_factor`, `may_buy_out`, `may_withhold` so the switches drive the probes (spec: a switch must change a probe).
No undeclared close-assault gaps or overlaps: no `cna` PR needed.
Next: merge PR #1, then slice 2 plan (drafted this run) on autopilot/slice-2.
Blocked: none

## 2026-09-26 11:08 MST — autopilot/slice-2
Done: slice 1 merged (PR #1); slice 2 plan written and executed (PR #2): barrage, anti-armour and terrain loaders, exact distributions, R-009/R-015 switches and probes; review fixes (R-015 assumption wording, R009 variant renamed PhasingFireOnly).
Deviation: `Terrain::shifts` instead of the plan's `Terrain::find` (plan updated in the same PR).
Next: merge PR #2, then slice 3 (plan drafted this run: cohesion ledger, R-001 switch, morale table, morale gap in chart-oddities).
Blocked: none

## 2026-09-26 11:13 MST — autopilot/slice-3
Done: slice 2 merged (PR #2); slice 3 plan written and executed (PR #3): morale table with coverage test (no undeclared gaps), cohesion ledger, R-001 switch and probe, morale −4/56 gap in chart-oddities; review fixes.
Deviation: R-001 probe plots positive measures (the board's chart has no negative axis); plan updated.
Next: merge PR #3, then slice 4 (plan drafted: R-018 default option 3, chart-vs-text probe on the guarded-dump check).
Blocked: none

## 2026-09-26 11:18 MST — autopilot/slice-4
Done: slice 3 merged (PR #3); slice 4 plan written and executed (PR #4): Construction Chart, dice sums, R-018 switch (default option 3, per its Rationale) and probe, chart-vs-text probe; test that every switch changes its probe; probe values rounded to 6 decimals.
MISSION 1 COMPLETE (all four parts merged once PR #4 lands).
Next: nothing under Mission 1; await a new mission in AUTOPILOT.md or PROGRESS Feedback.
Blocked: none

## 2026-10-07 — main (interactive session)
Done: AUTOPILOT.md gains **The goal** (shared with cna) and a mission queue, 2–9, from map and game state to the full campaign. Requests to cna go in PROGRESS **Requests for cna**; the old "small PR on cna" exception is gone.
Next: Mission 2 Part 0, branch `autopilot/m2-spec`: write the spec and plan, PR labelled `needs-brian`, then wait for Feedback approval.
Update: queue realigned to Brian's agreed path (small scenario, headless game state, minimal UI; the rest later in no fixed order).
Blocked: none.

## 2026-10-08 13:54 MST — autopilot/m2-spec
Done: Mission 2 Part 0 started. Bumped `vendor/cna` to 39b9ad7 (Graziani's Offensive, OA sheets, schedule landed in cna), 47 new rulings listed in NOT_SIMULATED (0c1bf9d). Draft PR #5.
In flight: spec `docs/designs/2026-10-08-game-state-design.md` and slice plans.
Next: finish the spec, mark PR #5 ready with `needs-brian`, wait for Feedback.
Blocked: none

## 2026-10-08 14:05 MST — autopilot/m2-spec
Done: spec `docs/designs/2026-10-08-game-state-design.md` (c3a8c3a) and slice plans 1–6 (075897e); gates pass. PR #5 ready, labelled `needs-brian`.
In flight: nothing; Mission 2 building waits for Brian's Feedback.
Next: if Feedback approves, merge PR #5 and start slice 1 (map) on `autopilot/m2-slice-1`; if it asks for changes, make them on PR #5.
Blocked: Brian's review of PR #5. Four Requests for cna posted (cross-sheet join, weapon systems, victory supply, track cost).

## 2026-10-08 14:06 MST — autopilot/m2-spec
Done: fact-checked spec and plans against cna and fixed six errors (combat step order and CP, strength rounding, schedule and supply counts). Two more Requests for cna on main (stacking equivalents; combat inconsistencies), six in all.
In flight: nothing. Stopped as AUTOPILOT.md requires: Mission 2 builds nothing before Brian's Feedback.
Next: on approval, merge PR #5 and start slice 1 (map). On change requests, edit PR #5.
Blocked: Brian's review of PR #5.

## 2026-10-08 19:55 MST — autopilot/m2-spec
Done: cna answered all six requests. Bumped `vendor/cna` to 18ed96e, probes regenerated, R-097–R-113 rows fixed in `NOT_SIMULATED.md` (07cae55). Spec and plans now use seams.json, weapon-systems.json (`vv` falls back to an ID code), R-110 supply trace, per-row equivalents, the probe refund as cp-costs.json has it (2306cc8, b3e9393, 59b88ce). Gates pass.
In flight: nothing. Feedback still empty, so Mission 2 builds nothing.
Next: on approval, merge PR #5 and start slice 1 (map) on `autopilot/m2-slice-1`.
Blocked: Brian's review of PR #5.

## 2026-10-08 22:53 MST — autopilot/m2-spec
Done: checked Feedback (empty) and PR #5 (no review). cna main is 45 commits past 18ed96e, all Air Game (Mission 7, R-111–R-116); nothing the spec uses changed, so no bump. Noted on main (a3a4446).
In flight: nothing.
Next: on approval, merge PR #5 and start slice 1 (map), bumping vendor/cna at its start.
Blocked: Brian's review of PR #5.

## 2026-10-09 01:58 MST — autopilot/m2-spec
Done: restored `vendor/cna` to 18ed96e (0e1aa11 had bumped it by accident, so `cna-probe check` failed in CI); gates pass locally.
In flight: none.
Next: wait for Brian's Feedback on PR #5; on approval, merge it and start slice 1 (map) with the cna bump and regenerated probes.
Blocked: Brian's review of PR #5.

## 2026-10-09 07:56 MST — autopilot/m2-spec
Done: nothing built; Feedback empty, PR #5 CI green, cna main has only quota rows since 11:55Z.
In flight: none.
Next: wait for Brian's Feedback on PR #5; on approval, merge it and start slice 1 (map) with the cna bump and regenerated probes.
Blocked: Brian's review of PR #5.

## 2026-10-12 19:50 MST — autopilot/m2-slice-1-map
Done: Mission 2 slice 1 (map) complete in `cna_data::map`: hexes, sheets and adjacency (ef435ff), hexsides (2d18d5f), places and regions (0fb4b82), cross-sheet seams (b2baedf), `RowShift` rename (f2e30af). 17 map tests; all three gates pass. PR #6 ready.
In flight: nothing.
Next: merge PR #6 once CI passes, then start slice 2 (scenario and pieces) on `autopilot/m2-slice-2-scenario` from the plan `docs/plans/2026-10-08-m2-slice-2-scenario.md`.
Blocked: none. Note for slice 4: cna's `seams.json` has no `up` for seam slopes/escarpments (18 + 7 pairs), so `climbs()` is false across a seam until cna records it; request it when slice 4 needs the climb cost.

## 2026-10-12 20:05 MST — autopilot/m2-slice-2-scenario
Done: Mission 2 slice 2 (scenario and pieces) complete: `cna_data::oa` and `characteristics` (8adcb64), `scenario` and `schedule` (3751e2e), new `cna-game` crate with `piece` and `setup::expand` (03f1f91); `map::Side` renamed `Direction` so the player `Side` is the only one (8db2b20). Graziani expands to 216 Axis + 66 CW pieces plus 22+6 and 15+3 supply units; all three gates pass. PR #8 ready.
In flight: nothing.
Next: merge PR #8 once CI passes, then start slice 3 (state, clock, orders) on `autopilot/m2-slice-3-state` from `docs/plans/2026-10-08-m2-slice-3-state.md`; it should key pieces by `PieceId`, not `UnitId`, since supply units are pieces without an OA id.
Blocked: none. Deviations from the plan are in PR #8's body (subtree arrival filter per 4.42, the San Giorgio skipped, dummy supply units as a fourth PieceKind, Option CPA/morale, Placement as struct + Locator enum, Setup struct return).

## 2026-10-12 20:24 MST — autopilot/m2-slice-3-state
Done: Mission 2 slice 3 (state, clock, orders) complete: `clock` (92c4189), `state`/`order`/`event`/`rng`/`data` and Ruleset JSON (b7f0221), set-up `Place` orders with 59.2 area, not_within and 9.14 ceiling (cf7766c), initiative, weather and skipped phases (0d87083), ruleset defaults follow cna's Decisions, R-018 → option 2 (242b311), `log`, `replay`, `cna-play` and `tests/games/setup.json` (894b654). All three gates pass. PR #9 ready.
In flight: nothing.
Next: merge PR #9 once CI passes, then slice 4 (movement) on `autopilot/m2-slice-4-movement` from `docs/plans/2026-10-08-m2-slice-4-movement.md`. Note `apply` takes `&Data` (static tables) explicitly; `Pass` ends F or G until slice 4 adds `DesignateReserve`/`EndSegment`; placement stacking is the plain sum by equivalent (shells, 9.16 exceptions wait for slice 4).
Blocked: none. Rommel has no OA counter, so the `rommel-on-map` initiative rating is unreachable until cna gives him one (not needed for Graziani).

## 2026-10-12 21:10 MST — autopilot/m2-slice-3-state (review fixes)
Done: PR #9 review. Blocking: the engine asked git for the `vendor/cna` commit (`GameState::new`, `from_json`, `replay`), so it could not run in a browser; `Data` now carries `rules_commit` (filled by `Data::load`, natively) and `from_json` takes `&Data` (6712708). Rejections pinned as state-unchanged, 8.37 sea test (0672959). README notes the fixture pin.
In flight: nothing.
Next: merge PR #9 once CI passes, then slice 4. Notes for slice 4 from the review: (1) `setup::expand`'s `unchecked` constraint list is dropped by `GameState::new`; put it in the state or a `SetUp` event and print it in `show`, since the engine accepts Axis supply in Egyptian sheet C and CW supply anywhere on sheet D. (2) `Weather` storms keep the sheets but not the die-3 `delta_sandstorm_is_normal` flag; carry it on the state. (3) Phase II (naval convoy) and the turn-level steps I/II are not in `Clock` and emit no `PhaseSkipped`; adding II changes the Clock's JSON and the fixture. (4) `Ruleset` JSON refuses a missing switch, so every save and log stops loading when a switch is added; a migration story is needed before Mission 3 saves exist. (5) `from_json` does not validate clock invariants (stage 1–3, half only in F–L, segment only in G). (6) `Data::load` (file reads and git) belongs behind a feature flag or in cna-play before the WASM build.
Blocked: none.

## 2026-10-12 23:40 MST — autopilot/m2-slice-4-movement
Done: Mission 2 slice 4 (movement) complete, PR #11 ready: step cost (f86c72c), Move orders (0146d91), stacking at EndSegment (666daad), ZOC and contact (31d7b19), the two-hex rule (d7150db), reaction (62a3390), seam slopes climb both ways (8314a2c), cross-sheet test (85b1d44), reserves (6aae470), off-map boxes and reinforcements (72d845f), R-003/4/6/7/8 as decided in NOT_SIMULATED.md (e52774e). origin/main (vendor bump, PR #10) merged in. All three gates pass.
In flight: nothing.
Next: merge PR #11 once CI passes, then slice 5 (combat) from `docs/plans/2026-10-08-m2-slice-5-combat.md`. For slice 5: `Piece::released` carries the reserve combat limits (one assault from I; one assault or probe and +1 DP from II); `Order::Move { assault }` is the pinning declaration; ZOC eligibility counts tank and gun defence as 0 until weapon-systems.json is wired; `engaged` is set by nothing yet.
Blocked: none. Two questions for cna in the PR body (the 8.8x map-end hex, A2802 being sea in hexes.json; the 8.89 chart's 21–24 CPA gap).
