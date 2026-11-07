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

test("in phase C a unit attaches to an HQ not its parent, and asks to be reassigned there (19.2x)", async ({ page }) => {
  await openFixture(page, "set-up-done");
  await page.getByTestId("phase-controls").locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await expect(page.getByTestId("organise-hint")).toBeVisible();
  // At C4321 the independent 30th GaF anti-tank battalion stands with the
  // Trivioli regiment, whose three tank battalions fill its chart row.
  const AT = "unit:it:30-gaf-antitank-bn";
  const TRIVIOLI = "unit:it:trivioli-2nd-regt-hq";
  await clickHex(page, "C4321");
  await expect(row(page, AT).locator('[data-order="reassign"]')).toHaveCount(0);
  await row(page, AT).locator(`[data-order="attach-elsewhere"][data-to="${TRIVIOLI}"]`).click();
  await expect(log(page).filter({ hasText: /attaches to/ })).toHaveCount(1);
  expect(await attachedTo(page, AT)).toBe(TRIVIOLI);
  await row(page, AT).locator('[data-order="reassign"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (19.2x)");
  await expect(page.getByTestId("toast")).toContainText("no free slot");
  expect(await page.evaluate((id) => window.__cna.view().pieces[id].parent ?? null, AT)).toBeNull();
  await expectDomMatchesView(page);
});

test("in the Commonwealth's phase C two battalions standing alone trade brigades (19.2x)", async ({ page }) => {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "cw");
  // The 1st Royal Fusiliers (5th Indian Brigade) and the 2nd Cameron
  // Highlanders (11th Indian Brigade): each detaches to stand alone.
  const RFSLR = "unit:cw:1-rfslr";
  const CMRN = "unit:cw:2-cmrn";
  const parentOf = (id: string) => page.evaluate((id) => window.__cna.view().pieces[id].parent ?? null, id);
  const hexOf = (id: string) => page.evaluate((id) => (window.__cna.view().pieces[id].location as { hex: string }).hex, id);
  const [fifth, eleventh] = [await parentOf(RFSLR), await parentOf(CMRN)];
  expect(fifth).toBe("unit:cw:5-in-hq");
  expect(eleventh).toBe("unit:cw:11-in-hq");
  for (const id of [RFSLR, CMRN]) {
    await clickHex(page, await hexOf(id));
    if (await attachedTo(page, id)) await row(page, id).locator('[data-order="detach"]').click();
    expect(await attachedTo(page, id)).toBeNull();
  }
  await clickHex(page, await hexOf(RFSLR));
  const swap = row(page, RFSLR).locator('[data-order="swap-battalions"]');
  await expect(swap.locator(`option[data-with="${CMRN}"]`)).toHaveCount(1);
  await swap.selectOption({ index: await swap.locator("option").evaluateAll((os, id) => os.findIndex((o) => o.getAttribute("data-with") === id), CMRN) });
  await expect(log(page).filter({ hasText: /is assigned to/ })).toHaveCount(2);
  expect(await parentOf(RFSLR)).toBe(eleventh);
  expect(await parentOf(CMRN)).toBe(fifth);
  await expectDomMatchesView(page);
});

test("in May 1942 the 3/1 Punjab shifts from the 5th Indian Brigade to the 25th, attached to it (19.24)", async ({ page }) => {
  await openFixture(page, "parent-shift");
  const PUNJAB = "unit:cw:3-1-pjb";
  const TWENTY_FIFTH = "unit:cw:25-in-hq";
  const hex = await page.evaluate((id) => (window.__cna.view().pieces[id].location as { hex: string }).hex, PUNJAB);
  await clickHex(page, hex);
  const shift = row(page, PUNJAB).locator(`[data-order="shift-parent"][data-to="${TWENTY_FIFTH}"]`);
  await expect(shift).toHaveText(/^Shift to /);
  // Not yet attached to the 25th: the engine refuses.
  await shift.click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (19.24)");
  await expect(page.getByTestId("toast")).toContainText("must be attached");
  await row(page, PUNJAB).locator(`[data-order="attach-elsewhere"][data-to="${TWENTY_FIFTH}"]`).click();
  await expect(log(page).filter({ hasText: /attaches to/ })).toHaveCount(1);
  expect(await attachedTo(page, PUNJAB)).toBe(TWENTY_FIFTH);
  await shift.click();
  await expect(log(page).filter({ hasText: /is assigned to/ })).toHaveCount(1);
  expect(await page.evaluate((id) => window.__cna.view().pieces[id].parent ?? null, PUNJAB)).toBe(TWENTY_FIFTH);
  // Made once: no shift is offered again.
  await expect(row(page, PUNJAB).locator('[data-order="shift-parent"]')).toHaveCount(0);
  await expectDomMatchesView(page);
});
