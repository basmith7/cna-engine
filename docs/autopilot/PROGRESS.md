# Autopilot progress

Human-facing journal for the unattended runs (see `AUTOPILOT.md`). The
autopilot rewrites **Status** and **Next steps** at the end of every run and
the cron script appends to **Runs and quota**. The only section a human
writes is **Feedback**.

## Feedback


## Addressed

- 2026-10-29 (Brian) approved the Mission 4 spec, PR #25, with the defaults for all three questions → #25 merged 2026-11-02 (main merged in, the spec's header marked approved, the answers written in); slice 1 under way as PR #92.
- 2026-10-13 (Brian, interactive session) asked for a multiplayer server with real accounts and many games, ahead of the rest of the Land Game turn; chose Discord sign-in, open sign-up and turn alerts by Discord DM → queue row 4, land rules moved to row 5.
- 2026-10-12 (Brian) approved the Mission 2 spec, PR #5, in an interactive session → merged 2026-10-12; that session builds slices 1–6 and holds the autopilot lock while it does.

## Status

**MISSION 5 COMPLETE** (2026-11-06): the rest of the Land Game turn, for
every scenario. Last pieces this run: #145 bumps `vendor/cna` to e9a90b3
and builds R-160 (Rommel's Arrival's unattached motorisation points, which
each side divides among cna's hexes at set-up from the set-up panel;
`unattached_motorisation` module); #147 draws roads and railroads a
scenario builds at set-up as finished; #146 closes the mission's docs
(design Status, README, queue). **Mission 6, the Logistics Game, Part 0:**
spec and plan in PR #148, labelled `needs-brian`, waiting on your review;
cna has since answered both of its requests (R-163, R-166).

**Mission 7, the Air Game, Part 0 (2026-11-06, late night):** spec and
plan in PR #161, labelled `needs-brian`, waiting on your review beside
#148. It builds the Air Game (§33–47) on §32 as §47 changes it, so it does
not wait for the Logistics Game; Air with full logistics is its last
slice. A fresh review's 15 findings are in (the abstract air is switched
off where it actually runs, 32.37's air lift kept, the turn's opening
reordered for strategic planning). Seven requests for cna are posted
below, the biggest being typed starting air facilities and scenario
aircraft keyed to the characteristics chart (only 9 of 82 printed names
match). Also merged this run: #162, a browser control for the
Commonwealth battalion swap (*Swap brigade with…* in the stack panel).
#163 (`armour_limits`: when turn 19, 71 or 92 tightens a Commonwealth
armoured formation's chart row, the units it no longer has room for go
independent) merged after a fresh review's fixes.

**2026-11-07 late morning:** #168 (merged) fixes 19.5's nesting: a
unit of a parent's own structure attached under one of its
sub-formations (Maletti's artillery under its 5th Libyan Regiment) no
longer counts against the parent's Maximum Attachment Chart row. #169
builds the chariot raid (30.4x) as the `chariot_raid` module: in a
campaign game, from July 1941, once, the Axis secretly plans it in phase
C (*Plan chariot raid*), and seven OpStages later the human torpedoes
roll on the Chariot Raid Table against the ships in Alexandria. A fresh
review found no bugs. Three new requests for cna are below; one is a
real gap: Rommel's Arrival's 18th Australian Brigade never enters the
game, since its landing is given only in prose.

**2026-11-07 morning:** cna answered nine requests (seven for the Air
Game, halftracks R-177, historical shifts of parent R-176). #165 (merged)
bumps `vendor/cna` to 4565419; the air rulings (R-165, R-167–R-174) wait
in `NOT_SIMULATED.md` for Mission 7. #166 builds halftracks (14.48,
E-004) as the `halftracks` module (merged): German `j` and Italian `t` battalions
are infantry class, armoured only while their motorisation points carry
the whole unit, doubled when alone under anti-armour fire, at most two
points lost to it a segment, each taking a motorisation point. #167
builds historical shifts of parent (19.24) as `parent_shifts` (merged): in the
month its OA note names, or never (*Shift to ⟨parent⟩* in the stack
panel). Fresh reviews ran on both; their fixes are in.

**2026-11-07 early morning:** #164 (merged) builds 14.23, the "no
armour" rule, as the `no_armour_release` module (on in new games): when
anti-armour fire hits a hex that holds no armour, up to half the points
aimed at it (rounded down) may join a close assault on it. In the
browser, a *No armour found* panel in the close assault step offers
*Release n to assault*. A fresh review found five faults, all fixed: two
leaks of the points aimed (to the other side and to spectators), the step
not opening when no assault was assigned, probes not judged again, and a
gun losing 14.14's vulnerability. Two new requests for cna are below:
halftracks (14.48: German `j` and Italian `t` battalions have armour
protection 1 even when on foot) and dummy tank formations under §32.

**2026-11-06 night:** #159 (merged) builds reassignment (19.2x) as
the `reassignment` module. An eliminated parent's surviving units become
independent. In phase C an independent unit attached to a parent takes a
free slot of its 19.3 chart row (Reassign, in the stack panel), except
the Italian battalions whose parent never reached Africa (19.27). The
Commonwealth swaps battalions between brigades (engine only). The stack
panel also gains Attach to ⟨HQ⟩ for HQs other than the unit's own parent,
which had no browser control. A fresh review found four slot-counting
faults, all fixed. #160 (merged) adds the tests `capture_on_move`
lacked at its reaction and retreat call sites.

**2026-11-06 late evening:** cna answered the 0-TOE request (R-164).
#156 (merged) bumps `vendor/cna` to 111220a and builds it as the `zero_toe`
module: a combat unit with no TOE left (61.38's battalions whose tanks
start broken down) is a non-combat unit until rebuilt: no ZOC, no
holding off, no entering an unoccupied enemy ZOC, no reacting or
escorting, holds no victory hex, and is captured alone in an enemy ZOC.
#157 (merged; `capture_on_move` module) captures any lone
non-combat unit when the enemy moves next to it, not only when the
enemy's next movement segment opens, as Rommel already was (R-138).
Fresh reviews ran on both; no scripted game plays differently. #158
(merged) folds `cna-play new`'s 21 `--no-<module>` flags into
one `--without <module>,…`.

**2026-11-06 evening:** #154 (merged) bumps `vendor/cna` to 9336fcc (cna's
answers to four requests) and builds two of them. R-161: the Tobruk
forts in Rommel's Arrival and the Bardia forts in The Last Chance are a
maximum, so the set-up panel offers Done before they are placed. R-162:
a set-up work whose hex the owner chooses may not stand in an enemy ZOC
or enemy-held hex. If the other side's set-up later puts it in one, the
work comes off and its owner places it again, works only. A fresh review
caught a redeploy loop, now fixed. R-163 and R-166 belong to the
Logistics Game and wait for Mission 6. #155 (merged) brings `NOT_SIMULATED.md`'s
lagging rows up to date (ammunition and fuel, replacements).

**MISSION 4 COMPLETE** (2026-11-03): online play, built and tested
locally, not hosted. **Slice 6, ship, merged as #97:** one image
(`Dockerfile`: the server plus the browser client, 180 MB, non-root,
stops cleanly on SIGTERM), `docker-compose.prod.yml` with Postgres for
Tower, a `v*` tag build of `ghcr.io/basmith7/cna` through the shared
`basmith7/workflows` template, `deploy-now` through `.tower-deploy`
(project `cna`), `deploy/smoke.sh` (the prod compose file with a fake
Discord: health, sign-in, a game created; it passes), README
*Self-hosting*. The outside steps are Brian's checklist in the spec.
Spec merged
(#25, approved 2026-10-29). **Slice 1, per-side knowledge in the
engine, merged as #92 (2026-11-02):** `crates/cna-embed` (the data compiled in, shared
by the browser and the server; `CNA_RULES_COMMIT` for a Docker build),
`cna_game::view` with `view_for` (the state as one side may see it,
3.61), `events_for` (all 73 event kinds classified), `refusal_for`,
keyed supply aliases (a dummy's id no longer gives it away), the
shared `leaks` predicate and a test that replays the golden game and
finds nothing either side may not know; `preview` now refuses to show
events that rolled dice. The hot-seat browser is unchanged (it plays
with open information). Golden hashes now live beside their fixtures
(`tests/games/*.sha256`), written by the regenerate tests. Its
choices ledger is `docs/plans/2026-10-13-m4-choices.md`. **Slice 2,
the server headless, merged as #93 (2026-11-02):** `crates/cna-server`
(axum, sqlx on Postgres 18), health, schema, the game registry,
orders applied in one transaction with snapshots, restart by replay,
old games moved to new rules or marked stale, the HTTP API, the SSE
live channel, the golden game over HTTP hashing to its pin, and a CI
`server` job. A fresh review found no leaks; its fixes are in (a failed
commit no longer wedges a game, seats are checked before a game loads,
the creator is seated with the game, catch-up replays from a
snapshot). **Slice 3, accounts, merged as #94 (2026-11-03):** the fake Discord,
Discord sign-in with PKCE and sessions in Postgres (our own store:
the sqlx store crate lags sqlx 0.9), rate limits (IPv6 by /64) and
CSRF checks, admins and suspension, the fake Discord and server in the
dev compose; a curl sign-in as `alice` works. A fresh review's fixes
are in. **Slice 4, the lobby and the network client, merged as
#95 (2026-11-03):** the lobby API, the browser's
engine behind an `Engine` interface (hot-seat unchanged, its 46
Playwright tests green), a `NetworkEngine` over the API and live
stream, the lobby screens (my games with my move first, open games,
create with an invite link shown once, the alerts switch, an admin's
account list with suspend/restore), the game page `/game/{id}` and
invite links `/join/{id}`, and a two-player Playwright run (alice and
bob sign in through the fake Discord and play the golden game's first
77 orders by clicks over the server, bob's traffic checked for leaks,
the final hash equal to a pinned replay) with Postgres in CI. A fresh review found no leak; its fixes are in. **Slice
5, turn alerts, merged as #96 (2026-11-03):** the guild
join at sign-in, the outbox written with the order (none within 15
minutes or while the player watches the game), the DM sender with
retries (none once the game ends or the move has passed), the
lobby's switch and its "can't reach you" notice, an e2e that bob is
DM'd once and not with alerts off, and the enemy's log lines no
longer print "undefined" where the server withholds a number.

**MISSION 5** (the rest of the Land Game turn, started ahead of Mission 4
on 2026-10-13 while Brian reviews #25) is built for Graziani's Offensive
bar a short list (`docs/designs/2026-10-13-land-turn-design.md`, Status).
Merged through 2026-10-26: supply, spending, supply movement, arrivals,
the air lift, breakdown and repair, attachment, prisoners, guards,
replacements, construction, convoy bombing, gun losses, shells, the
stack panel's organisation, formations in the size table, every
scenario smoke-tested, points victories, the browser's scenario picker
(#31–#64), Rommel as a commander piece and replacement counters at
set-up (#65), 31.0's initiative, recall and morale (#66). **On
2026-10-26, fourth run:** cna answered both requests (R-136, R-137).
#67 bumps `vendor/cna` to fc78330 and places `apart` blocks, so
Rommel's Arrival and both campaigns set up: all nine scenarios now set
up and play a Game-Turn, and the browser offers every one. #68 builds
the rest of 31.0 bar the LRDG raid: the +5 CPA escort (R-137), his
reactions, and his free retreat through enemy ZOC (merged). #69
(merged) lets a side take back a piece during its set-up. **Fifth
run:** cna answered the lone-Rommel request (R-138); #70 (merged) bumps
`vendor/cna` to fcf8ea7 and builds it: the mover's own ZOC no longer
bars his reaction, and alone in an enemy ZOC with no escape left he is
captured. New requests: whose ZOC 8.53 means for every unit, which
19.3 chart components count toward a shell, and Rommel after his stack
is destroyed. #71 (merged) adds the last assignment tests; #72 (merged)
makes the reach search 3x faster under wasm (Mission 3's slow shading).
**Sixth run:** cna answered all three requests (R-139, R-140, R-141).
#73, #74 and #75 are merged. #73 bumps `vendor/cna` to eafa8a3 and builds R-139: a reaction is barred
only by an enemy ZOC the unit stood in before the mover arrived, so the
mover's own ZOC no longer pins. #74 (R-140): Rommel whose stack an
assault destroys retreats alone at once, captured if he stops short of
safety. #75 (R-141): shells are judged against the 19.3 chart's counted
components; in Graziani's Offensive the 2nd NZ Division and the
Trivioli tank regiment now start as shells. **Seventh run
(2026-10-27):** #77 (merged): a combat unit entering a hex of lone
enemy supply captures it (32.1x). #78 (merged): reactions and every retreat pay
fuel (32.22); a unit that cannot pay does not react or retreat before
assault, and a stack ordered to retreat stands and pays (R-120). The
module-on golden game changes with it: 4 RHA has no fuel to react on
GT1 and surrenders with Halfaya. #75's ledger item 1 is real and waits
on a cna request (below). #79 (merged) builds combat units and HQs by rail,
2 SP a run (8.7x), the first task of its plan.
**2026-11-03, morning run:** cna answered both requests (R-142,
R-143–R-145). #98 merged. #99 bumps `vendor/cna` to 442a172 and gives
the four rulings homes (R-145: how many escaped prisoners have rejoined
is their side's alone in a networked view). #100 judges shells slot by
slot from cna's new `fills` (R-142): attached non-assigned units count,
and #75's ledger item 1 is fixed. #101 builds substitution (19.4x) on
the same slots. #102 builds the Axis's convoy lanes beyond Tripoli
(32.63, 56.1x, 55.17), engine and browser. #103 lets a rail run set
pieces down and take others on at stops (R-005), engine and browser.
A fresh review of #100–#102 found four faults, all fixed (a unit
assigned elsewhere could substitute; substitute status could shift; a
cancelled convoy was rerouted; the Bizerta roll changed old logs).
#100–#103 merged (2026-11-03). Graziani's land rules are complete bar
the LRDG raid on Rommel (27.6x). Draft #104 plans it together with the
desert raiders (§27); two cna requests are filed below.

**2026-11-03, third run:** #105 merged: `vendor/cna` to ccc285d and
R-146 (a support group fills the type IV armoured division's infantry
brigade slot). #104 builds the desert raiders (§27) bar forming them:
hidden movement in the enemy's view, spotting, and the LRDG raid on
Rommel, engine and browser, with a Playwright raid by clicks. A fresh
review found a hidden raider's hex leaking three ways; all fixed. With
it Graziani's land rules are complete.

**2026-11-03, fourth run:** cna answered all three requests (R-147
raider counters, R-148/R-149 raiders under §32, the Last Chance's
armoured rosters). #104 (desert raiders) merged. #106 merged: `vendor/cna`
to 8e10e86; Rommel's Arrival now sets up LRDG 1 at Jalo, and the Last
Chance's 1st, 7th and 10th Armoured Divisions set up with their
battalions (brigades placed `under` their roster's division). #107
(merged) builds the rest of §27 for the land game: forming and re-forming raiders
in phase C (a Form button in the stack panel) and raids on supply units
as dumps (R-149, engine only). A fresh review found two faults (a hidden
raider captured in an enemy ZOC; the dump-raid limit resetting mid-stage),
both fixed. #108 (merged) adds a Raid-the-dump button.

**2026-11-03, fifth run:** #109 builds the enemy's spotting try
(27.42–27.44), merged. An Axis unit entering a raider's hex is told a raider is
there, unnamed, and stops; it may spend 1 CP on a die, and a 6
eliminates the raider. Previews online never give the raider away. A
fresh review found two faults, both fixed. #110: patrols. Phase
L had been skipped on the wrong ground that patrols are an Air Game
rule.

**2026-11-03, sixth run:** #110 merged: patrols (phase L, §16), engine
and browser, on for new games; a second fresh review found no server
leak (whether phase L opens says a little about the phasing side's own
units, as phase K's does: accepted). #111 merged: `vendor/cna` to
50e66bf and R-150 (an untrained unit forms no raider; 6th Australian
Cavalry may not). #112 merged: the fleet plan. #113 (merged): fleet Task 1
(ships arrive in Alexandria by the schedule, phase E, `Station`, the
port cycle, R-019's row limit), behind an off-by-default `fleet`
module; its review found ships captured by an Axis ZOC and blocking a
prisoner march, both fixed. #114 (merged): a set-up bug: "west of Mersa
Matruh" compared hex rows, not columns, so CW supply could set up south
of Matruh; fixed, golden game re-pinned. #115 (merged): the patrol panel's skip
list (16.53).

**2026-11-03, seventh run:** cna answered the warship-ratings request
(#286, R-151). #116 (merged) bumps `vendor/cna` to b142151: each ship's
gun and AA ratings load, and R-151 (the *San Giorgio*) is a
NOT_SIMULATED row until it is placed. #117 (merged): fleet Task 2,
off-shore bombardment (30.2x): a stationed ship fires its guns in a
barrage step, BB/CA also into an adjacent hex at half, then two stages
in Alexandria. A fresh review found ships paying combat CP and the
step's opening reading hidden TOE; both fixed. #118 (merged): fleet
Task 3, strikes on the fleet (32.7x), damage, the half-guns bar,
sinking and repair; its review found the Commonwealth's preview
showing a strike to come, fixed. #119 (merged): fleet Task 4, the browser's
phase E panel, ships on the map and offered in the barrage step, a
Playwright spec by clicks. A fresh look at its screenshots found
display faults; #120 (ready, CI running) fixes three (the hex panel
lists a ship, its label sits in its hex, its points read as fixed).
The fleet stays off in new games until the scripted games and
Playwright drivers are re-pinned with phase E open.

**2026-11-03, eighth run:** cna answered the captured-equipment
request (#287, R-152). #121 (merged) bumps `vendor/cna` to 70239b1.
#122 (merged) builds R-152 for surrenders behind a new `captured_equipment`
module (on in new games): a surrendering gun, tank or armoured-recce
unit's points become the captor's replacement points in its hex, by
class, absorbed in phase C by a unit of that type; the browser logs the
capture and offers the absorb. #123 (merged) splits 15.89's captured
share the same way and stops capturing vulnerability losses; a fresh
review's four points are fixed there (spoils by unit type, so an HQ's
and an infantry unit's guns are prisoners; split assaults counted once).
#124 (draft): the *San Giorgio* (R-151), built by the previous run
but left unpushed; behind the fleet module, so off in new games. A fresh
review's retreat bug is fixed; its ammunition and removal points are
open on the PR.

**2026-11-03, ninth run:** #124 merged: the *San Giorgio* (R-151), a
fixed Axis wreck in Tobruk that barrages an adjacent hex and is removed
by a Commonwealth engineer; its ammunition waits on cna (below). #125
merged: the fleet is on in new games (`cna-play new --no-fleet` turns it
off), no ship sailing before its scenario's `first_sail` (cna #288);
CI caught the browser throwing on the Axis's first strike on GT1 (no
log lines for the strike events), fixed there. #126 merged: facility repair of broken-down vehicles in
phase K (22.3x) at the scenario's repair facilities (R-099), engine
and browser. #127 (draft): towing them toward a facility (21.6x), engine
only so far; destroyed tanks are planned after it.

**2026-11-06 run:** #127 (towing, 21.6x, with a browser tow panel)
and #128 (destroyed tanks: markers, capture by a battalion, repair at a
facility, 14.5x, 22.4x, R-014) merged; a fresh review of #128 found tank
losses counted twice with captured equipment on (marker and the
captor's share), fixed there. #129 bumps `vendor/cna` to 4cedfb7 for
R-153 (the *San Giorgio* needs no ammunition, as the engine already
played it). #130: The Long Retreat now sets up (a child scenario's
`sides` replaced its parent's, so it had no units); the scenario smoke
test now checks both sides have units. #131 and #132 (merged):
Operation Crusader's railway starts at its railhead (C3430), and
Crusader and The Last Chance start with their forts round Tobruk and
Bardia, behind two new modules (`rail_ends`, `set_up_works`, on in new
games). #133 (merged): Group One's first two Axis convoys are
planned before play, the first unbombed (R-097, 60.37).

**2026-11-06, second run:** cna answered both requests (#290, #291).
All merged: #134 bumps `vendor/cna` to fff7856 and gives R-154–R-159
homes; #135, tanks that start broken down (Crusader's Alexandria
shops, the Last Chance's Tobruk, repair held until the printed
OpStage); #136, Commonwealth withdrawals (R-154–R-157: who goes, the
75 % test, motorisation under §32, returns; the browser has no pick
control yet, so the engine picks); #137, the campaign games' victory
levels by ratio (64.76); #138, Rommel's Arrival's release limit out of
Cairo and Alexandria (61.38, R-159). A fresh review of #138 found
three faults (a free HQ carried its held units out, rail boarders went
unchecked, moves inside the cities were refused), all fixed there.
Each lands behind its own module, on in new games.

**2026-11-06, third and fourth runs:** the third run built three
drafts and was killed; the fourth recovered them. Merged: #140, the
campaigns' automatic wins (64.71, 64.72) and the end-of-game supply test
(64.73), behind `campaign_victory`; #143, its fresh review's two fixes
(the Commonwealth could read Axis supply loads from the hold count; a
unit with no CPA, Tobruk's coastal guns, failed 64.73 on a loaded supply
unit in its own hex); #141, forts and minefields a player places at
set-up (Rommel's Arrival 61.6, The Last Chance 63.76–63.77) with a
set-up panel control, dummy fields secret, behind `placed_works`;
#144, no set-up fort in a major city (#141's review: Tobruk and Bardia
lost a level); #139, the 7th Armoured's tanks starting broken down with
their units (61.38), behind `tanks_with_units`, its review's fixes in
(the 75 % release test is per battalion; points held for repair keep
their room under the TOE ceiling); #142, `vendor/cna` to
e5479f0 and roads and railroads built at set-up (cna #292), behind
`roads_built`; The Last Chance's railway now runs from its cut west of
El Alamein.

Mission 5 is the current mission again. **#80 merged (2026-11-03):** rail runs one stack each way a stage, attached units ride with their parent, and phase J opens for a Commonwealth unit alone; a Playwright test rails a battalion by clicks; no golden re-pin was needed (supply already opened phase J wherever units could ride).

**MISSION 4, Part 0** (2026-10-13, merged 2026-11-02): the spec
`docs/designs/2026-10-13-multiplayer-server-design.md` and six slice
plans (#25).

**MISSION 3 COMPLETE** (2026-10-13): a minimal browser UI for Graziani's
Offensive in `web/` on `cna-game` compiled to WebAssembly (`crates/cna-web`,
the `vendor/cna` data built in). Slices 1–6 merged as PRs #18–#23: the
engine in the browser, the map and counters, set-up, movement, combat
and the end, then save/load/autosave with the golden game played through
the UI by Playwright (`web/tests/golden.spec.ts`: 446 clicks' worth of
orders, a save, a reload and a load in the middle, the final state
byte-equal to `replay`). The spec is
`docs/designs/2026-10-13-minimal-ui-design.md` (Status: built). Local
only, nothing hosted; secrecy is hot-seat only (README).

**MISSION 2 COMPLETE** (2026-10-13): a headless game state for Graziani's
Offensive in `cna-game`, driven by `cna-play`. Slices 1–6 merged as PRs
#6, #8, #9, #11, #12 and #15: map, scenario and pieces, state and the
sequence of play, movement, combat, victory. Two scripted games in
`tests/games/` replay in CI; the short game's final state and victory
report are pinned. The spec is `docs/designs/2026-10-08-game-state-design.md`
(Status: built).

PR #12's review follow-ups are all closed: 15.84/14.14 gun losses (#54),
18.2x's released units (#53 and earlier), 10.36's must-attack Pass (#27),
the rule tests (15.36, 6.26, 13.1, 12.44 and 12.1x already had them;
#71, merged, adds 15.1x and 13.28).

**MISSION 1 COMPLETE** (2026-09-26): rules core with switches for R-001,
R-009, R-011, R-012, R-015, R-018 and eight probes in `probes/` (PRs #1–#4).

## Next steps

- **Review the Mission 7 spec: PR #161** (the Air Game, §33–47, as one module on §32 as §47 changes it; independent of #148). Three questions, each with a default: Air Game the default for new games (no: a tick-box); build it before the Logistics Game (yes); the engine forms squadrons at set-up, changeable (yes).
- **Review the Mission 6 spec: PR #148** (the Logistics Game, §48–58, as
  one module). It asks you three questions, each with a default: whether
  the Logistics Game becomes the default for new games (no: a tick-box);
  whether the browser picks fuel and ammunition sources (yes, shown and
  changeable); whether to build variants V-002 and V-004 (no). Nothing is
  built for it until your **Feedback** approves it.
- One call in #169 to glance at: the chariot raid strikes as phase E
  closes (there is no land support air phase under §32), before that
  stage's strike, and the engine lays its damage as it lays a strike's.
- Calls to glance at in #166 and #167: the halftrack two-point cap
  counts per side across a whole anti-armour segment (not per hex), and
  the engine picks the points lost; a shift to no parent keeps the unit
  attached only where the old parent's 19.5 row still takes it.
- One call in #164 to glance at: a hex whose only armour is withheld
  from combat counts as holding armour, so it frees no anti-armour
  points (the reading closest to "the hex holds none").
- Two calls in #163 to glance at: reinforcements and set-ups that bring
  an armoured formation more than its current row are not trimmed (the
  OA stands, so The Last Chance's armoured divisions keep two brigades),
  and the engine, not the Commonwealth player, picks which unit leaves.
- Mission 5 follow-ups, this run (2026-11-06 afternoon): #150 (merged)
  an assault eliminates a 0-TOE battalion left alone in its hex (19.62,
  3.5, 15.55; asked of cna below); #149 (merged) a repair can name which
  unit's wreck it credits, and the wreck list shows owners; #152 (merged)
  `view_for` at The Last Chance's set-up 212 ms → 13 ms, views unchanged.
  #151 (Mission 3's polish residue: hex labels, tray counters, badge,
  "More below" cue, end dialog at 390 px) merged; a fresh
  critique's fixes are in. #153 (merged): `cargo fmt` over the
  workspace and `cargo fmt --all --check` in CI. Bardia's forts and
  set-up works vs enemy ZOC: answered by cna and built in #154.
- `vendor/cna` is at cna 4565419 (#165).
- 10.29 under `capture_on_move`: "alone" means with no friendly combat
  unit, so Rommel or an HQ with TOE does not shield a supply unit (as at
  a segment's start before). Its call sites are tested since #160.
- New games: `cna-play new --without <module>,…` replaces the old
  `--no-<module>` flags (#158).
- Reassignment (#159), two calls to glance at: the battalion swap needs
  both battalions standing alone in phase C (my reading of "at will");
  an orphaned unit may take a slot under another nation's parent (19.2x
  does not forbid it). The swap has a browser control since #162 and the
  armour limits' changes are #163; historical shifts of parent wait on
  cna (requested below).

- **Ship Mission 4: Brian's checklist in the Mission 4 spec**
  (`docs/designs/2026-10-13-multiplayer-server-design.md`, *Brian's
  checklist*): the Discord application and bot, the first tag, the GHCR
  package made public, the `cna` project on Tower, Pangolin, backups.
  The tag build has not run yet (no tag is pushed unattended).
- #100 changes starting shells outside Graziani's (its body lists them):
  the 15th Panzer is whole in The Last Chance now, and the 7th Support
  Group, the 21st Panzer and the 1st and 7th Armoured are shells. R-146
  (#105) lets their support groups fill the turn-92 infantry brigade
  slot, and since #106 the divisions set up with their battalions
  (cna #284).
- Two turn-alert calls to glance at (#96's body): a second handover
  within 15 minutes of the last alert sends nothing, as the plan says,
  so a quick reply can leave a player untold (delaying it instead is a
  small change); and the design says turning alerts off leaves the
  Discord server, while the plan and code leave that to the player.
  #95's review left a follow-up: the two-player e2e judges each
  message's leaks against the state at check time, not send time.
  `CNA_TRUST_PROXY=1` trusts `X-Forwarded-For` from anyone reaching
  port 3240 on the LAN or tailnet; it only picks a rate-limit bucket.
- The online game page reuses Mission 3's panels; a fresh critique of
  its screenshots found Mission 3's own residue (map names over hex
  numbers, the sidebar's counter grids clipped, small tray counters),
  for the polish slice below.
- One provisional call to glance at, ledger entry 1.3 in
  `docs/plans/2026-10-13-m4-choices.md`: a game with open information
  still hides the owner's secrets the hot-seat view hid (loads,
  motorisation, minefields, project items). 
- Slice 1 took stricter readings of 3.61 where the code had more than
  the spec named (#92's body lists them): an enemy's losses,
  disorganisation and combat CP are not shown to the other side at all;
  barrage points against a hex are (12.0), anti-armour numbers are not.
  POW and guard numbers are now their owner's, though `GameState`'s
  docs called them open information.
- Mission 3's accepted residue, for a polish slice when it matters: the
  count badge touches the top counter's CPA; frontier stacks overlap at
  the fitted view; the 390 px layout is the slice 2 one (mobile is a
  non-goal); ticks are offered in a sealed step though nothing moves;
  at 390 px
  the end dialog's table clips its right-hand columns (it scrolls).
- Mission 3's slow shading is closed: after #72, an uncapped motorised
  stack (11 Hussars, 3,614 hexes) paints in ~200 ms from pointer-down
  (198–227 ms, timed in the page in a Playwright test build; the engine
  is ~141 ms of it), down from ~0.6 s. Playwright's own click-to-text
  wait reads ~0.5 s because of its selector polling, not the app.
- Look at #75's two new starting shells (2nd NZ Division, the Trivioli
  tank regiment at Bardia): they follow the OA and the 19.3 chart, but
  change Graziani's opening.
- PR #23's review follow-ups are all merged: hostile saves refused
  (#26), the curtain on Load and Continue (#28), and 10.36's must-attack
  Pass (#27); #30 fixed the flaky wheel-zoom budget in `map.spec.ts`.
- Since Mission 2: #13, #14 (review follow-ups), #16 (14.34 anti-armour
  bans), #17 (seam `up`, R-117, R-118, R-119) merged; `vendor/cna` is at
  cna fcf8ea7.

## Requests for cna

What this repo needs from cna, one bullet each: what, why, which mission it
blocks. cna's autopilot reads this every run and answers with a PR; delete
the bullet once it lands and `vendor/cna` is bumped.

- **Dummy tank formations under §32** (16.4; Mission 5 follow-up, blocks building 16.4). 16.41 makes one in the construction segment from 10 stores in the hex, but abstract supply (§32) has no stores, only ammunition and fuel loads. Needed: what a dummy costs and where it is paid from with abstract supply, or a ruling that it is left out without the Logistics Game.
- **Type machine-gun and heavy-weapons infantry units in `data/oa/`** (19.5; Mission 5 follow-up, blocks the chart's infantry limit exception). `max-attachments.json`'s note says the chart's "infantry" limit excludes machine-gun and heavy-weapons units, but nothing in the data marks them: the characteristics chart's `unit_type` is just "Infantry Bn-Eq", and `fills` is the chart slot, not the unit's type (the German 2nd and 8th Machinegewehr Bns fill `infantry-battalion`; the 6th Rajputana Rifles fills `machinegun-battalion`, the Kopanski MG Coy only `unit`). Needed: a per-unit field (say `infantry_type: "machine-gun" | "heavy-weapons"`) or a list of id codes per nation.
- **A substitute when the unit it stands for arrives** (19.4x substitution; Mission 5 follow-up). An independent unit attached in place of a missing assigned unit stands in its slot; the rules do not say what happens when the missing unit later arrives or is rebuilt and attaches. The engine keeps the substitute as a substitute, so the parent can then carry more than its 19.5 row. Needed: a ruling (the substitute becomes an extra and must fit the row or detach; the arriving unit may not attach while its slot is filled; or it stands).
- **Rommel's Arrival: the 18th Australian Brigade's landing as data** (61.38; Mission 5 follow-up, a missing reinforcement). The scenario says the brigade lands in Alexandria on Game-Turn 27 OpStage 1 and moves from the next stage, but only in prose (`sides.cw.reinforcements[2]`); its row in `reinforcement-schedule.json` is Game-Turn 14 OpStage 3, before the scenario starts, and no set-up block places it, so the engine never brings it on. Needed: a structured per-scenario arrival (unit, Game-Turn and OpStage, place `alexandria`), which the engine would also need for any other scenario whose landings differ from the schedule.

## Runs and quota

| Started | Minutes | Weekly before→after | 5h before→after | Cost | Turns | Result |
|---|---|---|---|---|---|---|
| 2026-09-26 10:53 | 26 | 55→55 | 3→7 | $8.30 | 129 | Mission 1 is complete: all four slices are merged into `main` (PRs #1–#4), and I logged `MISSION 1 COMPLETE` in `PROGRESS.md` and the journal. On `main`, clippy |
| 2026-09-26 13:53 | 0 | 56→56 | 2→2 | $0.17 | 2 | Nothing to do this run: Mission 1 is complete and merged, and the standing orders say to do nothing further after that. - **Feedback:** the section in `PROGRESS |
| 2026-09-26 16:53 | 0 | 59→59 | 21→21 | $0.17 | 2 | I did nothing this run. Mission 1 is complete and merged (PRs #1–#4), and the standing orders say to stop once that happens. The **Feedback** section in `PROGRE |
| 2026-09-26 19:53 | 0 | 60→60 | 1→1 | $0.17 | 2 | I did nothing this run: Mission 1 is complete (PRs #1–#4 merged), and the standing orders say to do nothing more after that. The **Feedback** section in `PROGRE |
| 2026-09-26 22:53 | 0 | 62→62 | 15→15 | $0.17 | 2 | I did nothing this run. Mission 1 is complete: all four slices are merged (PRs #1–#4), and the standing orders say to stop once that happens. - **Feedback:** th |
| 2026-09-27 01:53 | 0 | 63→63 | 4→4 | $0.17 | 2 | I did nothing this run. Mission 1 is complete (PRs #1–#4 merged), and the standing orders say to do nothing more after that. The Feedback section in `PROGRESS.m |
| 2026-09-27 04:53 | 0 | 63→63 | 0→0 | $0.17 | 2 | I did nothing this run. Mission 1 is complete: all four slices are merged (PRs #1–#4), and the standing orders say to stop once that happens. The **Feedback** s |
| 2026-09-27 10:53 | 0 | 67→67 | 4→4 | $0.17 | 2 | I did nothing this run. Mission 1 is complete (PRs #1–#4 are merged), and the standing orders say to do nothing more after that. The **Feedback** section in `PR |
| 2026-09-27 13:53 | 0 | 70→70 | 18→18 | $0.17 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to stop once that happens. -  |
| 2026-09-27 16:53 | 0 | 70→70 | 0→0 | $0.17 | 2 | I did nothing this run: Mission 1 is complete (PRs #1–#4 merged), and the standing orders say to do nothing more after that. - **Feedback:** the section in `PRO |
| 2026-09-27 19:53 | 0 | 70→70 | 2→2 | $0.17 | 2 | I did nothing this run: Mission 1 is already complete, and the standing orders say to do nothing more after that. All four slices are merged (PRs #1–#4), there  |
| 2026-09-27 22:53 | 0 | 71→71 | 3→3 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete, and the standing orders say to stop once that happens. - **Feedback:** the Feedback section in `PROGRESS. |
| 2026-09-28 01:53 | 0 | 72→72 | 0→0 | $0.18 | 2 | I did nothing this run. Mission 1 is already complete: all four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after that. - * |
| 2026-09-28 04:53 | 0 | 72→72 | 0→0 | $0.18 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after t |
| 2026-09-28 07:53 | 0 | 73→73 | 8→8 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete, and the standing orders say to do nothing further after that. All four slices are merged (PRs #1–#4). - * |
| 2026-09-28 10:53 | 0 | 75→75 | 24→24 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-28 13:53 | 0 | 76→76 | 2→3 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-28 16:53 | 0 | 76→76 | 0→0 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-28 19:53 | 0 | 78→78 | 11→11 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-28 22:53 | 0 | 78→78 | 2→2 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-29 01:53 | 0 | 79→79 | 8→8 | $0.18 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-29 04:53 | 0 | 79→79 | 0→0 | $0.19 | 2 | Nothing to do this run. Mission 1 is already complete: all four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after that. The |
| 2026-09-29 07:53 | 0 | 79→79 | 0→0 | $0.18 | 2 | I did nothing this run. Mission 1 is already complete (PRs #1–#4 are merged), and the standing orders say to do nothing more after that. The Feedback section in |
| 2026-09-29 10:53 | 0 | 82→? | 22→? | $0.18 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after t |
| 2026-09-29 13:53 | 0 | 84→84 | 2→2 | $0.19 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. - **Feedback:** the sect |
| 2026-09-29 16:53 | 0 | 5→5 | 18→19 | $0.19 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The **Feedback** section |
| 2026-09-29 19:53 | 0 | 8→8 | 55→55 | $0.19 | 2 | I did nothing this run: Mission 1 is already complete (PRs #1–#4 merged), and the standing orders say to do nothing further after that. The Feedback section in  |
| 2026-09-30 07:53 | 0 | 22→22 | 8→9 | $0.19 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged and `PROGRESS.md` already records `MISSION 1 COMPLETE`. The standing orders say to d |
| 2026-09-30 10:53 | 0 | 24→24 | 33→33 | $0.19 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and the standing orders say to stop once that happens. The Feedback section |
| 2026-09-30 13:53 | 0 | 25→25 | 5→5 | $0.19 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged and `PROGRESS.md` already records `MISSION 1 COMPLETE`. The standing orders say to d |
| 2026-09-30 16:53 | 0 | 28→28 | 39→39 | $0.19 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say to do  |
| 2026-09-30 19:53 | 0 | 4→4 | 4→4 | $0.18 | 2 | Nothing to do this run. Mission 1 finished on 2026-09-26: all four slices are merged as PRs #1–#4, and the standing orders say to do nothing further after that. |
| 2026-09-30 22:53 | 0 | 5→5 | 6→6 | $0.19 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to do nothing further after t |
| 2026-10-01 01:53 | 0 | 5→5 | 0→0 | $0.19 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged as PRs #1–#4, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The |
| 2026-10-01 04:53 | 0 | 5→5 | 0→0 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged as PRs #1–#4, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The |
| 2026-10-01 07:53 | 0 | 5→5 | 0→0 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 10:53 | 0 | 6→6 | 6→6 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 13:53 | 0 | 10→10 | 1→1 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 16:53 | 0 | 16→16 | 37→37 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 19:53 | 0 | 18→18 | 11→11 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-01 22:53 | 0 | 20→20 | 0→0 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders sa |
| 2026-10-02 01:53 | 0 | 21→21 | 6→6 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-02 04:53 | 0 | 23→23 | 13→13 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-02 07:53 | 0 | 23→23 | 14→14 | $0.20 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-02 10:53 | 0 | 24→24 | 4→4 | $0.18 | 2 | Mission 1 is already complete, so I did nothing this run, as the standing orders require. - **Feedback:** the Feedback section in `PROGRESS.md` is empty, so the |
| 2026-10-02 13:53 | 0 | 24→24 | 0→0 | $0.20 | 2 | I did nothing this run. Mission 1 finished on 2026-09-26 (PRs #1–#4 are merged), and the standing orders say to do nothing more once it is complete. The Feedbac |
| 2026-10-02 16:53 | 0 | 25→25 | 6→6 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders sa |
| 2026-10-02 19:53 | 0 | 26→26 | 4→4 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-02 22:53 | 0 | 28→28 | 13→13 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-03 01:53 | 0 | 28→28 | 3→3 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders sa |
| 2026-10-03 04:53 | 0 | 28→28 | 3→3 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 07:53 | 0 | 28→28 | 0→0 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders |
| 2026-10-03 10:53 | 0 | 29→29 | 2→2 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 13:53 | 0 | 33→33 | 25→25 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 16:53 | 0 | 38→38 | 2→2 | $0.21 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 19:53 | 0 | 43→43 | 31→31 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-03 22:53 | 0 | 53→53 | 31→31 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-04 01:53 | 0 | 55→55 | 43→43 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-04 04:53 | 0 | 55→55 | 0→0 | $0.23 | 2 | Nothing to do this run. Mission 1 is already complete: PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders say to do  |
| 2026-10-04 07:53 | 0 | 55→55 | 0→0 | $0.22 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders say to do  |
| 2026-10-04 10:53 | 0 | 63→63 | 51→51 | $0.22 | 2 | I did nothing this run. Mission 1 is already complete: PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say to do n |
| 2026-10-04 13:53 | 0 | 67→67 | 0→0 | $0.19 | 2 | Feedback is empty, no open autopilot PRs, and Mission 1 is already complete, so I did nothing this run. The standing orders say to stop once all four parts are  |
| 2026-10-04 16:53 | 0 | 74→74 | 40→40 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, the Feedback section is em |
| 2026-10-04 19:53 | 0 | 74→74 | 0→0 | $0.18 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and the standing orders say to stop once that's done. - * |
| 2026-10-04 22:53 | 0 | 76→76 | 17→17 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 01:53 | 0 | 3→3 | 30→30 | $0.24 | 2 | Mission 1 is already done, so this run did nothing. All four slices are merged (PRs #1–#4), and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing or |
| 2026-10-05 04:53 | 0 | 3→3 | 33→33 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 07:53 | 0 | 3→3 | 0→1 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 10:53 | 0 | 90→90 | 7→7 | $0.24 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 13:53 | 0 | 14→14 | 60→61 | $0.24 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-05 16:53 | 0 | 15→15 | 0→0 | $0.24 | 2 | Nothing to do this run. Mission 1 is already complete: PRs #1–#4 are merged, and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say to do  |
| 2026-10-05 19:53 | 0 | 93→93 | 11→11 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The standing orders say |
| 2026-10-05 22:53 | 0 | 24→24 | 14→14 | $0.24 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged, `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the standing orders sa |
| 2026-10-06 01:53 | 0 | 98→98 | 0→1 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, so the standing orders  |
| 2026-10-06 04:53 | 0 | 31→31 | 0→0 | $0.25 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, which the standing orde |
| 2026-10-06 07:53 | 0 | 37→37 | 100→100 | $0.00 | 1 | You've hit your session limit · resets 8:20am (America/Phoenix) |
| 2026-10-06 13:53 | 0 | 43→43 | 100→100 | $0.00 | 1 | You've hit your session limit · resets 2:10pm (America/Phoenix) |
| 2026-10-06 16:53 | 0 | 1→1 | 4→4 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. PRs #1–#4 are merged and `PROGRESS.md` already says `MISSION 1 COMPLETE`, so the standing orders  |
| 2026-10-06 19:53 | 0 | 51→51 | 0→0 | $0.23 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), and `PROGRESS.md` already says `MISSION 1 COMPLETE`. The  |
| 2026-10-06 22:53 | 0 | 55→55 | 49→49 | $0.22 | 2 | I did nothing this run, because Mission 1 is already complete. All four slices are merged (PRs #1–#4), `PROGRESS.md` already says `MISSION 1 COMPLETE`, and the  |
| 2026-10-08 13:53 | 12 | 37→37 | 0→3 | $6.23 | 7 | The Mission 2 spec and its six slice plans are written and up for your review in **PR #5**, which is marked ready and labelled `needs-brian`. Per the standing o |
| 2026-10-08 19:53 | 3 | 40→40 | 0→1 | $1.60 | 45 | PR #5 (the Mission 2 spec) is updated with cna's answers to all six of our requests, and it is still waiting for your review. Nothing for Mission 2 has been bui |
| 2026-10-08 22:53 | 0 | 42→42 | 8→8 | $0.43 | 10 | Nothing was built this run. The Mission 2 spec (PR #5) is still waiting on your review, and the standing orders don't allow building until you approve it under  |
| 2026-10-09 01:53 | 1 | 43→43 | 8→8 | $0.43 | 11 | PR #5's CI was failing, and it's green again. Nothing else was built, because Mission 2 is still waiting on your review of the spec. - **Cause:** the previous r |
| 2026-10-09 04:53 | 0 | 43→43 | 0→0 | $0.32 | 6 | Nothing was built this run. Mission 2 is still waiting for your review of PR #5, and the standing orders don't let me build anything until you approve it under  |
| 2026-10-09 07:53 | 0 | 44→44 | 0→1 | $0.31 | 6 | Nothing was built this run. Mission 2 is still waiting on your review of the spec in PR #5, and the standing orders don't let me build anything for it until you |
| 2026-10-09 10:53 | 0 | 44→44 | 0→0 | $0.27 | 4 | I built nothing this run because PR #5, the Mission 2 spec, is still waiting on your review. The standing orders don't let me build anything for Mission 2 until |
| 2026-10-09 13:53 | 0 | 45→45 | 5→5 | $0.29 | 5 | Nothing was built this run. The Mission 2 spec in PR #5 is still waiting on your review, and the standing orders don't let me build anything for Mission 2 until |
| 2026-10-09 16:53 | 0 | 47→47 | 7→7 | $0.29 | 5 | Nothing was built this run. The Mission 2 spec (PR #5) is still waiting on your review, and the standing orders don't allow any Mission 2 work until you approve |
| 2026-10-09 19:53 | 0 | 47→47 | 7→8 | $0.29 | 5 | I didn't build anything this run: the Mission 2 spec (PR #5) is still waiting for your review, and nothing for Mission 2 can start until you approve it under ** |
| 2026-10-09 22:53 | 0 | 48→48 | 7→7 | $0.29 | 5 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting on your review, and the standing orders say nothing for Mission 2 gets built until you app |
| 2026-10-10 01:53 | 0 | 49→49 | 0→0 | $0.28 | 4 | Nothing got built this run. The Mission 2 spec in PR #5 is still waiting on your review, and the standing orders don't let any Mission 2 work start until you ap |
| 2026-10-10 04:53 | 0 | 49→49 | 0→0 | $0.28 | 4 | Nothing was built this run: the Mission 2 spec (PR #5) is still waiting for your review. The standing orders say nothing for Mission 2 gets built until you appr |
| 2026-10-10 07:53 | 0 | 49→49 | 0→0 | $0.29 | 5 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting for your review, and the standing orders don't let me build anything for Mission 2 until y |
| 2026-10-10 10:53 | 0 | 50→50 | 4→4 | $0.28 | 4 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting for your review, and the standing orders say nothing for Mission 2 gets built until you ap |
| 2026-10-10 13:53 | 0 | 50→50 | 0→0 | $0.28 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-10 16:53 | 0 | 51→51 | 4→5 | $0.30 | 5 | Nothing got built this run. The Mission 2 spec (PR #5) is still waiting for your review. The standing orders say no Mission 2 work starts until you approve it u |
| 2026-10-10 19:53 | 0 | 53→53 | 3→3 | $0.30 | 5 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-10 22:53 | 0 | 54→54 | 11→11 | $0.28 | 4 | I didn't build anything this run. The Mission 2 spec in PR #5 is still waiting for your review, and the standing orders don't allow any Mission 2 work until you |
| 2026-10-11 01:53 | 0 | 56→? | 10→? | $0.31 | 5 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 04:53 | 0 | 57→? | 1→? | $0.29 | 4 | I didn't build anything this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until y |
| 2026-10-11 07:53 | 0 | 57→? | 2→? | $0.24 | 5 | I built nothing this run, because Mission 2 is still waiting for your review of PR #5. The standing orders say to build nothing for that mission until you appro |
| 2026-10-11 10:53 | 0 | 57→57 | 2→2 | $0.28 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 13:53 | 0 | 58→58 | 6→7 | $0.29 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 19:53 | 0 | 58→? | 2→? | $0.30 | 5 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting for your review. The standing orders don't allow any Mission 2 work until you appro |
| 2026-10-11 22:53 | 0 | 61→61 | 18→18 | $0.31 | 4 | I built nothing this run because the Mission 2 spec (PR #5) is still waiting on your review. The standing orders don't allow any Mission 2 work until you approv |
| 2026-10-12 01:53 | 0 | 61→? | 0→? | $0.29 | 5 | I built nothing this run. The Mission 2 spec (PR #5) still has no approval under **Feedback**, and the standing orders say nothing for Mission 2 gets built unti |
| 2026-10-12 04:53 | 0 | 61→? | 0→? | $0.27 | 4 | Brian, I built nothing this run. Mission 2 is still waiting on your review of its spec, PR #5, and the standing orders don't allow building it until you approve |
| 2026-10-12 07:53 | 0 | 63→? | 11→? | $0.25 | 5 | I didn't build anything this run. Mission 2 is still waiting for your review of the spec in PR #5, and the standing orders say nothing gets built until you appr |
| 2026-10-12 10:53 | 0 | 64→64 | 22→22 | $0.35 | 6 | Brian, nothing got built this run. The Mission 2 spec in PR #5 still has no approval under **Feedback**, and the standing orders don't allow any Mission 2 work  |
| 2026-10-12 13:53 | 0 | 65→65 | 3→3 | $0.32 | 4 | I built nothing this run. The Mission 2 spec (PR #5) is still waiting for your review, and the standing orders don't allow any Mission 2 work until you approve  |
| 2026-10-12 16:53 | 0 | 66→? | 0→? | $0.31 | 4 | I built nothing this run: the Mission 2 spec (PR #5) still needs your review, and the standing orders don't allow any Mission 2 work until you approve it under  |
| 2026-10-13 08:53 | 29 | 1→4 | 10→68 | $4.03 | 3 | Brian, the Mission 4 spec is up as **PR #25**, labelled `needs-brian`, and nothing for Mission 4 will be built until you approve it under **Feedback**. **What's |
| 2026-10-13 12:53 | 11 | 6→7 | 93→100 | $2.51 | 1 | You've hit your session limit · resets 1:30pm (America/Phoenix) |
| 2026-10-24 08:53 | 67 | 20→21 | 0→4 | $15.17 | 246 | Mission 5 slice 2 is merged (PR #32). Slice 3 (supply movement) is about half built on draft PR #34. Everything is pushed and the gates pass locally: clippy, `c |
| 2026-10-24 10:53 | 75 | 21→22 | 4→11 | $18.97 | 3 | Rule 21.41 is done and pushed on PR #35 (fd815bb). If a unit's move began next to an enemy brigade-sized unit, or two hexes from one across an open hex, half of |
| 2026-10-25 16:53 | 69 | 27→29 | 0→10 | $16.44 | 3 | All three steps were already done in the last turn, about 20 minutes before the 01:23Z deadline: - **Playwright on #37:** `/tmp/pw-s4.txt` shows 33 passed (8.7m |
| 2026-10-25 20:53 | 71 | 30→31 | 18→2 | $13.68 | 2 | Both CI runs for #44's last commit have now finished green (test and web in each). That covers the duplicate run that was still pending when I merged, so the me |
| 2026-10-25 22:53 | 66 | 31→33 | 3→13 | $12.34 | 197 | Two Mission 5 slices merged this run, #45 and #46. The third, slice 8, is planned on draft PR #47 but nothing is built for it yet. - **#45, slice 6b (prisoners, |
| 2026-10-26 00:53 | 82 | 33→34 | 15→18 | $13.53 | 209 | Four PRs merged this run: slice 8 (prisoner guards and escape) and all three parts of slice 9 (fortifications and minefields). Mission 5 keeps moving while Miss |
| 2026-10-26 02:53 | 85 | 34→35 | 0→7 | $14.67 | 201 | Ten PRs merged this run (#51–#58, #60). Main passes the full Rust suite after the last merge. The only open autopilot PR besides the Mission 4 spec is draft #59 |
| 2026-10-26 04:53 | 82 | 35→35 | 7→12 | $9.19 | 157 | I merged five PRs this run (#59, #61, #62, #63, #64) and nothing is open at handoff. On the merged `main`, clippy and 511 tests passed locally; #62's later chan |
| 2026-10-26 06:53 | 68 | 35→? | 12→? | $9.88 | 4 | The Last Chance now sets up and plays in the engine and the browser, which makes six of the nine scenarios playable. Rommel is in the game with his initiative,  |
| 2026-10-26 09:53 | 74 | 36→37 | 4→11 | $10.94 | 3 | This run merged three Mission 5 PRs (#67, #68, #69). All nine scenarios now set up and play a Game-Turn, and the browser offers every one of them. - **#67:** cn |
| 2026-10-26 11:53 | 61 | 38→39 | 16→5 | $11.62 | 206 | I merged three PRs this run: the new Rommel ruling, two missing rule tests, and a 3× faster move search. Mission 4 is still waiting on your review of the spec,  |
| 2026-10-26 16:53 | 75 | 40→? | 7→? | $10.41 | 3 | cna answered all three of this repo's open rulings requests (R-139, R-140, R-141). All three are now built and merged into the engine, with the docs updated. Yo |
| 2026-10-26 18:53 | 59 | 41→42 | 9→13 | $13.25 | 5 | I merged three PRs this run (#77, #78, #79), plus `PROGRESS.md` updates on `main`. Each one passed clippy, `cargo test` and `cna-probe check` locally and was gr |
| 2026-11-02 13:44 | 81 | 36→39 | none→none | $19.65 | 1 | That notification is only the 22:05 timer I set, and nothing has changed since my last summary. The run is wrapped up: the work is pushed, the hand-off is writt |
| 2026-11-02 16:44 | 76 | 40→43 | none→none | $15.44 | 2 | Slices 2 and 3 of Mission 4 are merged, and slice 4 is partly built as draft PR #95 with CI green. - **Slice 2, the headless server (#93), merged.** A separate  |
| 2026-11-02 19:44 | 79 | 44→? | none→? | $19.87 | 2 | #96 isn't merged: its `server` and `test` checks pass, but `web` is still running and should finish around 04:11Z, too close to the 04:14Z cutoff to merge safel |
| 2026-11-02 22:44 | 82 | 50→53 | none→none | $9.14 | 2 | Mission 4 (online play) is finished and merged. Nothing is hosted yet; that's your checklist, below. **Merged this run:** - **#96, turn alerts.** You get a Disc |
| 2026-11-03 00:06 | 74 | 53→57 | none→none | $23.95 | 2 | All four stacked PRs (#100–#103) are merged, and main's CI on the #103 merge (run 41600130717) passed: web, test and server. Locally on main, clippy is clean, 6 |
| 2026-11-03 01:45 | 79 | 58→61 | none→none | $12.84 | 3 | #105 merged, and #104 is ready but not merged: its browser CI job (`web`, the Playwright run) was still running at 10:04Z with the deadline at 10:15Z. **#105: m |
| 2026-11-03 04:45 | 63 | 61→62 | none→none | $14.94 | 8 | Everything planned for this run is merged: forming raiders, raids on supply dumps, and the `vendor/cna` bump for cna's answers. Nothing needs you, apart from Mi |
| 2026-11-03 05:48 | 69 | 62→64 | none→none | $15.42 | 124 | #109 is merged, and draft #110 (patrols) has its plan and engine built, reviewed and fixed. Nothing needs you. **#109, the spotting try against desert raiders ( |
| 2026-11-03 09:44 | 72 | 67→68 | none→none | $9.94 | 3 | Six PRs merged this run (#110–#115). No autopilot PR is left open. For each merge, the PR's CI run passed and the gates passed locally. / PR / What it does / /- |
| 2026-11-03 10:56 | 85 | 68→70 | none→none | $15.41 | 2 | The fleet engine is built and merged: warship ratings load, ships bombard the coast, and Axis strikes damage, sink and repair them. Its browser panel is merged  |
| 2026-11-03 12:21 | 15 | 70→? | none→? | $6.63 | 8 | 70 merged autopilot branches deleted from origin (each verified as already in `main`); `autopilot/m5-slice-4-arrivals` has unmerged work, so I left it. Waiting  |
| 2026-11-03 13:45 | 53 | 70→71 | none→none | $11.48 | 173 | R-152 (captured equipment) is built and merged in #121–#123. The *San Giorgio* (R-151) is open as draft PR #124. A previous run had built it and was killed befo |
| 2026-11-03 15:44 | 73 | 71→0 | none→none | $9.79 | 125 | Three PRs merged this run (#124, #125, #126). The towing slice is open as draft #127 for the next run. - **#124, the *San Giorgio*:** merged on green CI. Its ba |
| 2026-11-06 00:44 | 84 | 4→5 | none→none | $13.61 | 211 | Six PRs merged this run. One more (#133) is a draft because one browser test fails in CI. **Merged** - **#127, towing (21.6x):** picked up from the last run wit |
| 2026-11-06 03:44 | 71 | 5→6 | none→none | $13.64 | 9 | Brian, I merged six PRs this run (#133–#138), and Mission 5's scenario set-ups moved forward on cna's new data. Each new rule sits behind its own module, which  |
| 2026-11-06 04:55 | 11 | 6→6 | none→none | $7.80 | 18 | The three agents are working; the run waits on them before review and merge. |
| 2026-11-06 05:44 | 79 | 6→7 | none→none | $15.32 | 4 | I recovered everything the killed third run left behind and merged six PRs. No autopilot PRs are open. **Merged this run (Mission 5, scenario set-ups):** - **#1 |
| 2026-11-06 07:03 | 64 | 7→8 | none→none | $10.85 | 7 | Mission 5 is complete, and the Mission 6 spec (the Logistics Game) is waiting on your review as PR #148. **Merged this run** (each green on CI before merging):  |
| 2026-11-06 08:44 | 71 | 8→10 | none→none | $11.48 | 4 | Four follow-up PRs and the formatting cleanup are merged. Mission 6 (the Logistics Game) is still waiting on your review of its spec, PR #148, so I built nothin |
| 2026-11-06 13:44 | 47 | 10→10 | none→none | $6.53 | 10 | I merged two PRs this run. Mission 6 (the Logistics Game) is still waiting for your review of #148. **#154** moves `vendor/cna` to 9336fcc, which brings in cna' |
| 2026-11-06 14:44 | 71 | 10→10 | none→none | $7.62 | 103 | All three PRs merged to main. CI was green on each, and so were the local gates, the `web/` checks and Playwright (65/65). Feedback was empty and #148 (the Miss |
| 2026-11-06 15:55 | 60 | 10→11 | none→none | $8.73 | 46 | Reassignment (SPI 19.2x) is built and merged as #159, and #160 adds the missing `capture_on_move` tests. Brian's review of the Mission 6 spec (#148) is still th |
| 2026-11-06 18:44 | 61 | 11→12 | none→none | $15.39 | 172 | This run produced the Mission 7 spec (the Air Game), which now waits on your review, and merged two Mission 5 follow-ups. Mission 6 (#148) is still waiting on y |
| 2026-11-06 19:46 | 42 | 12→13 | none→none | $7.55 | 5 | The "no armour" rule (14.23) is now in the engine and the browser, and merged to main as #164. Missions 6 and 7 are still waiting on your review. **What 14.23 d |
| 2026-11-07 00:44 | 56 | 13→14 | none→none | $9.93 | 6 | Three PRs went in this run. They build cna's answers to the requests this repo posted: R-177 (halftracks) and R-176 (historical changes of a unit's parent). Mis |
