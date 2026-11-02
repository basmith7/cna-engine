// Abstract supply's facts for the panels (§32): a supply unit's load and
// a hex's motorisation points. The view already hides both from the side
// not to act (3.61), so these only read what is there.
import type { Piece, PieceId, Reach, View } from "./types";

/** "ammo n · fuel n", or null when the view carries no load. */
export function loadLabel(p: Pick<Piece, "load">): string | null {
  return p.load ? `ammo ${p.load.ammo} · fuel ${p.load.fuel}` : null;
}

/** The side to act's motorisation points in `place` (a hex id or box), null for none. */
export function motorisationAt(view: View, place: string): number | null {
  if (!view.to_act) return null;
  return view.motorisation?.[view.to_act]?.[place] ?? null;
}

/** "n fuel" or "lo–hi fuel" over the stack's destinations; null when none burns fuel. */
export function fuelSummary(reaches: Pick<Reach, "fuel">[]): string | null {
  const owed = reaches.map((r) => r.fuel ?? 0);
  const hi = Math.max(0, ...owed);
  if (hi === 0) return null;
  const lo = Math.min(...owed);
  return lo === hi ? `${hi} fuel` : `${lo}–${hi} fuel`;
}

/** "name at hex, fuel n": a supply unit offered to pay a move's fuel. */
export function sourceLabel(view: View, id: PieceId, name: (id: PieceId) => string): string {
  const p = view.pieces[id];
  const at = p && typeof p.location === "object" && "hex" in p.location ? ` at ${p.location.hex}` : "";
  return `${name(id)}${at}, fuel ${p?.load?.fuel ?? "?"}`;
}

/** What a firer's or committer's fire or assault owes in ammunition (32.21), and whether nothing in range can pay; null when nothing is owed. */
export function ammoNote(q: { ammo?: number; ammo_short?: boolean }, act: "fire" | "assault"): string | null {
  if (!q.ammo) return null;
  return `${act === "fire" ? "firing" : "assaulting"} costs ${q.ammo} ammunition${q.ammo_short ? ": none in range" : ""}`;
}
