import { describe, expect, it } from "vitest";
import { marchable, prisonersAt } from "./prisoners";
import type { View } from "./types";

const view = {
  to_act: "axis",
  clock: { phase: "truck-convoy" },
  prisoners: { axis: { C4021: 12, C4120: 5 }, cw: { C4021: 3 } },
  marched: ["C4120"],
} as unknown as View;

describe("prisoners in a hex", () => {
  it("lists each captor's points there, open to both sides (28.1x)", () => {
    expect(prisonersAt(view, "C4021")).toEqual([
      { side: "axis", points: 12 },
      { side: "cw", points: 3 },
    ]);
    expect(prisonersAt(view, "C4022")).toEqual([]);
    expect(prisonersAt({ ...view, prisoners: undefined }, "C4021")).toEqual([]);
  });
});

describe("prisoners that may march", () => {
  it("are the side to act's, in its truck convoy phase, not yet marched in (28.13)", () => {
    expect(marchable(view)).toEqual([{ hex: "C4021", points: 12 }]);
  });

  it("are none outside phase H or with nobody to act", () => {
    expect(marchable({ ...view, clock: { ...view.clock, phase: "rail" } })).toEqual([]);
    expect(marchable({ ...view, to_act: null })).toEqual([]);
  });
});
