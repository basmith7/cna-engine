// Pins the TypeScript geometry to cna's own renderer: the polygon points
// in the committed `site/public/map/C.svg` (tools/map_render.py over
// tools/map_geom.py) are the agreed look.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import hexesFile from "../../vendor/cna/data/map/hexes.json";
import seamsFile from "../../vendor/cna/data/map/seams.json";
import { W, centre, corners, sheetOffset, type HexId } from "./geom";

const svg = readFileSync(
  new URL("../../vendor/cna/site/public/map/C.svg", import.meta.url),
  "utf8",
);

function svgPoints(id: string): [number, number][] {
  const m = svg.match(new RegExp(`<polygon id="${id}" class="hex[^"]*" points="([^"]+)"`));
  if (!m) throw new Error(`${id} not in C.svg`);
  return m[1].split(" ").map((p) => p.split(",").map(Number) as [number, number]);
}

describe("geometry", () => {
  it("matches cna's C.svg corners once C's theatre offset is taken off", () => {
    const [ox, oy] = sheetOffset("C");
    for (const id of ["C0101", "C2617", "C5105"] as HexId[]) {
      const ours = corners(centre(id)).map(([x, y]) => [x - ox, y - oy]);
      const theirs = svgPoints(id);
      expect(ours.length).toBe(6);
      for (let i = 0; i < 6; i++) {
        expect(Math.abs(ours[i][0] - theirs[i][0])).toBeLessThan(0.01);
        expect(Math.abs(ours[i][1] - theirs[i][1])).toBeLessThan(0.01);
      }
    }
  });

  it("puts every seam pair one hex width apart", () => {
    // seams.json lists each pair of hexes that touch across a sheet
    // join (straight across, and diagonal where the row is shifted).
    // Real neighbours are exactly one hex width apart.
    const apart = (a: HexId, b: HexId) => Math.hypot(...centre(a).map((v, i) => v - centre(b)[i]));
    let n = 0;
    for (const seam of seamsFile.seams) {
      for (const p of seam.pairs) {
        expect(Math.abs(apart(p.west as HexId, p.east as HexId) - W), `${p.west}|${p.east}`).toBeLessThan(1e-6);
        n++;
      }
    }
    expect(n).toBeGreaterThan(300);
  });

  it("keeps the Malta inset clear of sheet A", () => {
    const aHexes = hexesFile.hexes.filter((h) => h.sheet === "A").map((h) => centre(h.id as HexId));
    for (const h of hexesFile.hexes.filter((h) => h.sheet === "M")) {
      const [mx, my] = centre(h.id as HexId);
      for (const [ax, ay] of aHexes) {
        expect(Math.hypot(ax - mx, ay - my), `${h.id} touches sheet A`).toBeGreaterThan(W);
      }
    }
  });

  it("gives every hex its own centre", () => {
    const seen = new Set<string>();
    for (const h of hexesFile.hexes) {
      const [x, y] = centre(h.id as HexId);
      const key = `${x.toFixed(3)},${y.toFixed(3)}`;
      expect(seen.has(key), `${h.id} shares a centre`).toBe(false);
      seen.add(key);
    }
    expect(seen.size).toBe(7073);
  });
});
