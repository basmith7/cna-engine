// The base layer: terrain, hexsides, settlements and labels as one SVG
// string, built once from the static map JSON and never touched by game
// state (spec, *Map rendering and performance*). Overlays are separate
// `<g>` layers. The classes are cna's (vendor/cna/tools/map_render.py)
// so map.css can be a copy of its style sheet.
import hexesFile from "../../vendor/cna/data/map/hexes.json";
import hexsidesFile from "../../vendor/cna/data/map/hexsides.json";
import placesFile from "../../vendor/cna/data/map/places.json";
import { SIZE, W, bounds, centre, polygonPoints, type HexId } from "./geom";

type Hex = { id: HexId; sheet: string; terrain: string; settlement: string | null; coastal: boolean };
type Hexside = { key: string; a: HexId; b: HexId | null; side: string | null; features: string[]; up: HexId | null };
type Place = { id: string; hex: HexId; hexes?: HexId[]; name: string; kind: string; port: boolean };

const HEXES = hexesFile.hexes as Hex[];
const HEXSIDES = hexsidesFile.hexsides as Hexside[];
const PLACES = placesFile.places as Place[];
const BY_ID = new Map(HEXES.map((h) => [h.id, h]));
const PLACE_AT = new Map(PLACES.map((p) => [p.hex, p]));

export const HEX_IDS: HexId[] = HEXES.map((h) => h.id);
/** The map end of the off-map road (8.8x): where `move-off-map` lands and leaves. */
export const NOFILIA: HexId = PLACES.find((p) => p.id === "nofilia")!.hex;
export const THEATRE_BOUNDS = bounds(HEX_IDS);
/** The start view: sheets B–E, where Graziani is fought (spec, *Pan and zoom*). */
export const START_BOUNDS = bounds(HEXES.filter((h) => "BCDE".includes(h.sheet)).map((h) => h.id));

export function isHex(id: string): id is HexId {
  return BY_ID.has(id);
}

const f = (v: number) => v.toFixed(2);
const esc = (t: string) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

/** What the stack panel says about a hex. */
export function hexInfo(id: HexId): { terrain: string; coastal: boolean; settlement: string | null; place: string | null } {
  const h = BY_ID.get(id);
  if (!h) throw new Error(`unknown hex ${id}`);
  return { terrain: h.terrain, coastal: h.coastal, settlement: h.settlement, place: PLACE_AT.get(id)?.name ?? null };
}

function hexSvg(h: Hex): string {
  const cls = `hex terrain-${h.terrain}${h.coastal ? " coastal" : ""}`;
  return `<polygon class="${cls}" data-hex="${h.id}" points="${polygonPoints(h.id)}"/>`;
}

/** The two ends of an edge from its midpoint and the unit normal across it. */
function edgeEnds(mx: number, my: number, nx: number, ny: number): [number, number][] {
  const half = SIZE / 2;
  const [tx, ty] = [-ny, nx];
  return [
    [mx - half * tx, my - half * ty],
    [mx + half * tx, my + half * ty],
  ];
}

/** The shared edge of two adjacent hexes, its midpoint and the unit normal towards b. */
function edge(a: HexId, b: HexId) {
  const [ax, ay] = centre(a);
  const [bx, by] = centre(b);
  const d = Math.hypot(bx - ax, by - ay);
  const [nx, ny] = [(bx - ax) / d, (by - ay) / d];
  const [mx, my] = [(ax + bx) / 2, (ay + by) / 2];
  return { ends: edgeEnds(mx, my, nx, ny), mid: [mx, my] as const, normal: [nx, ny] as const };
}

const SIDE_DEG: Record<string, number> = { E: 0, SE: 60, SW: 120, W: 180, NW: 240, NE: 300 };

/** A hex's edge on the map's rim, where there is no hex across it. */
function edgeOnSide(a: HexId, side: string) {
  const [ax, ay] = centre(a);
  const rad = (Math.PI / 180) * SIDE_DEG[side];
  const [nx, ny] = [Math.cos(rad), Math.sin(rad)];
  const apo = W / 2;
  const [mx, my] = [ax + apo * nx, ay + apo * ny];
  return { ends: edgeEnds(mx, my, nx, ny), mid: [mx, my] as const, normal: [nx, ny] as const };
}

function hexsideSvg(s: Hexside): string {
  const e = s.b === null ? edgeOnSide(s.a, s.side!) : edge(s.a, s.b);
  const [[x0, y0], [x1, y1]] = e.ends;
  const line = `M ${f(x0)} ${f(y0)} L ${f(x1)} ${f(y1)}`;
  return s.features
    .map((feat) => {
      let tick = "";
      if ((feat === "slope" || feat === "escarpment") && s.up) {
        // The tick points to the down hex.
        const sign = s.up === s.a ? 1 : -1;
        tick = ` M ${f(e.mid[0])} ${f(e.mid[1])} l ${f(sign * e.normal[0] * SIZE * 0.3)} ${f(sign * e.normal[1] * SIZE * 0.3)}`;
      }
      return `<path class="hexside ${feat}" data-a="${s.a}" data-b="${s.b ?? ""}" data-up="${s.up ?? ""}" d="${line}${tick}"/>`;
    })
    .join("");
}

function settlementSvg(h: Hex): string {
  if (!h.settlement) return "";
  const [cx, cy] = centre(h.id);
  if (h.settlement === "major-city") {
    const s = SIZE * 0.5;
    return `<rect class="settlement major-city" data-hex="${h.id}" x="${f(cx - s / 2)}" y="${f(cy - s / 2)}" width="${f(s)}" height="${f(s)}"/>`;
  }
  return `<circle class="settlement ${h.settlement}" data-hex="${h.id}" cx="${f(cx)}" cy="${f(cy)}" r="${f(SIZE * 0.15)}"/>`;
}

/** A place that covers several hexes shades all of them; the marker and name sit on `hex`. */
function placeAreaSvg(p: Place): string {
  if (!p.hexes || p.hexes.length < 2) return "";
  return p.hexes.map((h) => `<polygon class="place-area" data-hex="${h}" points="${polygonPoints(h)}"/>`).join("");
}

function hexnumSvg(h: Hex): string {
  const [cx, cy] = centre(h.id);
  return `<text class="hexnum" x="${f(cx)}" y="${f(cy + SIZE * 0.75)}">${h.id.slice(1)}</text>`;
}

/** The name sits at the top of its hex, clear of a counter on it, and after every hex number so its halo wins. */
function placeNameSvg(p: Place): string {
  const [cx, cy] = centre(p.hex);
  return `<text class="place" x="${f(cx)}" y="${f(cy - SIZE * 0.88)}">${esc(p.name)}</text>`;
}

/** The whole base layer: hexes, place areas, hexsides, settlements, labels, in cna's order. */
export function baseSvg(): string {
  return [
    `<g class="hexes">${HEXES.map(hexSvg).join("")}</g>`,
    `<g class="place-areas">${PLACES.map(placeAreaSvg).join("")}</g>`,
    `<g class="hexsides">${HEXSIDES.map(hexsideSvg).join("")}</g>`,
    `<g class="settlements">${HEXES.map(settlementSvg).join("")}</g>`,
    `<g class="labels">${HEXES.map(hexnumSvg).join("")}${PLACES.map(placeNameSvg).join("")}</g>`,
  ].join("");
}
