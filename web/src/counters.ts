// Counters are drawn from the view, never from art (spec, *Counters*):
// this module decides what each piece shows and how the pieces group,
// and the Svelte components only draw it.
import { W } from "./geom";
import type { Equivalent, HexId, OffMapBox, Piece, PieceId, PieceKind, Side, View } from "./types";

/** A counter's side: 0.8 of a hex (spec, *Counters*). */
export const COUNTER = 0.8 * W;

/** The branch glyph in the NATO frame, keyed from the characteristics chart's `unit_type`. */
export type Glyph = "infantry" | "armour" | "artillery" | "hq" | "recce" | "engineer" | "anti-tank" | "anti-air" | "supply" | "other";

/**
 * The chart's unit types are prose ("Infantry Bn-Eq", "Reconnaissance
 * Battalion-Eq", "Light Anti-air Bn-Eq"); the first matching word wins,
 * most specific first, so "Anti-tank" is not read as "Tank".
 */
const BY_WORD: [RegExp, Glyph][] = [
  [/headquarters/i, "hq"],
  [/anti-tank/i, "anti-tank"],
  [/anti-air/i, "anti-air"],
  [/tank|armou?r/i, "armour"],
  [/artillery/i, "artillery"],
  [/recce|reconnaissance/i, "recce"],
  [/engineer|construction/i, "engineer"],
  [/infantry/i, "infantry"],
];

export function glyphFor(kind: PieceKind, unitType: string | null): Glyph {
  if (kind === "supply-unit" || kind === "dummy-supply-unit") return "supply";
  if (unitType) {
    for (const [re, glyph] of BY_WORD) if (re.test(unitType)) return glyph;
  }
  return "other";
}

/** The NATO size mark above the frame; HQs and supply carry none. */
export function sizeMark(equivalent: Equivalent | null): string {
  switch (equivalent) {
    case "brigade":
      return "X";
    case "battalion":
      return "II";
    case "company":
      return "I";
    default:
      return "";
  }
}

/**
 * The counter's short label and the panel's long one. Supply units are
 * all "Supply": a dummy must look like a real one to both players.
 */
export function label(p: Piece): { short: string; long: string } {
  if (p.kind === "supply-unit" || p.kind === "dummy-supply-unit") return { short: "Supply", long: "Supply unit" };
  return { short: p.abbreviation ?? p.id_code ?? p.id, long: p.name ?? p.id };
}

/** HQs on top, then combat units, then supply; ties by id so a stack is stable. */
const KIND_ORDER: Record<PieceKind, number> = { hq: 0, combat: 1, "supply-unit": 2, "dummy-supply-unit": 2 };

export function topFirst(pieces: Piece[]): Piece[] {
  return [...pieces].sort((a, b) => KIND_ORDER[a.kind] - KIND_ORDER[b.kind] || a.id.localeCompare(b.id));
}

/** The pieces on the map, by hex, each stack top first; hexes in id order. */
export function stacks(view: View): Map<HexId, Piece[]> {
  const byHex = new Map<HexId, Piece[]>();
  for (const p of Object.values(view.pieces)) {
    if (typeof p.location === "object" && "hex" in p.location) {
      const list = byHex.get(p.location.hex) ?? [];
      list.push(p);
      byHex.set(p.location.hex, list);
    }
  }
  return new Map([...byHex].sort(([a], [b]) => a.localeCompare(b)).map(([h, ps]) => [h, topFirst(ps)]));
}

export const OFF_MAP_BOXES: OffMapBox[] = ["tripoli", "tunisia", "tripolitania", "malta", "italy-sicily", "crete"];

export const BOX_NAMES: Record<OffMapBox, string> = {
  tripoli: "Tripoli",
  tunisia: "Tunisia",
  tripolitania: "Tripolitania",
  malta: "Malta",
  "italy-sicily": "Italy and Sicily",
  crete: "Crete",
};

export interface OffMap {
  boxes: { box: OffMapBox; pieces: Piece[] }[];
  transit: Piece[];
  tray: Record<Side, Piece[]>;
}

/** Everything not on a hex: the off-map boxes (all six, empty or not), pieces in transit, and each side's pieces still to place. */
export function offMap(view: View): OffMap {
  const boxes = OFF_MAP_BOXES.map((box) => ({ box, pieces: [] as Piece[] }));
  const transit: Piece[] = [];
  const tray: Record<Side, Piece[]> = { axis: [], cw: [] };
  for (const p of topFirst(Object.values(view.pieces))) {
    const l = p.location;
    if (typeof l !== "object") continue;
    if ("box" in l) boxes.find((b) => b.box === l.box)!.pieces.push(p);
    else if ("transit" in l) transit.push(p);
    else if ("pending" in l) tray[p.side].push(p);
  }
  return { boxes, transit, tray };
}

/** The small marks a counter carries, from the piece and the combat segment's sets. */
export type Marker = "reserve-I" | "reserve-II" | "contact" | "engaged" | "pinned" | "forward";

export function markers(p: Piece, view: View): Marker[] {
  const out: Marker[] = [];
  if (p.reserve) out.push(`reserve-${p.reserve}`);
  if (p.engaged) out.push("engaged");
  else if (p.contact) out.push("contact");
  const combat = view.combat as { pinned?: PieceId[]; forward?: PieceId[] };
  if (combat.pinned?.includes(p.id)) out.push("pinned");
  if (combat.forward?.includes(p.id)) out.push("forward");
  return out;
}
