# Rulings without a switch

Every ruling in `vendor/cna/rulings/` either has a switch in
`cna-rules::ruleset` or a row here saying why not (yet). A test enforces it.

| Ruling | Why not (yet) |
|---|---|
| R-002 | map art (road stubs): a question of what the map shows, not of numbers |
| R-005 | interpretive: what one rail stack may carry; also needs movement |
| R-010 | interpretive: retreat before assault direction; also needs the map |
| R-013 | the difference is linear: ammunition scales with the pinned share; no probe adds information |
| R-014 | interpretive: what may capture a destroyed-tank marker |
| R-016 | interpretive: when a Tank Delivery Squadron moves |
| R-017 | interpretive: what a Tank Delivery Squadron may do in combat and retreat |
| R-019 | interpretive: the fleet's western limit; a map question, not one of numbers |
| R-020 | needs the Logistics Game (§47–58) restated in cna |
| R-021 | needs the Logistics Game (§47–58) restated in cna |
| R-022 | needs the Logistics Game (§47–58) restated in cna |
| R-023 | printed chart gap, decided in cna; the `chart-oddities` probe already measures how often it comes up |
| R-024 | printed chart gap, decided in cna; the `chart-oddities` probe already measures how often it comes up |
| R-025 | Logistics Game (§48–58): not in the engine yet |
| R-026 | Logistics Game (§48–58): not in the engine yet |
| R-027 | Logistics Game (§48–58): port efficiency; not in the engine yet |
| R-030 | Logistics Game (§48–58): not in the engine yet |
| R-031 | Air Game: not in the engine yet |
| R-032 | Air Game and Logistics Game: not in the engine yet |
| R-040 | Logistics Game (§48–58): not in the engine yet |
| R-041 | Logistics Game (§48–58): not in the engine yet |
| R-042 | Logistics Game (§48–58): not in the engine yet |
| R-043 | Logistics Game (§48–58): not in the engine yet |
| R-050 | Logistics Game (§48–58): not in the engine yet |
| R-051 | Logistics Game (§48–58): not in the engine yet |
| R-052 | Logistics Game (§48–58): not in the engine yet |
| R-053 | Logistics Game (§48–58): not in the engine yet |
| R-060 | Logistics Game (§48–58): not in the engine yet |
| R-061 | Logistics Game (§48–58): not in the engine yet |
| R-062 | Logistics Game (§48–58): not in the engine yet |
| R-070 | Logistics Game (§48–58): not in the engine yet |
| R-071 | Logistics Game (§48–58): not in the engine yet |
| R-072 | Logistics Game (§48–58): not in the engine yet |
| R-073 | Logistics Game (§48–58): not in the engine yet |
| R-080 | Logistics Game (§48–58): not in the engine yet |
| R-081 | Logistics Game (§48–58): not in the engine yet |
| R-082 | Logistics Game (§48–58): not in the engine yet |
| R-083 | Logistics Game (§48–58): not in the engine yet |
| R-090 | abstract logistics and air (§32): not in the engine yet |
| R-091 | abstract logistics and air (§32): not in the engine yet |
| R-092 | abstract logistics and air (§32): not in the engine yet |
| R-093 | abstract logistics and air (§32): not in the engine yet |
| R-094 | scenario set-up: applied in cna's scenario data, so the engine follows it by loading the data |
| R-095 | scenario set-up: applied in cna's scenario data, so the engine follows it by loading the data |
| R-096 | scenario set-up: applied in cna's scenario data, so the engine follows it by loading the data |
| R-097 | scenario ruling (other scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-098 | scenario set-up: applied in cna's scenario data, so the engine follows it by loading the data |
| R-099 | scenario ruling (other scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-100 | which arrival wins (schedule over OA sheet): no game state yet; Mission 2 slice 2 follows the schedule, which is the accepted option (spec, PR #5) |
| R-101 | scenario ruling (other scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-102 | scenario ruling (other scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-103 | scenario ruling (other scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-104 | scenario ruling (other scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-105 | scenario ruling (other scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-106 | scenario set-up: applied in cna's scenario data, so the engine follows it by loading the data |
| R-107 | scenario set-up: applied in cna's scenario data, so the engine follows it by loading the data |
| R-108 | scenario ruling (El Alamein scenarios); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-109 | scenario ruling (campaign games); the engine loads only Graziani's Offensive so far (Mission 2) |
| R-110 | Land-only victory supply in Graziani's Offensive: no game state yet; Mission 2 slice 6 implements it as a route trace (spec, PR #5) |
| R-111 | Air Game: not in the engine yet |
| R-112 | Air Game: not in the engine yet |
| R-113 | Air Game: not in the engine yet |
| R-114 | Air Game: not in the engine yet |
| R-115 | Air Game: not in the engine yet |
| R-116 | Air Game: not in the engine yet |

# Rulings implemented as decided

Interpretive rulings cna decided with one option worth playing: the engine
plays the Decision and offers no switch (Mission 2 spec, *Interpretive
rulings*). Each names where it is enforced.

| Ruling | Decision as built |
|---|---|
| R-003 | contact is fixed as every movement segment opens, either side's (`cna-game::zoc::fix_contact`); a unit entering an enemy ZOC mid-segment is stopped, not yet in contact |
| R-004 | a mixed stack pays the dearest piece's step cost and obeys every piece's prohibitions (`cna-game::movement::plan`) |
| R-006 | non-motorised units never react, whatever they stack with (`cna-game::reaction::reactors_next_to`); truck convoys are not in the engine yet |
| R-007 | a reaction spends CP on movement only: `Order::React` is a path and nothing else |
| R-008 | a unit in an enemy ZOC, in contact or engaged never reacts; no break-off fee arises in a reaction |
