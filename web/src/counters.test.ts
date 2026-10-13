import { describe, expect, it } from "vitest";
import charsFile from "../../vendor/cna/data/tables/unit-characteristics.json";
import { glyphFor, label, offMap, sizeMark, stacks, topFirst } from "./counters";
import type { Piece, View } from "./types";

function piece(over: Partial<Piece> & { id: string }): Piece {
  return {
    side: "axis",
    kind: "combat",
    nation: "it",
    id_code: "x",
    parent: null,
    toe: 5,
    cpa: "10",
    morale: 0,
    status: null,
    equivalent: "battalion",
    location: { hex: "C4131" },
    cp_spent: 0,
    cp_enemy_half: 0,
    cohesion: { level: 0, dp_tally: 0 },
    contact: false,
    engaged: false,
    unit_type: "Infantry Bn-Eq",
    name: "A battalion",
    abbreviation: "A bn",
    ...over,
  };
}

function view(pieces: Piece[]): View {
  return {
    scenario: "grazianis-offensive",
    rules_commit: "0".repeat(40),
    ruleset: {},
    clock: { game_turn: 1, stage: 1, phase: "initiative-declaration", half: null, segment: null },
    initiative: null,
    player_a: null,
    weather: "normal",
    pieces: Object.fromEntries(pieces.map((p) => [p.id, p])),
    pending: { "set-up": { side: "axis" } },
    cycle: 0,
    moved: [],
    zoc: {},
    continuing: [],
    shifted: [],
    combat: {},
    over: null,
    to_act: "axis",
  };
}

describe("counter glyphs", () => {
  it("has a glyph for every unit type on the characteristics chart, none the fallback", () => {
    const types = new Set(charsFile.rows.map((r) => r.unit_type));
    expect(types.size).toBeGreaterThan(20);
    for (const t of types) {
      expect(glyphFor("combat", t), t).not.toBe("other");
    }
  });

  it("draws supply and dummy supply alike, and an unknown type as other", () => {
    expect(glyphFor("supply-unit", null)).toBe("supply");
    expect(glyphFor("dummy-supply-unit", null)).toBe("supply");
    expect(glyphFor("hq", "Headquarters")).toBe("hq");
    expect(glyphFor("combat", "Tank Bn-Eq")).toBe("armour");
    expect(glyphFor("combat", "Recce Bn-Eq")).toBe("recce");
    expect(glyphFor("combat", "Reconnaissance Company-Eq")).toBe("recce");
    expect(glyphFor("combat", "Anti-tank Coy-Eq")).toBe("anti-tank");
    expect(glyphFor("combat", "Light Anti-air Bn-Eq")).toBe("anti-air");
    expect(glyphFor("combat", "Engineer Bn/Coy-Eq")).toBe("engineer");
    expect(glyphFor("combat", "Construction (Road/RR)")).toBe("engineer");
    expect(glyphFor("combat", "Zeppelin")).toBe("other");
    expect(glyphFor("combat", null)).toBe("other");
  });

  it("marks size from the stacking equivalent", () => {
    expect(sizeMark("brigade")).toBe("X");
    expect(sizeMark("battalion")).toBe("II");
    expect(sizeMark("company")).toBe("I");
    expect(sizeMark("hq")).toBe("");
    expect(sizeMark(null)).toBe("");
  });

  it("labels a supply unit without telling a dummy from a real one", () => {
    const real = piece({ id: "supply:axis:16", kind: "supply-unit", nation: null, id_code: null, name: null, abbreviation: null });
    const dummy = piece({ id: "supply:axis:dummy-1", kind: "dummy-supply-unit", nation: null, id_code: null, name: null, abbreviation: null });
    expect(label(real)).toEqual(label(dummy));
    expect(label(piece({ id: "unit:it:x" }))).toEqual({ short: "A bn", long: "A battalion" });
  });
});

describe("stacks", () => {
  it("groups placed pieces by hex with the HQ on top, then combat, then supply", () => {
    const v = view([
      piece({ id: "supply:axis:1", kind: "supply-unit", nation: null, id_code: null, name: null, abbreviation: null }),
      piece({ id: "unit:it:b" }),
      piece({ id: "unit:it:hq", kind: "hq", equivalent: "hq", unit_type: "Headquarters" }),
      piece({ id: "unit:it:a" }),
      piece({ id: "unit:it:elsewhere", location: { hex: "C4021" } }),
      piece({ id: "unit:it:boxed", location: { box: "tripoli" } }),
      piece({ id: "unit:it:gone", location: "eliminated" }),
    ]);
    const s = stacks(v);
    expect([...s.keys()]).toEqual(["C4021", "C4131"]);
    expect(s.get("C4131")!.map((p) => p.id)).toEqual(["unit:it:hq", "unit:it:a", "unit:it:b", "supply:axis:1"]);
    expect(topFirst(s.get("C4131")!)[0].id).toBe("unit:it:hq");
  });

  it("sorts the off-map pieces into boxes, transit and a tray per side", () => {
    const v = view([
      piece({ id: "unit:it:boxed", location: { box: "tripoli" } }),
      piece({ id: "unit:it:moving", location: { transit: { from: "tripoli", to: "nofilia", done: 1 } } }),
      piece({ id: "unit:it:later", location: { pending: { sheets: ["A", "B"] } } }),
      piece({ id: "unit:cw:later", side: "cw", nation: "cw", location: { pending: { hexes: ["E3613"] } } }),
      piece({ id: "unit:it:here" }),
    ]);
    const o = offMap(v);
    expect(o.boxes.map((b) => [b.box, b.pieces.map((p) => p.id)])).toEqual([
      ["tripoli", ["unit:it:boxed"]],
      ["tunisia", []],
      ["tripolitania", []],
      ["malta", []],
      ["italy-sicily", []],
      ["crete", []],
    ]);
    expect(o.transit.map((p) => p.id)).toEqual(["unit:it:moving"]);
    expect(o.tray.axis.map((p) => p.id)).toEqual(["unit:it:later"]);
    expect(o.tray.cw.map((p) => p.id)).toEqual(["unit:cw:later"]);
  });
});
