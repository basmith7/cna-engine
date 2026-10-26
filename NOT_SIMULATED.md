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
| R-120 | abstract supply, a unit that cannot pay: the land move and the attack are refused as decided (`cna-game::fuel`, `cna-game::ammo`); a committed defender out of ammunition does not defend, and when every committed defender that owes ammunition is out the hex surrenders, HQs included (`cna-game::combat::assault`) |

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
| R-123 | a foot unit (printed CPA 1–10, or a 0+ gun) takes one motorisation point per TOE point in phase C and moves at 20 (15 for a gun) while it has at least that many; `+` units on the map are served first (`cna-game::organisation`); the points pay a fuel a stage. An HQ's CPA reads its motorised units at the truck CPA |
| R-124 | the air lift's destinations are division and brigade-echelon HQs, its forward limit the farthest division HQ counter (`cna-game::air_lift`) |
| R-125 | a recce unit's TOE points check for breakdown and repair in the field by its OA vehicle class; `none` never breaks down (`cna-game::breakdown`, `cna-game::repair`) |
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
| Prisoners (28.0) beyond points, guards and the march | surrenders and 15.89 rolls make prisoner points that the captor marches in phase H to Alexandria or Cairo, or the Axis to A2802 (no Sirte box in cna yet), where they leave play (`cna-game::prisoners`). Guards are points detailed from infantry TOE in phase C or the owner's phase H, a guard to five on the march, one to a camp standing still; the unguarded escape at the end of the captor's phase H (`cna-game::guards`). Not built: guards as counters that fight and react (28.21: 0/1, CPA 10), so a hex of prisoners and guards with no captor's combat unit is freed by any enemy combat unit entering it, the guards taken; escapees walking toward friends and losing 10 % a waterless stage (28.25), so they wait where they got away until an enemy entering recaptures them; the month's delay before escapees return as replacement points (they leave play, tallied in `escaped`); liberation by retreat or advance after combat (only moves and reactions free or recapture); the three-hex shift at surrender; feeding stores (Logistics Game); trucks carrying them. 15.89's points all count as prisoners, guns and tanks included |
| Captured equipment (28.3) | captured guns and tanks are lost, not used by the captor |
| Supply dumps (24.9) | under abstract supply none are built: the supply unit is the dump, and what 24.9 adds there waits on cna |
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
| Abstract supply and supply units (§32), Mission 5 in progress | with the `abstract_supply` module on (new `cna-play` and browser games): supply units carry loads, motorisation points stand where the trucks did, supply stacking and 32.8x's AA thinning apply, land moves pay fuel and barrages and assaults pay ammunition from supply in range; in phase C supply units take or give up their motorisation points and trade loads, and with them move alongside combat units in G or alone in H; a side may destroy its own supply; in phase J the Commonwealth runs two supply units a stage one way along the Mersa Matruh–Alexandria railway; a real supply unit captured alone in an enemy ZOC (10.29) goes over to the captor with its load and points. Once a Game-Turn in phase C a side may fly one supply unit without motorisation points between major cities, villages and the Tripoli and Tunisia boxes, for 12 fuel, to a division or brigade HQ no farther forward than its farthest division HQ (32.37, R-124); the browser flies to hexes and, for the Axis, the Tripoli and Tunisia boxes. Not yet: combat units by rail, capture by entering a hex of lone enemy supply (refused as entering an enemy hex), Axis convoy bombing and lanes beyond Tripoli (32.6x: needs 56.1's lanes from cna and §19's full divisions), reactions and retreats paying fuel. Each Game-Turn's naval convoy stage runs inside the engine as the turn opens: both sides' planning rolls (32.4x, on dice of their own), arrivals in phase D (the Commonwealth's in Cairo, one in three at Alexandria; the Axis's in the Tripoli box, three a convoy, the rest a turn later; a dummy for every three), and in a month's first turn the previous month's motorisation losses (32.5x), taken only from the rear map's pools. With the module off (older saves and logs) every unit is supplied without limit and supply units are inert |
| Breakdown (§21) and field repair (§22.2x), Mission 5 | with the `breakdown` module on (new games): motorised units collect each step's breakdown points in moves, reactions and retreats, and a unit that stops in a higher column checks its tanks and SP guns (BAR by model, hot weather a column right, a sandstorm move another) and a recce unit's points by its vehicle class (R-125: armoured cars, armoured recce, the desert patrols' trucks; camels and motorcycles never). Broken-down points stay in the hex (half in the start hex near a big enemy unit, 21.41), show on the map and in the stack panel, and a battalion moving next to the enemy's takes them where they stand. Phase K offers one field roll per tank type or recce class per hex (a fuel a point under abstract supply; armoured cars and trucks by the chart's points, tanks by its percentage); repaired points wait in the hex they were repaired in, and a unit standing there absorbs them in phase C at a CP per two, up to its printed strength (19.61, 20.41; `cna-game::replacements`); they do not move to a unit as replacement counters (20.42). Not yet: a unit's own truck points (8.91; none in Graziani's OA), capture's three-hex placement and passable-terrain test, cargo and dismounting (21.43–21.45), towing (21.6x) and facility repair (22.3x, 22.4x), which need repair facilities, and the tank delivery squadrons (22.6x) |
| Organisation (§19, §20): attachment, detachment, replacements, Mission 5 slice 6 in progress | a unit set up or arriving with the parent it is assigned to starts attached (4.25); stacking and an HQ's CPA read attachment; a unit attaches to its own parent in its hex or detaches, in phase C or its side's movement segment, for a CP each from it and its parent; a move, reaction or retreat of a parent carries what is attached to it, an attached unit never moves alone, and a unit detached while moving stops with its parent unless they go on together (19.4x). Not yet: non-assigned attachments and 19.5's limits, shells (9.26), substitution and nesting limits, reassignment (19.2x), replacements (§20) |
| Engineering and construction (§24–26) | with the `construction` module on, an engineer-capable unit and a 3+ TOE infantry battalion build fortification levels in phase C, three idle stages a level (`cna-game::construction`); major cities defend at their own level (2, or 3 for Alexandria and Cairo) and the best single in-hex benefit applies. Engineers lay real and dummy minefields in one stage, in clear, sand/gravel or rough hexes (R-134), spending no supplies under §32 (R-133); whether a field is real is its owner's secret. Entering a field costs the TEC's extra CP (friendly 1 on foot, 4 motorised; enemy 4, or the unit's CPA motorised; +4 or nothing with an engineer battalion or CW E-HQ in the stack) and an enemy entering turns it face up; a dummy so revealed is taken off at the end of that movement phase. Unescorted motorised combat units entering an enemy real field roll a die each, a 5–6 costing a TOE point (the engine's usual loss choice, not tanks or motorised infantry by name). Anti-armour and close assault shift one column for a defender in its own field or against attackers standing in one of its fields, never two (26.26). An engineer clears a field, friend or foe, as a one-stage project. Not built: the trucks' roll, mine costs on a step into an enemy ZOC (the engine's ZOC entry is free, 10.2x); roads, railways, air and repair facilities; barrage and bombing reducing levels (12.5x, Air Game); pinned builders losing the stage (24.23); E-HQs' engineering judged by the `^E` in the OA name |
| Destroyed tanks | leave the map: their capture and facility repair (22.4x) come with repair facilities |
| Patrols (phase L, §27) | skipped; barrage targets need no identification |
| Rail (phase J, §31) | with the `abstract_supply` module on, Commonwealth supply units only (32.3x); combat units do not ride (R-005's units half), the line ends at Mersa Matruh (no construction), no bombing. Module off: skipped |
| The Commonwealth fleet and the Axis convoy (phases E, II) | skipped; reinforcements arrive by the schedule at Cairo and Tripoli |
| The Air Game (§33–47) | left out, as 60.9 allows |
| The Logistics Game (§48–58) | left out, as 60.92 allows; victory's supply clause is R-110's route trace |
| Limited intelligence (3.6x), dummy supply units as hidden information | the state holds everything; a per-player view waits for the server |
