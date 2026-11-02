// Repaired points waiting as replacements (22.5, 20.41): the side to
// act's own, by hex and weapon system; a unit in the hex absorbs them in
// phase C.
import type { HexId, View } from "./types";

/** The side to act's repaired points waiting in `hex`, by weapon system. */
export function repairedAt(view: View, hex: HexId): { weapon: string; points: number }[] {
  if (!view.to_act) return [];
  return Object.entries(view.repaired?.[view.to_act]?.[hex] ?? {}).map(([weapon, points]) => ({ weapon, points }));
}
