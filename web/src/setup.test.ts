import { describe, expect, it } from "vitest";
import { placementWords } from "./setup";

describe("a placement in words", () => {
  it("names a sheet, with the not-within rule", () => {
    expect(placementWords({ sheets: ["C"], not_within: { of: "enemy-unit", hexes: 4 } })).toBe(
      "any hex of sheet C, not within 4 hexes of an enemy unit",
    );
    expect(placementWords({ sheets: ["D", "E"] })).toBe("any hex of sheets D or E");
    expect(placementWords({ sheets: ["A", "B", "C"] })).toBe("any hex of sheets A, B or C");
  });

  it("names a region, a radius and a hex list", () => {
    expect(placementWords({ region: "libya" })).toBe("any hex in Libya");
    expect(placementWords({ within: { of: "C0716", hexes: 3 } })).toBe("any hex within 3 of C0716");
    expect(placementWords({ hexes: ["C4131"] })).toBe("C4131");
    expect(placementWords({ hexes: ["C4131", "C4130"] })).toBe("any of C4131 or C4130");
  });

  it("does not repeat the constraint prose: that is the tray's 'also'", () => {
    expect(placementWords({ sheets: ["C"], constraint: "Libyan part of sheet C only" })).toBe("any hex of sheet C");
  });
});
