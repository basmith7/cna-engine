import { expect, test, type Page } from "@playwright/test";
import { clickHex, expectDomMatchesView, openFixture, throughCurtain } from "./helpers.ts";

// Slice 6 (plan, *Testing*): a battalion detached from its formation in
// phase C, attached back and detached again, then moved alone in G.
const TANKS = "unit:it:iim-tank-bn";

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");
const row = (page: Page, id: string) => page.getByTestId("stack-panel").locator(`li[data-unit="${id}"]`);
const attachedTo = (page: Page, id: string) => page.evaluate((id) => window.__cna.view().pieces[id].attached_to ?? null, id);

test("a battalion detaches in phase C, attaches back, and moves alone in G", async ({ page }) => {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await expect(page.getByTestId("organise-hint")).toBeVisible();

  // Gruppo Maletti at C3617: the II M tank battalion is attached to it
  // from the set-up (4.25), so its row offers Detach.
  await clickHex(page, "C3617");
  await expect(row(page, TANKS).getByTestId("attached")).toBeVisible();
  const parent = await attachedTo(page, TANKS);
  expect(parent).not.toBeNull();
  await row(page, TANKS).locator('[data-order="detach"]').click();
  await expect(log(page).filter({ hasText: /detaches from/ })).toHaveCount(1);
  expect(await attachedTo(page, TANKS)).toBeNull();
  await expect(row(page, TANKS).getByTestId("attached")).toHaveCount(0);

  // Standing with the parent it is assigned to, it may attach back.
  await row(page, TANKS).locator('[data-order="attach"]').click();
  await expect(log(page).filter({ hasText: /attaches to/ })).toHaveCount(1);
  expect(await attachedTo(page, TANKS)).toBe(parent);
  await row(page, TANKS).locator('[data-order="detach"]').click();
  await expect(log(page).filter({ hasText: /detaches from/ })).toHaveCount(2);
  await expectDomMatchesView(page);

  // On to the Axis movement segment.
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "cw");
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "axis");
  await controls.locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).toContainText("Player A · movement");

  // Tick the tanks alone and send them one hex west.
  await clickHex(page, "C3617");
  const panel = page.getByTestId("stack-panel");
  for (const li of await panel.locator("li[data-unit]").all()) {
    const tick = li.locator("input.tick");
    if ((await tick.count()) === 0) continue;
    if ((await li.getAttribute("data-unit")) === TANKS) await tick.check();
    else await tick.uncheck();
  }
  await expect(panel.locator("input.tick:checked")).toHaveCount(1);
  await expect(page.locator('polygon.reach[data-hex="C3616"]')).toBeAttached();
  await page.locator('polygon.hex[data-hex="C3616"]').click();
  await expect.poll(() => page.evaluate((id) => window.__cna.view().pieces[id].location, TANKS)).toEqual({ hex: "C3616" });
  await expect(page.getByTestId("log-entries").locator("li.illegal")).toHaveCount(0);
  await expectDomMatchesView(page);
});

test("a battalion in an off-map box detaches from the move panel, and then stops with its HQ", async ({ page }) => {
  await openFixture(page, "off-map");
  const box = page.locator('[data-where="box-tripoli"]');
  const bn = "unit:it:131st-infantry-bn";
  await box.locator(`[data-unit="${bn}"]`).click();
  const moves = page.getByTestId("off-map-moves");
  await moves.locator(`[data-order="detach"][data-unit="${bn}"]`).click();
  await expect(log(page).filter({ hasText: /detaches from/ })).toHaveCount(1);
  expect(await attachedTo(page, bn)).toBeNull();
  // Detached in a movement segment, it goes on only with its HQ (19.4x).
  await moves.locator('[data-to="tripolitania"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (19.4x)");
  await expectDomMatchesView(page);
});

test("the stack panel nests attached units under their parent and marks a shell (9.26)", async ({ page }) => {
  await openFixture(page, "set-up-done");
  await page.getByTestId("phase-controls").locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await clickHex(page, "C3617");
  const parent = (await attachedTo(page, TANKS))!;
  const rows = page.getByTestId("stack-panel").locator("li[data-unit]");
  const order = async () => rows.evaluateAll((lis) => lis.map((li) => li.getAttribute("data-unit")));
  // The tanks sit one level under Gruppo Maletti, right after it or its other units.
  const parentDepth = Number(await row(page, parent).getAttribute("data-depth"));
  await expect(row(page, TANKS)).toHaveAttribute("data-depth", String(parentDepth + 1));
  const ids = await order();
  expect(ids.indexOf(TANKS)).toBeGreaterThan(ids.indexOf(parent));
  await expect(row(page, parent).getByTestId("shell")).toHaveCount(0);

  // Detached, the tanks stand at the top level; every badge matches the view.
  await row(page, TANKS).locator('[data-order="detach"]').click();
  await expect(row(page, TANKS)).toHaveAttribute("data-depth", "0");
  const shells = await page.evaluate(() => Object.entries(window.__cna.view().pieces).filter(([, p]) => (p as { shell?: boolean }).shell).map(([id]) => id));
  for (const id of await order()) {
    await expect(row(page, id!).getByTestId("shell")).toHaveCount(shells.includes(id!) ? 1 : 0);
  }
  expect(shells).not.toContain(parent);

  // The Libyan Tank Command sets up a shell: its hex shows the badge.
  const LTC = "unit:it:libyan-tank-command-hq";
  expect(shells).toContain(LTC);
  const hex = await page.evaluate((id) => (window.__cna.view().pieces[id].location as { hex: string }).hex, LTC);
  await clickHex(page, hex);
  await expect(row(page, LTC).getByTestId("shell")).toHaveAttribute("title", /9\.26/);
});
