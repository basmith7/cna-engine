# Rulings without a switch

Every ruling in `vendor/cna/rulings/` either has a switch in
`cna-rules::ruleset` or a row here saying why not (yet). A test enforces it.

| Ruling | Why not (yet) |
|---|---|
| R-002 | map art (road stubs): a question of what the map shows, not of numbers |
| R-005 | as decided: one stack each way a stage carries units and/or supplies and may change its load at stops along the line, the limits checked after each (`cna-game::rail`, `Order::Rail`'s `stops`) |
| R-014 | destroyed-tank markers are not in the engine yet (slice 5 removes destroyed armour outright); when they are, only a battalion-size combat unit captures one (the Decision) |
| R-016 | interpretive: when a Tank Delivery Squadron moves |
| R-017 | interpretive: what a Tank Delivery Squadron may do in combat and retreat |
| R-019 | interpretive: the fleet's western limit; a map question, not one of numbers. Applied as decided: no station west of hex row xx29 on sheet B (`cna-game::fleet`) |
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
| R-126 | an Axis convoy whose lane's first threshold no Commonwealth division HQ has reached is not attacked and draws no dice (`cna-game::convoy`) |
| R-127 | a 5 % convoy result is read like 10 %: it hits only motorisation and replacement points, which convoys do not carry yet, so it sinks nothing (`cna-game::convoy::supply_sunk`) |
| R-128 | dummies for an Axis convoy are one per three supply units that land after bombing (`cna-game::convoy::supply_arrivals`) |
| R-129 | confirms R-120's reading above: an all-out hex surrenders though HQs in it owe no ammunition (`cna-game::combat::assault`) |
| R-130 | each HQ's `attachment_parent` picks its Maximum Attachment Chart row, so German brigades take the regiment row, German artillery regiment HQs the artillery brigade HQ row, army tank brigades the tank brigade rows (`cna-game::limits`) |
| R-131 | under abstract supply no supply dumps are built; the engine builds none (`cna-game::construction`) |
| R-132 | Axis prisoners marched to A2802 leave the map and leave play on reaching the Tripolitania box, three of the captor's phase Hs later (8.89 at CPA 8; `cna-game::prisoners`) |
| R-133 | under abstract supply construction spends no supplies (`cna-game::construction`) |
| R-134 | minefields are laid in clear, sand/gravel or rough hexes, not salt marsh (`cna-game::construction`) |
| R-135 | Rommel is his own counter (`PieceKind::Commander`, CPA 60), sets up with a German unit, stacks free and never fights; on the map he rates the Axis 6 for initiative, a 12 at a Game-Turn's start recalls him for the turn (rating 3), and Axis units in his hex add +1 to assault morale (`cna-game::rommel`). He is offered reactions like a motorised combat unit (`cna-game::reaction`). Retreating alone (before assault or forced) he ignores enemy ZOC and pays no break-off (`cna-game::movement`). An LRDG next to him may raid him (27.6x, 27.92; see Desert raiders). Not yet: alone in an enemy ZOC in the enemy's phase he is still lost like a lone HQ (10.29) |
| R-137 | every Axis combat unit and HQ in a commanding Rommel's hex as an OpStage begins moves on CPA +5 while it stays in his hex; any order that leaves them apart (a move, a retreat, a reaction) takes the bonus away and charges the DP it spared. 8.17's foot cap and the motorised test use the unit's own CPA (`cna-game::rommel`). An HQ with the bonus adds it to its slowest attached unit's CPA (6.15), wherever that unit stands; a 6.3 refund after parting is reckoned on the own CPA |
| R-138 | a commanding Rommel alone in an enemy ZOC in the enemy's phase is captured (removed, no prisoner points) when an enemy combat unit's move puts him in its ZOC and he is not offered a reaction, when he passes or is left behind by the reaction, at the end of the retreat-before-assault step, and at a segment's start (10.29). As for every unit (R-139), the approaching stack's own ZOC does not bar his reaction; he alone reacts out of it paying 2 CP to break off (`cna-game::rommel`, `reaction`). A Rommel left alone when his stack is destroyed in an assault retreats alone (R-140's row) |
| R-136 | an `apart` set-up block keeps each entry (a listed unit with everything it brings) that many hexes from every other: Rommel's Arrival's four Italian divisions (`cna-game::place`). Set-up offers only legal hexes but does not look ahead; a player who crowds the band takes pieces back with `Order::Unplace` (59.2; the stack panel's Take back, for pieces placed since the page loaded) |
| R-139 | only an enemy ZOC a unit stood in before the mover arrived bars its reaction (8.53c): the mover is counted at the hex it left, so its own new ZOC never pins, and a unit that stays is pinned by it at the next arrival. A unit reacts out of the mover's ZOC without breaking off (`cna-game::reaction`). 8.54's size limit never spares a unit from a ZOC, which needs 2 SP |
| R-140 | when an assault (or a surrender to one) leaves a commanding Rommel without a combat unit, he is ordered a forced retreat of his own at once, the result's hexes or at least one, through enemy ZOC free; wherever he ends, an enemy ZOC with no friendly combat unit captures him (`cna-game::rommel`, `combat::retreat`). The player picks the path: the engine does not force R-140's path toward the nearest dump or city, nor his going on past the ordered hexes, only captures him if he stops short of safety |
| R-141 | a division or brigade HQ whose OA `organisation` names a 19.3 chart row covering the game-turn is judged a shell (9.26-9.27) against that row's `counts_for_shell` components, filled as R-142's row says; an HQ with no such row falls back to the units the OA assigns it (`cna-game::limits`) |
| R-142 | a shell's share counts the most of its chart row's counted slots that the counters attached to it fill at once, assigned or not: each counter one slot it names in `fills`, or with others one it names in `fills_part`, filled at the chart's `part_slots` minimum; an attached shell fills nothing (`cna-game::limits::slots_filled`) |
| R-143 | as decided: a networked player sees the dice and the table result of every barrage, anti-armour fire and assault, never which TOE points the enemy lost, its DP or its disorganisation (`cna-game::view::events`: `Losses`, `Disorganised` are their owner's) |
| R-144 | as decided: the Commonwealth sees an Axis convoy's lane, bomb points, roll and percentage, not how many supply units sailed or sank (`cna-game::view::events`, `ConvoyBombed`). The Logistics Game's flak is not in the engine |
| R-145 | as decided: an escape and a recapture are shown to both sides with their numbers; how many escaped points have rejoined their side (`escaped`) is that side's alone (`cna-game::view::state`). Escapees do not walk or thirst in the engine yet, so their marker's count is the one both saw at the escape |
| R-146 | as decided: the data does it. A Commonwealth support group or motor brigade HQ lists `infantry-brigade` in its `fills`, so it fills the type IV armoured division's infantry brigade slot from turn 92, and is itself judged a shell against the chart's added support-group row for turns 92 on (three infantry battalions) (`cna-game::limits`) |
| R-147 | as decided: `unit:cw:lrdg-1`, `unit:cw:lrdg-2` and `unit:de:dsa` are `PieceKind::Raider` pieces from their `raider` values (CPA 50), and a set-up that places one starts it formed and off the map (`hidden`): Rommel's Arrival's LRDG 1 at Jalo (`cna-game::setup`). Only a Commonwealth raider, so an LRDG, raids Rommel. Formed in play by `Order::FormRaider` (see Desert raiders) |
| R-148 | as decided: a raider makes no supply call and pays no fuel, as no piece but a combat unit does (`cna-game::fuel`); the engine plays only the abstract block, so this is every game |
| R-149 | as decided: `Order::RaidDump` names an enemy supply unit, real or dummy (by its alias), as the dump (`cna-game::raiders`); a 1-2 burns 10 % of its ammunition and 10 % of its fuel, each rounded up, a unit left with neither is removed (32.11), and a raided dummy is revealed and removed. Options 2 and 3 are not built |
| R-150 | as decided: morale training (17.3x) is not simulated, so a unit that takes its OA sheet's morale where the sheet prints an untrained rating beside it (`basic_morale_alt`) never forms a raider; every other unit, at its assigned morale, does unless it is in training (`cna-game::raider_forming`, `cna-data::oa::Oa::untrained`). A sheet with no morale whose units take another's is read as printing none |
| R-151 | not yet: the *San Giorgio* (30.17, 30.39) is not placed or fired; its values load (`cna-data::oa::Blockship`) for the fleet's later tasks |
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
| Prisoners (28.0) beyond points, guards and the march | surrenders and 15.89 rolls make prisoner points that the captor marches in phase H to Alexandria or Cairo, or the Axis to A2802 and on, off the map, to the Tripolitania box three phase Hs later (R-132), where they leave play (`cna-game::prisoners`). Guards are points detailed from infantry TOE in phase C or the owner's phase H, a guard to five on the march, one to a camp standing still; the unguarded escape at the end of the captor's phase H (`cna-game::guards`). Not built: guards as counters that fight and react (28.21: 0/1, CPA 10), so a hex of prisoners and guards with no captor's combat unit is freed by any enemy combat unit entering it, the guards taken; escapees walking toward friends and losing 10 % a waterless stage (28.25), so they wait where they got away until an enemy entering recaptures them; the month's delay before escapees return as replacement points (they leave play, tallied in `escaped`); liberation by retreat or advance after combat (only moves and reactions free or recapture); the three-hex shift at surrender; feeding stores (Logistics Game); trucks carrying them. 15.89's points all count as prisoners, guns and tanks included |
| Captured equipment (28.3) | captured guns and tanks are lost, not used by the captor |
| Supply dumps (24.9) | under abstract supply none are built (R-131); the Logistics Game, which keeps them, is not built |
| Advance after combat | none: cna's 50-stacking-and-zoc.md says movement in the next segment is not an advance |
| Points victories (Crusader, The Long Retreat, the campaigns) | a place scores for the side whose combat units alone occupy any of its hexes; the occupation's supply conditions are prose and unchecked; a level cna maps no victory level to (Crusader's draw, The Long Retreat's Commonwealth win at 0) gives none (`cna-game::victory`) |
| The owner's choice of which TOE points die (12.45, 14.4x, 15.83) | the engine removes the points that absorb most per point first (`cna-game::combat::losses`) |
| The "no armour" reassignment of anti-armour points to close assault (14.23) | the points fire at nothing |
| Patrol identification of barrage targets (12.2x) | the firer names any target in the hex |
| Formations in the size table (15.53, 15.55) | a piece counts as the formation of the topmost HQ it is attached under on its own hex (a whole division, a brigade, or one size smaller as a shell), whether or not the rest of that formation joins the assault (`cna-game::combat::assault`) |

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
| Abstract supply and supply units (§32), Mission 5 in progress | with the `abstract_supply` module on (new `cna-play` and browser games): supply units carry loads, motorisation points stand where the trucks did, supply stacking and 32.8x's AA thinning apply, land moves, reactions and retreats pay fuel (a unit that cannot pay makes none of them, and a stack ordered to retreat stands and pays 10 % a hex whatever R-012 says, R-120) and barrages and assaults pay ammunition from supply in range; in phase C supply units take or give up their motorisation points and trade loads, and with them move alongside combat units in G or alone in H; a side may destroy its own supply; in phase J the Commonwealth runs one stack each way along the Mersa Matruh–Alexandria railway, each up to 2 SP of combat units and HQs that spent no CP, with what is attached to them (8.7x, 19.4x), and up to two supply units a stage, all one way, riding in a stack (32.3x, R-005); any of them on the open line opens the phase; a real supply unit captured alone in an enemy ZOC (10.29), or in a hex an enemy combat unit enters (32.1x, on a move, a reaction or a retreat), goes over to the captor with its load and points, a dummy is removed; a stack that brings its own supply into the hex is checked for supply stacking without the units it captures there. Once a Game-Turn in phase C a side may fly one supply unit without motorisation points between major cities, villages and the Tripoli and Tunisia boxes, for 12 fuel, to a division or brigade HQ no farther forward than its farthest division HQ (32.37, R-124); the browser flies to hexes and, for the Axis, the Tripoli and Tunisia boxes. A run may stop on its way to set pieces down and take others on, the limits holding after each stop (8.77, R-005; `Order::Rail`'s `stops`; the browser's move panel adds them). Each Game-Turn's naval convoy stage runs inside the engine as the turn opens: both sides' planning rolls (32.4x, on dice of their own), arrivals in phase D (the Commonwealth's in Cairo, one in three at Alexandria; the Axis's by the lanes it names in phase C (32.63; none named, lane 2 to the Tripoli box), three a lane in its order, at the lane's port (Benghazi, Tobruk, the Tripoli or Tunisia box), the rest a turn later; a lane opens at its first Game-Turn (56.14) and Bizerta's once the Axis rolls 12 on two dice, rolled each turn from GT35 while the Axis names it, its share going on to the next lane named until then (55.17); a convoy to a port where a Commonwealth combat unit stands does not sail and its share waits for the next turn (56.15, the engine's reading of a captured port and of "never sails"), each convoy bombed on the way from the westernmost Commonwealth division HQ, the engine's reading of §19's full division (32.6x, R-126, R-127; a 20 %+ result sinks that share of the supply units, at least one, and smaller results nothing, since convoys carry no motorisation or replacement points yet), on dice of their own; a dummy for every three that land, R-128), and in a month's first turn the previous month's motorisation losses (32.5x), taken only from the rear map's pools. With the module off (older saves and logs) every unit is supplied without limit and supply units are inert |
| Breakdown (§21) and field repair (§22.2x), Mission 5 | with the `breakdown` module on (new games): motorised units collect each step's breakdown points in moves, reactions and retreats, and a unit that stops in a higher column checks its tanks and SP guns (BAR by model, hot weather a column right, a sandstorm move another) and a recce unit's points by its vehicle class (R-125: armoured cars, armoured recce, the desert patrols' trucks; camels and motorcycles never). Broken-down points stay in the hex (half in the start hex near a big enemy unit, 21.41), show on the map and in the stack panel, and a battalion moving next to the enemy's takes them where they stand. Phase K offers one field roll per tank type or recce class per hex (a fuel a point under abstract supply; armoured cars and trucks by the chart's points, tanks by its percentage); repaired points wait in the hex they were repaired in, and a unit standing there absorbs them in phase C at a CP per two, up to its printed strength (19.61, 20.41; `cna-game::replacements`); they do not move to a unit as replacement counters (20.42). Not yet: a unit's own truck points (8.91; none in Graziani's OA), capture's three-hex placement and passable-terrain test, cargo and dismounting (21.43–21.45), towing (21.6x) and facility repair (22.3x, 22.4x), which need repair facilities, and the tank delivery squadrons (22.6x) |
| Organisation (§19, §20): attachment, detachment, replacements, Mission 5 slice 6 in progress | a unit set up or arriving with the parent it is assigned to starts attached (4.25); stacking and an HQ's CPA read attachment; a unit attaches to its own parent in its hex or detaches, in phase C or its side's movement segment, for a CP each from it and its parent; a move, reaction or retreat of a parent carries what is attached to it, an attached unit never moves alone, and a unit detached while moving stops with its parent unless they go on together (19.4x). A non-assigned unit attaches in phase C for 2 CP each, within the parent's 19.5 row (max units, company/artillery-only rows, type limits, the German brigade alternatives, two free company-equivalents for divisions and brigades), counted through nested attachments; a brigade-equivalent attaches non-assigned only where the row has alternatives. An independent unit (no OA parent) attached in place of an assigned unit not in play (never arrived, or gone) whose chart slot it fills (R-142) is a substitute, one per missing unit, judged when it attaches: it counts against no row, its parent's or a larger one's (19.4x). Shells (9.26–9.28) are judged against the HQ's 19.3 chart row, slot by slot (R-141, R-142), and stack one level smaller. Not yet: the "non-SP" and machine-gun/heavy-weapons exceptions to the chart's types, nested assigned units counting against a larger parent, reassignment (19.2x); a substitute keeps its place when the unit it stands for later arrives and attaches, so the parent may then carry more than its row |
| Engineering and construction (§24–26) | with the `construction` module on, an engineer-capable unit and a 3+ TOE infantry battalion build fortification levels in phase C, three idle stages a level (`cna-game::construction`); major cities defend at their own level (2, or 3 for Alexandria and Cairo) and the best single in-hex benefit applies. Engineers lay real and dummy minefields in one stage, in clear, sand/gravel or rough hexes (R-134), spending no supplies under §32 (R-133); whether a field is real is its owner's secret. Entering a field costs the TEC's extra CP (friendly 1 on foot, 4 motorised; enemy 4, or the unit's CPA motorised; +4 or nothing with an engineer battalion or CW E-HQ in the stack) and an enemy entering turns it face up; a dummy so revealed is taken off at the end of that movement phase. Unescorted motorised combat units entering an enemy real field roll a die each, a 5–6 costing a TOE point (the engine's usual loss choice, not tanks or motorised infantry by name). Anti-armour and close assault shift one column for a defender in its own field or against attackers standing in one of its fields, never two (26.26). An engineer clears a field, friend or foe, as a one-stage project. Not built: the trucks' roll, mine costs on a step into an enemy ZOC (the engine's ZOC entry is free, 10.2x); roads, railways, air and repair facilities; barrage and bombing reducing levels (12.5x, Air Game); pinned builders losing the stage (24.23); E-HQs' engineering judged by the `^E` in the OA name |
| Destroyed tanks | leave the map: their capture and facility repair (22.4x) come with repair facilities |
| Replacement counters arriving in play (OA `replaces`) | a set-up that places one starts with the replacement made (R-096: 21st Panzer for 5th Light, II/155 for I/75); one arriving by the schedule comes in beside the counter it replaces, which stays in play with its units |
| Desert raiders (§27), Mission 5 in progress | `PieceKind::Raider` moves as a recce vehicle on CPA 50 and never past it, takes no stacking points, exerts no ZOC, need not stop in one, and never breaks down or fights; a `hidden` one leaves the other side's view and events (27.31, `cna-game::raiders`, `cna-game::view`). Entering a hex an enemy combat unit could control it rolls one die, a 6 eliminating it (27.41); the enemy's declarations are not asked for, so a hex the enemy would have declared exerting no ZOC still spots, and a raider that survives stays hidden, the roll unseen by the enemy. `Order::RaidRommel`: a Commonwealth raider next to Rommel's hex in its movement segment spends 10 CP and rolls on the Raid on Rommel Table, four raids in a campaign game (the four scenarios whose slug names a campaign) and one in any other scenario (27.6x, 27.92); it raids from the next hex, as no unit enters an enemy-held one (8.1x), and one the raid leaves at 50 CP spent is eliminated. Only a Commonwealth raider raids, and every Commonwealth raider is an LRDG (R-147). Rommel's Arrival sets up LRDG 1 at Jalo, formed and hidden (R-147). `Order::FormRaider` forms a raider counter not in play in phase C, in the hex of a combat unit of its own nation whose OA `vehicle` is armoured car or armoured recce, taking one of its TOE points for good (27.13, R-147); one eliminated is formed again from the fourth Game-Turn after, a stage later (27.16, E-026). Morale training (17.3x) is not simulated, so any unit not in training counts as at maximum morale, and a unit never gives up its last TOE point. A raider that moves is off the map again (27.31). An enemy combat unit enters a raider's hex freely (a raider holds no hex: it blocks no move, attack or retreat and opens no combat segment); in phase G its owner announces the raider, unnamed, and the move stops there so each combat unit of the stack may spend 1 CP on `Order::TrySpot`, one die per raider in the hex, a 6 eliminating (27.42–27.44, `cna-game::spot_try`); a unit tries once a stage, only in the hex it was told of, and the try burns no fuel. Only a move ordered in phase G announces: a reaction, a retreat or an advance after combat into a raider's hex does not. The server previews on the board as its side knows it, so a preview never stops or announces where the mover was not told of a raider. Reaction by raiders is the ordinary rule's (27.36); the novelists' +1 (27.65) is out. Raiders need no supply call under the abstract block (27.34, R-148), and a raider is never in contact, so never captured alone in an enemy ZOC (10.29). `Order::RaidDump`: a raider in its movement segment, in or next to an enemy supply unit's hex outside a major city, spends 5 CP (never past CPA 50) to raid it as a supply dump (27.55, 27.56, R-149), once per raider per hex a stage; it goes on the map and may move on. Enemy combat units in the hex guard it: two dice equal to or above their raw close-assault defence get past (the chart, not the text's "above"), else the raider is eliminated; then one die, 1-2 burning 10 % of each kind of supply and, among guards, a 6 eliminating the raider. A dummy is revealed and removed even when the raider dies at the guards. The raider's side sees the dice and the dump's type, not what burned nor the guards' strength. Raids on the pipeline, airfields, aircraft on the ground and truck convoys (27.5x) are left out: under the abstract block water is ignored and the Air and Logistics Games are not built. A raider moves only with other raiders; the enemy sees a raider only when it is formed, spotted or raids, never its moves. The Axis initiative pin of a raid's 2 runs through the game-turn two after the raid, so a mid-turn re-roll in the raid's own turn also reads 3 |
| Patrols (phase L, §16), Mission 5 | with the `patrols` module on (in `Modules::ALL`, so `cna-play new` and the browser open phase L; `--no-patrols` turns it off): phase L opens for the phasing player in a stage where it fired no anti-armour and made no close assault and some recce, light-tank or motorised-infantry unit at cohesion above −8 has an enemy hex within five. `Order::Patrol` checks 16.1x/16.2x (16.13's two points a hex and three a target across all of the phase's patrols) and rolls Patrol Survival, Reconnaissance and Objective Loss on the game's dice. The defender's choices are made by the engine: battalions are described first (never HQ or AA while combat units are present, never an excluded unit), then by id; the stated TOE is exact, not "within two"; the Reconnaissance Table's number counts units described, not battalion-equivalents (a regiment counts one); the TOE lost is the defender's combat units' first, then AA, engineers, HQ, by id. The patroller's losses come off its sources in order. Not yet: fuel and ammunition (none under the abstract block), the path's terrain test (the path only avoids enemy-held and enemy-adjacent hexes bar the target's neighbours), bombed hexes (Air Game), dummy tank formations (16.4) and trucks among the defender's losses. Barrage targets need no identification |
| Rail (phase J, §31) | with the `abstract_supply` module on, the Commonwealth's combat units, HQs and supply units ride, one stack each way a stage (8.7x, 32.3x); a stack may set pieces down and take others on at stops along its path, 2 SP and two supply units holding after each (R-005), the line ends at Mersa Matruh (no construction), no bombing. Module off: skipped |
| The Mediterranean Fleet (phase E, §30, 32.7x), Mission 5 in progress | with the `fleet` module on (not yet in new games: the browser has no phase E panel; `cna-play new --fleet` opts in): each warship of `cw-fleet-schedule.json` is a Commonwealth `PieceKind::Ship` (`ship:cw:<name>`) that arrives in Alexandria in phase E of the OpStage the schedule gives, a later scenario having every ship deployed by its start (The Last Chance's own counts are not applied); a ship has no stacking value or ZOC, holds no hex and never moves in G. Phase E opens for the Commonwealth while a ship may sail; `Order::Station` places one in a sea or coastal hex reached by sea from Alexandria and not west of hex row xx29 on sheet B (R-019), then `Pass`; a ship not placed is in Alexandria that OpStage. At most three OpStages at sea in a row, then two in port for each (the OpStage it comes back counting as the first). Each ship's gun and AA ratings load from the schedule. Not yet: off-shore bombardment (30.2x) and the two stages in Alexandria after firing, strikes on the fleet (32.7x), damage, the half-guns bar, sinking and repair (30.3x), the Valletta set-up options and the Malta shuttle, the chariot raid (30.4x), naval transport (30.5x), the *San Giorgio*, and the scenarios' prose bars on sailing early (Graziani's "no warship moves before Game-Turn 2 OpStage 1", Rommel's Arrival's GT27) |
| The Air Game (§33–47) | left out, as 60.9 allows |
| The Logistics Game (§48–58) | left out, as 60.92 allows; victory's supply clause is R-110's route trace |
| Limited intelligence (3.6x), dummy supply units as hidden information | online, each side sees only `view_for` its side (Mission 4); hot-seat in the browser shows the whole state |

# Online play leaves out (Mission 4)

| What | Instead |
|---|---|
| Spectators | only the two seated players see a game |
| Chat | none; players talk on Discord |
| Clocks, teams, ratings, rematch, undo | none; a game with no order for 60 days is marked abandoned, and either player may leave one |
| Simultaneous sealed answers | the phasing side's sealed answer is taken first; the other side's view never holds it |
| More than one server process | the game registry lives in one process; a second would not see the first's live streams |
