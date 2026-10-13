// The movement UI's own state (spec, *Moving*): which pieces of the
// selected hex are ticked, how a CP figure reads, and the custom path
// shift-clicks build. Pure functions; the engine prices every step.
import type { HexId, Piece, PieceId, Reach, Side, View } from "./types";

/** The pieces of `hex` that start ticked for `side`: its own combat units and HQs, top of the stack first. Supply units are inert, so they start unticked. */
export function defaultTicked(view: View, hex: HexId, side: Side | null): PieceId[] {
  if (!side) return [];
  return Object.values(view.pieces)
    .filter((p) => p.side === side && typeof p.location === "object" && "hex" in p.location && p.location.hex === hex)
    .filter((p) => p.kind === "combat" || p.kind === "hq")
    .map((p) => p.id);
}

/** Whether a piece may be ticked at all: the side to act's, on the map. */
export function tickable(p: Piece, side: Side | null): boolean {
  return p.side === side && typeof p.location === "object" && ("hex" in p.location || "box" in p.location || "transit" in p.location);
}

export function toggle(ticked: PieceId[], id: PieceId): PieceId[] {
  return ticked.includes(id) ? ticked.filter((x) => x !== id) : [...ticked, id];
}

/** "½", "2", "2½": the engine's CP as a number of points, in halves. */
export function cpLabel(cp: number): string {
  const whole = Math.floor(cp);
  const half = cp - whole >= 0.5;
  if (whole === 0 && half) return "½";
  return `${whole}${half ? "½" : ""}`;
}

/** The reach by hex, for a click or a hover to look up. */
export function byHex(reaches: Reach[]): Map<HexId, Reach> {
  return new Map(reaches.map((r) => [r.hex, r]));
}

/**
 * A shift-click extends the custom path by the hex clicked; the engine
 * alone judges it when it is sent, so nothing is checked here except
 * that clicking the last hex again takes it back off.
 */
export function extendPath(path: HexId[], hex: HexId): HexId[] {
  if (path.length > 0 && path[path.length - 1] === hex) return path.slice(0, -1);
  return [...path, hex];
}
