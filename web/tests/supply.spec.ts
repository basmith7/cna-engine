import { expect, test, type Page } from "@playwright/test";
import { clickHex, openFixture, throughCurtain } from "./helpers.ts";
import type { Firer, Piece } from "../src/types.ts";

// Mission 5 slice 2: fuel in the move panel, loads in the stack panel
// (32.1x, 32.22), on the module-on golden game's set-up.

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");

/** From set-up done to the Axis' first movement segment (golden orders 59–60). */
async function toMovement(page: Page) {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  // Phase C: the Axis, then the Commonwealth, pass; then F.
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "cw");
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "axis");
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
    const held = new Set((Object.values(v.pieces) as Piece[]).map((p) => (typeof p.location === "object" && "hex" in p.location ? p.location.hex : "")));
    const hexes = [...document.querySelectorAll("polygon.reach")].map((e) => e.getAttribute("data-hex")!);
    return hexes.find((h) => !held.has(h)) ?? null;
  });
  expect(target).not.toBeNull();
  await page.locator(`polygon.hex[data-hex="${target}"]`).click();
  await expect(page.getByTestId("log-entries").locator("li.illegal").last()).toContainText("Illegal (32.22)");
});

test("the barrage panel says what each firer's fire costs in ammunition, as the engine quotes it", async ({ page }) => {
  const gun = "unit:it:1st-libyan-artillery-regt";
  await openFixture(page, "combat-start");
  const sealed = page.getByTestId("sealed-panel");
  await sealed.locator(`[data-gun="${gun}"]`).check();
  await sealed.locator('[data-order="gun-positions"]').click();
  await page.getByTestId("curtain").click();
  await sealed.locator('[data-order="gun-positions"]').click();
  await page.getByTestId("curtain").click();
  await expect(page.getByTestId("pending")).toContainText("Sealed barrage · Axis");
  const quote = await page.evaluate((gun) => {
    const o = window.__cna.sealedOptions();
    return "ok" in o && "barrage" in o.ok ? o.ok.barrage.firers.find((f: Firer) => f.piece === gun)?.ammo : undefined;
  }, gun);
  expect(quote).toBeGreaterThan(0);
  await expect(sealed.locator(`[data-unit="${gun}"] [data-ammo]`)).toHaveText(`firing costs ${quote} ammunition`);
});

test("phase C: attaching motorisation to a supply unit is the engine's call (32.31)", async ({ page }) => {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await expect(page.getByTestId("clock")).toContainText("C · organisation");
  await expect(page.getByTestId("organise-hint")).toBeVisible();
  // supply:axis:1 stands at C4807 with 5 points in the hex: short of 30.
  await clickHex(page, "C4807");
  await page.locator('[data-unit="supply:axis:1"] input.tick').check();
  await controls.locator('[data-order="attach-motorisation"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (32.31)");
  await expect(page.getByTestId("toast")).toContainText("needs 30 motorisation points and C4807 has 5");
});

test("phase C: two ticked supply units in one hex trade ammunition and fuel within the caps (32.1x)", async ({ page }) => {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  // C4321 holds supply:axis:4 and :5, each 10 ammunition and 40 fuel.
  await clickHex(page, "C4321");
  await page.locator('[data-unit="supply:axis:4"] input.tick').check();
  await page.locator('[data-unit="supply:axis:5"] input.tick').check();
  await controls.locator("[data-transfer-ammo]").fill("5");
  await controls.locator("[data-transfer-fuel]").fill("20");
  await controls.locator('[data-order="transfer-load"]').click();
  // 40 + 20 fuel is the 60 cap: accepted.
  await expect(page.getByTestId("log-entries").locator("li.event").last()).toContainText("5 ammunition and 20 fuel move from");
  await expect(page.locator('[data-unit="supply:axis:5"] [data-testid="load"]')).toHaveText("ammo 15 · fuel 60");
  await expect(page.locator('[data-unit="supply:axis:4"] [data-testid="load"]')).toHaveText("ammo 5 · fuel 20");
  // One more fuel point would pass it: refused.
  await controls.locator("[data-transfer-ammo]").fill("0");
  await controls.locator("[data-transfer-fuel]").fill("1");
  await controls.locator('[data-order="transfer-load"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (32.1x)");
});

test("the movement segment: a side destroys its own supply in a hex before moving, one die each (32.1x)", async ({ page }) => {
  await toMovement(page);
  await clickHex(page, "C4321");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="destroy-supply"]').click();
  // C4321 holds supply:axis:4 and :5: two dice.
  await expect(page.getByTestId("log-entries").locator("li.event", { hasText: "Die for destroy supply" })).toHaveCount(2);
  const left = await page.evaluate(() => ["supply:axis:4", "supply:axis:5"].filter((id) => window.__cna.view().pieces[id].location !== "eliminated").length);
  await expect(page.getByTestId("log-entries").locator("li.event", { hasText: "destroyed by its own side." })).toHaveCount(2 - left);
});

test("phase J: two Commonwealth supply units ride the railway west from Alexandria (32.3x)", async ({ page }) => {
  await openFixture(page, "rail");
  await expect(page.getByTestId("clock")).toContainText("rail");
  // Alexandria holds three supply units; all start ticked, one over the two a stage.
  await clickHex(page, "E3613");
  await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "rail");
  await expect(page.getByTestId("move-refusal")).toContainText("32.3x");
  await page.locator('[data-unit="supply:cw:12"] input.tick').uncheck();
  await expect(page.getByTestId("reach-count")).toContainText("along the railway");
  const target = await page.evaluate(() => {
    const hexes = [...document.querySelectorAll("polygon.reach")].map((e) => e.getAttribute("data-hex")!);
    return hexes.includes("E3312") ? "E3312" : null;
  });
  expect(target).toBe("E3312");
  await page.locator('polygon.hex[data-hex="E3312"]').click();
  await expect(page.getByTestId("log-entries").locator("li.event", { hasText: "by rail E3613 → E3312." })).toHaveCount(1);
  const at = await page.evaluate(() => ["supply:cw:10", "supply:cw:11"].map((id) => window.__cna.view().pieces[id].location));
  expect(at).toEqual([{ hex: "E3312" }, { hex: "E3312" }]);
  // The stage's two have ridden: the third may not.
  await clickHex(page, "E3613");
  await expect(page.getByTestId("move-refusal")).toContainText("32.3x");
  await page.getByTestId("phase-controls").locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).not.toContainText("rail");
});
