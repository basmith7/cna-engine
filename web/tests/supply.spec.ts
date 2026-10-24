import { expect, test, type Page } from "@playwright/test";
import { clickHex, openFixture } from "./helpers.ts";

// Mission 5 slice 2: fuel in the move panel, loads in the stack panel
// (32.1x, 32.22), on the module-on golden game's set-up.

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");

/** From set-up done to the Axis' first movement segment (golden orders 59–60). */
async function toMovement(page: Page) {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await controls.locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).toContainText("movement");
}

test("a paid move: the panel quotes the fuel and its sources, the log names the payer", async ({ page }) => {
  await toMovement(page);
  // A supply unit's load is shown to its owner.
  const supplyHex = await page.evaluate(() => {
    const l = window.__cna.view().pieces["supply:axis:4"].location;
    return typeof l === "object" && "hex" in l ? l.hex : null;
  });
  expect(supplyHex).not.toBeNull();
  await clickHex(page, supplyHex!);
  await expect(page.locator('[data-unit="supply:axis:4"] [data-testid="load"]')).toHaveText(/^ammo \d+ · fuel \d+$/);

  // Golden order 61's stack at Fort Capuzzo owes 1 fuel to Sollum.
  await clickHex(page, "C4020");
  await expect(page.getByTestId("fuel-owed")).toContainText("fuel");
  await page.locator('polygon.hex[data-hex="C4021"]').hover();
  await expect(page.getByTestId("fuel-hovered")).toHaveText("1 to C4021");
  const from = page.getByTestId("fuel-from");
  await expect(from.locator("option")).toHaveCount(3);
  await from.selectOption({ index: 2 });
  const payer = await from.locator("option").nth(2).textContent();
  await page.locator('polygon.hex[data-hex="C4021"]').click();
  await expect(log(page).filter({ hasText: "C4020 → C4021 for 1 CP." })).toHaveCount(1);
  await expect(log(page).filter({ hasText: /burns 1 fuel from / })).toHaveCount(1);
  expect(payer).toContain(" at ");
});

test("a refused move: no supply in range has fuel, and the engine says 32.22", async ({ page }) => {
  await toMovement(page);
  await clickHex(page, "C0414");
  await expect(page.getByTestId("fuel-none")).toBeVisible();
  const target = await page.evaluate(() => {
    const v = window.__cna.view();
    const held = new Set(Object.values(v.pieces).map((p) => (typeof p.location === "object" && "hex" in p.location ? p.location.hex : "")));
    const hexes = [...document.querySelectorAll("polygon.reach")].map((e) => e.getAttribute("data-hex")!);
    return hexes.find((h) => !held.has(h)) ?? null;
  });
  expect(target).not.toBeNull();
  await page.locator(`polygon.hex[data-hex="${target}"]`).click();
  await expect(page.getByTestId("log-entries").locator("li.illegal").last()).toContainText("Illegal (32.22)");
});
