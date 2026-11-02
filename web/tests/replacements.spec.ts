import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { clickHex, here, openStart } from "./helpers.ts";

// Mission 5 slice 7: repaired points wait in their hex and a unit there
// absorbs them in phase C (19.61, 20.41).

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");

test("phase C: the tank battalion absorbs its repaired points at a CP per two (20.41)", async ({ page }) => {
  // The set-up, with the II Medium Tank Battalion at C3617 three points
  // down and those three repaired in its hex.
  const save = JSON.parse(readFileSync(here("fixtures/set-up-done.json"), "utf8"));
  const tanks = save.pieces["unit:it:iim-tank-bn"];
  const entry = tanks.weapons.find((w: { points: number }) => w.points >= 3);
  entry.points -= 3;
  tanks.toe -= 3;
  save.repaired = { axis: { C3617: { [entry.weapon]: 3 } } };
  await openStart(page);
  await page.evaluate((json) => window.__cna.load(json), JSON.stringify(save));
  await page.locator("[data-stack]").first().waitFor();
  await page.getByTestId("phase-controls").locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await expect(page.getByTestId("clock")).toContainText("C · organisation");
  await clickHex(page, "C3617");
  const line = page.locator(`[data-testid="repaired"][data-weapon="${entry.weapon}"]`);
  await expect(line).toContainText(`3 repaired ${entry.weapon} points waiting`);
  await line.getByLabel(`Unit to absorb ${entry.weapon}`).selectOption("unit:it:iim-tank-bn");
  await line.locator('[data-order="absorb"]').click();
  await expect(log(page).filter({ hasText: new RegExp(`absorbs 3 repaired ${entry.weapon.replace("/", "\\/")} points for 2 CP\\.`) })).toHaveCount(1);
  await expect(page.locator('[data-testid="repaired"]')).toHaveCount(0);
  const after = await page.evaluate(() => window.__cna.view().pieces["unit:it:iim-tank-bn"]);
  expect(after.toe).toBe(tanks.toe + 3);
});
