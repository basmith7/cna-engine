import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import type { View } from "../src/types.ts";
import "./cna.d.ts";

// Slice 2's proving screenshots go where the spec says (Testing,
// Screenshots) and into Playwright's artefacts; override with CNA_SHOTS.
const shots = process.env.CNA_SHOTS ?? join(homedir(), "Projects/cna-m3-shots/slice-2");

test.beforeAll(() => mkdirSync(shots, { recursive: true }));

async function shoot(page: Page, name: string) {
  const path = join(shots, name);
  await page.screenshot({ path });
  await test.info().attach(name, { path, contentType: "image/png" });
}

/** Every placed piece has a counter at its hex and every pending piece is in its side's tray; nothing else carries a data-unit. */
async function expectDomMatchesView(page: Page) {
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

test("a new game shows the theatre, every placed piece at its hex and the set-up trays", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · set-up");
  await expect(page.locator("polygon.hex[data-hex]")).toHaveCount(7073);
  await expect(page.locator("polygon.hex.terrain-sea").first()).toBeAttached();
  await expect(page.locator("path.hexside.road").first()).toBeAttached();

  const { placed, pending } = await expectDomMatchesView(page);
  expect(placed).toBe(241);
  expect(pending).toEqual({ axis: 36, cw: 23 });
  await expect(page.locator("[data-where=tray-axis] [data-unit]")).toHaveCount(36);
  await expect(page.locator("[data-where=tray-cw] [data-unit]")).toHaveCount(23);
  await expect(page.locator("[data-where=box-tripoli] [data-unit]")).toHaveCount(30);

  // The start view is fitted to sheets B–E: labels are hidden that far out.
  await expect(page.getByTestId("map")).toHaveClass(/far/);
  await shoot(page, "start.png");
});

test("stays within the performance budget: first counters under 3 s, a wheel zoom step under 100 ms", async ({ page }) => {
  await page.goto("/");
  await page.locator("[data-stack]").first().waitFor();
  // Time since navigation start, measured in the page once the counters are in the DOM.
  const firstCounters = await page.evaluate(() => performance.now());
  const zoomStep = await page.evaluate(async () => {
    const svg = document.querySelector("svg.map")!;
    const r = svg.getBoundingClientRect();
    const t0 = performance.now();
    svg.dispatchEvent(new WheelEvent("wheel", { deltaY: -100, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2, bubbles: true, cancelable: true }));
    // Two frames: the one that applies the viewBox and the one that paints it.
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    return performance.now() - t0;
  });
  test.info().annotations.push({ type: "first counters (ms)", description: firstCounters.toFixed(0) }, { type: "wheel zoom step (ms)", description: zoomStep.toFixed(1) });
  console.log(`first counters ${firstCounters.toFixed(0)} ms; wheel zoom step ${zoomStep.toFixed(1)} ms`);
  expect(firstCounters).toBeLessThan(3000);
  expect(zoomStep).toBeLessThan(100);
});

test("zooms to Sidi Barrani, where a stack badge and the stack panel show the three pieces", async ({ page }) => {
  await page.goto("/");
  await page.locator("[data-stack]").first().waitFor();
  await page.getByLabel("Go to hex").fill("c4131");
  await page.getByLabel("Go to hex").press("Enter");
  await expect(page.getByTestId("hex")).toHaveText("hex C4131");
  await expect(page.getByTestId("map")).not.toHaveClass(/far/);
  await expect(page.locator("text.place", { hasText: "Sidi Barrani" })).toBeVisible();

  const stack = page.locator("[data-stack=C4131]");
  await expect(stack).toHaveAttribute("data-count", "3");
  await expect(stack.locator(".badge text")).toHaveText("3");
  const panel = page.getByTestId("stack-panel");
  await expect(panel).toContainText("Sidi Barrani");
  await expect(panel.locator("[data-unit]")).toHaveCount(3);
  await expect(panel.locator('[data-unit="unit:cw:2-sctgds"]')).toContainText("2nd Scots Guards");
  // A supply unit is just "Supply": real and dummy look the same.
  await expect(panel.locator('[data-unit="supply:cw:6"]')).toContainText("Supply unit");
  await expect(panel.locator('[data-unit="supply:cw:6"]')).not.toContainText("supply:cw:6");

  // A click on another hex after the go-to moves the panel there.
  await page.locator("polygon.hex[data-hex=C4130]").click();
  await expect(page.getByTestId("hex")).toHaveText("hex C4130");
  await expect(panel).toContainText("Nothing here.");
  await page.locator("[data-stack=C4131]").click();
  await expect(page.getByTestId("hex")).toHaveText("hex C4131");
  await expect(panel.locator("[data-unit]")).toHaveCount(3);
  // The count badge is the obvious thing to click to see what is in a stack.
  await page.locator("polygon.hex[data-hex=C4130]").click();
  await expect(page.getByTestId("hex")).toHaveText("hex C4130");
  await stack.locator(".badge circle").click();
  await expect(page.getByTestId("hex")).toHaveText("hex C4131");
  await expect(panel.locator("[data-unit]")).toHaveCount(3);
  // The selected stack is painted last, so no neighbour's fan covers it.
  await expect(page.locator("[data-stack]").last()).toHaveAttribute("data-stack", "C4131");
  await shoot(page, "sidi-barrani.png");
});

test("a mouse click alone selects a hex, and a stack; a drag pans without selecting", async ({ page }) => {
  await page.goto("/");
  await page.locator("[data-stack]").first().waitFor();
  await expect(page.getByTestId("stack-panel")).toContainText("Click a hex to see what is in it.");

  // An empty hex at the fitted view, then a stack, by click only: no go-to.
  await page.locator("polygon.hex[data-hex=D2010]").click();
  await expect(page.getByTestId("hex")).toHaveText("hex D2010");
  await expect(page.getByTestId("stack-panel")).toContainText("Nothing here.");
  await page.locator("[data-stack=C4131]").click();
  await expect(page.getByTestId("hex")).toHaveText("hex C4131");
  await expect(page.locator("[data-stack=C4131]")).toHaveClass(/selected/);
  await expect(page.getByTestId("stack-panel").locator("[data-unit]")).toHaveCount(3);

  // A click after a wheel zoom still lands on the hex under the pointer.
  const box = (await page.locator("polygon.hex[data-hex=C4131]").boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  for (let i = 0; i < 10; i++) await page.mouse.wheel(0, -100);
  await expect(page.getByTestId("map")).not.toHaveClass(/far/);
  await page.locator("polygon.hex[data-hex=C4229]").click();
  await expect(page.getByTestId("hex")).toHaveText("hex C4229");

  // Dragging pans without selecting.
  const before = await page.getByTestId("map").getAttribute("viewBox");
  await page.mouse.move(600, 500);
  await page.mouse.down();
  await page.mouse.move(500, 450, { steps: 5 });
  await page.mouse.up();
  expect(await page.getByTestId("map").getAttribute("viewBox")).not.toBe(before);
  await expect(page.getByTestId("hex")).toHaveText("hex C4229");
});

test("fits the spec's 1280 px desktop minimum with the map still the larger part", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto("/");
  await page.locator("[data-stack]").first().waitFor();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  const map = (await page.getByTestId("map").boundingBox())!;
  expect(map.width).toBeGreaterThan(900);
  await expect(page.getByTestId("stack-panel")).toBeInViewport();
  await shoot(page, "start-1280.png");
});

test("stays usable at a phone width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await page.locator("[data-stack]").first().waitFor();
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  // Nothing overflows the viewport sideways; the panel is reached by scrolling down.
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.getByTestId("off-map").scrollIntoViewIfNeeded();
  await expect(page.getByTestId("tray-axis")).toBeInViewport();
  await page.evaluate(() => window.scrollTo(0, 0));
  await shoot(page, "start-390.png");
});
