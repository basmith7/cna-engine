// Rail stops (8.77, R-005): the stops a Commonwealth rail run makes on its
// way, where pieces aboard get off and pieces standing there get on. The
// engine judges the run as a whole; these only build the order.
import type { HexId, PieceId, View } from "./types";

export type RailStop = { at: HexId; off: PieceId[]; on: PieceId[] };

/** `stops` with a stop at `at` added, unless it is already one. */
export function addStop(stops: readonly RailStop[], at: HexId): RailStop[] {
  return stops.some((s) => s.at === at) ? [...stops] : [...stops, { at, off: [], on: [] }];
}

/** The pieces aboard as the train reaches stop `i`: the ticked stack, less those off and plus those on at earlier stops. */
export function aboardAt(ticked: readonly PieceId[], stops: readonly RailStop[], i: number): PieceId[] {
  let aboard = [...ticked];
  for (const s of stops.slice(0, i)) aboard = [...aboard.filter((id) => !s.off.includes(id)), ...s.on];
  return aboard;
}

/** Stop `i` with `id` named or dropped in its `field` list. */
export function toggleAtStop(stops: readonly RailStop[], i: number, field: "off" | "on", id: PieceId): RailStop[] {
  return stops.map((s, j) => {
    if (j !== i) return s;
    const list = s[field].includes(id) ? s[field].filter((x) => x !== id) : [...s[field], id];
    return { ...s, [field]: list };
  });
}

/** The side's pieces standing loose at `at` that could get on a train: units, HQs and supply, not attached. */
export function waitingAt(view: View, at: HexId, ticked: readonly PieceId[]): PieceId[] {
  return Object.values(view.pieces)
    .filter((p) => p.side === view.to_act && !p.attached_to && !ticked.includes(p.id))
    .filter((p) => p.kind === "combat" || p.kind === "hq" || p.kind === "supply-unit" || p.kind === "dummy-supply-unit")
    .filter((p) => typeof p.location === "object" && "hex" in p.location && p.location.hex === at)
    .map((p) => p.id);
}

/** The stops the run to the end of `path` passes, in its order: those on the path before its end, each with someone getting off or on. */
export function stopsOn(stops: readonly RailStop[], path: readonly HexId[]): RailStop[] {
  const before = path.slice(0, -1);
  return stops
    .filter((s) => before.includes(s.at) && s.off.length + s.on.length > 0)
    .sort((a, b) => before.indexOf(a.at) - before.indexOf(b.at));
}
