import { expect, test } from "@playwright/test";
import { clickHex, expectDomMatchesView, openFixture } from "./helpers.ts";

// 20.9, 64.75: in the Campaign Game the Commonwealth withdraws a ready
// battalion from Cairo and brings it back for two points.
test("the Commonwealth withdraws a battalion voluntarily and brings it back (64.75)", async ({ page }) => {
  await openFixture(page, "voluntary-withdrawal");
  const offer = await page.evaluate(() => window.__cna.view().voluntary_options!);
  expect(offer.returnable).toEqual([]);
  const id = offer.withdrawable[0];
  const hex = await page.evaluate((id) => {
    const l = window.__cna.view().pieces[id].location;
    return typeof l === "object" && "hex" in l ? l.hex : "";
  }, id);
  await clickHex(page, hex);
  const go = page.getByTestId("stack-panel").locator(`li[data-unit="${id}"] [data-order="withdraw-voluntarily"]`);
  await expect(go).toHaveText("Withdraw (64.75)");
  await go.click();
  await expect(page.getByTestId("log-entries").locator("li.event").filter({ hasText: /withdrawn voluntarily/ })).toHaveCount(1);
  expect(await page.evaluate((id) => window.__cna.view().pieces[id], id)).toBeUndefined();

  const back = page.getByTestId("voluntary-panel");
  await expect(back).toBeVisible();
  await back.locator(`li[data-unit="${id}"] [data-order="return-withdrawn"]`).click();
  await expect(page.getByTestId("log-entries").locator("li.event").filter({ hasText: /brought back to Cairo/ })).toHaveCount(1);
  await expect(back).toHaveCount(0);
  expect(await page.evaluate(() => window.__cna.view().voluntary_options!.halves)).toBe(-4);
  await expectDomMatchesView(page);
});
