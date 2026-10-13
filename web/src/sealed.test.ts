import { describe, expect, it } from "vitest";
import { assignOrder, barrageOrder, committed, fireRowFor, passConsequence, retreatOrderWords, roleRowFor } from "./sealed";
import type { Committer, Firer } from "./types";

const gun: Firer = { piece: "u:gun", points: 6, forward: true, targets: ["u:krrc", "u:cldgds"] };
const bn: Committer = { piece: "u:bn", points: 5, anti_armour: false, anti_armour_targets: [], close_assault_targets: ["C3922"], one_assault: false };
const tanks: Committer = { piece: "u:tanks", points: 4, anti_armour: true, anti_armour_targets: ["C3922"], close_assault_targets: ["C3922", "C3921"], one_assault: true };

describe("the barrage panel", () => {
  it("starts a firer's row at its whole TOE and first target, and sends the rows as fires", () => {
    const row = fireRowFor(gun);
    expect(row).toEqual({ piece: "u:gun", points: 6, target: "u:krrc" });
    expect(barrageOrder([row, { ...row, points: 2, target: "u:cldgds" }])).toEqual({
      barrage: { fires: [{ piece: "u:gun", points: 6, target: "u:krrc" }, { piece: "u:gun", points: 2, target: "u:cldgds" }] },
    });
    expect(barrageOrder([])).toEqual({ barrage: { fires: [] } });
  });
});

describe("the assignment panel", () => {
  it("names a hex for the phasing side only", () => {
    expect(roleRowFor(bn, "close-assault", true)).toEqual({ piece: "u:bn", role: "close-assault", points: 5, target: "C3922" });
    expect(roleRowFor(bn, "close-assault", false)).toEqual({ piece: "u:bn", role: "close-assault", points: 5, target: "" });
    expect(roleRowFor(bn, "anti-armour", true).target).toBe("");
  });

  it("groups rows by unit into assignments, each role only where it has rows", () => {
    const rows = [roleRowFor(bn, "close-assault", true), roleRowFor(tanks, "anti-armour", true), { ...roleRowFor(tanks, "close-assault", true), points: 2 }];
    expect(assignOrder(rows, true)).toEqual({
      "assign-forces": {
        assignments: [
          { piece: "u:bn", close_assault: [{ target: "C3922", points: 5 }] },
          { piece: "u:tanks", anti_armour: [{ target: "C3922", points: 4 }], close_assault: [{ target: "C3922", points: 2 }] },
        ],
      },
    });
    expect(assignOrder([roleRowFor(bn, "close-assault", false)], false)).toEqual({
      "assign-forces": { assignments: [{ piece: "u:bn", close_assault: [{ points: 5 }] }] },
    });
    expect(assignOrder([], true)).toEqual({ "assign-forces": { assignments: [] } });
    expect(committed(rows, "u:tanks")).toBe(6);
    expect(committed(rows, "u:nobody")).toBe(0);
  });
});

describe("the forced retreat panel", () => {
  it("spells out what Pass does from the reason", () => {
    expect(passConsequence("must-attack", 3, 0)).toMatch(/^Pass surrenders the stack/);
    expect(passConsequence("assault-result", 2, 42)).toBe("Pass stands where it is and pays 10 % of its 42 raw points per hex short: about 8 points for all 2 (15.82, R-012).");
    expect(passConsequence("withheld", 1, 15)).toContain("about 1 point for all 1");
  });
  it("puts the order in words", () => {
    expect(retreatOrderWords({ side: "cw", hexes: 2, from: ["C4021", "C4021"], reason: "assault-result" })).toBe("Commonwealth must retreat 2 hexes from C4021 for the assault result.");
    expect(retreatOrderWords({ side: "axis", hexes: 3, from: ["C3922", "C3923"], reason: "must-attack" })).toBe("Axis must retreat 3 hexes from C3922, C3923 for neither assaulting nor holding off the enemy.");
  });
});
