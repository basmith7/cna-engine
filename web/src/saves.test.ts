import { describe, expect, it } from "vitest";
import { describeSave, parseSeed, randomSeed, saveName } from "./saves";

describe("saveName", () => {
  it("names the file after the clock at the moment of saving", () => {
    expect(saveName({ clock: { game_turn: 1, stage: 1, phase: "initiative-declaration", half: null, segment: null } })).toBe("graziani-gt1-os1.json");
    expect(saveName({ clock: { game_turn: 7, stage: 3, phase: "movement-combat", half: "B", segment: "barrage" } })).toBe("graziani-gt7-os3.json");
  });
});

describe("parseSeed", () => {
  it("takes a whole number, with spaces around it", () => {
    expect(parseSeed("7")).toBe(7);
    expect(parseSeed(" 0 ")).toBe(0);
    expect(parseSeed("4294967295")).toBe(4294967295);
  });
  it("refuses anything the engine's u64 would not be given exactly", () => {
    for (const bad of ["", "-1", "1.5", "seven", "1e3", "99999999999999999999"]) expect(parseSeed(bad), bad).toBeNull();
  });
});

describe("randomSeed", () => {
  it("is a whole number the field accepts", () => {
    for (let i = 0; i < 20; i++) {
      const s = randomSeed();
      expect(parseSeed(String(s))).toBe(s);
    }
  });
});

describe("describeSave", () => {
  it("reads the clock line of a save and nothing from anything else", () => {
    expect(describeSave(JSON.stringify({ clock: { game_turn: 2, stage: 3, phase: "reserve-designation", half: "A", segment: null }, pending: null }))).toBe(
      "Game-Turn 2 · OpStage 3 · F · reserve designation · Player A",
    );
    expect(describeSave(JSON.stringify({ clock: { game_turn: 1, stage: 1, phase: "initiative-declaration", half: null, segment: null }, pending: { "set-up": { side: "axis" } } }))).toBe(
      "Game-Turn 1 · set-up",
    );
    expect(describeSave("not json")).toBeNull();
    expect(describeSave("{}")).toBeNull();
    expect(describeSave("[]")).toBeNull();
  });
});
