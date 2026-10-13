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

## 2026-10-12 22:33 MST — autopilot/m2-slice-5-combat
Done: Mission 2 slice 5 (combat) complete, PR #12 ready: weapon systems and strengths (ababd69), sampling entry points (c92859d, 9049e06), the six combat steps with sealed plots (00b4be1), barrage (5718078), retreat before assault (8932f5a), anti-armour and close assault with forced retreats (08dbf3d), must-attack (82c25ed), reserve limits, the G cycle test and NOT_SIMULATED.md (67618a8). 98 cna-game tests; all three gates pass.
In flight: nothing.
Next: merge PR #12 once CI passes, then slice 6 from its plan. For slice 6: losses are the engine's choice (`combat::losses`), a pending owner choice would slot in there; `Event::Prisoners` is logged but nothing changes hands; ammunition, trucks, destroyed-tank markers and dummy formations are listed as not simulated.
Blocked: none. Deviations from the plan are in PR #12's body (computed probe, no advance after combat, infantry defends in full unless assigned, morale `surrender` as −4, no divisions in the size table).

## 2026-10-12 — autopilot/m2-slice-5-combat (review fixes)
Done: PR #12 review, six blocking findings fixed test-first. Hexside shifts add up so assaulting down a slope or escarpment is the attacker's +1 (44b1fc8); the 14.35 anti-armour hexside check follows the Terrain Effects Chart through `hexside_shift`, with the prose/chart disagreement recorded in the PR body and asked of cna (7c0ed6f); plot and assignment totals no longer overflow (e3fdb81); anti-armour fire is simultaneous, return fire hits armour assigned to either role, and an assault whose attackers were all eliminated no longer deadlocks the close assault step (1e3de45, ba817ba); a holding-off barrage counts battalion equivalents by stacking points (ae236bb). 105 cna-game tests; all three gates pass.
In flight: nothing.
Next: merge PR #12 once CI passes, then slice 6. Nonblocking items left for slice 6 or a follow-up: rule tests for 15.36, 6.26, 13.1, 15.1x, 13.2x; withheld points in the defender's loss base (15.1x); the 10 % per hex short folded into the table percentage before rounding (15.83); a must-attack `Pass` surrendering a stack that has a legal path (10.36); attacking forward guns that did not fight and artillery in the anti-armour role at vulnerability 2 (15.84, 14.14); whether a released unit can assault again in a later combat segment of the same phase (18.2x); `Pending::Sealed` with an empty `waiting` from a malformed save panics.
Blocked: none.

## 2026-10-12 — autopilot/m2-slice-5-review-fixes
Done: PR #12's second review (after merge), fixed test-first on a follow-up branch. Blocking: the −4 bad-attack refund (6.3) now gives 2 CP back of whatever the defended hex was charged this segment, anti-armour included, instead of charging the assault net of a refund that could never apply once step 5 had taken the hex to its ceiling (`refund_combat_hex`, `Event::CombatCpRefunded`); the 50 % probe rule (15.9x) is judged when the assignments are revealed, before anti-armour fire, and kept in `CombatSegment::probing` (4421b6a). Nonblocking one-liners: unsupported tanks capped at four per assault (15.4), the stand-and-pay loss rounds down (15.83), a sealed answer with nobody waiting is refused not panicked, the probe test's comment (06f5447). The pinned full-assault test asserts its real outcome.
In flight: nothing.
Next: merge the follow-up PR once CI passes, then slice 6. Still open from the review: the must-attack retreat (10.36) charges the whole CPA before the retreat and the retreat's CP on top, so every retreat step is overspend and DP — asked of cna in the PR body; the 10 % per hex short is still applied after the table loss is rounded, not folded in before one rounding (15.83); HQs of infantry class defend in full unless assigned, like infantry — PR #12's body said otherwise, the follow-up PR's body corrects it; a refund does not reverse overspend DP already earned; the rule tests listed last time (15.36, 6.26, 13.1, 15.1x, 15.0, 15.21), state-unchanged checks on rejections, a holding-off barrage test, and `adjusted_morale` tie-breaking (15.6 averages).
Blocked: none.

## 2026-10-12 — autopilot/m2-slice-5-review-fixes (third review)
Done: the one blocking finding, test-first. `refund_combat_hex` lowered `cp_spent` but left the DP that CP had earned through `charge()` (6.21), so a defender already at its CPA kept DP for CP it never net spent and earned them again on its next charge. The refund now reverses the DP of the band it gives back (`Unit::refund_dp`, `dp` on `Event::CombatCpRefunded`): a −4 probe nets 0 CP and 0 DP, a −4 full assault 1 and 1, the same as charging net (4979411). Tests `a_refund_takes_back_the_dp_the_refunded_cp_earned` (cp.rs, both cases) and `a_bad_attack_on_a_defender_at_its_cpa_costs_the_dp_of_the_net_cp` (assault.rs, through step 5) assert `dp_tally` and `level`; the existing refund test asserts cohesion too. Doc notes for the nonblocking items: a probed hex that took anti-armour fire keeps 1 CP (the CP table's "2 back"; 15.9x's "no CP at all" could be read as 0, a question for cna if it matters); 15.4's 1–3 bands are counted hex by hex before the cap of four. Gates green.
In flight: nothing.
Next: merge PR #14 once CI passes, then slice 6. Still open: the must-attack retreat (10.36) question in the PR body; 15.83's 10 % per hex short applied after rounding; the rule tests listed last time; state-unchanged checks on rejections; a holding-off barrage test; `adjusted_morale` tie-breaking.
Blocked: none.

## 2026-10-12 — autopilot/m2-slice-4-fixes (PR #11 review, after merge)
Done: PR #11's review landed after the merge, so the fixes are a follow-up branch from main. Blocking, test-first: an engaged unit pays the 4 CP disengage wherever it stands, not only in an enemy ZOC (dfde862, 8.6x; live on main since slice 5 sets `engaged`); a railroad opens swamp and major-river hexsides as a road does, swamp by rail at the road rate (63ec94b, 8.37 and footnote 11). Nonblocking one-liners: a vehicle in delta stays put in rain unless it leaves by road or rail (fc12093, 29.5x); Reserve II units are not offered a reaction, and a unit in contact is tested not to be (c313533); a Move from a hex next to an undeclared enemy is checked for adjacency, map, enemy and Marble Arch before it is parked on the declaration (6733317). Two cna requests posted (swamp-by-rail price, seam `up`).
In flight: nothing.
Next: slice 6. Left from the review, not one-liners: 18.23/18.24 caps are checked against own-half CP only (arguable; Brian's call); 6.15 read per piece rather than the stack's lowest CPA (recorded deviation; Brian should confirm); 9.13 top-HQ stacking without shells (recorded); `Data::load` reads disk and shells to git (Mission 3 needs an in-memory constructor); a shared `CARGO_TARGET_DIR` across trees leaves stale rlibs (use one per tree).
Blocked: none.

## 2026-10-13 00:40 MST — autopilot/m2-slice-6-victory
Done: Mission 2 slice 6 (victory and the golden game) complete, PR #15 ready: `victory` typed in cna-data (b4aa9a6); `cna_game::victory::evaluate` with the R-110 route trace and hand-built tests per level, a holder cut off by enemy ZOC and one traced around it (eed6ac7); `GameState::over: Option<VictoryReport>`, `Event::GameOver { report }`, orders refused with 60.22, `cna-play show` prints the report (e13e2fc); `tests/games/graziani-short.json`, 446 orders, final state SHA-256 and report pinned (ede3aec); the determinism property test (3588794, 1c1992b); README, NOT_SIMULATED.md *Systems* table, spec Status, AUTOPILOT queue, PROGRESS.md (this commit). 112 cna-game tests; all three gates pass.
**MISSION 2 COMPLETE.**
In flight: nothing.
Next: Mission 3 Part 0, the UI spec, on `autopilot/m3-spec`. Slice 5's nonblocking follow-ups stay open (none touches the scripted games): 15.83's short percentage is rounded separately, a must-attack `Pass` surrenders a stack that has a path (10.36), withheld points in the defender's loss base (15.1x vs 15.83, asked of cna in PROGRESS.md), rule tests for 15.36, 6.26, 13.1, 15.1x, 13.2x, and the empty sealed `waiting` panic on a malformed save. Decisions this slice: the trace runs in normal weather (a route, not a move); an HQ alone holds nothing; a points victory (other scenarios) is reported unchecked.
Blocked: none.

## 2026-10-13 01:10 MST — autopilot/m3-slice-1-engine
Done: Mission 3 slice 1 (engine in the browser), PR #18 ready: `cna_data::source::Source` with `Fs` and `MemSource`, every loader and `errata::load_table` behind a `Source`, `Data::load_from(slug, src, rules_commit)` pinned field by field against `Data::load` (dbbbf78); `crates/cna-web` with `build.rs` embedding `vendor/cna/data/{map,tables,oa,scenarios,errata}` and `RULES_COMMIT`, `api::Game` (new/load/save/view/apply as JSON) tested natively incl. the 76-order `setup.json` against `replay`, and the wasm-bindgen `Game` (4e774cc); Node wasm tests, release wasm 4.14 MB raw / 759 KB gzipped (e48394b); `web/` (Vite + Svelte 5 + TS), `geom.ts` pinned to cna's `C.svg` and seams, the 7 073-outline grid drawn once with `data-hex`, the top bar from `view()` (a90c29d); Playwright smoke e2e with the proving screenshots and CI jobs for wasm and web (dc1882d); critique fix (25e429c). All gates pass locally.
In flight: nothing.
Next: merge PR #18 once CI passes, then slice 2 (`docs/plans/2026-10-13-m3-slice-2-map.md`). Deviations recorded in the PR: `Data` parity by `Debug` form rather than new `PartialEq` derives; `Fs::list` returns files only; Malta inset nests in sheet A's empty corner; Playwright pinned to 1.63.0 (the browsers already on Brian's machine); vitest 5 (npm's resolver crashed on vitest 4's peer set). At 390 px the fitted grid is a grey mass until slice 2's pan/zoom; the page itself stays usable. Watch for the shared `CARGO_TARGET_DIR`: a stale `cna-data` rlib from another tree failed `cargo test` once (fixed by a `touch`), and a stale `cna-play` binary reported another tree's `rules_commit`.
Blocked: none.
