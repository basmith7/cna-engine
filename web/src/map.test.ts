// The base layer is one SVG string built from the map JSON: cna's
// classes (tools/map_render.py) on every hex, hexside, settlement and
// label, so map.css can be a copy of cna's style sheet.
import { describe, expect, it } from "vitest";
import hexesFile from "../../vendor/cna/data/map/hexes.json";
import hexsidesFile from "../../vendor/cna/data/map/hexsides.json";
import placesFile from "../../vendor/cna/data/map/places.json";
import { HEX_IDS, baseSvg, hexInfo } from "./map";

const svg = baseSvg();

describe("the base layer", () => {
  it("gives every hex its terrain class and its id", () => {
    const polygons = svg.match(/<polygon class="hex terrain-[a-z-]+[^"]*" data-hex="[A-EM]\d{4}"/g) ?? [];
    expect(polygons.length).toBe(HEX_IDS.length);
    expect(svg).toContain('class="hex terrain-clear coastal" data-hex="C3922"');
  });

  it("draws every hexside feature as a path with cna's class, edge sides included", () => {
    const paths = svg.match(/<path class="hexside [a-z-]+"/g) ?? [];
    const features = hexsidesFile.hexsides.reduce((n, s) => n + s.features.length, 0);
    expect(paths.length).toBe(features);
    const edge = hexsidesFile.hexsides.find((s) => s.b === null)!;
    expect(svg).toContain(`data-a="${edge.a}" data-b=""`);
  });

  it("ticks a slope or escarpment towards the down hex", () => {
    const up = hexsidesFile.hexsides.find((s) => s.features.includes("escarpment") && s.up)!;
    const m = svg.match(new RegExp(`<path class="hexside escarpment" data-a="${up.a}" data-b="${up.b}" data-up="${up.up}" d="([^"]+)"`));
    expect(m, `${up.a}|${up.b}`).not.toBeNull();
    // A line plus a tick: "M x y L x y M x y l dx dy".
    expect(m![1].split(" M ").length).toBe(2);
  });

  it("marks each settlement on its hex", () => {
    const villages = hexesFile.hexes.filter((h) => h.settlement === "village").length;
    expect((svg.match(/class="settlement village"/g) ?? []).length).toBe(villages);
    expect(svg).toMatch(/<rect class="settlement major-city"[^>]*data-hex="E1730"/);
  });

  it("labels a place on its marker hex and shades every hex it covers", () => {
    const cairo = placesFile.places.find((p) => p.id === "cairo")!;
    expect(cairo.hexes!.length).toBeGreaterThan(1);
    expect((svg.match(/<text class="place"[^>]*>Cairo</g) ?? []).length).toBe(1);
    for (const h of cairo.hexes!) {
      expect(svg).toContain(`class="place-area" data-hex="${h}"`);
    }
  });

  it("numbers every hex", () => {
    expect((svg.match(/<text class="hexnum"/g) ?? []).length).toBe(HEX_IDS.length);
    expect(svg).toContain(">3922</text>");
  });

  it("describes a hex for the panel", () => {
    expect(hexInfo("E1730")).toEqual({ terrain: "major-city", coastal: false, settlement: "major-city", place: "Cairo" });
    expect(hexInfo("C3922")).toEqual({ terrain: "clear", coastal: true, settlement: null, place: null });
    expect(hexInfo("A0101").place).toBeNull();
  });
});
