import { describe, expect, it } from "vitest";
import { clockLine, toActLine } from "./clock";
import type { View } from "./types";

function view(partial: Partial<View>): View {
  return {
    scenario: "grazianis-offensive",
    rules_commit: "x",
    ruleset: {},
    clock: { game_turn: 1, stage: 1, phase: "initiative-declaration", half: null, segment: null },
    initiative: null,
    player_a: null,
    weather: "normal",
    pieces: {},
    pending: null,
    cycle: 0,
    moved: [],
    zoc: {},
    continuing: [],
    shifted: [],
    combat: {},
    over: null,
    to_act: "axis",
    ...partial,
  };
}

describe("top bar lines", () => {
  it("names the set-up while placements are pending", () => {
    const v = view({ pending: { "set-up": { side: "axis" } } });
    expect(clockLine(v)).toBe("Game-Turn 1 · set-up");
    expect(toActLine(v)).toBe("Axis to act");
  });

  it("spells out stage, phase, half and segment in play", () => {
    const v = view({
      clock: { game_turn: 3, stage: 2, phase: "movement-combat", half: "B", segment: "close-assault" },
      to_act: "cw",
    });
    expect(clockLine(v)).toBe("Game-Turn 3 · OpStage 2 · G · movement and combat · Player B · close assault");
    expect(toActLine(v)).toBe("Commonwealth to act");
  });

  it("says game over when nobody is to act", () => {
    expect(toActLine(view({ to_act: null }))).toBe("Game over");
  });
});
