// Pan and zoom as `viewBox` maths (spec, *Pan and zoom*): pure functions
// over a view box and the port (the `<svg>`'s pixel size), with
// `preserveAspectRatio="xMidYMid meet"` assumed, so a box narrower than
// the port is letterboxed in the middle.

export interface ViewBox {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Port {
  width: number;
  height: number;
}

/** Pixels per map unit at which labels become readable (5 px hex numbers). */
export const LABEL_SCALE = 1.2;
/** Pixels per map unit at the closest zoom: one hex about 350 px wide. */
export const MAX_SCALE = 10;
/** At the farthest zoom the whole theatre is a tenth of the port. */
export const MIN_SCALE = 0.02;

/** Pixels per map unit. */
export function scale(vb: ViewBox, port: Port): number {
  return Math.min(port.width / vb.width, port.height / vb.height);
}

/** The smallest view box at the port's aspect that holds `box`, centred on it. */
export function fit(box: ViewBox, port: Port): ViewBox {
  const s = scale(box, port);
  const width = port.width / s;
  const height = port.height / s;
  return { x: box.x - (width - box.width) / 2, y: box.y - (height - box.height) / 2, width, height };
}

/** The map point under a pixel of the port. */
export function toMap(vb: ViewBox, port: Port, px: number, py: number): [number, number] {
  const s = scale(vb, port);
  const ox = (port.width - vb.width * s) / 2;
  const oy = (port.height - vb.height * s) / 2;
  return [vb.x + (px - ox) / s, vb.y + (py - oy) / s];
}

/** Zoom by `factor` (over 1 is closer) keeping the map point under the pixel where it is. */
export function zoomAt(vb: ViewBox, port: Port, factor: number, px: number, py: number): ViewBox {
  const s = scale(vb, port);
  const target = Math.min(MAX_SCALE, Math.max(MIN_SCALE, s * factor));
  const f = target / s;
  const [mx, my] = toMap(vb, port, px, py);
  const next = { x: 0, y: 0, width: vb.width / f, height: vb.height / f };
  const [nx, ny] = toMap(next, port, px, py);
  return { ...next, x: mx - nx, y: my - ny };
}

/** Drag the view by a pixel delta: the map moves with the pointer. */
export function panBy(vb: ViewBox, port: Port, dx: number, dy: number): ViewBox {
  const s = scale(vb, port);
  return { ...vb, x: vb.x - dx / s, y: vb.y - dy / s };
}

/** The same zoom, centred on a map point. */
export function centreOn(vb: ViewBox, [x, y]: [number, number]): ViewBox {
  return { ...vb, x: x - vb.width / 2, y: y - vb.height / 2 };
}

/** Too far out for labels and settlements to be readable. */
export function isFar(vb: ViewBox, port: Port): boolean {
  return scale(vb, port) < LABEL_SCALE;
}
