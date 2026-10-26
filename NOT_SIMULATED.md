# Rulings without a switch

Every ruling in `vendor/cna/rulings/` either has a switch in
`cna-rules::ruleset` or a row here saying why not (yet). A test enforces it.

| Ruling | Why not (yet) |
|---|---|
| R-002 | map art (road stubs): a question of what the map shows, not of numbers |
| R-005 | interpretive: what one rail stack may carry; also needs movement |
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
| R-120 | abstract supply, a unit that cannot pay: the land move and the attack are refused as decided (`cna-game::fuel`, `cna-game::ammo`); a defender out of ammunition still defends and no hex surrenders yet (a later Mission 5 slice) |
| R-123 | motorising foot units with motorisation points: a later Mission 5 slice |
| R-124 | HQ echelons for air lift: the Air Game is not in the engine yet |
| R-125 | breakdown class of recce units: the breakdown slice (Mission 5 slice 5) |

# Rulings implemented as decided

Interpretive rulings cna decided with one option worth playing: the engine
plays the Decision and offers no switch (Mission 2 spec, *Interpretive
rulings*). Each names where it is enforced.

| Ruling | Decision as built |
|---|---|
| R-013 | with abstract supply on, only units committed to a barrage or close assault pay ammunition; pinned, retreated-in and withheld units are never in the commit lists and pay none (`cna-game::ammo`) |
| R-003 | contact is fixed as every movement segment opens, either side's (`cna-game::zoc::fix_contact`); a unit entering an enemy ZOC mid-segment is stopped, not yet in contact |
| R-004 | a mixed stack pays the dearest piece's step cost and obeys every piece's prohibitions (`cna-game::movement::plan`) |
| R-006 | non-motorised units never react, whatever they stack with (`cna-game::reaction::reactors_next_to`); truck convoys are not in the engine yet |
| R-007 | a reaction spends CP on movement only: `Order::React` is a path and nothing else |
| R-008 | a unit in an enemy ZOC, in contact or engaged never reacts; no break-off fee arises in a reaction |
| R-010 | retreat before assault is ordinary movement in any direction under the 13.2x limits (`cna-game::combat::retreat::retreat_before_assault`) |
| R-023 | a defender reading of 34–36 on the +2 column inflicts 5 % (`cna-rules::close_assault::loss_pct_decided`) |
| R-024 | a morale reading of 56 at cohesion −4 is −2 (`cna-game::combat::assault::adjusted_morale`) |
| R-110 | Land-only victory supply is a route a medium truck could follow from the holding unit to the level's source, through no enemy hex and no enemy-ZOC hex without a friendly piece (`cna-game::victory`); every enemy hex able to exert a ZOC does, since 10.16's declarations belong to a movement segment |
| R-117 | a swamp hex entered from a railroad hex across a constructed railroad hexside costs the road rate: 1 CP non-motorised, ½ motorised, breakdown ½; an unconstructed railroad opens nothing, and hexside features on the step are charged as off a road (`cna_rules::movement::step_cost`) |
| R-118 | a must-attack retreat (10.36) is paid as ordinary movement from the CP the unit has left; `cp_spent` then rises to the CPA with no DP for the difference, and 3 DP follow; only a retreat costing more than was left overspends (`cna-game::combat::retreat::forced_retreat`); with the `breakdown` module on, the retreat collects breakdown points and checks where it stops (§21) |
| R-121 | a company-equivalent pays half a battalion-equivalent's ammunition; a side's companies in one barrage or assault pay `ceil(n * rate / 2)` together, the first company taking the rounded-up half (`cna_rules::supply::company_ammo_share`, `cna-game::ammo`) |
| R-122 | a committed defender counts at its whole TOE in the loss base, held-back points included; a wholly withheld unit only in an overrun; pinned and retreated-in units always (`cna-game::combat::assault::defender_loss_shares`); held-back points are not committed for 15.86 disorganisation |
| R-119 | the victory route trace and hold test (`cna-game::victory`) look at combat units only: an enemy supply unit or dummy blocks no route and denies no hold, a friendly one lifts no enemy ZOC |

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

# The browser UI enforces nothing (Mission 3)

`web/` draws the engine's `view()` and sends `Order` JSON back; it holds
no rule of its own. What it shows as allowed (placement shading, the
reachable overlay, the sealed panels' options) is the engine's answer to
`placements`, `reachable` and `sealed_options`, and `apply` alone judges
what is sent. Where that leaves a gap, it is the engine's gap, listed
above: in particular the set-up `constraint` prose (59.2) is shown under
the tray and trusted to the owner, never checked.

# Systems not simulated (Mission 2)

The game state plays Graziani's Offensive with the Land Game's movement
and combat only. Each of these is a later mission; the sequence of play
passes through its phase with a `PhaseSkipped` event, so adding one changes
only that phase.

| System | What the engine does instead |
|---|---|
| Abstract supply and supply units (§32), Mission 5 in progress | with the `abstract_supply` module on (new `cna-play` and browser games): supply units carry loads, motorisation points stand where the trucks did, supply stacking and 32.8x's AA thinning apply, land moves pay fuel and barrages and assaults pay ammunition from supply in range; in phase C supply units take or give up their motorisation points and trade loads, and with them move alongside combat units in G or alone in H; a side may destroy its own supply; in phase J the Commonwealth runs two supply units a stage one way along the Mersa Matruh–Alexandria railway; a real supply unit captured alone in an enemy ZOC (10.29) goes over to the captor with its load and points. Not yet: the air lift (needs HQ echelons from cna), combat units by rail, motorising infantry, capture by entering a hex of lone enemy supply (refused as entering an enemy hex), Axis convoy bombing and lanes beyond Tripoli (32.6x: needs 56.1's lanes from cna and §19's full divisions), reactions and retreats paying fuel. Each Game-Turn's naval convoy stage runs inside the engine as the turn opens: both sides' planning rolls (32.4x, on dice of their own), arrivals in phase D (the Commonwealth's in Cairo, one in three at Alexandria; the Axis's in the Tripoli box, three a convoy, the rest a turn later; a dummy for every three), and in a month's first turn the previous month's motorisation losses (32.5x), taken only from the rear map's pools. With the module off (older saves and logs) every unit is supplied without limit and supply units are inert |
| Breakdown (§21) and field repair (§22.2x), Mission 5 | with the `breakdown` module on (new games): motorised units collect each step's breakdown points in moves, reactions and retreats, and a unit that stops in a higher column checks its tanks and SP guns (BAR by model, hot weather a column right, a sandstorm move another). Broken-down points stay in the hex (half in the start hex near a big enemy unit, 21.41), show on the map and in the stack panel, and a battalion moving next to the enemy's takes them where they stand. Phase K offers one field roll per tank type per hex (a fuel a point under abstract supply); repaired points wait in `GameState.repaired` until organisation (§20) brings them back as replacements. Not yet: armoured cars, recce and trucks inside a unit's TOE (cna's R-125 now types recce; a later bump), capture's three-hex placement and passable-terrain test, cargo and dismounting (21.43–21.45), towing (21.6x) and facility repair (22.3x, 22.4x), which need repair facilities, and the tank delivery squadrons (22.6x) |
| Organisation (§20): attachment, detachment, replacements | phase C is skipped; the set-up's attachments are fixed for the game |
| Engineering and construction (§24–26) | no minefields, fortifications, roads or airfields are built or demolished; the scenario starts with none (60.7) |
| Destroyed tanks | leave the map: their capture and facility repair (22.4x) come with repair facilities |
| Patrols (phase L, §27) | skipped; barrage targets need no identification |
| Rail (phase J, §31) | with the `abstract_supply` module on, Commonwealth supply units only (32.3x); combat units do not ride (R-005's units half), the line ends at Mersa Matruh (no construction), no bombing. Module off: skipped |
| The Commonwealth fleet and the Axis convoy (phases E, II) | skipped; reinforcements arrive by the schedule at Cairo and Tripoli |
| The Air Game (§33–47) | left out, as 60.9 allows |
| The Logistics Game (§48–58) | left out, as 60.92 allows; victory's supply clause is R-110's route trace |
| Limited intelligence (3.6x), dummy supply units as hidden information | the state holds everything; a per-player view waits for the server |
