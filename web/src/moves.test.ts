import { describe, expect, it } from "vitest";
import { NOFILIA } from "./map";
import { allOffMapCapable, allOnMap, attachElsewhereOrders, attachmentOrder, byHex, cpLabel, defaultTicked, extendPath, lossOrderDraft, OFF_MAP_PLACES, raise, offMapCapable, reassignOrder, shiftOrders, swapOrders, tickable, toggle, toggleOffMap } from "./moves";
import type { Piece, View } from "./types";

function piece(id: string, side: "axis" | "cw", kind: Piece["kind"], hex: string): Piece {
  return { id, side, kind, location: { hex } } as unknown as Piece;
}

const view = {
  pieces: {
    hq: piece("hq", "axis", "hq", "C4020"),
    bn: piece("bn", "axis", "combat", "C4020"),
    supply: piece("supply", "axis", "supply-unit", "C4020"),
    elsewhere: piece("elsewhere", "axis", "combat", "C4021"),
    enemy: piece("enemy", "cw", "combat", "C4020"),
  },
} as unknown as View;

describe("defaultTicked", () => {
  it("ticks the side's combat units and HQs in the hex, not its supply nor the enemy", () => {
    expect(defaultTicked(view, "C4020", "axis")).toEqual(["hq", "bn"]);
    expect(defaultTicked(view, "C4020", "cw")).toEqual(["enemy"]);
    expect(defaultTicked(view, "C4020", null)).toEqual([]);
    expect(defaultTicked(view, "C9999", "axis")).toEqual([]);
  });
  it("ticks the raiders of a hex with no unit of the side's, as raiders move only with raiders", () => {
    const v = { pieces: { ...view.pieces, lrdg: piece("lrdg", "cw", "raider", "C4021"), sk: piece("sk", "axis", "raider", "C4020") } } as unknown as View;
    expect(defaultTicked(v, "C4021", "cw")).toEqual(["lrdg"]);
    expect(defaultTicked(v, "C4020", "axis")).toEqual(["hq", "bn"]);
  });
  it("in phases H and J ticks the side's supply units instead, the only pieces that move there (32.31, 32.3x)", () => {
    for (const phase of ["truck-convoy", "rail"]) {
      const v = { ...view, clock: { phase } } as unknown as View;
      expect(defaultTicked(v, "C4020", "axis")).toEqual(["supply"]);
    }
  });
});

describe("tickable", () => {
  it("is the side to act's pieces on the map or off it, never the enemy's or a piece to place", () => {
    expect(tickable(view.pieces.supply, "axis")).toBe(true);
    expect(tickable(view.pieces.enemy, "axis")).toBe(false);
    expect(tickable({ ...view.pieces.bn, location: { box: "tripoli" } } as Piece, "axis")).toBe(true);
    expect(tickable({ ...view.pieces.bn, location: { pending: {} } } as unknown as Piece, "axis")).toBe(false);
    expect(tickable(view.pieces.bn, null)).toBe(false);
  });
});

describe("toggle", () => {
  it("adds a piece not ticked and removes one that is", () => {
    expect(toggle(["a"], "b")).toEqual(["a", "b"]);
    expect(toggle(["a", "b"], "a")).toEqual(["b"]);
  });
});

describe("cpLabel", () => {
  it("writes halves with the ½ sign", () => {
    expect(cpLabel(0.5)).toBe("½");
    expect(cpLabel(1)).toBe("1");
    expect(cpLabel(2.5)).toBe("2½");
    expect(cpLabel(0)).toBe("0");
  });
});

describe("extendPath", () => {
  it("appends a hex and takes the last one back off when clicked again", () => {
    expect(extendPath([], "C1")).toEqual(["C1"]);
    expect(extendPath(["C1"], "C2")).toEqual(["C1", "C2"]);
    expect(extendPath(["C1", "C2"], "C2")).toEqual(["C1"]);
  });
});

describe("byHex", () => {
  it("indexes reaches by hex", () => {
    const r = { hex: "C1", cp: 1, path: ["C1"], dp: 0, ends_in_zoc: false, may_be_interrupted: false, short: 0, fuel: 0 };
    expect(byHex([r]).get("C1")).toBe(r);
  });
});

describe("off-map", () => {
  const boxed = { ...piece("a", "axis", "combat", "C1"), location: { box: "tripoli" } } as Piece;
  const boxed2 = { ...piece("b", "axis", "combat", "C1"), location: { box: "tripoli" } } as Piece;
  const transit = { ...piece("t", "axis", "combat", "C1"), location: { transit: { from: "tripoli", to: "nofilia", done: 1 } } } as Piece;
  const nofilia = piece("n", "axis", "combat", NOFILIA);
  const v = { pieces: { a: boxed, b: boxed2, t: transit, n: nofilia, bn: view.pieces.bn } } as unknown as View;

  it("knows where an off-map move can start", () => {
    expect(offMapCapable(boxed)).toBe(true);
    expect(offMapCapable(transit)).toBe(true);
    expect(offMapCapable(nofilia)).toBe(true);
    expect(offMapCapable(view.pieces.bn)).toBe(false);
    expect(allOffMapCapable(v, ["a", "n"])).toBe(true);
    expect(allOffMapCapable(v, ["a", "bn"])).toBe(false);
    expect(allOffMapCapable(v, [])).toBe(false);
    expect(allOnMap(v, ["n", "bn"])).toBe(true);
    expect(allOnMap(v, ["a"])).toBe(false);
  });

  it("ticks pieces of one box together and starts afresh elsewhere", () => {
    expect(toggleOffMap(v, [], "a")).toEqual(["a"]);
    expect(toggleOffMap(v, ["a"], "b")).toEqual(["a", "b"]);
    expect(toggleOffMap(v, ["a", "b"], "a")).toEqual(["b"]);
    expect(toggleOffMap(v, ["bn"], "a")).toEqual(["a"]);
    expect(toggleOffMap(v, ["a"], "t")).toEqual(["t"]);
  });

  it("offers every place on the chart; the engine says which can hold a unit", () => {
    expect(OFF_MAP_PLACES.map((p) => p.place)).toEqual(["tunis", "gabes", "tripoli", "tripolitania", "nofilia"]);
  });
});

describe("attachmentOrder", () => {
  const moving = { phase: "movement-combat", segment: "movement" };
  const at = (pieces: Record<string, Partial<Piece>>, clock: object = moving, pending: unknown = null) =>
    ({
      clock,
      pending,
      to_act: "axis",
      pieces: Object.fromEntries(Object.entries(pieces).map(([id, p]) => [id, { id, side: "axis", kind: "combat", location: { hex: "C3617" }, parent: null, ...p }])),
    }) as unknown as View;

  it("detaches an attached unit and attaches an assigned one standing with its parent (19.4x)", () => {
    const v = at({
      hq: { kind: "hq" },
      bn: { parent: "hq", attached_to: "hq" },
      loose: { parent: "hq" },
      away: { parent: "hq", location: { hex: "C3618" } },
      free: {},
    });
    expect(attachmentOrder(v, v.pieces.bn)).toEqual({ detach: { piece: "bn" } });
    expect(attachmentOrder(v, v.pieces.loose)).toEqual({ attach: { piece: "loose", to: "hq" } });
    expect(attachmentOrder(v, v.pieces.away)).toBeNull();
    expect(attachmentOrder(v, v.pieces.free)).toBeNull();
    expect(attachmentOrder(v, v.pieces.hq)).toBeNull();
  });

  it("offers nothing outside phase C and the movement segment, while something is pending, or for the other side", () => {
    const pieces = { hq: { kind: "hq" as const }, bn: { parent: "hq", attached_to: "hq" } };
    expect(attachmentOrder(at(pieces, { phase: "organisation" }), at(pieces).pieces.bn)).toEqual({ detach: { piece: "bn" } });
    for (const v of [at(pieces, { phase: "movement-combat", segment: "close-assault" }), at(pieces, moving, { reaction: {} })]) {
      expect(attachmentOrder(v, v.pieces.bn)).toBeNull();
    }
    const enemy = at({ hq: { kind: "hq", side: "cw" }, bn: { side: "cw", parent: "hq", attached_to: "hq" } });
    expect(attachmentOrder(enemy, enemy.pieces.bn)).toBeNull();
  });
});

describe("reassignment and attaching elsewhere in phase C", () => {
  const at = (pieces: Record<string, Partial<Piece>>, clock: object = { phase: "organisation" }) =>
    ({
      clock,
      pending: null,
      to_act: "axis",
      pieces: Object.fromEntries(Object.entries(pieces).map(([id, p]) => [id, { id, side: "axis", kind: "combat", location: { hex: "C3617" }, parent: null, ...p }])),
    }) as unknown as View;
  const pieces = {
    own: { kind: "hq" as const },
    other: { kind: "hq" as const },
    far: { kind: "hq" as const, location: { hex: "C3618" } },
    enemy: { kind: "hq" as const, side: "cw" as const },
    bn: { parent: "own" },
    loose: { attached_to: "other" },
    kept: { parent: "own", attached_to: "other" },
  };

  it("offers attaching to any other HQ of the side in the hex (19.4x)", () => {
    const v = at(pieces);
    expect(attachElsewhereOrders(v, v.pieces.bn)).toEqual([{ attach: { piece: "bn", to: "other" } }]);
    expect(attachElsewhereOrders(v, v.pieces.loose)).toEqual([]);
    const moving = at(pieces, { phase: "movement-combat", segment: "movement" });
    expect(attachElsewhereOrders(moving, moving.pieces.bn)).toEqual([]);
  });

  it("offers reassignment to an independent unit attached to a parent (19.2x)", () => {
    const v = at(pieces);
    expect(reassignOrder(v, v.pieces.loose)).toEqual({ reassign: { piece: "loose" } });
    expect(reassignOrder(v, v.pieces.kept)).toBeNull();
    expect(reassignOrder(v, v.pieces.bn)).toBeNull();
    const moving = at(pieces, { phase: "movement-combat", segment: "movement" });
    expect(reassignOrder(moving, moving.pieces.loose)).toBeNull();
  });

  it("offers the shifts of parent the engine dates to this month (19.24)", () => {
    const v = { ...at(pieces), parent_shifts: [{ piece: "bn", to: "other" }, { piece: "kept", to: null }] } as View;
    expect(shiftOrders(v, v.pieces.bn)).toEqual([{ "shift-parent": { piece: "bn", to: "other" } }]);
    expect(shiftOrders(v, v.pieces.kept)).toEqual([{ "shift-parent": { piece: "kept", to: null } }]);
    expect(shiftOrders(v, v.pieces.loose)).toEqual([]);
    const moving = { ...at(pieces, { phase: "movement-combat", segment: "movement" }), parent_shifts: v.parent_shifts } as View;
    expect(shiftOrders(moving, moving.pieces.bn)).toEqual([]);
  });
});

describe("swapping Commonwealth battalions between brigades in phase C (19.2x)", () => {
  const at = (pieces: Record<string, Partial<Piece>>, extra: object = {}) =>
    ({
      clock: { phase: "organisation" },
      pending: null,
      to_act: "cw",
      ...extra,
      pieces: Object.fromEntries(
        Object.entries(pieces).map(([id, p]) => [id, { id, side: "cw", kind: "combat", equivalent: "battalion", location: { hex: "E1830" }, parent: null, ...p }]),
      ),
    }) as unknown as View;
  const pieces = {
    a: { parent: "5-in" },
    b: { parent: "11-in", location: { hex: "E2132" } },
    same: { parent: "5-in" },
    loose: {},
    attached: { parent: "11-in", attached_to: "hq" },
    carrier: { parent: "16-bde" },
    rider: { parent: "16-bde", attached_to: "carrier" },
    brigade: { parent: "4-div", equivalent: "brigade" as const },
    gone: { parent: "11-in", location: "eliminated" as const },
    axis: { parent: "x", side: "axis" as const },
  };

  it("offers every other battalion standing alone outside its brigade, anywhere on the map, independents included", () => {
    const v = at(pieces);
    expect(swapOrders(v, v.pieces.a).map((o) => o["swap-battalions"].with)).toEqual(["b", "loose"]);
  });

  it("offers nothing for a battalion not standing alone, with no parent, or not a battalion", () => {
    const v = at(pieces);
    expect(swapOrders(v, v.pieces.attached)).toEqual([]);
    expect(swapOrders(v, v.pieces.carrier)).toEqual([]);
    expect(swapOrders(v, v.pieces.loose)).toEqual([]);
    expect(swapOrders(v, v.pieces.brigade)).toEqual([]);
  });

  it("offers nothing outside the Commonwealth's phase C or with a step pending", () => {
    expect(swapOrders(at(pieces, { to_act: "axis" }), at(pieces).pieces.a)).toEqual([]);
    const moving = at(pieces, { clock: { phase: "movement-combat", segment: "movement" } });
    expect(swapOrders(moving, moving.pieces.a)).toEqual([]);
    const pending = at(pieces, { pending: { "retreat-before-assault": {} } });
    expect(swapOrders(pending, pending.pieces.a)).toEqual([]);
  });
});

describe("the standing loss order a unit's owner sets (12.45, 14.4x, 15.83)", () => {
  const unit = (extra: Partial<Piece> = {}) =>
    ({ id: "rtr", side: "cw", kind: "combat", location: { hex: "C4021" }, weapons: [{ weapon: "A9", points: 4 }, { weapon: "A10", points: 2 }, { weapon: "A13", points: 1 }], ...extra }) as Piece;
  const v = { pending: null, to_act: "cw", pieces: {} } as unknown as View;

  it("puts the order set first, then the other entries as listed", () => {
    expect(lossOrderDraft(v, unit())).toEqual(["A9", "A10", "A13"]);
    expect(lossOrderDraft(v, unit({ loss_order: ["A13"] }))).toEqual(["A13", "A9", "A10"]);
  });

  it("offers nothing for an enemy unit, while something is pending, or with fewer than two entries with points", () => {
    expect(lossOrderDraft({ ...v, to_act: "axis" } as View, unit())).toBeNull();
    expect(lossOrderDraft({ ...v, pending: { "forced-retreat": {} } } as unknown as View, unit())).toBeNull();
    expect(lossOrderDraft(v, unit({ weapons: [{ weapon: "A9", points: 4 }, { weapon: "A10", points: 0 }] }))).toBeNull();
    expect(lossOrderDraft(v, unit({ weapons: undefined }))).toBeNull();
  });

  it("raises an entry one place, and leaves the first where it is", () => {
    expect(raise(["a", "b", "c"], 2)).toEqual(["a", "c", "b"]);
    expect(raise(["a", "b", "c"], 0)).toEqual(["a", "b", "c"]);
  });
});
