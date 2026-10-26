import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { clickHex, here, openStart } from "./helpers.ts";

// Mission 5 slice 9b: engineers and infantry begin a fortification in
// phase C (24.41).

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");

test("phase C: the 201st Engineers and the IX Libyans begin fortifying C4218 (24.41)", async ({ page }) => {
  // The set-up, with the IX Libyan Battalion moved in beside the 201st
  // Engineer Battalion at C4218 (clear, a village, out of reach).
  const save = JSON.parse(readFileSync(here("fixtures/set-up-done.json"), "utf8"));
  const builders = ["unit:it:201st-engineer-bn", "unit:it:ix-libyan-bn"];
  save.pieces["unit:it:ix-libyan-bn"].location = { hex: "C4218" };
  await openStart(page);
  await page.evaluate((json) => window.__cna.load(json), JSON.stringify(save));
  await page.locator("[data-stack]").first().waitFor();
  await page.getByTestId("phase-controls").locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await expect(page.getByTestId("clock")).toContainText("C · organisation");
  await clickHex(page, "C4218");
  const panel = page.getByTestId("stack-panel");
  for (const li of await panel.locator("li[data-unit]").all()) {
    const id = (await li.getAttribute("data-unit"))!;
    const tick = li.locator("input.tick");
    if ((await tick.count()) === 0) continue;
    if (builders.includes(id)) await tick.check();
    else await tick.uncheck();
  }
  await panel.locator('[data-order="build"]').click();
  await expect(log(page).filter({ hasText: "Axis begins a fortification in C4218." })).toHaveCount(1);
  await expect(page.getByTestId("project")).toHaveText("Axis fortification under way: 0 of 3 stages");
  await expect(panel.locator('[data-order="build"]')).toHaveCount(0);
  const view = await page.evaluate(() => window.__cna.view());
  expect(view.projects?.C4218?.builders).toEqual(builders);
});
