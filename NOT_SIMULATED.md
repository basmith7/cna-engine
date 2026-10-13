# Rulings without a switch

Every ruling in `vendor/cna/rulings/` either has a switch in
`cna-rules::ruleset` or a row here saying why not (yet). A test enforces it.

| Ruling | Why not (yet) |
|---|---|
| R-002 | map art (road stubs): a question of what the map shows, not of numbers |
| R-005 | interpretive: what one rail stack may carry; also needs movement |
| R-013 | ammunition is not in the engine yet (Mission 2 has no supply states); when it is, pinned, retreated-in and withheld units spend none (the Decision) |
| R-014 | destroyed-tank markers are not in the engine yet (slice 5 removes destroyed armour outright); when they are, only a battalion-size combat unit captures one (the Decision) |
| R-016 | interpretive: when a Tank Delivery Squadron moves |
| R-017 | interpretive: what a Tank Delivery Squadron may do in combat and retreat |
| R-019 | interpretive: the fleet's western limit; a map question, not one of numbers |
| R-020 | needs the Logistics Game (§47–58) restated in cna |
| R-021 | needs the Logistics Game (§47–58) restated in cna |
| R-022 | needs the Logistics Game (§47–58) restated in cna |
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
| R-111 | Air Game: not in the engine yet |
| R-112 | Air Game: not in the engine yet |
| R-113 | Air Game: not in the engine yet |
| R-114 | Air Game: not in the engine yet |
| R-115 | Air Game: not in the engine yet |
| R-116 | Air Game: not in the engine yet |
| R-118 | decided after slice 5: a must-attack retreat (10.36) still charges the unit's whole CPA and the retreat's CP on top (`cna-game::combat::retreat`); paying the retreat from the CP left and forfeiting the rest without disorganisation is a follow-up |
| R-119 | decided after Mission 2: the victory route trace (`cna-game::victory`) lets every enemy piece block and every friendly piece lift a ZOC, supply units and dummies included; counting combat units only is a follow-up |

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
| R-010 | retreat before assault is ordinary movement in any direction under the 13.2x limits (`cna-game::combat::retreat::retreat_before_assault`) |
| R-023 | a defender reading of 34–36 on the +2 column inflicts 5 % (`cna-rules::close_assault::loss_pct_decided`) |
| R-024 | a morale reading of 56 at cohesion −4 is −2 (`cna-game::combat::assault::adjusted_morale`) |
| R-110 | Land-only victory supply is a route a medium truck could follow from the holding unit to the level's source, through no enemy hex and no enemy-ZOC hex without a friendly piece (`cna-game::victory`); every enemy hex able to exert a ZOC does, since 10.16's declarations belong to a movement segment |
| R-117 | a swamp hex entered by railroad costs the road rate: 1 CP non-motorised, ½ motorised, breakdown ½ (`cna_rules::movement::step_cost`) |

# Combat systems not simulated (Mission 2 slice 5)

The combat segment runs on pieces without these; each is a later slice or
mission, and the engine neither models nor approximates them.

| System | What the engine does instead |
|---|---|
| Ammunition and fuel (SPI 6.4, 47–58) | nothing is deducted; no unit is ever out of ammunition |
| Trucks, halftracks and mounted infantry (12.46, 14.48) | no truck roll after a barrage; no halftrack cap |
| Destroyed-tank markers and their capture (14.4x, 14.5) | destroyed armour leaves the map |
| Dummy tank formations (16.4) | none exist |
| Prisoners and captured equipment (15.89, 28.0) | the captured share is logged (`Event::Prisoners`) and nothing changes hands |
| Advance after combat | none: cna's 50-stacking-and-zoc.md says movement in the next segment is not an advance |
| The owner's choice of which TOE points die (12.45, 14.4x, 15.83) | the engine removes the points that absorb most per point first (`cna-game::combat::losses`) |
| The "no armour" reassignment of anti-armour points to close assault (14.23) | the points fire at nothing |
| Patrol identification of barrage targets (12.2x) | the firer names any target in the hex |
| Divisions in the size table (15.53) | the largest unit is a brigade at most, so no division shift arises |

# Systems not simulated (Mission 2)

The game state plays Graziani's Offensive with the Land Game's movement
and combat only. Each of these is a later mission; the sequence of play
passes through its phase with a `PhaseSkipped` event, so adding one changes
only that phase.

| System | What the engine does instead |
|---|---|
| Abstract supply and supply units (§32) | every unit is supplied with unlimited ammunition and fuel; supply units are placed, captured when left alone in an enemy ZOC (10.29), and otherwise inert; motorisation points are not tracked |
| Breakdown (§21) | the breakdown segment of phase G is skipped |
| Organisation (§20): attachment, detachment, replacements | phase C is skipped; the set-up's attachments are fixed for the game |
| Engineering and construction (§24–26) | no minefields, fortifications, roads or airfields are built or demolished; the scenario starts with none (60.7) |
| Repair (phase K) and vehicle breakdown pools | skipped; destroyed armour leaves the map |
| Patrols (phase L, §27) | skipped; barrage targets need no identification |
| Rail (phase J, §31) | skipped; the railroad to Mersa Matruh carries nothing |
| The Commonwealth fleet and the Axis convoy (phases E, II) | skipped; reinforcements arrive by the schedule at Cairo and Tripoli |
| The Air Game (§33–47) | left out, as 60.9 allows |
| The Logistics Game (§48–58) | left out, as 60.92 allows; victory's supply clause is R-110's route trace |
| Limited intelligence (3.6x), dummy supply units as hidden information | the state holds everything; a per-player view waits for the server |
