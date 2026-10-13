import { expect, test, type Page } from "@playwright/test";
import { mkdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { View } from "../src/types.ts";
import "./cna.d.ts";

/** A file beside the specs. */
export const here = (rel: string) => fileURLToPath(new URL(rel, import.meta.url));

/** Opens the app at its start screen. */
export async function openStart(page: Page) {
  await page.goto("/");
  await expect(page.getByTestId("start")).toBeVisible();
}

/** Opens the app and starts a new game with `seed` from the start screen (7 is the golden game's). */
export async function newGame(page: Page, seed = 7) {
  await openStart(page);
  await page.getByTestId("seed").fill(String(seed));
  await page.getByTestId("new-game").click();
  await page.locator("[data-stack]").first().waitFor();
}

/** Opens the app and, for test builds, starts from a fixture state from `tests/fixtures/<name>.json`. */
export async function openFixture(page: Page, name: string) {
  await openStart(page);
  await page.evaluate((json) => window.__cna.load(json), readFileSync(here(`fixtures/${name}.json`), "utf8"));
  await page.locator("[data-stack]").first().waitFor();
  await expect(page.getByTestId("log-entries").locator("li.event")).toHaveCount(0);
}

/** Clicks the hex as a player would: go to it, then click what is drawn there (the stack if one is, else the polygon). */
export async function clickHex(page: Page, hex: string, modifiers: ("Shift" | "Control")[] = []) {
  await page.getByLabel("Go to hex").fill(hex);
  await page.getByLabel("Go to hex").press("Enter");
  await expect(page.getByTestId("hex")).toHaveText(`hex ${hex}`);
  const stack = page.locator(`[data-stack="${hex}"]`);
  await ((await stack.count()) > 0 ? stack : page.locator(`polygon.hex[data-hex="${hex}"]`)).click({ modifiers });
}

/** Dismisses the hot-seat curtain, which must be down for `side`. */
export async function throughCurtain(page: Page, side: "axis" | "cw") {
  await expect(page.getByTestId("curtain")).toHaveAttribute("data-side", side);
  await page.getByTestId("curtain").click();
  await expect(page.getByTestId("curtain")).toHaveCount(0);
}

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
