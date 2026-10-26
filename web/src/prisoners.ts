// Prisoner points (§28): open information, each captor's by hex; the
// side to act marches its own in its truck convoy phase.
import type { HexId, Side, View } from "./types";

/** Every captor's prisoner points in `hex`. */
export function prisonersAt(view: View, hex: HexId): { side: Side; points: number }[] {
  return (["axis", "cw"] as Side[]).flatMap((side) => {
    const points = view.prisoners?.[side]?.[hex];
    return points ? [{ side, points }] : [];
  });
}

/** The side to act's prisoners that may still march this phase H (28.13): not in a hex marched into. */
export function marchable(view: View): { hex: HexId; points: number }[] {
  if (!view.to_act || view.clock?.phase !== "truck-convoy") return [];
  const done = new Set(view.marched ?? []);
  return Object.entries(view.prisoners?.[view.to_act] ?? {})
    .filter(([hex]) => !done.has(hex))
    .map(([hex, points]) => ({ hex, points }));
}
