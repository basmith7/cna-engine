import { expect, test, type Page } from "@playwright/test";
import type { PatrolSource, View } from "../src/types.ts";
import { clickHex, expectDomMatchesView, openFixture, shoot as shootInto, shotsDir, throughCurtain } from "./helpers.ts";

// Patrols (phase L, §16), plan Task 2's proving screenshots.
const shots = shotsDir("m5-patrols");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");

test("phase L: a patrol goes out by clicks, its dice in the log, and Pass ends the phase (16.x)", async ({ page }) => {
  await openFixture(page, "patrol");
  const curtain = page.getByTestId("curtain");
  if ((await curtain.count()) > 0) await throughCurtain(page, (await curtain.getAttribute("data-side")) as "axis" | "cw");
  await expect(page.getByTestId("clock")).toContainText("L · patrol");
  const { side, first } = await page.evaluate(() => {
    const view: View = window.__cna.view();
    return { side: view.to_act!, first: view.patrol_options![0] as PatrolSource };
  });
  const panel = page.getByTestId("patrol-panel");
  await expect(panel.locator(`li[data-unit="${first.piece}"]`)).toBeVisible();
  await expect(panel.locator('[data-order="patrol"]')).toBeDisabled();

  // Ticking a source marks the hexes it may patrol; a click on one is the target.
  await panel.locator(`li[data-unit="${first.piece}"] input[type="checkbox"]`).check();
  await expect(page.locator('[data-testid="marks"][data-kind="patrol"]')).toHaveAttribute("data-count", String(first.targets.length));
  await panel.locator(`li[data-unit="${first.piece}"] [data-points]`).fill("1");
  const target = first.targets[0];
  await clickHex(page, target);
  await expect(page.getByTestId("patrol-target")).toHaveText(`Target: ${target}`);
  await shoot(page, "patrol-armed.png");
  await panel.locator('[data-order="patrol"]').click();

  const sideName = side === "axis" ? "Axis" : "Commonwealth";
  await expect(log(page).filter({ hasText: new RegExp(`^${sideName} patrols ${target} with .+ 1: .*objective loss \\d`) })).toHaveCount(1);
  await expect(page.getByTestId("toast")).toHaveCount(0);
  await shoot(page, "patrol-sent.png");
  await expectDomMatchesView(page);

  // Pass ends phase L.
  await page.getByTestId("phase-controls").locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).not.toContainText("L · patrol");
});

test("a unit ticked skip at the target goes in the patrol's exclude (16.53)", async ({ page }) => {
  await openFixture(page, "patrol");
  const curtain = page.getByTestId("curtain");
  if ((await curtain.count()) > 0) await throughCurtain(page, (await curtain.getAttribute("data-side")) as "axis" | "cw");
  // A source and one of its targets where the view shows an enemy unit.
  const { source, target, enemy } = await page.evaluate(() => {
    const view: View = window.__cna.view();
    for (const o of view.patrol_options!)
      for (const t of o.targets) {
        const p = Object.values(view.pieces).find((p) => p.side !== view.to_act && typeof p.location === "object" && "hex" in p.location && p.location.hex === t);
        if (p) return { source: o.piece, target: t, enemy: p.id };
      }
    throw new Error("no target shows an enemy unit");
  });
  const panel = page.getByTestId("patrol-panel");
  await panel.locator(`li[data-unit="${source}"] input[type="checkbox"]`).check();
  await panel.locator(`li[data-unit="${source}"] [data-points]`).fill("1");
  await expect(panel.getByTestId("patrol-exclude")).toHaveCount(0);
  await clickHex(page, target);
  const skip = panel.locator(`[data-testid="patrol-exclude"] li[data-unit="${enemy}"] input[type="checkbox"]`);
  await skip.check();
  await shoot(page, "patrol-exclude.png");
  const before = await page.evaluate(() => window.__cna.orders());
  await panel.locator('[data-order="patrol"]').click();
  await expect.poll(() => page.evaluate(() => window.__cna.orders())).toBe(before + 1);

  const sent = await page.evaluate(() => JSON.stringify((JSON.parse(window.__cna.save()) as { log: unknown[] }).log.at(-1)));
  expect(sent).toContain(`"exclude":["${enemy}"]`);
  await expect(page.getByTestId("toast")).toHaveCount(0);
  // The send cleared the target, and with it the skip list.
  await expect(panel.getByTestId("patrol-exclude")).toHaveCount(0);
});
