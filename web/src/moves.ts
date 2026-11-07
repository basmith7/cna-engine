// The movement UI's own state (spec, *Moving*): which pieces of the
// selected hex are ticked, how a CP figure reads, and the custom path
// shift-clicks build. Pure functions; the engine prices every step.
import { NOFILIA } from "./map";
import type { HexId, OffMapPlace, Order, Piece, PieceId, Reach, Side, View } from "./types";

/** The 8.89 chart's places, as `move-off-map` names them; the engine says which it can hold a unit at. */
export const OFF_MAP_PLACES: { place: OffMapPlace; name: string }[] = [
  { place: "tunis", name: "Tunis" },
  { place: "gabes", name: "Gabes" },
  { place: "tripoli", name: "Tripoli" },
  { place: "tripolitania", name: "Tripolitania" },
  { place: "nofilia", name: "Nofilia (the map)" },
];

/** Whether a piece stands where an off-map move can start: a box, in transit, or Nofilia. */
export function offMapCapable(p: Piece): boolean {
  const l = p.location;
  return typeof l === "object" && ("box" in l || "transit" in l || ("hex" in l && l.hex === NOFILIA));
}

/** Whether every ticked piece is on a hex, so `reachable` has a stack to price. */
export function allOnMap(view: View, ticked: PieceId[]): boolean {
  return ticked.length > 0 && ticked.every((id) => typeof view.pieces[id]?.location === "object" && "hex" in (view.pieces[id].location as object));
}

/** Whether every ticked piece could go off-map. */
export function allOffMapCapable(view: View, ticked: PieceId[]): boolean {
  return ticked.length > 0 && ticked.every((id) => view.pieces[id] && offMapCapable(view.pieces[id]));
}

/** Where a ticked piece is, as a key: its hex, box or transit leg; two pieces with the same key are a stack. */
export function whereKey(p: Piece): string | null {
  const l = p.location;
  if (typeof l !== "object") return null;
  if ("hex" in l) return `hex:${l.hex}`;
  if ("box" in l) return `box:${l.box}`;
  if ("transit" in l) return `transit:${l.transit.from}:${l.transit.to}:${l.transit.done}`;
  return null;
}

/** A click on an off-map piece: toggles it among ticks of the same place, or starts a fresh tick elsewhere. */
export function toggleOffMap(view: View, ticked: PieceId[], id: PieceId): PieceId[] {
  const key = whereKey(view.pieces[id]);
  const same = ticked.every((t) => whereKey(view.pieces[t]) === key);
  return same ? toggle(ticked, id) : [id];
}

/** The pieces of `hex` that start ticked for `side`: its own combat units and HQs, top of the stack first; in phases H and J its supply units, which alone move there. */
export function defaultTicked(view: View, hex: HexId, side: Side | null): PieceId[] {
  if (!side) return [];
  // Only supply units move in the truck convoy phase and by rail.
  const supplyOnly = view.clock?.phase === "truck-convoy" || view.clock?.phase === "rail";
  const here = Object.values(view.pieces).filter((p) => p.side === side && typeof p.location === "object" && "hex" in p.location && p.location.hex === hex);
  if (supplyOnly) return here.filter((p) => p.kind === "supply-unit" || p.kind === "dummy-supply-unit").map((p) => p.id);
  const units = here.filter((p) => p.kind === "combat" || p.kind === "hq");
  // Raiders move only with raiders (27.3x): ticked when no unit is.
  return (units.length > 0 ? units : here.filter((p) => p.kind === "raider")).map((p) => p.id);
}

/** Whether a piece may be ticked at all: the side to act's, on the map. */
export function tickable(p: Piece, side: Side | null): boolean {
  return p.side === side && p.kind !== "ship" && p.kind !== "blockship" && typeof p.location === "object" && ("hex" in p.location || "box" in p.location || "transit" in p.location);
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

/** Phase C: an independent unit attached to a parent may ask to take one
 * of its free slots (19.2x); the engine judges whether there is one. */
export function reassignOrder(view: View, p: Piece): Extract<Order, { reassign: unknown }> | null {
  if (view.clock.phase !== "organisation" || view.pending || p.side !== view.to_act) return null;
  if (p.parent || !p.attached_to || !view.pieces[p.attached_to]) return null;
  if (p.kind !== "combat" && p.kind !== "hq") return null;
  return { reassign: { piece: p.id } };
}

/** The weapon entries of the side to act's `p`, in the order the owner
 * would have them lost: its standing loss order first, then the rest as
 * listed; null when it has fewer than two entries with points or the
 * side may not give orders now (12.45, 14.4x, 15.83). */
export function lossOrderDraft(view: View, p: Piece): string[] | null {
  if (view.pending || p.side !== view.to_act || p.location === "eliminated") return null;
  const entries = (p.weapons ?? []).filter((w) => w.points > 0).map((w) => w.weapon);
  if (entries.length < 2) return null;
  const set = (p.loss_order ?? []).filter((w) => entries.includes(w));
  return [...set, ...entries.filter((w) => !set.includes(w))];
}

/** `list` with its `i`th entry one place earlier. */
export function raise<T>(list: T[], i: number): T[] {
  if (i <= 0 || i >= list.length) return list;
  const out = [...list];
  [out[i - 1], out[i]] = [out[i], out[i - 1]];
  return out;
}

/** Phase C: the historical shifts of parent the engine offers `p` this
 * month (19.24); it judges attachment and slots when one is sent. */
export function shiftOrders(view: View, p: Piece): Extract<Order, { "shift-parent": unknown }>[] {
  if (view.clock.phase !== "organisation" || view.pending || p.side !== view.to_act) return [];
  return (view.parent_shifts ?? []).filter((s) => s.piece === p.id).map((s) => ({ "shift-parent": { piece: p.id, to: s.to } }));
}

/** Whether `p` is a battalion standing alone in 19.2x's sense: in play,
 * attached to nothing, carrying nothing. */
function battalionAlone(view: View, p: Piece): boolean {
  if (p.kind !== "combat" || p.equivalent !== "battalion" || p.attached_to || p.location === "eliminated") return false;
  return !Object.values(view.pieces).some((q) => q.attached_to === p.id);
}

/** Phase C: a Commonwealth battalion assigned to a brigade may trade
 * brigades with another standing alone, wherever it is (19.2x). The
 * engine judges whether each brigade keeps its slots; this offers every
 * battalion it could, sorted by name. */
export function swapOrders(view: View, p: Piece): Extract<Order, { "swap-battalions": unknown }>[] {
  if (view.clock.phase !== "organisation" || view.pending || view.to_act !== "cw" || p.side !== "cw") return [];
  if (!p.parent || !battalionAlone(view, p)) return [];
  return Object.values(view.pieces)
    .filter((q) => q.side === "cw" && q.id !== p.id && q.parent !== p.parent && battalionAlone(view, q))
    .sort((x, y) => x.id.localeCompare(y.id))
    .map((q) => ({ "swap-battalions": { piece: p.id, with: q.id } }));
}

/** Phase C: a unit standing alone may attach to another HQ of its side in
 * its place than its own parent, for 2 CP each within the chart's limits,
 * which the engine judges (19.4x, 19.5). */
export function attachElsewhereOrders(view: View, p: Piece): Extract<Order, { attach: unknown }>[] {
  if (view.clock.phase !== "organisation" || view.pending || p.side !== view.to_act) return [];
  if (p.attached_to || (p.kind !== "combat" && p.kind !== "hq")) return [];
  const here = JSON.stringify(p.location);
  return Object.values(view.pieces)
    .filter((q) => q.kind === "hq" && q.side === p.side && q.id !== p.id && q.id !== p.parent && JSON.stringify(q.location) === here)
    .map((q) => ({ attach: { piece: p.id, to: q.id } }));
}

/**
 * The attachment order the stack panel offers for `p` (19.4x): detach it
 * from the unit it is attached to, or attach it to the parent it is
 * assigned to when they stand together. Only for the side to act, in
 * phase C or a movement segment with nothing pending; the engine checks
 * the rest (costs, the stop after a detachment).
 */
export function attachmentOrder(view: View, p: Piece): Extract<Order, { attach: unknown } | { detach: unknown }> | null {
  const c = view.clock;
  const when = c.phase === "organisation" || (c.phase === "movement-combat" && c.segment === "movement");
  if (!when || view.pending || p.side !== view.to_act) return null;
  if (p.attached_to) return { detach: { piece: p.id } };
  const parent = p.parent ? view.pieces[p.parent] : undefined;
  if (!parent || JSON.stringify(parent.location) !== JSON.stringify(p.location)) return null;
  return { attach: { piece: p.id, to: parent.id } };
}
