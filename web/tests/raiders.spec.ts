import { expect, test, type Page } from "@playwright/test";
import type { View } from "../src/types.ts";
import { clickHex, expectDomMatchesView, openFixture, shoot as shootInto, shotsDir } from "./helpers.ts";

// Desert raiders (§27), plan Task 2's proving screenshots.
const shots = shotsDir("m5-raiders");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");

/** The fixture's test-only LRDG, hidden at C3518 next to Rommel at C3618. */
const LRDG = "unit:cw:lrdg-test";

test("a hidden LRDG next to Rommel raids him by clicks", async ({ page }) => {
  await openFixture(page, "raid-rommel");
  await expect(page.getByTestId("to-act")).toHaveText("Commonwealth to act");
  // The raider's own side sees it, marked hidden (27.31).
  await expect(page.locator('[data-stack="C3518"] .marker.hidden')).toHaveCount(1);

  // A combat unit gets no raid button: only a lone raider does.
  const cw = await page.evaluate(() => {
    // Inside the page the record's value type is lost; the view is the engine's.
    const view: View = window.__cna.view();
    return Object.values(view.pieces).find((p) => p.side === "cw" && p.kind === "combat" && typeof p.location === "object" && "hex" in p.location)!.location as { hex: string };
  });
  await clickHex(page, cw.hex);
  await expect(page.getByTestId("move-panel")).toBeVisible();
  await expect(page.getByTestId("raid-rommel")).toHaveCount(0);

  await clickHex(page, "C3518");
  await expect(page.getByTestId("move-panel")).toContainText("1 piece ticked");
  const raid = page.getByTestId("raid-rommel");
  await expect(raid).toHaveText("Raid Rommel");
  await shoot(page, "raid-offered.png");
  await raid.click();

  await expect(log(page).filter({ hasText: /^Raider raids Rommel at C3618: \d+, / })).toHaveCount(1);
  await expect(page.getByTestId("toast")).toHaveCount(0);
  // The raid puts it on the map, or it died there: either way, no longer hidden.
  await expect(page.locator(".marker.hidden")).toHaveCount(0);
  const after = await page.evaluate((id) => window.__cna.view().pieces[id], LRDG);
  expect(after.hidden).toBeUndefined();
  // One raid in a scenario that is not a campaign (27.64): no second offer.
  if (typeof after.location === "object" && "hex" in after.location) {
    await clickHex(page, after.location.hex);
    await expect(page.getByTestId("move-panel")).toContainText("1 piece ticked");
    await expect(page.getByTestId("raid-rommel")).toHaveCount(0);
  }
  await shoot(page, "raid-done.png");
  await expectDomMatchesView(page);
});
