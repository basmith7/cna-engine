import { expect, test, type Page } from "@playwright/test";
import type { View } from "../src/types.ts";
import { readFileSync } from "node:fs";
import { clickHex, expectDomMatchesView, here, openFixture, openStart, shoot as shootInto, shotsDir, throughCurtain } from "./helpers.ts";

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

test("phase C: the Commonwealth forms LRDG 1 from the 11th Hussars by a click (27.13)", async ({ page }) => {
  await openStart(page);
  await page.evaluate((json) => window.__cna.load(json), readFileSync(here("fixtures/set-up-done.json"), "utf8"));
  await page.locator("[data-stack]").first().waitFor();
  await page.getByTestId("phase-controls").locator('[data-order="declare-initiative"][data-side="cw"]').click();
  await expect(page.getByTestId("clock")).toContainText("C · organisation");
  await throughCurtain(page, "cw");
  await expect(page.getByTestId("to-act")).toHaveText("Commonwealth to act");
  const hus = await page.evaluate(() => {
    const view: View = window.__cna.view();
    const p = view.pieces["unit:cw:11-hus"];
    return { hex: (p.location as { hex: string }).hex, toe: p.toe ?? 0 };
  });
  await clickHex(page, hus.hex);
  const form = page.getByTestId("stack-panel").locator('[data-order="form-raider"][data-raider="unit:cw:lrdg-1"]');
  await expect(form).toHaveText(/^Form LRDG 1 from /);
  await shoot(page, "form-offered.png");
  await form.click();
  await expect(log(page).filter({ hasText: `Long Range Desert Group 1 is formed at ${hus.hex} from a TOE point of 11th Hussars.` })).toHaveCount(1);
  await expect(page.getByTestId("toast")).toHaveCount(0);
  const after = await page.evaluate(() => {
    const view: View = window.__cna.view();
    return { lrdg: view.pieces["unit:cw:lrdg-1"], toe: view.pieces["unit:cw:11-hus"].toe };
  });
  expect(after.lrdg.kind).toBe("raider");
  expect(after.lrdg.location).toEqual({ hex: hus.hex });
  expect(after.toe).toBe(hus.toe - 1);
  // The counter is in play now: it is offered no more.
  await expect(form).toHaveCount(0);
  await shoot(page, "formed.png");
});
