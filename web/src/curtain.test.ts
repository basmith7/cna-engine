import { describe, expect, it } from "vitest";
import { curtainAfter } from "./curtain";

describe("the hot-seat curtain", () => {
  it("drops when the side to act changes", () => {
    expect(curtainAfter("axis", "cw")).toBe("cw");
    expect(curtainAfter("cw", "axis")).toBe("axis");
  });

  it("stays up while the same side goes on", () => {
    expect(curtainAfter("axis", "axis")).toBeNull();
    expect(curtainAfter("cw", "cw")).toBeNull();
  });

  it("has nobody to show once the game is over", () => {
    expect(curtainAfter("axis", null)).toBeNull();
    expect(curtainAfter(null, null)).toBeNull();
  });
});
