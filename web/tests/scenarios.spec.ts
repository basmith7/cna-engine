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
