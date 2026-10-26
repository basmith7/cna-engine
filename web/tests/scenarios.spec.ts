import { expect, test } from "@playwright/test";
import { openStart } from "./helpers.ts";

test("the start screen offers the scenarios that set up, Graziani's Offensive first picked", async ({ page }) => {
  await openStart(page);
  const picker = page.getByTestId("scenario");
  await expect(picker).toHaveValue("grazianis-offensive");
  await expect(picker.locator("option", { hasText: "Operation Crusader" })).toHaveCount(1);
  await expect(picker.locator('option[value="the-last-chance"]')).toHaveCount(1);
  // Waiting on cna (cna-game/src/scenarios.rs): not offered.
  await expect(picker.locator('option[value="rommels-arrival"]')).toHaveCount(0);
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
