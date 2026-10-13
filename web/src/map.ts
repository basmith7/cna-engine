// The base layer: every hex outline as one SVG string, built once from
// the static map JSON and never touched by game state (spec, *Map
// rendering and performance*). Overlays are separate `<g>` layers.
import hexesFile from "../../vendor/cna/data/map/hexes.json";
import { bounds, polygonPoints, type HexId } from "./geom";

export const HEX_IDS: HexId[] = hexesFile.hexes.map((h) => h.id);

export function outlinesSvg(): string {
  return HEX_IDS.map((id) => `<polygon class="hex" data-hex="${id}" points="${polygonPoints(id)}"/>`).join("");
}

export const THEATRE_BOUNDS = bounds(HEX_IDS);
