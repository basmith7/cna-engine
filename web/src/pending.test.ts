import { describe, expect, it } from "vitest";
import { pendingKind, pendingTitle, zocQuestion } from "./pending";
import type { Pending } from "./types";

const all: Pending[] = [
  { "set-up": { side: "axis" } },
  { "declare-zoc": { side: "cw", hexes: ["C3922"], at_start: true } },
  { reaction: { side: "cw", eligible: ["unit:cw:4-rha"] } },
  { sealed: { step: "barrage", waiting: ["axis", "cw"] } },
  { "retreat-before-assault": { side: "cw" } },
  { "forced-retreat": { side: "cw", pieces: [], hexes: 2, from: ["C4021"], reason: "assault-result", raw: 3, overrun: false } },
];

describe("pending", () => {
  it("names every variant", () => {
    expect(all.map(pendingKind)).toEqual(["set-up", "declare-zoc", "reaction", "sealed", "retreat-before-assault", "forced-retreat"]);
    expect(all.map(pendingTitle)).toEqual([
      "Set-up · Axis",
      "ZOC declaration · Commonwealth",
      "Reaction · Commonwealth",
      "Sealed barrage · Axis",
      "Retreat before assault · Commonwealth",
      "Forced retreat · Commonwealth",
    ]);
  });
  it("throws, not guesses, on a variant it does not know", () => {
    expect(() => pendingKind({ teleport: {} } as unknown as Pending)).toThrow(/unknown pending/);
  });
  it("asks the ZOC question for the moment", () => {
    expect(zocQuestion({ hexes: ["C3922"], at_start: true })).toMatch(/^The movement segment opens.*this hex of yours exerts/);
    expect(zocQuestion({ hexes: ["C3922", "C3721"], at_start: false })).toMatch(/^An enemy stack is about to move.*which of these 2 hexes/);
  });
});
