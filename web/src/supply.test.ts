import { describe, expect, it } from "vitest";
import { fuelSummary, loadLabel, motorisationAt, sourceLabel } from "./supply";
import type { Reach, View } from "./types";

describe("a supply unit's load", () => {
  it("reads ammunition then fuel", () => {
    expect(loadLabel({ load: { ammo: 12, fuel: 30 } })).toBe("ammo 12 · fuel 30");
  });

  it("is nothing when the view hides it (the other side's, or module off)", () => {
    expect(loadLabel({})).toBeNull();
  });
});

describe("motorisation points in a hex", () => {
  const view = { to_act: "axis", motorisation: { axis: { C4807: 6, tripoli: 2 } } } as unknown as View;

  it("are the side to act's points there", () => {
    expect(motorisationAt(view, "C4807")).toBe(6);
  });

  it("are null where it has none, or when the module is off", () => {
    expect(motorisationAt(view, "C4808")).toBeNull();
    expect(motorisationAt({ ...view, motorisation: undefined }, "C4807")).toBeNull();
    expect(motorisationAt({ ...view, to_act: null }, "C4807")).toBeNull();
  });
});

describe("the fuel a stack's moves owe", () => {
  const r = (fuel: number) => ({ fuel }) as Reach;

  it("is nothing when no move burns fuel (or the module is off)", () => {
    expect(fuelSummary([])).toBeNull();
    expect(fuelSummary([r(0), r(0)])).toBeNull();
  });

  it("is one figure when every destination owes the same", () => {
    expect(fuelSummary([r(2), r(2)])).toBe("2 fuel");
  });

  it("is a range when going past the CPA owes more", () => {
    expect(fuelSummary([r(4), r(2), r(2)])).toBe("2–4 fuel");
  });
});

describe("a fuel source's label", () => {
  const view = {
    to_act: "axis",
    pieces: { "s:1": { id: "s:1", location: { hex: "C4807" }, load: { ammo: 5, fuel: 30 } } },
  } as unknown as View;

  it("says where it stands and what fuel it has", () => {
    expect(sourceLabel(view, "s:1", (id) => id.toUpperCase())).toBe("S:1 at C4807, fuel 30");
  });
});
