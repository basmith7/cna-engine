// The sealed panels' own state (spec, *Sealed options*): what the
// player has typed against the engine's options, and the order it
// becomes. Pure functions; the engine alone judges the points.
import { sideName } from "./clock";
import { formatEvent, type Namer } from "./format";
import type { Allocation, Assignment, ApplyResult, Committer, Fire, Firer, HexId, Order, PieceId, RetreatReason } from "./types";

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

/**
 * What answering a forced retreat with Pass does, from the engine's
 * preview of it: the shortfall and losses it would take, a surrender,
 * nothing at all, or a refusal. Each as the panel's line and its button.
 */
export function passConsequence(preview: ApplyResult, name: Namer): { text: string; button: string } {
  if ("illegal" in preview) {
    return { text: `Pass would be refused: ${preview.illegal.reason} (${preview.illegal.case}).`, button: "Pass" };
  }
  // `set-up-complete` is the one bare-string event; it never follows a Pass here.
  const events = preview.ok.filter((e) => typeof e === "object");
  const surrendered = events.find((e) => "surrendered" in e);
  if (surrendered && "surrendered" in surrendered) {
    return { text: `Pass surrenders ${surrendered.surrendered.pieces.map(name).join(", ")}.`, button: "Pass: surrender" };
  }
  const bought = events.find((e) => "retreat-bought-out" in e);
  if (bought && "retreat-bought-out" in bought) {
    const short = bought["retreat-bought-out"].short;
    const losses = events.flatMap((e) => ("losses" in e ? [e.losses] : []));
    const total = losses.reduce((n, l) => n + l.points, 0);
    const gone = events.flatMap((e) => ("eliminated" in e ? [`; ${name(e.eliminated.piece)} eliminated`] : [])).join("");
    const points = `${total} point${total === 1 ? "" : "s"}`;
    // One loser is named outright; several are listed with their shares.
    const loss =
      losses.length === 1
        ? `${name(losses[0].piece)} loses ${points} for it`
        : `loses ${points} for it${losses.length > 1 ? `: ${losses.map((l) => `${name(l.piece)} ${l.points}`).join(", ")}` : ""}`;
    return {
      text: `Pass stands where it is, ${short} hex${short === 1 ? "" : "es"} short of the order, and ${loss}${gone}.`,
      button: "Pass: stand and pay",
    };
  }
  if (events.length === 0) return { text: "Pass stands where it is at no cost: the stack already meets the order.", button: "Pass: stand" };
  return { text: `Pass stands where it is: ${events.map((e) => formatEvent(e, name)).join(" ")}`, button: "Pass: stand" };
}

/** The forced retreat's order, in words. */
export function retreatOrderWords(p: { side: string; hexes: number; from: HexId[]; reason: RetreatReason }): string {
  const why = { "assault-result": "the assault result", withheld: "committing nothing", "must-attack": "neither assaulting nor holding off the enemy" }[p.reason];
  const from = [...new Set(p.from)].join(", ");
  return `${sideName(p.side as "axis" | "cw")} must retreat ${p.hexes} hex${p.hexes === 1 ? "" : "es"} from ${from} for ${why}.`;
}
