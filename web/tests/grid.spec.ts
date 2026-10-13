import { expect, test } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import "./cna.d.ts";

// Slice 1's proving screenshots go where the spec says (Testing,
// Screenshots) and into Playwright's artefacts; override with CNA_SHOTS.
const shots = process.env.CNA_SHOTS ?? join(homedir(), "Projects/cna-m3-shots/slice-1");

test.beforeAll(() => mkdirSync(shots, { recursive: true }));

test("a new game shows the theatre grid and the opening clock", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · set-up");
  await expect(page.locator("[data-hex]")).toHaveCount(7073);

  const view = await page.evaluate(() => window.__cna.view());
  expect(view.to_act).toBe("axis");
  expect(view.pending).toEqual({ "set-up": { side: "axis" } });
  expect(view.clock.game_turn).toBe(1);
  expect(view.rules_commit).toMatch(/^[0-9a-f]{40}$/);

  await page.locator("[data-hex=C3922]").click();
  await expect(page.getByTestId("hex")).toHaveText("hex C3922");

  await page.screenshot({ path: join(shots, "grid.png") });
  await test.info().attach("grid", { path: join(shots, "grid.png"), contentType: "image/png" });
});

test("stays usable at a phone width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  await expect(page.locator("[data-hex]")).toHaveCount(7073);
  // Nothing overflows the viewport sideways.
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.screenshot({ path: join(shots, "grid-390.png") });
});
