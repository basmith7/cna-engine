import { expect, test } from "@playwright/test";
import { expectDomMatchesView, openFixture } from "./helpers.ts";

// R-154: GT97/2's withdrawal takes the 22nd Guards Brigade and three
// infantry battalions the Commonwealth picks (20.82).
test("the Commonwealth picks the battalions a withdrawal takes (20.82)", async ({ page }) => {
  await openFixture(page, "withdrawal-pick");
  const panel = page.getByTestId("withdrawal-panel");
  await expect(panel).toBeVisible();
  const offer = await page.evaluate(() => window.__cna.view().withdrawal_pick_options![0]);
  expect(offer.count).toBe(3);
  // Not yet picked: ticked as the engine would pick.
  await expect(panel.getByTestId("withdrawal-picked")).toHaveText(/^Not picked/);
  const boxes = (id: string) => panel.locator(`li[data-unit="${id}"] input[type="checkbox"]`);
  for (const id of offer.candidates.slice(0, 3)) await expect(boxes(id)).toBeChecked();
  await expect(panel.locator("input:checked")).toHaveCount(3);

  // Two of three: the engine refuses.
  await boxes(offer.candidates[0]).uncheck();
  const send = panel.locator('[data-order="pick-withdrawal"]');
  await expect(send).toHaveText("Pick for withdrawal (2 of 3 ticked)");
  await send.click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (20.82)");

  // Three, one of them the last offered: the pick is made and shown back.
  const last = offer.candidates[offer.candidates.length - 1];
  await boxes(last).check();
  await send.click();
  await expect(page.getByTestId("log-entries").locator("li.event").filter({ hasText: /picked to withdraw/ })).toHaveCount(1);
  const chosen = [...offer.candidates.slice(1, 3), last];
  expect(await page.evaluate(() => window.__cna.view().withdrawal_pick_options![0].picked)).toEqual(chosen);
  await expect(panel.getByTestId("withdrawal-picked")).toHaveText(/^Picked: /);
  await expect(panel.locator("input:checked")).toHaveCount(3);
  await expect(boxes(last)).toBeChecked();
  await expectDomMatchesView(page);
});
