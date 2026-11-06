import { expect, test } from "@playwright/test";
import { clickHex, openFixture, openStart } from "./helpers.ts";

test("the start screen offers every scenario, Graziani's Offensive first picked", async ({ page }) => {
  await openStart(page);
  const picker = page.getByTestId("scenario");
  await expect(picker).toHaveValue("grazianis-offensive");
  await expect(picker.locator("option", { hasText: "Operation Crusader" })).toHaveCount(1);
  await expect(picker.locator('option[value="the-last-chance"]')).toHaveCount(1);
  await expect(picker.locator('option[value="rommels-arrival"]')).toHaveCount(1);
});

test("Operation Crusader starts from the start screen, its pieces pending set-up", async ({ page }) => {
  await openStart(page);
  await page.getByTestId("scenario").selectOption({ label: "Operation Crusader" });
  await page.getByTestId("seed").fill("7");
  await page.getByTestId("new-game").click();
  await page.locator("[data-stack]").first().waitFor();
  const view = await page.evaluate(() => window.__cna.view());
  expect(view.scenario).toBe("operation-crusader");
  expect(view.pending).toHaveProperty("set-up");
  await expect(page.locator("header.topbar .title")).toHaveText("Operation Crusader");
  await expect(page.getByTestId("setup")).toContainText("Set-up");
  await expect(page.locator("[data-stack]").first()).toBeVisible();
});

test("Operation Crusader: roads the scenario builds at set-up are drawn finished over the map's unfinished ones (59.12)", async ({ page }) => {
  await openFixture(page, "crusader-set-up");
  const built = (await page.evaluate(() => window.__cna.view())).built_hexsides!;
  expect(built.roads.length).toBeGreaterThan(0);
  const drawn = page.getByTestId("built-hexsides").locator("path.hexside.road");
  await expect(drawn).toHaveCount(built.roads.length);
  const [a, b] = built.roads[0];
  // Under it, the base map's own unfinished road on the same hexside.
  const base = page.locator(`g.hexsides path.hexside.unfinished-road[data-a="${a}"][data-b="${b}"], g.hexsides path.hexside.unfinished-road[data-a="${b}"][data-b="${a}"]`);
  await expect(base).toHaveCount(1);
});

test("Operation Crusader set up: Rommel stands with the Savona Division, whose units show his +5 CPA (31.0, R-137)", async ({ page }) => {
  await openFixture(page, "crusader-set-up");
  await clickHex(page, "C3618");
  const panel = page.getByTestId("stack-panel");
  await expect(panel.locator('li[data-unit="unit:de:rommel"]')).toHaveCount(1);
  const hq = panel.locator('li[data-unit="unit:it:55th-savona-div-hq"]');
  await expect(hq.getByTestId("rommel-cpa")).toHaveText("+5");
  await expect(panel.locator('li[data-unit="unit:de:rommel"]').getByTestId("rommel-cpa")).toHaveCount(0);
  await page.screenshot({ path: test.info().outputPath("crusader-rommel.png") });
});

test("Rommel's Arrival: the Axis divides its 290 unattached motorisation points along the coast road (61.72, R-160)", async ({ page }) => {
  await openFixture(page, "rommel-motorisation");
  const panel = page.getByTestId("set-up-motorisation");
  await expect(panel).toContainText("290 points to place");
  const before = (await page.evaluate(() => window.__cna.view())).motorisation?.axis ?? {};
  await panel.getByTestId("motorisation-points").fill("200");
  await panel.getByTestId("place-motorisation").click();
  const shaded = await page.locator("polygon.placement").evaluateAll((ps) => ps.map((p) => p.getAttribute("data-hex")!));
  expect(shaded.length).toBe(20);
  expect(shaded).toContain("A1816");
  expect(shaded).not.toContain("A1807");
  await clickHex(page, "A1816");
  await expect(panel).toContainText("90 points to place");
  const placed = (await page.evaluate(() => window.__cna.view())).motorisation?.axis ?? {};
  expect(placed.A1816).toBe((before.A1816 ?? 0) + 200);
  await page.screenshot({ path: test.info().outputPath("rommel-motorisation.png") });
  // More than is left places what is left.
  await panel.getByTestId("motorisation-points").fill("500");
  await clickHex(page, "A2703");
  // The Axis is done; the Commonwealth's 150 and 90 are next.
  await expect(page.getByTestId("setup")).toContainText("Set-up · Commonwealth");
  await expect(panel).toContainText("240 points to place");
  const view = await page.evaluate(() => window.__cna.view());
  expect(view.pending).toEqual({ "set-up": { side: "cw" } });
  // The Commonwealth's view keeps the Axis's points secret (3.61).
  expect(view.motorisation?.axis).toBeUndefined();
  expect(view.motorisation_to_place?.axis).toBeUndefined();
});

test("Rommel's Arrival: the Commonwealth places up to four Tobruk forts from the set-up panel, then play begins (61.6, R-161)", async ({ page }) => {
  await openFixture(page, "rommel-works");
  const works = page.getByTestId("set-up-works");
  await expect(works).toContainText("0 required left, up to 4 optional");
  // The forts are a maximum: Done is offered before any is placed.
  await expect(works.getByTestId("works-done")).toBeVisible();
  await expect(works.locator('button[data-work="real-minefield"]')).toHaveCount(0);
  await works.locator('button[data-work="fortification"]').click();
  // The engine's hexes shaded: Tobruk's land within two hexes.
  const shaded = await page.locator("polygon.placement").evaluateAll((ps) => ps.map((p) => p.getAttribute("data-hex")!));
  expect(shaded.length).toBeGreaterThanOrEqual(4);
  const options = (await page.evaluate(() => window.__cna.view())).work_options!.fortification!;
  expect([...shaded].sort()).toEqual([...options].sort());
  for (const [i, h] of shaded.slice(0, 3).entries()) {
    await clickHex(page, h);
    await expect(works).toContainText(`up to ${3 - i} optional`);
  }
  await clickHex(page, shaded[3]);
  await expect(works).toHaveCount(0);
  const view = await page.evaluate(() => window.__cna.view());
  expect(view.pending === null || !("set-up" in view.pending)).toBe(true);
  for (const h of shaded.slice(0, 4)) expect(view.fortifications?.[h]).toBe(1);
});
