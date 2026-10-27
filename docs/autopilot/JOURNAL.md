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

## 2026-10-13 02:00 MST — autopilot/m3-slice-2-map
Done: Mission 3 slice 2 (map and counters), PR #19 ready: `view()` adds each unit's OA `name` and `abbreviation` beside `unit_type` (96dcd6e); the base layer in cna's `map_render.py` style (terrain fills, hexsides with slope ticks, settlements, Cairo's place area, hex numbers and place names) still one string with one delegated handler (7371030); `viewport.ts` pure pan/zoom maths with Vitest, drag, wheel about the cursor, +/−/fit and a go-to-hex box, start fitted to B–E, labels below a zoom threshold (4802eb7); `counters.ts` + `Counter`/`Counters`/`StackPanel`/`OffMap` drawn from the view only, a `data-unit` per piece, stacks with a count badge and a minimum pixel size, the trays and six boxes; Vitest pins a glyph for all 24 chart unit types (a0b0e77); Playwright: DOM equals the view (241 placed, 36 + 23 to place), Sidi Barrani stack and panel, drag, 390 px, and the budget measured in-page at 701 ms to first counters and 53 ms per wheel step (6712d35); screenshot-critique fixes (badge off the ID code, name above the counter with a halo). All gates pass locally.
In flight: nothing.
Next: merge PR #19 once CI passes, then slice 3 (`docs/plans/2026-10-13-m3-slice-3-setup.md`). Deviations recorded in the PR: `name`/`abbreviation` added to the view; no "motorised" glyph (the chart has no such type; anti-tank and anti-air have glyphs instead); tasks 4 and 5 are one spec and one commit; supply units all read "Supply" so dummies are not told apart; the critique was argued adversarially in the session (no sub-agent tool available). Accepted residue: at the fitted view the frontier stacks overlap; a place name's halo clips the tops of the two hex numbers above it; at 390 px the board is mostly blank around the strip (mobile layout is a non-goal).
Blocked: none.

## 2026-10-13 02:45 MST — autopilot/m3-slice-3-setup
Done: Mission 3 slice 3 (set-up), PR #20 ready: `GameState::placements` (the placement area kept where `place` accepts it; `place` split into `check_place` and the mutation, `place::area` promoted out of `test_support::candidates`, whose two hand filters stay test-only) exposed as `Game.placements` natively and in wasm (29b281d); the clickable tray, the placement shading, the set-up panel with the locator in words and the constraint as "also: …", the go-to box no longer a click (fd90198); the cna request for typed constraints in PROGRESS.md (1fcff96); the curtain (55f2a48); `format.ts` for all 38 events, the log panel with clock headers, the illegal toast (dde6883); Playwright clicking orders 0–58 of the golden game to GT1 phase A with `save()` hashing to `web/tests/fixtures/set-up-done.sha256`, pinned by a `cna-web` test to `replay` (fab1b9b); critique fixes (efe030b). All gates pass locally.
In flight: nothing.
Next: merge PR #20 once CI passes, then slice 4 (`docs/plans/2026-10-13-m3-slice-4-movement.md`). Deviations recorded in the PR: `placements` is empty for a side not setting up (what `apply` says); no clone per hex, a check/mutate split instead; the fixture is a hash, not a state; the PROGRESS.md request went on the branch, not `main`; the critique was argued adversarially (no sub-agent tool). Accepted residue: the engine's refusal prose names pieces by id; the 390 px layout is slice 2's.
Blocked: none.

## 2026-10-13 03:48 MST — autopilot/m3-slice-4-movement
Done: Mission 3 slice 4 (movement), PR #21 ready: `cna_game::reach` — `GameState::reachable(side, pieces, purpose)` returns every end hex with path, CP, DP, ZOC stop and may-stop-short flags, a Dijkstra over `movement.rs`'s own `start`/`step_into` (which `plan_with` now shares) with a random-walk test both ways, exposed natively and in wasm (11f8d2b); tick boxes, the reachable overlay with CP in the hex, DP and dotted outlines, hover path, click to go, shift-click path with Enter, assault, refusals in the panel (20c102f); off-map buttons (003cd51); phase controls (1209b8b); declare-ZOC and reaction prompts, ZOC marks, slice-5 placeholders (6ed15f3); state fixtures from the golden log pinned current by a test (f2243c9); Playwright clicking orders 59–65 to `movement-done.sha256`, plus ZOC, off-map, reserves, 390 px, and critique fixes (ada7fbd). All gates pass locally.
In flight: nothing.
Next: merge PR #21 once CI passes, then slice 5 (combat prompts). Deviations recorded in the PR: reaction triggers also flag may-stop-short; collapse bound for uncapped vehicles; `rand_chacha` not proptest; `set-up-done.json` and `reserve-release.json` state fixtures; phase controls hidden while anything is pending; a plain click on a friendly-held reachable hex selects; the critique was argued adversarially (no sub-agent tool). Accepted residue: the +/−/fit bar overlaps a top-row counter at 390 px (slice 2's layout).
Blocked: none.

## 2026-10-13 04:05 MST — autopilot/m3-slice-4-movement (review fixes)
Done: PR #21 review. Blocking: the random-walk test only ever exercised `Purpose::Move`; split into one test per purpose (`check_both_ways` now takes the purpose and builds the matching order), the React one from golden prefix 62 over every subset of the eligible reactors, 300 walks of up to 12 hexes each, with a floor on accepted walks so it cannot pass vacuously; a mutation that hid one React hex failed it (`C3821 ... not returned`). Nonblocking: the spec's *Scripted games* claim that the golden game never moves with `assault: true` corrected (order 61 does). Two commits carry a `Claude Fable 5.1` trailer; left as is since fixing needs a force push.
In flight: nothing.
Next: merge PR #21 once CI passes, then slice 5. Accepted residue: selecting an uncapped motorised stack takes ~580 ms to shade ~2900 hexes (bounded by collapse only); M2's `react` has no CP cap (8.5x follow-up); `may_be_interrupted` is judged on the cheapest path with assault=false, so it over-flags on the safe side.
Blocked: none.

## 2026-10-13 04:45 MST — autopilot/m3-slice-5-combat
Done: Mission 3 slice 5 (combat and the end), PR #22 ready: `GameState::sealed_options` from per-item predicates the three plot checks now share, `view()` redacting the other side's sealed answers plus `assaults_to_resolve` (60b666c); `reachable` for `retreat-before-assault` (13.22's cap) and `forced-retreat` (`Reach.short`, never for must-attack), random walks both ways against `apply` under two R-012 settings (7c79049); the sealed, retreat-before-assault, forced-retreat and close-assault panels, the amber short-hex overlay (c5bb35d); result lines and the end-of-game dialog (6035129); fixtures incl. the seed-3 forced retreat and `combat-done.sha256` (e9f6ffb); Playwright clicking golden orders 66–76 to the pinned hash, the retreats, the end dialog, 390 px (8a3ec3a). All gates pass locally; 18 e2e.
In flight: nothing.
Next: merge PR #22 once CI passes, then slice 6 (`docs/plans/2026-10-13-m3-slice-6-e2e.md`). Deviations recorded in the PR: `assaults_to_resolve` is `pub` on `GameState` (not `pub(crate)`), each hex once; a firer with no target in reach is not offered; `Gun.next_to_enemy` added so the panel can fold the 50-odd far guns away; a refused sealed answer goes to the toast only, not the log (the reason repeats the answer); `Reach.short` follows the plan (a short hex is returned where `apply` lets the stack stand and pay) rather than the spec's shorter wording; the critique was argued adversarially (no sub-agent tool). Accepted residue: the forward marker shows only on a stack's top counter (the stack panel lists it per piece); the count badge still touches the top counter's CPA (slice 2); ticks are offered in a sealed step though nothing moves (slice 4); the whole Fort Capuzzo stack cannot retreat before assault (10.2x, as `apply` says) and the forced-retreat pending repeats `from` hexes (deduped in the panel only).
Blocked: none.

## 2026-10-13 06:20 MST — autopilot/m3-slice-6-e2e (review fixes)
Done: PR #23 review. Blocking: the golden e2e never finished because of three driver bugs, all fixed in cdbe548 and seen green locally (446 orders, 5.1 min, `save()` hashing to `golden-done.sha256`): a one-hex move onto a hex holding a stack now takes Shift + Enter (the app selects on a plain click there, and the counter sits over the polygon); the save/reload/load block runs at the top of order 87's iteration, after the curtain; the 390 px Close button is scrolled into view; a 15 s default timeout names the stuck order. `golden-end.png`, `golden-end-390.png` and `start-continue.png` captured; critique argued adversarially (no sub-agent). Nonblocking one-liners: the Continue hint now reads "Autosave at …" (09df1fe). Task 6 docs committed with this entry. **MISSION 3 COMPLETE** once #23 merges.
In flight: nothing.
Next: merge PR #23 once CI passes; then Mission 4's spec (PROGRESS.md, Next steps). Not done from the review, recorded in PROGRESS.md: `check_invariants` extended to every piece and hex reference and duplicate `waiting` sides (a hostile save can still panic the engine later); the curtain on Load/Continue; the 390 px end-dialog table clipping MET.
Blocked: none.

## 2026-10-13 09:25 MST — autopilot/hostile-saves
Done: PR #26: `check_invariants` checks every piece and hex a save names, duplicate `waiting` sides, and reaction/forced-retreat pieces' sides (03629f4); 13 failing-first tests and a golden load sweep. Gates pass locally.
In flight: nothing. Mission 4 waits on Brian's review of PR #25.
Next: merge #26 once CI passes. Then nothing for Mission 4 until **Feedback** approves #25. Still open from #23's review: no curtain on Load/Continue. Also `arrival.rs`, `off_map.rs` and `reaction.rs` are not rustfmt-clean on `main`.
Blocked: Brian's review of PR #25.

## 2026-10-13 14:00 MST — autopilot/m5-spec
Done: merged PR #27 (10.36 must-attack Pass; its run left no journal entry). Re-ran PR #28's flaky `web` job (the 100 ms wheel-step budget). Mission 5 spec and plans, PR #29 (04209e8), written while Mission 4's spec #25 waits on Brian; four requests for cna on `main` (b9f1da6).
In flight: slice 1 (supply state) on `autopilot/m5-slice-1-supply-state`.
Next: merge #29 once gates pass; merge #28 once its re-run is green; continue slice 1 from its plan. If **Feedback** approves #25, finish the slice in hand and switch to Mission 4.
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-13 14:35 MST — autopilot/m5-slice-1-supply-state
Done: spec #29 and #28 merged. Slice 1, PR #31: loads (e000001), motorisation points (bb3f138), supply stacking (c7869ef), the `abstract_supply` module and view redaction (7c7248a), browser stays module-off (8cea02f), 32.8x AA thinning (e413911). Gates pass locally. PR #30 fixes the flaky zoom budget (368a941). Slice 2 is PR #32: rates (867ad5d) and the draw range (a650fb8).
In flight: slice 2 Task 3 (fuel) on `autopilot/m5-slice-2-spending`.
Next: merge #30, #31 once CI is green, then #32's base becomes main; continue slice 2 at the first unchecked task in its plan.
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-13 14:45 MST — autopilot/m5-slice-2-spending
Done: slice 2 Tasks 1–5 on PR #32 (draft, stacked on #31): rates (867ad5d), draw range (a650fb8), `Role::Assault` (a86583e), fuel (672688e), ammunition (a3e0cb0), brigade counters pay per three battalions (d31bdbe), NOT_SIMULATED (58a4a32). Gates pass locally. The module-on golden drops 4 RHA's return move and 7 Med's GT1 barrage (no supply in range).
In flight: Task 6 (UI) not started.
Next: merge #30 and #31 once CI is green; merge main into #32; Task 6: browser `Game::new` module-on, the golden e2e on `tests/games/graziani-m5.json`, regenerate `web/tests/fixtures` (`cargo test -p cna-web -- --ignored regenerate_e2e_fixtures`), stack panel loads and points, fuel source in the move panel, ammunition in the barrage and assign panels.
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-24 09:06 MST — autopilot/m5-slice-2-spending
Done: slice 2 Task 6 (UI). The log names the four supply events (4bb6c4f; it threw on them before); Reach.fuel and fuel_sources (b210689), only a Move quotes fuel; stack panel loads and motorisation, move panel fuel owed, hovered fuel and a payer picker; sealed options and panels quote ammunition. setup/map/golden specs follow the m5 log (56 placements, 441 orders, save after 82); Playwright supply.spec for a paid and a refused move.
In flight: the full Playwright run and gates before marking #32 ready.
Next: if #32 is merged, slice 3 from `docs/plans/2026-10-13-m5-slices-3-6.md`.
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-24 09:33 MST — autopilot/m5-slice-3-supply-movement
Done: #32 merged (39b403a; Playwright 27/27 locally, CI green). Slice 3 PR #34 (draft): plan; phase C waits on A then B with attach/detach motorisation and load transfers (golden re-recorded, 477 orders, cna-web indices and fixtures +2); PhaseControls attach/detach; motorised supply units move in G with a combat unit at CPA 15 and alone in H (H skipped when nothing to move).
In flight: Playwright on #34's head.
Next: Task 2's capture of points; Task 3 (phase J rail, air lift, destroying supply); a transfer-load panel; then the golden game through C/H with a real attach and supply move (Task 4).
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-24 09:45 MST — autopilot/m5-slice-3-supply-movement
Done: on #34, phase H for supply units (H skipped unless a motorised, unescorted supply unit exists, so the golden is unchanged), the transfer-load control, DestroySupply (engine, button, e2e), NOT_SIMULATED's §32 row; Playwright 29/29 at the transfer commit. PROGRESS on main: status, and a request for cna (motorisation points per foot unit).
In flight: a final full Playwright run and a review pass on #34's engine diff.
Next: phase J rail (40-movement.md *Rail movement*, R-005: one stack each way, any distance on finished track, units and supply), then the air lift, capture for the captor's use, and Task 4 (the golden game attaching points and moving a supply unit).
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-24 09:56 MST — autopilot/m5-slice-3-supply-movement
Done: Playwright 30/30 at 65bf6ba; review-pass fixes (points back to the pool when a dump goes, no DP or pin drag from supply units, escorted redacted); supply stacking now binds at a move's end (32.1x; was set-up only). Supply and movement specs 11/11 after.
In flight: nothing uncommitted. #34 stays draft.
Next: the review's open items in #34's body (H ZOC declaration and stacking check, the escort rule's start/stay reading), then phase J rail (R-005), the air lift, capture for the captor's use, Task 4's golden extension.
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-24 11:12 MST — autopilot/m5-slice-3-supply-movement
Done: merged main; phase J rail for Commonwealth supply (dcd235d: `rail.rs`, `Order::Rail`, `Event::Railed`, golden 495 orders re-pinned); `Purpose::Rail` in the reach query, the move panel, H/J default-tick supply, rail e2e (0477d24, ff78d22). Fixed `golden-done` pinned at 477 of 495 orders.
In flight: full Playwright run on ff78d22.
Next: the air lift in phase C (32.3x), then capture for the captor's use and Task 4's golden extension; then mark #34 ready.
Blocked: Mission 4 on Brian's review of PR #25.

## 2026-10-24 11:26 MST — autopilot/m5-slice-5-breakdown
Done: #34 (slice 3) ready, gates green locally (Playwright 31/31 + 15/15 after review fixes 0ea83e2, 53f8439); CI running at handoff. Air lift moved out (needs HQ echelons; request on main). Slice 4 plan on `autopilot/m5-slice-4-arrivals` (no PR), blocked on a Game-Turn calendar (request on main). Slice 5 draft PR #35: breakdown tables and rules core (b5edb02, 28a197d).
In flight: slice 5 Task 2 (per-piece accumulation and checks) not started.
Next: merge #34 if CI is green (`gh pr merge 34 --merge`), merge main into #35, then Task 2: `Piece.breakdown` from `cna_rules::breakdown::step_points` at every step (move, react, retreat), a check at each stop, tank BARs from `weapon-systems.json`'s `bar`.
Blocked: Mission 4 on Brian's review of PR #25; slice 4 on cna's calendar.

## 2026-10-24 11:44 MST — autopilot/m5-slice-5-breakdown
Done: #34 merged (04a4cf2). Slice 5 on #35: `breakdown` module (8d9141e), checks at each stop for tanks/SP guns with BAR and hot weather (4e0fde6), sandstorm shift (4bf76f3), wrecks kept per hex (8773cde), capture by a battalion (ebcba89). Golden re-pinned (dice moved; report unchanged).
In flight: full Playwright on #35 (wasm at 0fd4419+breakdown checks); a fresh review of the slice 5 engine diff.
Next: act on the review; then 21.41 (half left behind near the enemy), phase K towing and field repair (§22, `vehicle-repair.json`), wrecks in the stack panel; armoured cars/recce need a typed vehicle class from cna (name-only today).
Blocked: Mission 4 on Brian's review of #25; slice 4 on cna's Game-Turn calendar; the air lift on HQ echelons.

## 2026-10-24 11:52 MST — autopilot/m5-slice-5-breakdown
Done: review of slice 5 acted on (cb4897b: retreats collect and check, one check per BAR, no reaction to a self-destroyed stack, reactions capture; 67e92a6 retreat test). Playwright 32/32 at ebcba89's wasm; requests for cna on main: Game-Turn calendar, HQ echelons, vehicle classes.
In flight: a last full Playwright run on 67e92a6 (result in the PR #35 comments if it landed, else rerun).
Next: 21.41 (half left in the start hex near the enemy), phase K towing (21.6x) and field repair (§22, `vehicle-repair.json`), wrecks in the stack panel; then mark #35 ready. Slice 6 if cna's calendar is still missing.
Blocked: Mission 4 on Brian's review of #25; slice 4 on cna's Game-Turn calendar; the air lift on HQ echelons.

## 2026-10-24 11:56 MST — autopilot/m5-slice-5-breakdown
Done: 21.41 (fd815bb): a move begun next to an enemy brigade-equivalent, or two hexes from one through an open hex, leaves half of what breaks down in its start hex. PROGRESS on main (c905fd1).
In flight: nothing. Playwright 32/32 on fd815bb (in the #35 comments).
Next: capture's three-hex placement and passable-terrain test; phase K towing (21.6x) and field repair (§22, `vehicle-repair.json`); wrecks in the stack panel; then mark #35 ready.
Blocked: Mission 4 on Brian's review of #25; slice 4 on cna's Game-Turn calendar; the air lift on HQ echelons.

## 2026-10-25 17:08 MST — autopilot/m5-slice-5-breakdown
Done: phase K field repair (70aaaa5: `Order::Repair`, `cna-data::repair`, a fuel a point, one roll per type per hex, repaired points pooled for §20); web repair buttons, wrecks on the map and in the stack panel, `repaired` redacted, `repair` e2e fixture (0c493ac); BP in the stack panel, plan/spec/NOT_SIMULATED (4821d40). Towing and facility repair moved to the construction slice (they need facilities); spec row updated.
In flight: full Playwright run on 4821d40's parent; a fresh review of the repair diff. A parallel agent is bumping `vendor/cna` on `autopilot/cna-bump` (cna answered every request: calendar, echelons, R-120–R-125, set-up constraints).
Next: act on the review, mark #35 ready, merge when CI is green; then the cna-bump PR; then slice 4 (arrivals, now unblocked by `calendar.json`).
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-25 17:46 MST — autopilot/m5-slice-4-arrivals-2
Done: #35 (slice 5) merged (eae994d) after review fixes (3384b90). The cna bump (#36, by a subagent: typed placements, R-120–R-122, motorisation data) got main merged in (197fb5d). Slice 4 on draft #37, branched from #36: calendar (0220a47), monthly motorisation losses (03ce4e1), planning rolls on their own dice (38939b6), Commonwealth arrivals in phase D (9bd388e).
In flight: full Playwright on #36's merge; #36 to merge when green, then main into #37.
Next: slice 4 Task 3 (Axis convoys to Tripoli with the bombing chart), Task 5 (log, NOT_SIMULATED); delete the answered request bullets once #36 merges; Helwan and R-120's defence side are #36's open follow-ups.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-25 18:00 MST — autopilot/m5-slice-4-arrivals-2
Done: #36 (cna bump) merged (11c1a5e); PROGRESS on main (fc16e3e): every cna request answered, list cleared. Slice 4 on draft #37: Axis convoys to Tripoli, three a lane (b44a5a0); log lines, NOT_SIMULATED (e2617da); main merged in (3e53608). Playwright 33/33 on b44a5a0.
In flight: nothing uncommitted.
Next: on #37, convoy bombing (32.6x) with R-124's echelons for "westernmost full Commonwealth division" and the Air Bombardment Table; then mark #37 ready (per-side event redaction, e.g. `supply-planned`, is the Mission 4 server's job: hot-seat shares one log). After: Helwan, R-120 defence, R-123, R-125, the air lift; slice 6.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-25 19:07 MST — autopilot/m5-air-lift
Done: #37 marked ready (6665386: convoy bombing moved out); request for cna on main (b6e9e2a): 56.1's lanes and a 32.66 ruling. Air lift on draft #38, stacked on #37: `Echelon` in cna-data and `Order::AirLift` (5d56fde), web Fly… button and e2e (3103e02, supply.spec.ts 10/10).
In flight: full Playwright run on #38; a fresh review of the air-lift diff.
Next: merge #37 when its web CI is green, merge main into #38, act on the review, mark #38 ready. Then R-120 defence / R-123 / R-125 / Helwan follow-ups, then slice 6's plan (§19 attachment as explicit state).
Blocked: Mission 4 on Brian's review of #25; convoy bombing on cna's lane table.

## 2026-10-25 19:30 MST — autopilot/m5-recce-breakdown
Done: #37 (slice 4) merged. #38 (air lift) ready, Playwright 34/34 locally. #39: R-125 recce breakdown/repair (af3ad0c), R-123 motorised foot units (892c103, 4a9f958), review fixes (a994d30: phase H waits on supply only; top-up). Reviewer's "motorisation points never break down" finding is the rule (32.5x), not a bug. Draft #40 (R-120 defence, own branch off main): defenders out of ammunition sit out, an all-out hex surrenders; tests pass.
In flight: full Playwright on #39 (/tmp/pw-39.txt, lost with the run if unfinished: rerun). #40's module-on golden game no longer replays: GT1's Axis assault on C3922 now takes the hex by surrender (CW has no ammunition in range).
Next: merge #38 when web CI is green, merge main into #39, mark ready, merge. Then #40: re-script the module-on golden (`golden.rs` script, `tests/games/graziani-m5.json`), regenerate the web e2e fixtures (`cna-web` the_e2e_fixtures_are_current) and `golden.spec.ts`'s pins.
Blocked: Mission 4 on Brian's review of #25; convoy bombing on cna's lane table (request on main).

## 2026-10-25 19:42 MST — autopilot/m5-collapse-surrender
Done this run: #37 slice 4 and #38 air lift merged; #41 slice 6 plan merged. Ready, CI running at handoff: #39 (R-125 recce breakdown, R-123 motorised foot units, box flights; Playwright 34/35, the one a runner clash, rerun 14/14), #42 (convoy bombing core: chart readers, 32.65 losses), #43 (6.26: a −26 unit surrenders to an enemy alongside). Draft #40 (R-120 defence). Requests for cna on main: 56.1 lanes + 32.66 readings; R-120 with HQs; each HQ's organisation type (slice 6 Task 3).
In flight: nothing uncommitted.
Next: merge #39, #42, #43 when green (`gh pr merge N --merge`). Then #40: re-script the module-on golden fight at Halfaya (R-120 makes the CW surrender there), regenerate `tests/games/graziani-m5.json`, the cna-web e2e fixtures and golden.spec pins. Then slice 6 Task 1–2 (attachment state and orders; no golden move splits an attached unit, checked).
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-25 21:15 MST — autopilot/m5-slice-6-organisation
Done: #40 (R-120 defence) re-scripted: module-on golden surrenders at Halfaya, 439 orders (6cd86b1, e2e 2nd commit); Playwright 36/36; marked ready. Slice 6 Task 1 on draft #44: `Piece.attached_to`, set-up/arrival attachment, old-save fill, stacking and HQ CPA read it.
In flight: Task 2 (Attach/Detach orders, moves carry attached units).
Next: merge #40 when web CI is green; merge main into #44 and regenerate e2e fixtures (`cargo test -p cna-web -- --ignored regenerate_e2e_fixtures`) and golden hashes; then Task 2.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-25 21:32 MST — autopilot/m5-slice-6-organisation
Done: #40 merged (eb6fb15). On draft #44: Task 2 (d970fd3: Attach/Detach, moves carry attached units, a lone attached unit refused; test fixtures stand split units alone via `GameState::put`/`stand_alone`), main merged and fixtures re-pinned (e7a6d29), detach-while-moving stop (88208c9), stack panel Attach/Detach + e2e `attachment.spec.ts` (430fcfc), docs (a7e531b).
In flight: full Playwright on e7a6d29 (/tmp/pw-44.txt); `attachment.spec.ts` not yet run; a fresh review of the engine diff.
Next: run `npx playwright test tests/attachment.spec.ts` and the full suite on HEAD; act on the review; then mark #44 ready only with Task 3–4 done or reslice (Tasks 1–2 + UI could merge alone: decide by what the review finds).
Blocked: Mission 4 on Brian's review of #25; Task 3 on cna's HQ organisation types.

## 2026-10-25 21:40 MST — autopilot/m5-slice-6-organisation
Done: review fixes (d208e3b: off-map moves carry, eliminations release attachments, arrivals join waiting units); e2e updates for detach-before-moving-alone (f8a3a89); off-map panel Attach/Detach (9ad6731); reslice: #44 = Tasks 1–2 + controls (plan, PR body).
In flight: full Playwright on f8a3a89 (/tmp/pw-44b.txt); 9ad6731's box test (`attachment.spec.ts`, 2nd test) not yet run.
Next: run `npx playwright test tests/attachment.spec.ts`; if the full run is green, mark #44 ready and merge on CI. Then Task 4 (24.9 dumps, §28 prisoners) — Task 3 waits on cna.
Blocked: Mission 4 on Brian's review of #25; Task 3 on cna's HQ organisation types.

## 2026-10-25 21:52 MST — autopilot/m5-slice-6-organisation
Done: box attach/detach (26a1754); full Playwright 37/37 on f8a3a89, targeted specs 19/19 on HEAD; #44 marked ready. PROGRESS on main (ef59195).
In flight: #44's CI at handoff.
Next: merge #44 when CI is green (`gh pr merge 44 --merge`); then slice 6 Task 4 (24.9 dumps under R-018 via `cna_rules::construction::dump_cost`; §28 prisoners) on a new branch.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 on cna's HQ organisation types.

## 2026-10-25 22:58 MST — autopilot/m5-slice-6b-dumps-prisoners
Done: #44 merged last run. Slice 6 Task 4 split: plan `docs/plans/2026-10-26-m5-slice-6b-prisoners.md` (§28 prisoner points, capture, phase-H march, departure). 24.9 dumps wait on cna (abstract supply's supply unit is the dump; request on `main`).
In flight: Task 1 (state and capture).
Next: Task 1–3 of the 6b plan.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 on cna's HQ organisation types.

## 2026-10-25 23:20 MST — autopilot/m5-slice-6b-dumps-prisoners
Done: #45 (slice 6b, §28 prisoners): points on surrender and 15.89 (25a4e94), UI march panel + e2e (7418b10), docs (323090e), review fixes (see PR). Playwright 38/38 before the review fixes. cna requests on `main` (80d80a4): 24.9 dumps under §32, the Sirte box.
In flight: #45's web CI; slice 7 (replacements from repair) on `autopilot/m5-slice-7-replacements` (worktree /tmp/cna-s7): engine + UI + `replacements.spec.ts` committed, e2e not yet run, no PR yet.
Next: merge #45 when CI is green; merge main into slice 7, run `npx playwright test tests/replacements.spec.ts` and the full suite, open its PR.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 on cna's HQ organisation types.

## 2026-10-25 23:45 MST — autopilot/m5-slice-7-replacements
Done: #45 merged (slice 6b, prisoners). #46 ready (slice 7): repaired points by hex (c64bbe5), `Order::Absorb` (a582acf), UI + `replacements.spec.ts` (0cb8bf1), review fixes; clippy, workspace tests, probe check, Playwright 39/39 green locally.
In flight: #46's CI.
Next: merge #46 when CI is green (`gh pr merge 46 --merge`). Then prisoner guards and escape (28.2x) or the PR #12 combat follow-ups (18.2x released units, 15.84/14.14 guns). Keep /tmp clear: a second worktree's `target/` filled tmpfs and broke a Playwright trace write.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 and 24.9 dumps on cna.

## 2026-10-26 00:00 MST — autopilot/m5-slice-8-guards
Done: #46 merged (slice 7). Plan for slice 8, guards and escape (28.2x): `docs/plans/2026-10-26-m5-slice-8-guards.md`, draft PR.
In flight: nothing built for slice 8.
Next: Task 1 of the slice 8 plan, test first. Golden indices shift again when the golden details guards: re-pin `cna-web` HASHED/`golden_prefix` indices and `golden.spec.ts` as #45 did.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 and 24.9 dumps on cna.

## 2026-10-26 01:20 MST — autopilot/m5-slice-8-guards
Done: slice 8 built: guards, march ratio, escape, liberation (37f14d0), UI + e2e (febca92), docs (plan revised as built: guards don't fight, detailed in C or H), one owner for point maps. Clippy, workspace tests, probe check, Playwright 39/39 green locally. #47 ready.
In flight: #47's CI.
Next: merge #47 when CI is green; then the PR #12 combat follow-ups (18.2x released units' later combat, 15.84/14.14 forward guns in anti-armour) as slice 9.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 and 24.9 dumps on cna.

## 2026-10-26 01:42 MST — autopilot/m5-slice-9b-construction
Done: #47 (slice 8, guards) merged. Plan for slice 9 (fortifications and construction) and 9a, fortification levels in combat (#48, ready). 9b on #49 (draft): `construction` module, Order::Build for fortifications, stages counted at each OpStage's end, Fortify control + `construction.spec.ts`; workspace tests and clippy green.
In flight: full Playwright run for 9b; #48's web CI.
Next: merge #48 on green CI, merge main into 9b, mark #49 ready once Playwright is 40/40; then 9c (minefields) per the plan, Task 3.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 and 24.9 dumps on cna.

## 2026-10-26 01:58 MST — autopilot/m5-slice-9c-minefields
Done: #48 (9a) merged; #49 (9b, construction module, fortifications, 3.61 item secret) ready, Playwright 40/40 locally. 9c on #50 (draft): laying (4586370), entry costs and reveal (a9f44ca), dummy removal, the mine roll, the combat shift; Rust gates green, Playwright 42-spec run in progress at handoff.
In flight: #49's CI; 9c's Playwright run (`/tmp/pw-s9c.log`).
Next: merge #49 on green CI; merge main into 9c; build clearing as `Order::ClearMinefield` (plan Task 3, last box), then mark #50 ready.
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 and 24.9 dumps on cna.

## 2026-10-26 02:05 MST — autopilot/m5-slice-9c-minefields
Done: #49 (9b) merged. 9c finished on #50 (ready): laying, entry costs and reveal, dummy removal, the mine roll, the combat shift, clearing (one owner for the field-engineer predicate). Gates green; Playwright 41/41 before the clearing commit.
In flight: #50's CI.
Next: merge #50 on green CI. Then slice 9 is done: refresh the land-turn design's Status and the README's systems line; then the PR #12 combat follow-ups (18.2x released units, 15.84/14.14 forward guns).
Blocked: Mission 4 on Brian's review of #25; slice 6 Task 3 and 24.9 dumps on cna.

## 2026-10-26 03:02 MST — autopilot/m5-cna-bump-2
Done: #51: vendor/cna bb0a8d1 -> 818fbf6 (c9be633; every request answered), R-133/R-134 minefields (no supplies under §32; rough not salt marsh), R-126..R-134 listed in NOT_SIMULATED, e2e hashes re-pinned. Rust gates green.
In flight: #51's CI; two stacked branches from it: `autopilot/m5-convoy-bombing` (32.6x with lanes, R-126..R-128) and `autopilot/m5-slice-6-limits` (slice 6 Task 3, caps and shells, R-130), worktrees in ../wt-*.
Next: merge #51 on green CI, then the two stacked PRs once their gates pass. R-132's off-map prisoner stages (3 at CPA 8) are a small follow-up.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 03:36 MST — autopilot/m5-docs
Done: merged #51 (cna 818fbf6, R-133/R-134), #52 (convoy bombing, subagent), #53 (reserve once per stage), #55 (attachment limits and shells, subagent). Opened #54 (15.84/14.14 gun losses; short golden re-pinned), #56 (design Status, README modules), #57 (shells one size smaller in 15.5x; fixes #55's battalion shells, empty OA TOE), #58 (R-132 prisoners off map; log line for ConvoyBombed, whose absence throws in the browser).
In flight: CI on #54/#56/#57/#58; slice 6 Task 5 UI (subagent, branch autopilot/m5-slice-6-ui, worktree ../wt-slice-6-ui).
Next: merge the four on green CI (#58 first); then division-sized formations in 15.5x's size table (plan 2026-10-26-m5-slice-6-organisation.md, Task 3 open item).
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 03:55 MST — autopilot/m5-docs
Done: also merged #54 (gun losses) and #58 (R-132 prisoners off map; ConvoyBombed log line, whose absence threw in the browser). #60 (stack-panel nesting, shell badge; subagent) fixed for tsc and merged with main; #57 merged with main; all green locally.
In flight: CI on #56, #57, #60; draft #59 (formations in 15.5x's size table) fails on purpose: set-up stacks HQs with their formations, so assaults see divisions and the short golden diverges at GT1 stage 1's close assault.
Next: merge #56/#57/#60 on green CI; then #59: confirm 15.55's reading, fix its test set-up, re-play both golden scripts and the e2e golden, re-pin.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 05:15 MST — autopilot/m5-division-size
Done: #59 finished: tests fixed, short golden re-played (Halfaya's defenders now stand and pay a 1-hex retreat) and re-pinned; m5 golden and e2e fixtures unchanged (cb0b169, 12ccbe9). #61 (ready): 61.38 reassignments no longer place 6 RTR twice; `cna-game::scenarios` smoke-tests every scenario (5 of 9 pass a turn). Two cna requests posted on main (ac2ee8d).
In flight: #59 local Playwright (/tmp/pw-59.log) and CI; subagent on `autopilot/m5-scenario-picker` (browser starts any scenario, stacked on #61).
Next: merge #59 and #61 on green CI; then the picker PR.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 05:40 MST — autopilot/m5-scenario-picker
Done: merged #59 (15.55 formations in the size table), #61 (every scenario smoke-tested; 61.38 reassignment), #63 (points victory, end dialog totals). #62 (browser scenario picker, subagent; Playwright 44/44 locally) merged with main and ready. Requests for cna: 21st Panzer replaces 5th Light, a road locator, Rommel as data.
In flight: #62's CI.
Next: merge #62 on green CI; refresh the land-turn design Status; then the next scenario land rule cna's data supports.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 07:30 MST — autopilot/m5-cna-bump-3
Done: #65: vendor/cna 803d5d5 (all three requests answered), stacked_with and Rommel as `PieceKind::Commander` (R-135), replacement counters at set-up (21st Panzer, II/155), The Last Chance sets up and plays; fixtures re-pinned (only rules_commit moved, checked). #66 (stacked): 31.0's initiative 6, recall roll, +1 assault morale, a scheduled Rommel arrives.
In flight: #65 Playwright/CI, then #66's gates.
Next: merge #65 then #66 on green CI; Rommel's +5 CPA escort (needs an order naming the unit), his reactions, the LRDG raid (27.9x).
Blocked: Mission 4 on Brian's review of #25; Rommel's Arrival family on one cna request (Italian divisions' hexes).

## 2026-10-26 07:35 MST — autopilot/m5-rommel
Done: #66 (stacked on #65): `cna-game::rommel`: Axis initiative 6 with Rommel on the map, 3 recalled; the recall roll each Game-Turn (RommelRecalled, logged in the browser); +1 assault morale in his hex (17.28); a scheduled Rommel arrives (2/20). Rust gates green; requested a cna ruling on the +5 CPA escort.
In flight: #65's CI, then #66's (local Playwright /tmp/pw-66.log).
Next: merge #65, then main into #66 and merge it on green CI. Then convoy lanes beyond Tripoli or the 19.3 chart as the shell denominator.
Blocked: Mission 4 on Brian's review of #25; Rommel's Arrival family and the escort bonus on cna.

## 2026-10-26 10:10 MST — autopilot/m5-rommel-escort
Done: #67 (autopilot/m5-cna-bump-4): vendor/cna fc78330 (R-136, R-137 answered both requests), `apart` placements; Rommel's Arrival and both campaigns set up, all nine scenarios smoke-tested, WAITING list removed (c50ec7c, eebaaec). #68 (stacked): R-137 escort, `Piece.rommel_cpa`, `base_cpa` for 8.17 and the motorised test, parting charges spared DP (41cc785, b829050). Rust gates and wasm-pack test green locally.
In flight: #67 local Playwright (/tmp/pw-67.log) and CI; #68 CI.
Next: merge #67 then #68 (retarget to main) on green CI. Then Rommel's reactions/retreats through ZOC or the LRDG raid (27.9x).
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 10:32 MST — autopilot/m5-rommel-escort
Done: #67 merged (ac7de18). #68 ready: R-137 escort, Rommel's reactions and lone free retreat (31.0), Crusader e2e fixture with the +5 badge seen in a screenshot; local Playwright 44/44 + 3/3; audit ledger posted as a #68 comment. PROGRESS on main (f8ab872): requests answered, new ruling request on a lone Rommel in enemy ZOC.
In flight: #68 CI.
Next: merge #68 on green CI. Then the audit's U1: an `Unplace` order during one's own set-up (store each piece's placement so it can go back to pending), since `apart` set-ups can strand themselves.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 10:50 MST — autopilot/m5-set-up-undo
Done: #68 merged; PROGRESS on main (7e7ce86). This branch: `Order::Unplace` (59.2), the audit's U1: the side setting up takes back a placed piece, rebuilt from the scenario's own set-up (location, motorisation points, attachments cleared); `unplaced` event and its log line in the browser. All local gates green.
In flight: the draft PR's CI; no browser button yet.
Next: a "Take back" button in the stack panel for one's own pieces during set-up, with a Playwright test (Rommel's Arrival); then mark ready and merge.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 12:11 MST — autopilot/m5-lone-rommel
Done: #70: vendor/cna fcf8ea7 (R-138), fixtures re-pinned (06ee10e, 0b19dc5); a lone Rommel reacts out of the mover's ZOC (2 CP break-off) and is captured once his escape is spent (8ef6001); review: one `pay_break_off` (ce0f3bd). Ledger comment on #70. PROGRESS on main (978e90f): request on 8.53's ZOC.
In flight: #70 CI and local Playwright (/tmp/pw-lone.log).
Next: merge #70 on green CI; then the 19.3 chart as the shell denominator.
Blocked: Mission 4 on Brian's review of #25.
## 2026-10-26 12:23 MST — autopilot/m3-reach-index
Done: nothing yet; measured `reachable` (11 Hussars, Graziani's) at 310 ms native, 84 ms with no other pieces.
In flight: `GameState::pieces_at(hex)` as the one owner of "pieces in a hex", backed by an index `reachable` installs for its search.
Next: finish, check the reach oracle tests and timing, open a draft PR; abandon if it isn't clean.
Blocked: none.

## 2026-10-26 12:38 MST — autopilot/m3-reach-index
Done: #70 merged (5d1d3a0), #71 merged (assignment tests 15.1x, 13.28). #72: `occupancy::pieces_at` plus an index `reachable` installs for its search; 310 → 91 ms native, 424 → 141 ms wasm (410ea18). Local Playwright 46/46, wasm-pack test green, ledger comment on #72. Three cna requests on main (8.53's ZOC, 19.3 shell components, Rommel after his stack is destroyed).
In flight: #72's CI (ready).
Next: merge #72 on green CI; time the browser's shading; then whichever cna request lands first.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 17:20 MST — autopilot/m5-r139-reaction-pin
Done: cna answered all three requests (R-139, R-140, R-141). #73 (draft): vendor/cna eafa8a3 (1e5a6c5-ish bump commit), R-139's reaction pin judged before the mover arrived, reactors leave the mover's ZOC free (022d06d + fmt). Goldens unchanged.
In flight: local Playwright (/tmp/pw-r139.log), then mark #73 ready.
Next: R-140 (Rommel's own forced retreat when his stack dies) on a branch after #73; then R-141 (`counts_for_shell` as the shell denominator).
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 17:40 MST — autopilot/m5-r141-shell-components
Done: #73 merged (R-139, vendor/cna eafa8a3), #74 merged (R-140, Rommel retreats alone; local Playwright 46/46 on both). #75: R-141, the 19.3 chart's counted components as the shell denominator (0b26d1b, by a subagent), merged with main. PROGRESS on main (e53278c). Browser shading timed: ~0.5 s click to shaded, engine ~141 ms of it.
In flight: #75 CI (ready).
Next: merge #75 on green CI; then substitution (19.x) or Axis convoy lanes beyond Tripoli; or profile the overlay render.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 17:55 MST — autopilot/m5-status-r139-r141
Done: #73 (R-139), #74 (R-140), #75 (R-141) merged; #75 choices ledger posted as a PR comment. Browser shading re-timed: ~200 ms for an uncapped stack (closed). PROGRESS on main.
In flight: #76 (design Status, docs only) on CI.
Next: merge #76; check #75 ledger item 1 (share matched by kind, not chart slot); then substitution (19.x) or Axis convoy lanes beyond Tripoli.
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 19:05 MST — autopilot/m5-capture-supply
Done: #75 ledger item 1 checked, real (share by equivalent, not chart slot: 15th Panzer's artillery-group, Polish MG slot); cna request posted on main (391c5bf). #77 (draft): a combat unit entering a hex of lone enemy supply captures it, on moves, reactions and retreats (1b75f87); `PieceKind::is_supply`. Local gates green.
In flight: #77 CI.
Next: mark #77 ready and merge on green; then reactions and retreats paying fuel (NOT_SIMULATED's abstract-supply row).
Blocked: Mission 4 on Brian's review of #25.

## 2026-10-26 19:20 MST — autopilot/m5-reaction-fuel
Done: #78 (draft, stacked on #77): reactions and retreats before assault pay fuel or are refused (32.22, R-120) (c74cba4). Module-on golden loses 4 RHA's reaction; graziani-m5.json, hashes, cna-web indices, e2e fixtures and three Playwright specs re-pinned.
In flight: local Playwright (/tmp/pw-fuel.log); combat spec's retreat-before-assault test fixed to click an on-screen hex (uncommitted until the rerun passes). #77 web CI.
Next: merge #77, merge main into #78, mark ready, merge. Then forced retreats paying fuel (treat no fuel as no path for R-012 and 10.36).

## 2026-10-26 19:30 MST — autopilot/m5-rail-units
Done: #77 merged (capture of lone supply). #78 ready: reactions and every retreat pay fuel, a stack without fuel stands and pays (R-120); local gates and Playwright green (23/23 on the changed specs, 45/46 full suite before the last spec fix). #79 (draft): plan for combat units by rail.
In flight: #78's web CI.
Next: merge #78 if green and not merged; then build #79's plan, Task 1 first (TDD).
Blocked: Mission 4 on Brian's review of #25.
