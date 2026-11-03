# Mission 5: repair facilities (phase K, §22.3x)

Spec: `docs/designs/2026-10-13-land-turn-design.md`. Phase K offered only
field repair (22.2x, `crates/cna-game/src/repair.rs`); the rules restate
facility repair in `vendor/cna/rules/80-engineering.md` (*Facility
repairs*), with R-099 letting each scenario's set-up name its facilities.
Every decision here is the autopilot's under Brian's delegation of
2026-09-26 and is reversible with one PR.

**Goal.** Broken-down vehicles left in a city with a repair facility come
back at better odds than in the field, and through storms and enemy ZOC
where the facility is major. Without it a player has no reason to fall
back on Alexandria or Tobruk to recover his tanks. This serves **The
goal**: two players at a distance, every rule enforced.

**Non-goals.** Towing (21.6x), destroyed tanks (22.4x, 14.5), building
facilities (24.8x), bombing and barrage of facilities (22.34): the later
slices below, or the Air Game.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Which facilities | each side's `repair` blocks in the scenario, by hex or place (every hex of the place); a side listing none has 22.31's: Alexandria and Cairo (Commonwealth), the Tripoli box (Axis), Tobruk for a side holding it alone | R-099 option 3; R-098 |
| Player-chosen facilities | blocks that name a region and a sentence ("any village or city") are not resolved | the player's set-up choice; no set-up order picks them yet |
| Began the OpStage there | a wreck counts the points that broke down this OpStage (`Wreck::this_stage`, cleared by `start_stage`); only the rest are attempted | 22.31: a vehicle that begins the segment in the facility hex; wrecks cannot move until towing exists |
| Bars | major: none; temporary: enemy ZOC (an undeclared one counts, as for field repair) outside a major city, rainstorm or sandstorm | 22.13, 22.3x |
| Roll | one per type per hex per phase K, shared with field repair; d6 on the temporary or major column, fractions up, a starred 10 % ignored for one point | 22.23, 22.8, `vehicle-repair.json` |
| Cost | with abstract supply a fuel a point attempted, paid first from supply in or in range of the hex, as field repair pays; the rules' store a point is not charged | supply units carry no stores under abstract supply (`piece::Load`); no ruling says otherwise |
| Order and event | `Order::FacilityRepair { hex, weapon, fuel_from }`, `Event::FacilityRepaired` (own, as `field-repaired`); the view's `facility_repairs` lists the offers | a separate variant keeps old logs and the log line distinct |

## Tasks

### Task 1: facility repair (this slice)
- [x] cna-data reads the repair blocks.
- [x] Failing tests: Alexandria's major facility repairs a Matilda in a
  sandstorm under an enemy ZOC; Mersa Matruh's temporary one stops under
  either; points that broke down this OpStage wait; a hex with no facility
  of the side's offers nothing; a fuel a point under abstract supply.
- [x] `Order::FacilityRepair`, `Event::FacilityRepaired`, the view's
  `facility_repairs`, the phase K buttons and log line.
- [x] `NOT_SIMULATED.md`: the Breakdown row.

### Task 2: towing (21.6x)
- [x] Towing broken-down points toward a facility in phase K: the towing
  unit, its CP and the hexes; towed points skip repair that phase (22.13)
  and arrive having not begun the OpStage there.

### Task 3: destroyed tanks (22.4x, 14.5)
- [ ] Destroyed-tank markers where tanks die in combat; capture by a
  battalion-size unit (R-014); repair only at a facility on the Destroyed
  Tank Repair Table (`destroyed-tank-repair.json`), pre-paying per point
  attempted, with repaired, junked and no-result outcomes.
