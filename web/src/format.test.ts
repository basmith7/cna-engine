import { describe, expect, it } from "vitest";
import { formatEvent } from "./format";
import type { Clock, Event, VictoryReport } from "./types";

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
  ["set-up-complete", "Set-up complete; play begins."],
  [{ arrived: { pieces: ["u:a"], at: { box: "tripoli" } } }, "A arrive at the tripoli box."],
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
  [{ prisoners: { side: "axis", points: 3 } }, "Axis takes 3 prisoner points."],
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
  [{ "motorisation-attached": { piece: "u:s", points: 30 } }, "S takes 30 motorisation points."],
  [{ "motorisation-detached": { piece: "u:s", points: 6 } }, "S gives up its 6 motorisation points."],
  [{ "load-transferred": { from: "u:s", to: "u:a", ammo: 5, fuel: 10 } }, "5 ammunition and 10 fuel move from S to A."],
  [{ railed: { pieces: ["u:s", "u:a"], from: "E3613", to: "D3715" } }, "S, A by rail E3613 → D3715."],
  [{ "wrecks-captured": { by: "u:b", hex: "C3920", points: 2 } }, "B captures 2 broken-down vehicle points at C3920."],
  [{ "broke-down": { piece: "u:b", weapon: "CV33", reading: 64, percent: 33, points: 3 } }, "B breakdown check, CV33: 64 → 33 %, 3 broken down."],
  [{ eliminated: { piece: "u:b" } }, "B eliminated."],
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
  it("covers all 49 variants", () => {
    const tags = new Set(cases.map(([e]) => (typeof e === "string" ? e : Object.keys(e)[0])));
    expect(tags.size).toBe(49);
  });

  it.each(cases)("%j", (event, line) => {
    expect(formatEvent(event, name)).toBe(line);
  });

  it("names pieces by id when no namer is given", () => {
    expect(formatEvent({ placed: { piece: "unit:it:x", hex: "C0101" } })).toBe("unit:it:x placed at C0101.");
  });
});
