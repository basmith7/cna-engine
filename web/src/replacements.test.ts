import { describe, expect, it } from "vitest";
import { repairedAt, wreckOwners } from "./replacements";
import type { View } from "./types";

describe("repaired points in a hex", () => {
  const view = { to_act: "axis", repaired: { axis: { C3920: { "M11/39": 3, "armoured-car": 1 } } } } as unknown as View;

  it("are the side to act's, by weapon system (20.41)", () => {
    expect(repairedAt(view, "C3920")).toEqual([
      { weapon: "M11/39", points: 3 },
      { weapon: "armoured-car", points: 1 },
    ]);
  });

  it("are none elsewhere or with nobody to act", () => {
    expect(repairedAt(view, "C3921")).toEqual([]);
    expect(repairedAt({ ...view, to_act: null }, "C3920")).toEqual([]);
  });
});

describe("the owners a repair may go to first (61.38)", () => {
  const wreck = (weapon: string, points: number, unit?: string, side = "cw") => ({ side, weapon, points, ...(unit ? { unit } : {}) });
  const view = (wrecks: object[]) => ({ to_act: "cw", broken_down: { E3613: wrecks } }) as unknown as View;

  it("lists each owner of the weapon's wrecks once, in the order held", () => {
    const v = view([wreck("Matilda", 2, "unit:cw:7-rtr"), wreck("Matilda", 1), wreck("Matilda", 3, "unit:cw:4-rtr"), wreck("Matilda", 1, "unit:cw:7-rtr"), wreck("Cruiser", 2, "unit:cw:2-rtr")]);
    expect(wreckOwners(v, "E3613", "Matilda")).toEqual(["unit:cw:7-rtr", "unit:cw:4-rtr"]);
  });

  it("offers no choice for a lone wreck, loose points only, or the enemy's", () => {
    expect(wreckOwners(view([wreck("Matilda", 2, "unit:cw:7-rtr")]), "E3613", "Matilda")).toEqual([]);
    expect(wreckOwners(view([wreck("Matilda", 2), wreck("Matilda", 1)]), "E3613", "Matilda")).toEqual([]);
    expect(wreckOwners(view([wreck("M13/40", 2, "unit:it:a", "axis"), wreck("M13/40", 2, "unit:it:b", "axis")]), "E3613", "M13/40")).toEqual([]);
    expect(wreckOwners(view([]), "C0101", "Matilda")).toEqual([]);
  });
});
