// The sealed panels' own state (spec, *Sealed options*): what the
// player has typed against the engine's options, and the order it
// becomes. Pure functions; the engine alone judges the points.
import { sideName } from "./clock";
import type { Allocation, Assignment, Committer, Fire, Firer, HexId, Order, PieceId, RetreatReason } from "./types";

/** One row of the barrage panel: a firer's points at a target, as typed. */
export interface FireRow {
  piece: PieceId;
  points: number;
  target: PieceId | "";
}

/** One row of the assignment panel: a unit's points in one role, at a hex for the phasing side. */
export interface RoleRow {
  piece: PieceId;
  role: "anti-armour" | "close-assault";
  points: number;
  target: HexId | "";
}

/** A fresh row for a firer: its whole TOE at its first target, the usual answer. */
export function fireRowFor(f: Firer): FireRow {
  return { piece: f.piece, points: f.points, target: f.targets[0] ?? "" };
}

/** A fresh row for a unit in a role: all its points, at its first target where it names one. */
export function roleRowFor(u: Committer, role: RoleRow["role"], phasing: boolean): RoleRow {
  const targets = role === "anti-armour" ? u.anti_armour_targets : u.close_assault_targets;
  return { piece: u.piece, role, points: u.points, target: phasing ? (targets[0] ?? "") : "" };
}

/** The barrage order the rows make: every row as a `Fire`, blank targets and all; the engine says what is wrong. */
export function barrageOrder(rows: FireRow[]): Order {
  const fires: Fire[] = rows.map((r) => ({ piece: r.piece, points: r.points, target: r.target }));
  return { barrage: { fires } };
}

/** The assignment order the rows make: one `Assignment` per unit with rows, a point never in both roles; an unlisted unit is withheld. */
export function assignOrder(rows: RoleRow[], phasing: boolean): Order {
  const byPiece = new Map<PieceId, Assignment>();
  for (const r of rows) {
    const a = byPiece.get(r.piece) ?? { piece: r.piece, anti_armour: [], close_assault: [] };
    const alloc: Allocation = phasing ? { target: r.target, points: r.points } : { points: r.points };
    (r.role === "anti-armour" ? a.anti_armour! : a.close_assault!).push(alloc);
    byPiece.set(r.piece, a);
  }
  const assignments = [...byPiece.values()].map((a) => ({
    piece: a.piece,
    ...(a.anti_armour!.length > 0 ? { anti_armour: a.anti_armour } : {}),
    ...(a.close_assault!.length > 0 ? { close_assault: a.close_assault } : {}),
  }));
  return { "assign-forces": { assignments } };
}

/** The points a unit's rows commit in all, against its TOE. */
export function committed(rows: RoleRow[], piece: PieceId): number {
  return rows.filter((r) => r.piece === piece).reduce((n, r) => n + r.points, 0);
}

/** What answering a forced retreat with Pass does, from its reason (15.82, 10.36, R-012). */
export function passConsequence(reason: RetreatReason, hexes: number, raw: number): string {
  if (reason === "must-attack") return `Pass surrenders the stack: a must-attack retreat goes the full ${hexes} hexes or not at all (10.36).`;
  const pct = 10 * hexes;
  const lost = Math.floor((raw * pct) / 100);
  return `Pass stands where it is and pays 10 % of its ${raw} raw points per hex short: about ${lost} point${lost === 1 ? "" : "s"} for all ${hexes} (15.82, R-012).`;
}

/** The forced retreat's order, in words. */
export function retreatOrderWords(p: { side: string; hexes: number; from: HexId[]; reason: RetreatReason }): string {
  const why = { "assault-result": "the assault result", withheld: "committing nothing", "must-attack": "neither assaulting nor holding off the enemy" }[p.reason];
  const from = [...new Set(p.from)].join(", ");
  return `${sideName(p.side as "axis" | "cw")} must retreat ${p.hexes} hex${p.hexes === 1 ? "" : "es"} from ${from} for ${why}.`;
}
