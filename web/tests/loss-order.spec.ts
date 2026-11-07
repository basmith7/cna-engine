import { expect, test } from "@playwright/test";
import type { View } from "../src/types.ts";
import { clickHex, openFixture } from "./helpers.ts";

// The owner's standing loss order (12.45, 14.4x, 15.83): set from the
// stack panel, judged and stored by the engine.
test("the side to act sets which weapon points of a unit go first", async ({ page }) => {
  await openFixture(page, "set-up-done");
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  const unit = await page.evaluate(() => {
    const view: View = window.__cna.view();
    const p = Object.values(view.pieces).find(
      (p) => p.side === "axis" && typeof p.location === "object" && "hex" in p.location && (p.weapons ?? []).filter((w) => w.points > 0).length >= 2,
    )!;
    return { id: p.id, hex: (p.location as { hex: string }).hex, weapons: p.weapons!.filter((w) => w.points > 0).map((w) => w.weapon) };
  });
  await clickHex(page, unit.hex);
  const control = page.locator(`[data-unit="${unit.id}"] [data-testid="loss-order"]`);
  await expect(control.locator(".entry")).toHaveText(unit.weapons);

  // Raise the second entry above the first, and set it.
  await control.getByRole("button", { name: `Lose ${unit.weapons[1]} before ${unit.weapons[0]}` }).click();
  const wanted = [unit.weapons[1], unit.weapons[0], ...unit.weapons.slice(2)];
  await expect(control.locator(".entry")).toHaveText(wanted);
  await control.locator('[data-order="set-loss-order"]').click();
  await expect(page.getByTestId("toast")).toHaveCount(0);
  await expect(page.getByTestId("log-entries").locator("li.event").filter({ hasText: `will lose ${wanted.join(", then ")} first.` })).toHaveCount(1);
  expect(await page.evaluate((id) => window.__cna.view().pieces[id].loss_order, unit.id)).toEqual(wanted);

  // Clear hands the choice back to the engine.
  await control.locator('[data-order="clear-loss-order"]').click();
  await expect(page.getByTestId("log-entries").locator("li.event").filter({ hasText: "leaves its losses to the engine." })).toHaveCount(1);
  expect(await page.evaluate((id) => window.__cna.view().pieces[id].loss_order, unit.id)).toBeUndefined();
});
