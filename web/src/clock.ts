// The top bar's words for the engine's clock: our names for cna's
// phases (rules/20-sequence-of-play.md), not SPI text.
import type { Phase, Segment, Side, View, Weather } from "./types";

export const PHASE_NAMES: Record<Phase, string> = {
  "initiative-declaration": "A · initiative",
  weather: "B · weather",
  organisation: "C · organisation",
  arrival: "D · arrival",
  "cw-fleet": "E · CW fleet",
  "reserve-designation": "F · reserve designation",
  "movement-combat": "G · movement and combat",
  "truck-convoy": "H · truck convoy",
  rail: "J · rail",
  repair: "K · repair",
  patrol: "L · patrol",
};

export const SEGMENT_NAMES: Record<Segment, string> = {
  movement: "movement",
  breakdown: "breakdown",
  "position-determination": "gun positions",
  barrage: "barrage",
  "retreat-before-assault": "retreat before assault",
  "force-assignment": "force assignment",
  "anti-armour": "anti-armour",
  "close-assault": "close assault",
  "reserve-release": "reserve release",
};

export function sideName(side: Side | null): string {
  return side === "axis" ? "Axis" : side === "cw" ? "Commonwealth" : "nobody";
}

export function weatherName(w: Weather): string {
  if (typeof w === "string") return w;
  if ("sandstorm" in w) return `sandstorm (${w.sandstorm.sheets.join(", ")})`;
  return `rainstorm (${w.rainstorm.sheets.join(", ")})`;
}

/** "Game-Turn 1 · set-up" or "Game-Turn 1 · OpStage 2 · G · movement and combat · Player A · movement". */
export function clockLine(view: View): string {
  const c = view.clock;
  const parts = [`Game-Turn ${c.game_turn}`];
  if (view.pending && "set-up" in view.pending) {
    parts.push("set-up");
    return parts.join(" · ");
  }
  parts.push(`OpStage ${c.stage}`, PHASE_NAMES[c.phase]);
  if (c.half) parts.push(`Player ${c.half}`);
  if (c.segment) parts.push(SEGMENT_NAMES[c.segment]);
  return parts.join(" · ");
}

/** "Axis to act", or "Game over" once the engine stops taking orders. */
export function toActLine(view: View): string {
  return view.to_act ? `${sideName(view.to_act)} to act` : "Game over";
}
