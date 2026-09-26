# Autopilot journal

Machine-to-machine handoff between runs; see AUTOPILOT.md.

## 2026-09-26 11:02 MST — autopilot/slice-1
Done: plan Tasks 1–9 (PR #1); review fixes (errata with a bad `table` now fails; R-011/R-012 findings state their assumptions).
Plan vs ruling: R-012 option 3 keeps the 15.82 buy-out when a path exists, so it equals option 2 there (the plan said option 1); followed the ruling. Attacker −11 expected loss is 930/36 ≈ 25.8 %, not the plan's "> 30".
Added `loss_base_factor`, `may_buy_out`, `may_withhold` so the switches drive the probes (spec: a switch must change a probe).
No undeclared close-assault gaps or overlaps: no `cna` PR needed.
Next: merge PR #1, then slice 2 plan (drafted this run) on autopilot/slice-2.
Blocked: none
