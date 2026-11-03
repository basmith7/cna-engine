// An enemy piece as `view_for` sends it under limited intelligence (3.61):
// only `ENEMY_PIECE_KEEPS`, no TOE, morale or cohesion. Each component
// that draws those shows "?" for them, rendered as the page would.
import { render } from "svelte/server";
import { describe, expect, it } from "vitest";
import Counter from "./Counter.svelte";
import StackPanel from "./StackPanel.svelte";
import type { Piece, View } from "./types";

const HEX = "C4131";

/** A Commonwealth battalion seen by the Axis: the fields the enemy keeps, nothing else. */
const enemy: Piece = {
  id: "unit:cw:x-bn",
  side: "cw",
  kind: "combat",
  nation: "cw",
  id_code: "x",
  cpa: "10",
  equivalent: "battalion",
  location: { hex: HEX },
  contact: false,
  engaged: false,
  unit_type: "Infantry Bn-Eq",
  name: "A battalion",
  abbreviation: "A bn",
};

const view = {
  scenario: "grazianis-offensive",
  rules_commit: "0".repeat(40),
  ruleset: {},
  clock: { game_turn: 1, stage: 1, phase: "movement-combat", half: null, segment: "movement" },
  initiative: null,
  player_a: null,
  assaults_to_resolve: [],
  weather: "normal",
  pieces: { [enemy.id]: enemy },
  pending: null,
  cycle: 0,
  moved: [],
  zoc: {},
  continuing: [],
  shifted: [],
  combat: {},
  over: null,
  to_act: "axis",
} as unknown as View;

const text = (html: string) => html.replace(/<!--[^>]*-->/g, "").replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");

describe("an enemy piece without its hidden fields", () => {
  it("counter: its strength is a ?", () => {
    const { body } = render(Counter, { props: { piece: enemy } });
    expect(body).toMatch(/<text class="toe[^"]*"[^>]*>\?<\/text>/);
    expect(body).toMatch(/<text class="cpa[^"]*"[^>]*>10<\/text>/);
  });

  it("stack panel: TOE, morale and cohesion are each a ?", () => {
    const { body } = render(StackPanel, { props: { view, hex: HEX } });
    expect(text(body)).toContain("TOE ? · CPA 10 · morale ? · cohesion ?");
  });
});
