import { describe, expect, it } from "vitest";
import { assignOrder, barrageOrder, committed, fireRowFor, passConsequence, retreatOrderWords, roleRowFor } from "./sealed";
import type { Committer, Firer, PieceId } from "./types";

const gun: Firer = { piece: "u:gun", points: 6, forward: true, targets: ["u:krrc", "u:cldgds"], ammo: 0, ammo_short: false };
const bn: Committer = { piece: "u:bn", points: 5, anti_armour: false, anti_armour_targets: [], close_assault_targets: ["C3922"], one_assault: false, ammo: 0, ammo_short: false };
const tanks: Committer = { piece: "u:tanks", points: 4, anti_armour: true, anti_armour_targets: ["C3922"], close_assault_targets: ["C3922", "C3921"], one_assault: true, ammo: 0, ammo_short: false };

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
  const name = (id: PieceId) => ({ "u:krrc": "1st KRRC", "u:gds": "3rd Coldstream Guards" })[id] ?? id;
  it("spells out what Pass does from the engine's preview of it", () => {
    const paid = passConsequence(
      {
        ok: [
          { "retreat-bought-out": { pieces: ["u:krrc", "u:gds"], short: 2 } },
          { losses: { piece: "u:krrc", points: 2, toe: 4 } },
          { losses: { piece: "u:gds", points: 1, toe: 5 } },
        ],
      },
      name,
    );
    expect(paid).toEqual({
      text: "Pass stands where it is, 2 hexes short of the order, and loses 3 points for it: 1st KRRC 2, 3rd Coldstream Guards 1.",
      button: "Pass: stand and pay",
    });
    expect(passConsequence({ ok: [{ "retreat-bought-out": { pieces: ["u:krrc"], short: 1 } }, { losses: { piece: "u:krrc", points: 1, toe: 0 } }, { eliminated: { piece: "u:krrc" } }] }, name)).toEqual({
      text: "Pass stands where it is, 1 hex short of the order, and 1st KRRC loses 1 point for it; 1st KRRC eliminated.",
      button: "Pass: stand and pay",
    });
    expect(passConsequence({ ok: [] }, name)).toEqual({ text: "Pass stands where it is at no cost: the stack already meets the order.", button: "Pass: stand" });
    expect(passConsequence({ ok: [{ losses: { piece: "u:krrc", points: 5, toe: 0 } }, { surrendered: { pieces: ["u:krrc"] } }] }, name)).toEqual({
      text: "Pass surrenders 1st KRRC.",
      button: "Pass: surrender",
    });
    expect(passConsequence({ ok: [{ disorganised: { piece: "u:krrc", dp: 3, level: -3 } }] }, name)).toEqual({
      text: "Pass stands where it is: 1st KRRC takes 3 DP, now cohesion level -3.",
      button: "Pass: stand",
    });
    expect(passConsequence({ illegal: { case: "15.82", reason: "the stack must retreat the full 3 hexes (R-012)" } }, name)).toEqual({
      text: "Pass would be refused: the stack must retreat the full 3 hexes (R-012) (15.82).",
      button: "Pass",
    });
  });
  it("puts the order in words", () => {
    expect(retreatOrderWords({ side: "cw", hexes: 2, from: ["C4021", "C4021"], reason: "assault-result" })).toBe("Commonwealth must retreat 2 hexes from C4021 for the assault result.");
    expect(retreatOrderWords({ side: "axis", hexes: 3, from: ["C3922", "C3923"], reason: "must-attack" })).toBe("Axis must retreat 3 hexes from C3922, C3923 for neither assaulting nor holding off the enemy.");
  });
});
