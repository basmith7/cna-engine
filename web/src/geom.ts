// Pointy-top hex geometry for `data/map/`, a port of cna's
// `tools/map_geom.py` (hex size 20; rows increase northward, columns
// eastward; a sheet's `odd_rows_shift` says which rows sit half a hex
// east) plus the theatre layout: sheets A–E side by side, each 33
// columns east of the last (`seams.json`: column 33 meets the next
// sheet's column 01; E's column 00 is a strip of half hexes and not a
// hex), and Malta (M) as an inset in sheet A's empty north-west corner.
import sheetsFile from "../../vendor/cna/data/map/sheets.json";

export type HexId = string;

export const SIZE = 20;
/** Hex width, corner to corner across the flat. */
export const W = Math.sqrt(3) * SIZE;
/** Vertical distance between row centres. */
export const PITCH = 1.5 * SIZE;

type Sheet = { odd_rows_shift: "east" | "west"; rows: [number, number]; cols: [number, number] };
export const SHEETS = sheetsFile.sheets as unknown as Record<string, Sheet>;

const THEATRE = "ABCDE";
/** The tallest sheet's top row: every theatre sheet shares its y origin. */
const THEATRE_TOP_ROW = Math.max(...THEATRE.split("").map((s) => SHEETS[s].rows[1]));
/**
 * Where the Malta inset's top row sits: sheet A has no hex west of column
 * 25 above row 44, so the inset's 7 columns fit there without touching one.
 */
const MALTA_INSET_TOP_ROW = 56;

export function parseHexId(id: HexId): [string, number, number] {
  const m = /^([A-EM])(\d{2})(\d{2})$/.exec(id);
  if (!m) throw new Error(`bad hex id ${id}`);
  return [m[1], Number(m[2]), Number(m[3])];
}

function rowShiftedEast(row: number, sheet: Sheet): boolean {
  return (row % 2 === 1) === (sheet.odd_rows_shift === "east");
}

/**
 * Where a sheet's own `map_geom.centre` origin sits in theatre space:
 * add it to a per-sheet coordinate to get the theatre coordinate.
 */
export function sheetOffset(sheet: string): [number, number] {
  const sh = SHEETS[sheet];
  const k = THEATRE.indexOf(sheet);
  if (k < 0) {
    return [0, (THEATRE_TOP_ROW - MALTA_INSET_TOP_ROW) * PITCH];
  }
  // Sheet-local x counts from cols[0]; the theatre counts from column 1
  // so that E's column 00 takes no room.
  return [k * 33 * W + (sh.cols[0] - 1) * W, (THEATRE_TOP_ROW - sh.rows[1]) * PITCH];
}

/** The hex's centre in theatre space. */
export function centre(id: HexId): [number, number] {
  const [s, r, c] = parseHexId(id);
  const sh = SHEETS[s];
  const [ox, oy] = sheetOffset(s);
  const x = (c - sh.cols[0]) * W + (rowShiftedEast(r, sh) ? W / 2 : 0) + W;
  const y = (sh.rows[1] - r) * PITCH + SIZE * 2;
  return [x + ox, y + oy];
}

/** The six corners, starting at the top-right and going clockwise, as cna draws them. */
export function corners([cx, cy]: [number, number], size = SIZE): [number, number][] {
  const out: [number, number][] = [];
  for (let i = 0; i < 6; i++) {
    const a = (Math.PI / 180) * (60 * i - 30);
    out.push([cx + size * Math.cos(a), cy + size * Math.sin(a)]);
  }
  return out;
}

/** The `points` attribute for a hex polygon, to two decimals as cna writes it. */
export function polygonPoints(id: HexId): string {
  return corners(centre(id))
    .map(([x, y]) => `${x.toFixed(2)},${y.toFixed(2)}`)
    .join(" ");
}

/** The bounding box of a set of hexes, with a margin of one hex. */
export function bounds(ids: Iterable<HexId>): { x: number; y: number; width: number; height: number } {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const id of ids) {
    const [x, y] = centre(id);
    minX = Math.min(minX, x); maxX = Math.max(maxX, x);
    minY = Math.min(minY, y); maxY = Math.max(maxY, y);
  }
  return { x: minX - W, y: minY - SIZE * 1.5, width: maxX - minX + 2 * W, height: maxY - minY + 3 * SIZE };
}
