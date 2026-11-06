import { describe, expect, it } from "vitest";
import { formatEvent } from "./format";
import type { Clock, Event, RaidResult, VictoryReport } from "./types";

const clock: Clock = { game_turn: 2, stage: 3, phase: "movement-combat", half: "A", segment: "movement" };
const report: VictoryReport = { axis: null, cw: "marginal", checked: [], unchecked: [] };
const roll = { reading: 4, pct: 10, raw_lost: 3, toe_lost: 1, engaged: false, retreat: 0, captured: false };

/** One sample per variant of `Event`, with what its line must say. */
const cases: [Event, string][] = [
  [{ "phase-started": { clock } }, "Game-Turn 2 OpStage 3: G · movement and combat (Player A), movement begins."],
  [{ "phase-skipped": { clock: { ...clock, segment: "barrage" } } }, "barrage: nothing to do, skipped."],
  [{ "die-rolled": { purpose: "initiative", value: 5 } }, "Die for initiative: 5."],
  [{ "initiative-determined": { holder: "axis", rolled: true } }, "Axis holds the initiative (by die roll)."],
  [{ "initiative-declared": { player_a: "cw" } }, "Commonwealth declares itself Player A."],
  [{ "weather-set": { weather: { sandstorm: { sheets: ["C", "D"] } } } }, "Weather: sandstorm on C, D."],
  [{ placed: { piece: "u:a", hex: "C0101" } }, "A placed at C0101."],
  [{ unplaced: { piece: "u:a" } }, "A taken back, to be placed again."],
  ["set-up-complete", "Set-up complete; play begins."],
  ["rommel-recalled", "Rommel is recalled to Germany for this Game-Turn."],
  [{ "raider-spotted": { piece: "u:a", hex: "C4131", roll: 6, eliminated: true } }, "A entering C4131 rolls 6 to stay unseen: spotted and eliminated."],
  [{ "raider-announced": { hex: "C4131", pieces: ["u:a", "u:b"] } }, "A raider is reported at C4131; A, B may try to spot it (27.43)."],
  [{ "raider-announced": { hex: "C4131", pieces: [] } }, "A raider is reported at C4131; no unit may try to spot it, each having tried this stage (27.44)."],
  [{ "spot-tried": { piece: "u:a", hex: "C4131", rolls: [3], eliminated: [] } }, "A spends 1 CP to spot at C4131: die 3 (a 6 spots); none spotted."],
  [{ "spot-tried": { piece: "u:a", hex: "C4131", rolls: [2, 6], eliminated: ["u:b"] } }, "A spends 1 CP to spot at C4131: dice 2, 6 (a 6 spots); B spotted and eliminated."],
  [{ "rommel-raided": { piece: "u:a", hex: "C3618", roll: 7, result: "lrdg-eliminated", lrdg_eliminated: true } }, "A raids Rommel at C3618: 7, the raid fails; the raider is eliminated."],
  [{ "raider-formed": { piece: "u:a", from: "u:b", hex: "C3020" } }, "A is formed at C3020 from a TOE point of B."],
  [{ "dump-raided": { piece: "u:a", target: "u:b", hex: "C4808", guard_roll: 9, guard_strength: 7, roll: 2, dummy: false, ammo_lost: 4, fuel_lost: 5, removed: false, raider_eliminated: false } }, "A raids B at C4808: guards 9 against 7, die 2, 4 ammunition and 5 fuel burnt; the raider survives."],
  [{ "dump-raided": { piece: "u:a", target: "u:b", hex: "C4808", guard_roll: null, roll: 1, dummy: false, removed: false, raider_eliminated: false } }, "A raids B at C4808: die 1; the raider survives."],
  [{ "dump-raided": { piece: "u:a", target: "u:b", hex: "C4808", guard_roll: 4, roll: null, dummy: true, removed: true, raider_eliminated: true } }, "A raids B at C4808: guards 4, a dummy, removed; the raider is eliminated."],
  [
    { patrolled: { side: "cw", target: "C4020", sources: [["u:a", 2], ["u:b", 1]], survival_die: 2, killed: 1, captured: 1, recon_die: 5, described: [{ piece: "u:c", toe: 6 }, { piece: "u:d", toe: 3 }], objective_die: 1, objective_killed: 1, objective_captured: 1 } },
    "Commonwealth patrols C4020 with A 2, B 1: survival 2, 1 killed and 1 captured; reconnaissance 5, sees u:c (6 TOE), u:d (3 TOE); objective loss 1, 1 TOE killed and 1 captured.",
  ],
  [{ arrived: { pieces: ["u:a"], at: { box: "tripoli" } } }, "A arrive at the tripoli box."],
  [{ arrived: { pieces: ["u:a", "u:b"], at: { port: "alexandria" } } }, "A, B arrive at Alexandria."],
  [{ stationed: { ship: "u:a", hex: "C4131" } }, "A puts to sea at C4131 (30.1x)."],
  [{ "in-port": { ships: ["u:a"] } }, "A is back in Alexandria (30.1x)."],
  [{ "fleet-strikes-rolled": { strikes: 1 } }, "The Axis rolls 1 strike on the fleet (32.72)."],
  [{ "fleet-struck": { column: 4, reading: 16, damage: 1, hits: { "u:a": 1 } } }, "Axis strike on Alexandria: column 4, dice 16, 1 gun point of damage (A 1)."],
  [{ "fleet-struck": { column: 1, reading: 2, damage: 0 } }, "Axis strike on Alexandria: column 1, dice 2, 0 gun points of damage."],
  [{ "fleet-struck": { column: 6, reading: 66, damage: 9, hits: { "u:a": 5, "u:b": 4 }, sunk: ["u:b"] } }, "Axis strike on Alexandria: column 6, dice 66, 9 gun points of damage (A 5, B 4). B is sunk."],
  [{ "ship-repaired": { ship: "u:a", guns: 4 } }, "A repairs a gun point, guns 4 now (30.37)."],
  [{ "moved-off-map": { piece: "u:a", location: { transit: { from: "nofilia", to: "tripoli", done: 1 } } } }, "A moves off-map to in transit nofilia → tripoli."],
  [{ moved: { pieces: ["u:a", "u:b"], from: "C4131", to: "C4130", cost: 3 } }, "A, B: C4131 → C4130 for 3 CP."],
  [{ disorganised: { piece: "u:a", dp: 2, level: 1 } }, "A takes 2 DP, now cohesion level 1."],
  [{ "move-interrupted": { pieces: ["u:a"], at: "C4130", before: "C4129", declare: ["C4128"] } }, "A stop at C4130 before C4129: ZOC of C4128 to be declared."],
  [{ "zoc-declared": { hex: "C4128", exerts: false } }, "C4128 does not exert a ZOC."],
  [{ "reaction-offered": { pieces: ["u:b"], at: "C4130" } }, "Reaction offered to B against C4130."],
  [{ "contact-broken": { piece: "u:a", cost: 4 } }, "A breaks contact for 4 CP."],
  [{ "reserve-designated": { pieces: ["u:a"] } }, "A designated reserve."],
  [{ "reserve-released": { pieces: ["u:a"], from: "I" } }, "A released from Reserve I."],
  [{ "reserve-flipped": { pieces: ["u:a"] } }, "A flip from Reserve I to Reserve II."],
  [{ sealed: { side: "axis", step: "assign-forces" } }, "Axis has sealed its force assignment."],
  [{ "gun-positions-set": { side: "cw", forward: [] } }, "Commonwealth guns forward: nothing."],
  [{ "barrage-plotted": { side: "axis", fires: [{ piece: "u:a", points: 6, target: "u:b" }] } }, "Axis barrage plotted: A 6 at B."],
  [
    { barrage: { side: "axis", firers: ["u:a"], target: "u:b", points: 6, shift: -1, reading: 3, result: "pinned" } },
    "Axis barrage on B by A: 6 points, shift -1, reading 3: pinned.",
  ],
  [{ "retreated-before-assault": { pieces: ["u:b"], to: "D0101" } }, "B retreat before assault to D0101."],
  [{ "anti-armour": { side: "cw", target: "C4130", points: 4, shift: 0, reading: 2, damage: 1 } }, "Commonwealth anti-armour fire at C4130: 4 points, shift 0, reading 2: 1 point of damage."],
  [
    { assault: { target: "C4130", attackers: ["u:a"], probe: false, attacker_raw: 12, defender_raw: 6, basic: 2, shifts: 1, column: "2:1", attacker: roll, defender: { ...roll, retreat: 2 } } },
    "Assault on C4130 by A: 12 v 6 raw, differential +2, +1 shifts, column 2:1; attacker rolls 4 and loses 10% (1 TOE), defender rolls 4 and loses 10% (1 TOE), retreats 2.",
  ],
  [{ prisoners: { side: "axis", points: 3 } }, "3 points of the Axis losses are prisoners."],
  [{ "prisoners-taken": { side: "cw", hex: "C3923", points: 5 } }, "Commonwealth takes 5 prisoner points in C3923."],
  [{ "equipment-captured": { side: "cw", hex: "C3923", weapon: "captured tank", points: 4 } }, "Commonwealth captures 4 tank replacement points in C3923."],
  [{ "prisoners-marched": { side: "axis", path: ["C4021", "C4020", "C4120"], points: 12, cp: 2 } }, "Axis marches 12 prisoner points C4021 → C4120 (2 CP)."],
  [{ "convoy-bombed": { lane: 2, bomb_points: 201, reading: 45, percent: 20, units: 3, sunk: 1 } }, "The Axis convoy on lane 2 is bombed at 201 points: 45, 20 %; 1 of 3 supply units sunk."],
  [{ "convoy-lanes-set": { lanes: [6, 2] } }, "Axis convoys sail by lane 6 (Tobruk), then lane 2 (Tripoli)."],
  [{ "convoy-lanes-set": { lanes: [] } }, "Axis convoys sail by the Tripoli lane."],
  [{ "convoy-cancelled": { lane: 3 } }, "No convoy sails on lane 3 (Benghazi): its port is in Commonwealth hands (56.15)."],
  ["bizerta-opened", "The Axis may now sail convoys to Bizerta (55.17)."],
  [{ "prisoners-left-map": { side: "axis", points: 4, stages: 3 } }, "4 Axis prisoner points leave the map; they reach the Tripolitania box in 3 truck convoy phases."],
  [{ "prisoners-gone": { side: "cw", hex: "E3613", points: 5 } }, "5 Commonwealth prisoner points leave play at E3613."],
  [{ "construction-begun": { side: "axis", item: "dummy-minefield", hex: "B1010", builders: ["u:a"] } }, "Axis begins a dummy minefield in B1010."],
  [{ "construction-lost": { side: "axis", item: "fortification", hex: "B1010" } }, "Axis loses its fortification project in B1010: a builder left."],
  [{ "fortification-built": { side: "axis", hex: "B1010", level: 1 } }, "Axis fortifies B1010 to level 1."],
  [{ "minefield-laid": { side: "cw", hex: "C3923" } }, "Commonwealth lays a minefield in C3923."],
  [{ "minefield-revealed": { hex: "C3923", real: false } }, "The minefield in C3923 is a dummy."],
  [{ "minefield-removed": { hex: "C3923" } }, "The minefield in C3923 is taken off."],
  [{ "guards-detailed": { piece: "unit:it:iim-tank-bn", hex: "C4021", points: 3 } }, "unit:it:iim-tank-bn details 3 guard points in C4021."],
  [{ "guards-released": { piece: "unit:it:iim-tank-bn", hex: "C4021", points: 1 } }, "1 guard points rejoin unit:it:iim-tank-bn in C4021."],
  [{ "prisoners-escaped": { side: "axis", hex: "E2020", rejoined: 0, escapees: 6 } }, "Axis prisoners escape in E2020: 6 wait there as escapees."],
  [{ "prisoners-escaped": { side: "axis", hex: "C3920", rejoined: 4, escapees: 0 } }, "Axis prisoners escape in C3920: 4 rejoin their side."],
  [{ "prisoners-freed": { side: "cw", hex: "C3923", points: 5 } }, "Commonwealth frees 5 of its men held prisoner in C3923."],
  [{ absorbed: { piece: "unit:it:iim-tank-bn", weapon: "M11/39", points: 3, cp: 2 } }, "unit:it:iim-tank-bn absorbs 3 repaired M11/39 points for 2 CP."],
  [{ absorbed: { piece: "unit:it:iim-tank-bn", weapon: "captured tank", points: 2, cp: 1 } }, "unit:it:iim-tank-bn absorbs 2 captured tank points for 1 CP."],
  [{ surrendered: { pieces: ["u:b"] } }, "B surrender."],
  [{ engaged: { pieces: ["u:a", "u:b"] } }, "A, B are engaged."],
  [{ "retreat-ordered": { pieces: ["u:b"], hexes: 1, reason: "must-attack" } }, "B must retreat 1 hex (must attack or retreat)."],
  [{ "retreat-bought-out": { pieces: ["u:b"], short: 2 } }, "B stand 2 hexes short of the ordered retreat and pay for it."],
  [{ losses: { piece: "u:b", points: 2, toe: 4 } }, "B loses 2 points, TOE now 4."],
  [{ "fuel-spent": { piece: "u:a", from: "u:s", fuel: 2 } }, "A burns 2 fuel from S."],
  [{ "ammunition-spent": { piece: "u:a", from: "u:s", ammo: 3 } }, "A fires 3 ammunition from S."],
  [{ "ammunition-short": { piece: "u:b" } }, "B has no ammunition in range and does not defend."],
  [{ "supply-unit-spent": { piece: "u:s" } }, "S is empty and removed."],
  [{ "supply-destroyed": { piece: "u:s" } }, "S destroyed by its own side."],
  [{ attached: { piece: "u:b", to: "u:s" } }, "B attaches to S."],
  [{ detached: { piece: "u:b", from: "u:s" } }, "B detaches from S."],
  [{ "motorisation-attached": { piece: "u:s", points: 30 } }, "S takes 30 motorisation points."],
  [{ "motorisation-detached": { piece: "u:s", points: 6 } }, "S gives up its 6 motorisation points."],
  [{ "load-transferred": { from: "u:s", to: "u:a", ammo: 5, fuel: 10 } }, "5 ammunition and 10 fuel move from S to A."],
  [{ railed: { pieces: ["u:s", "u:a"], from: "E3613", to: "D3715" } }, "S, A by rail E3613 → D3715."],
  [{ "wrecks-captured": { by: "u:b", hex: "C3920", points: 2 } }, "B captures 2 broken-down vehicle points at C3920."],
  [{ "facility-repaired": { hex: "E3613", weapon: "Matilda", kind: "major", attempted: 4, die: 5, repaired: 2 } }, "Major facility repair at E3613, Matilda: 5 → 2 of 4 repaired."],
  [{ "tanks-destroyed": { piece: "u:b", hex: "C3920", weapon: "A9", points: 2 } }, "B leaves 2 destroyed A9 at C3920."],
  [{ "destroyed-tanks-captured": { by: "u:b", hex: "C3920", points: 1 } }, "B captures 1 destroyed tank point at C3920."],
  [{ "destroyed-tanks-repaired": { hex: "E3613", weapon: "Matilda", kind: "major", attempted: 3, dice: [1, 4, 6], repaired: 1, junked: 1 } }, "Major facility, destroyed Matilda at E3613: 1, 4, 6 → 1 repaired, 1 junked of 3."],
  [{ towed: { side: "cw", from: "D3513", to: "D3714", weapon: "Matilda", points: 3 } }, "3 broken-down Matilda towed D3513 → D3714."],
  [{ "broke-down": { piece: "u:b", weapon: "CV33", reading: 64, percent: 33, points: 3 } }, "B breakdown check, CV33: 64 → 33 %, 3 broken down."],
  [{ eliminated: { piece: "u:b" } }, "B eliminated."],
  [{ withdrawn: { pieces: ["u:a", "u:b"], motorisation: 10 } }, "A, B are withdrawn with 10 motorisation points (20.8x)."],
  [{ withdrawn: { pieces: ["u:a"], motorisation: 0 } }, "A is withdrawn (20.8x)."],
  [{ "withdrawal-failed": { pieces: ["u:a"], reason: "not in Cairo or Alexandria" } }, "Withdrawal of A fails, not in Cairo or Alexandria: eliminated (20.83)."],
  [{ "withdrawal-picked": { fills: "infantry-battalion", pieces: ["u:a"] } }, "A picked to withdraw as infantry-battalion (20.82)."],
  [
    { "forces-assigned": { side: "axis", assignments: [{ piece: "u:a", anti_armour: [{ target: "C4130", points: 2 }], close_assault: [{ points: 4 }] }] } },
    "Axis forces assigned: A: 2 anti-armour at C4130, 4 assault.",
  ],
  [{ "combat-cp": { piece: "u:a", cost: 2 } }, "A pays 2 CP for combat."],
  [{ "combat-cp-refunded": { piece: "u:a", cp: 2, dp: 1 } }, "A refunded 2 CP and 1 DP."],
  [{ captured: { piece: "u:b", hex: "C4130" } }, "B captured at C4130."],
  [{ "game-over": { report } }, "Game over. Axis: no level; Commonwealth: marginal."],
];

const names: Record<string, string> = { "u:a": "A", "u:b": "B", "u:s": "S" };
const name = (id: string) => names[id] ?? id;

describe("the log line of every event variant", () => {
  it("covers all 93 variants", () => {
    const tags = new Set(cases.map(([e]) => (typeof e === "string" ? e : Object.keys(e)[0])));
    expect(tags.size).toBe(93);
  });

  it.each(cases)("%j", (event, line) => {
    expect(formatEvent(event, name)).toBe(line);
  });

  it("reads every row of the Raid on Rommel Table, and a raider not spotted", () => {
    const raid = (result: RaidResult, lrdg_eliminated = false) => formatEvent({ "rommel-raided": { piece: "u:a", hex: "C3618", roll: 2, result, lrdg_eliminated } }, name);
    expect(raid("rommel-removed")).toBe("A raids Rommel at C3618: 2, Rommel is removed for good; the raider survives.");
    expect(raid("axis-initiative-3-temporary")).toBe("A raids Rommel at C3618: 2, the Axis initiative rating drops to 3 for a while; the raider survives.");
    expect(raid("none")).toBe("A raids Rommel at C3618: 2, no effect; the raider survives.");
    // Rommel's hex held no combat or HQ unit: the result spares the raider (27.92).
    expect(raid("lrdg-eliminated")).toBe("A raids Rommel at C3618: 2, the raid fails; the raider survives.");
    expect(formatEvent({ "raider-spotted": { piece: "u:a", hex: "C4131", roll: 3, eliminated: false } }, name)).toBe("A entering C4131 rolls 3 to stay unseen: not spotted.");
  });

  it("says when a patrol met no combat unit, came back with nothing, or was wiped out", () => {
    const base = { side: "axis" as const, target: "C4020", sources: [["u:a", 1]] as [string, number][], killed: 0, captured: 0, objective_die: 6, objective_killed: 0, objective_captured: 0 };
    expect(formatEvent({ patrolled: { ...base, recon_die: 1 } }, name)).toBe("Axis patrols C4020 with A 1: no combat unit to meet it; reconnaissance 1, sees nothing; objective loss 6, no loss.");
    expect(formatEvent({ patrolled: { ...base, survival_die: 6, recon_die: 4, described: [{ piece: "u:b", toe: 2 }] } }, name)).toBe("Axis patrols C4020 with A 1: survival 6, none lost; reconnaissance 4, sees B (2 TOE); objective loss 6, no loss.");
    // Wiped out: no reconnaissance die, and the objective's captures read as kills (16.32).
    expect(formatEvent({ patrolled: { ...base, survival_die: 1, killed: 1, objective_die: 2, objective_killed: 1 } }, name)).toBe("Axis patrols C4020 with A 1: survival 1, 1 killed; none came back to report; objective loss 2, 1 TOE killed.");
  });

  it("names pieces by id when no namer is given", () => {
    expect(formatEvent({ placed: { piece: "unit:it:x", hex: "C0101" } })).toBe("unit:it:x placed at C0101.");
  });
});

// Online, the enemy's sealed answers and firers reach a player with
// those fields removed (`cna_game::view::events`, 3.61).
describe("the log line of an event as the enemy's player receives it", () => {
  it.each([
    [{ "barrage-plotted": { side: "axis" } }, "Axis barrage plotted."],
    [{ "forces-assigned": { side: "cw" } }, "Commonwealth forces assigned."],
    [{ barrage: { side: "axis", target: "u:b", points: 6, shift: -1, reading: 3, result: "pinned" } }, "Axis barrage on B: 6 points, shift -1, reading 3: pinned."],
  ] as Array<[Event, string]>)("%j", (event, line) => {
    expect(formatEvent(event, name)).toBe(line);
  });
});

// The fields `cna_game::view::events_for` takes out of an event for the
// side not entitled to them (crates/cna-game/src/view/events.rs).
const withheld: Record<string, string[]> = {
  moved: ["cost"],
  "contact-broken": ["cost"],
  "barrage-plotted": ["fires"],
  barrage: ["firers"],
  "anti-armour": ["points", "shift"],
  "forces-assigned": ["assignments"],
  assault: ["attacker_raw", "defender_raw"],
  prisoners: ["points"],
  "prisoners-taken": ["points"],
  "equipment-captured": ["points"],
  "prisoners-left-map": ["points"],
  "prisoners-gone": ["points"],
  "prisoners-marched": ["points", "cp"],
  "construction-begun": ["item"],
  "construction-lost": ["item"],
  "guards-detailed": ["points"],
  "guards-released": ["points"],
  "air-lifted": ["fuel_from"],
  "convoy-bombed": ["units", "sunk"],
};

describe("an event with the fields the server withholds", () => {
  const reduced = cases
    .filter(([e]) => Object.keys(e as object)[0] in withheld)
    .map(([e]) => {
      const [kind, body] = Object.entries(structuredClone(e) as Record<string, Record<string, unknown>>)[0];
      for (const f of withheld[kind]) delete body[f];
      if (kind === "assault") for (const r of ["attacker", "defender"]) for (const f of ["raw_lost", "toe_lost"]) delete (body[r] as Record<string, unknown>)[f];
      return { [kind]: body } as unknown as Event;
    });

  // The variant table above has no air lift; it gets its own.
  reduced.push({ "air-lifted": { piece: "u:s", from: { box: "tripoli" }, to: { hex: "C4021" } } } as unknown as Event);

  it("covers every reduced variant", () => {
    expect(reduced.map((e) => Object.keys(e)[0]).sort()).toEqual(Object.keys(withheld).sort());
  });

  it.each(reduced)("%j reads without a hole", (event) => {
    expect(formatEvent(event, name)).not.toMatch(/undefined|NaN|\?\?/);
  });
});
