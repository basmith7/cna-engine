// Repaired points waiting as replacements (22.5, 20.41): the side to
// act's own, by hex and weapon system; a unit in the hex absorbs them in
// phase C.
import type { HexId, PieceId, View } from "./types";

/** The side to act's repaired points waiting in `hex`, by weapon system. */
export function repairedAt(view: View, hex: HexId): { weapon: string; points: number }[] {
  if (!view.to_act) return [];
  return Object.entries(view.repaired?.[view.to_act]?.[hex] ?? {}).map(([weapon, points]) => ({ weapon, points }));
}

/**
 * The units the side to act's broken-down `weapon` in `hex` belongs to,
 * one each, in the order held: a repair may name one to go to first
 * (61.38). Empty when there is nothing to choose: no owned wreck, or a
 * single wreck that the plain repair reaches first anyway.
 */
export function wreckOwners(view: View, hex: HexId, weapon: string): PieceId[] {
  const wrecks = (view.broken_down?.[hex] ?? []).filter((w) => w.side === view.to_act && w.weapon === weapon && w.points > 0);
  const owners = [...new Set(wrecks.flatMap((w) => (w.unit ? [w.unit] : [])))];
  return wrecks.length > 1 ? owners : [];
}
