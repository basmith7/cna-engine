import { describe, expect, it } from "vitest";
import charsFile from "../../vendor/cna/data/tables/unit-characteristics.json";
import { glyphFor, label, nameParts, nested, offMap, sizeMark, stacks, topFirst } from "./counters";
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
    assaults_to_resolve: [],
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

  it("lifts the engineering mark out of an HQ's name as a superscript", () => {
    expect(nameParts("Ariete (132nd) Div HQ^E")).toEqual({ text: "Ariete (132nd) Div HQ", sup: "E" });
    expect(nameParts("2nd Scots Guards")).toEqual({ text: "2nd Scots Guards", sup: null });
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
  it("does not put the dummy supply units last: their order is no tell", () => {
    const supply = (id: string, kind: "supply-unit" | "dummy-supply-unit") =>
      piece({ id, kind, nation: null, id_code: null, name: null, abbreviation: null, location: { pending: { sheets: ["A"] } } });
    const real = ["16", "17", "18", "19", "20", "21"].map((n) => supply(`supply:axis:${n}`, "supply-unit"));
    const dummies = ["1", "2", "3", "4", "5", "6"].map((n) => supply(`supply:axis:dummy-${n}`, "dummy-supply-unit"));
    const order = topFirst([...real, ...dummies]).map((p) => p.kind);
    expect(order.slice(0, 6)).not.toEqual(Array(6).fill("supply-unit"));
    expect(order.slice(6)).not.toEqual(Array(6).fill("dummy-supply-unit"));
    // Still a stable order: the same set sorts the same way from any starting order.
    expect(topFirst([...dummies, ...real]).map((p) => p.id)).toEqual(topFirst([...real, ...dummies]).map((p) => p.id));
  });

  it("groups placed pieces by hex, north to south so a fan never covers its northern neighbour, with the HQ on top, then combat, then supply", () => {
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
    expect([...s.keys()]).toEqual(["C4131", "C4021"]);
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

describe("nested", () => {
  it("puts attached units under their parent, deeper each level", () => {
    const div = piece({ id: "hq:div" });
    const bde = piece({ id: "hq:bde", attached_to: "hq:div" });
    const bn = piece({ id: "unit:bn", attached_to: "hq:bde" });
    const lone = piece({ id: "unit:lone" });
    const stray = piece({ id: "unit:stray", attached_to: "hq:elsewhere" });
    expect(nested([bn, lone, bde, stray, div]).map((n) => [n.piece.id, n.depth])).toEqual([
      ["unit:lone", 0],
      ["unit:stray", 0],
      ["hq:div", 0],
      ["hq:bde", 1],
      ["unit:bn", 2],
    ]);
  });
});
