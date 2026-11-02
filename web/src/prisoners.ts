// Prisoner points (§28): open information, each captor's by hex; the
// side to act marches its own in its truck convoy phase, guarded by
// guard points (28.2x), and escapees wait where they got away.
import type { HexId, Side, View } from "./types";

type BySide = Partial<Record<Side, Record<HexId, number>>> | undefined;

function at(map: BySide, hex: HexId): { side: Side; points: number }[] {
  return (["axis", "cw"] as Side[]).flatMap((side) => {
    const points = map?.[side]?.[hex];
    return points ? [{ side, points }] : [];
  });
}

/** Every captor's prisoner points in `hex`. */
export function prisonersAt(view: View, hex: HexId): { side: Side; points: number }[] {
  return at(view.prisoners, hex);
}

/** Every side's guard points in `hex` (28.21). */
export function guardsAt(view: View, hex: HexId): { side: Side; points: number }[] {
  return at(view.guards, hex);
}

/** Escapees in `hex` by their own side (28.25). */
export function escapeesAt(view: View, hex: HexId): { side: Side; points: number }[] {
  return at(view.escapees, hex);
}

/** Whether the side to act may detail or release guards now: phase C, or its own phase H (28.21). */
export function guardPhase(view: View): boolean {
  return !!view.to_act && (view.clock?.phase === "organisation" || view.clock?.phase === "truck-convoy");
}

/** The side to act's prisoners that may still march this phase H (28.13): not in a hex marched into. */
export function marchable(view: View): { hex: HexId; points: number }[] {
  if (!view.to_act || view.clock?.phase !== "truck-convoy") return [];
  const done = new Set(view.marched ?? []);
  return Object.entries(view.prisoners?.[view.to_act] ?? {})
    .filter(([hex]) => !done.has(hex))
    .map(([hex, points]) => ({ hex, points }));
}
