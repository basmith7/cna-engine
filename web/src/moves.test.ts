import { describe, expect, it } from "vitest";
import { byHex, cpLabel, defaultTicked, extendPath, tickable, toggle } from "./moves";
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
    const r = { hex: "C1", cp: 1, path: ["C1"], dp: 0, ends_in_zoc: false, may_be_interrupted: false };
    expect(byHex([r]).get("C1")).toBe(r);
  });
});
