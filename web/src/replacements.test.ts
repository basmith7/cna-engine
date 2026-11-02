import { describe, expect, it } from "vitest";
import { repairedAt } from "./replacements";
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
