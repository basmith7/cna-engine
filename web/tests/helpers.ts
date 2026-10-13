import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { View } from "../src/types.ts";
import "./cna.d.ts";

/** Where a slice's proving screenshots go (spec, Testing, Screenshots); CI overrides with CNA_SHOTS. */
export function shotsDir(slice: string): string {
  const dir = process.env.CNA_SHOTS ? join(process.env.CNA_SHOTS, slice) : join(homedir(), "Projects/cna-m3-shots", slice);
  mkdirSync(dir, { recursive: true });
  return dir;
}

export async function shoot(page: Page, dir: string, name: string) {
  const path = join(dir, name);
  await page.screenshot({ path });
  await test.info().attach(name, { path, contentType: "image/png" });
}

/** Every placed piece has a counter at its hex and every pending piece is in its side's tray; nothing else carries a data-unit on the map. */
export async function expectDomMatchesView(page: Page) {
  // One trip into the page: the DOM is compared with the view there, and
  // only the mismatches come back (hundreds of locator round trips hit
  // Playwright's test timeout on a CI runner).
  const found = await page.evaluate(() => {
    // Inside the page the record's value type is lost; the view is the engine's.
    const view: View = window.__cna.view();
    const missing: string[] = [];
    let placed = 0;
    const pending = { axis: 0, cw: 0 };
    const one = (sel: string) => {
      if (document.querySelectorAll(sel).length !== 1) missing.push(sel);
    };
    for (const p of Object.values(view.pieces)) {
      const loc = p.location;
      if (typeof loc !== "object") continue;
      if ("hex" in loc) {
        one(`svg.map g[data-unit="${p.id}"][data-hex="${loc.hex}"]`);
        placed++;
      } else if ("pending" in loc) {
        one(`[data-where="tray-${p.side}"] [data-unit="${p.id}"]`);
        pending[p.side]++;
      } else if ("box" in loc) {
        one(`[data-where="box-${loc.box}"] [data-unit="${p.id}"]`);
      }
    }
    return { missing, placed, pending, onMap: document.querySelectorAll("svg.map g[data-unit]").length };
  });
  expect(found.missing).toEqual([]);
  expect(found.onMap).toBe(found.placed);
  return { placed: found.placed, pending: found.pending };
}
