// The hot-seat curtain (spec, *Secrecy*): shown whenever an accepted
// order hands the keyboard to the other side, so the next player sees
// the screen only once they have it.
import type { Side } from "./types";

/**
 * The side to show the curtain for after an order moved `to_act` from
 * `before` to `after`; null when nobody new is at the keyboard (the same
 * side goes on, or the game is over).
 */
export function curtainAfter(before: Side | null, after: Side | null): Side | null {
  if (after === null || after === before) return null;
  return after;
}
