import { describe, expect, it } from "vitest";
import { LABEL_SCALE, MAX_SCALE, centreOn, fit, isFar, panBy, scale, toMap, zoomAt, type Port, type ViewBox } from "./viewport";

const port: Port = { width: 1000, height: 500 };
const box = { x: 100, y: 200, width: 400, height: 400 };

describe("the viewport", () => {
  it("fits a box into the port at the port's aspect, centred", () => {
    const vb = fit(box, port);
    expect(vb.height).toBeCloseTo(400);
    expect(vb.width).toBeCloseTo(800);
    expect(vb.x).toBeCloseTo(100 - 200);
    expect(vb.y).toBeCloseTo(200);
    expect(scale(vb, port)).toBeCloseTo(1.25);
  });

  it("maps a pixel to map space and back through the centre", () => {
    const vb: ViewBox = { x: 0, y: 0, width: 2000, height: 1000 };
    expect(toMap(vb, port, 500, 250)).toEqual([1000, 500]);
    expect(toMap(vb, port, 0, 0)).toEqual([0, 0]);
    // A port wider than the box's aspect letterboxes: the box is centred (xMidYMid meet).
    const tall: ViewBox = { x: 0, y: 0, width: 500, height: 500 };
    expect(toMap(tall, port, 500, 250)).toEqual([250, 250]);
    expect(toMap(tall, port, 250, 0)[0]).toBeCloseTo(0);
  });

  it("zooms about the cursor so the map point under it stays put", () => {
    const vb: ViewBox = { x: 0, y: 0, width: 2000, height: 1000 };
    const at = toMap(vb, port, 800, 100);
    const z = zoomAt(vb, port, 2, 800, 100);
    expect(z.width).toBeCloseTo(1000);
    expect(z.height).toBeCloseTo(500);
    expect(toMap(z, port, 800, 100)[0]).toBeCloseTo(at[0]);
    expect(toMap(z, port, 800, 100)[1]).toBeCloseTo(at[1]);
  });

  it("stops zooming past the limits", () => {
    const vb: ViewBox = { x: 0, y: 0, width: 2000, height: 1000 };
    let z = vb;
    for (let i = 0; i < 40; i++) z = zoomAt(z, port, 2, 500, 250);
    expect(scale(z, port)).toBeCloseTo(MAX_SCALE);
    let far = vb;
    for (let i = 0; i < 40; i++) far = zoomAt(far, port, 0.5, 500, 250);
    expect(scale(far, port)).toBeGreaterThan(0.01);
  });

  it("pans by pixels", () => {
    const vb: ViewBox = { x: 0, y: 0, width: 2000, height: 1000 };
    const p = panBy(vb, port, 100, -50);
    expect(p.x).toBeCloseTo(-200);
    expect(p.y).toBeCloseTo(100);
    expect(p.width).toBe(2000);
  });

  it("centres on a point without changing the zoom", () => {
    const vb: ViewBox = { x: 0, y: 0, width: 2000, height: 1000 };
    const c = centreOn(vb, [300, 300]);
    expect(c).toEqual({ x: -700, y: -200, width: 2000, height: 1000 });
  });

  it("calls the fitted theatre far and a hex-wide view near", () => {
    expect(isFar({ x: 0, y: 0, width: 5000, height: 2500 }, port)).toBe(true);
    expect(isFar({ x: 0, y: 0, width: 500, height: 250 }, port)).toBe(false);
    expect(scale({ x: 0, y: 0, width: 1000 / LABEL_SCALE, height: 500 / LABEL_SCALE }, port)).toBeCloseTo(LABEL_SCALE);
  });
});
