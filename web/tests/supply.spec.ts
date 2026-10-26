import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { clickHex, here, openFixture, openStart, throughCurtain } from "./helpers.ts";
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

test("phase C: a supply unit flies from Tobruk to an HQ at El Adem, once a Game-Turn (32.37)", async ({ page }) => {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await clickHex(page, "C4807");
  const tick = () => page.locator('[data-unit="supply:axis:1"] input.tick').check();
  await tick();
  // Bardia (column 21) lies east of the farthest Axis division HQ, at Fort Capuzzo's column 20 (R-124).
  await controls.locator('[data-order="air-lift"]').click();
  await expect(page.getByTestId("fly-hint")).toBeVisible();
  await clickHex(page, "C4321");
  await expect(page.getByTestId("toast")).toContainText("Illegal (32.37)");
  await expect(page.getByTestId("toast")).toContainText("beyond the farthest axis division HQ");
  // El Adem holds the 240th Legion's HQ.
  await clickHex(page, "C4807");
  await tick();
  await controls.locator('[data-order="air-lift"]').click();
  await clickHex(page, "C4507");
  await expect(log(page).filter({ hasText: "flown C4807 → C4507, 12 fuel from" })).toHaveCount(1);
  const s = await page.evaluate(() => window.__cna.view().pieces["supply:axis:1"]);
  expect(s.location).toEqual({ hex: "C4507" });
  expect(s.load?.fuel).toBe(38);
  // A second flight this Game-Turn is refused.
  await clickHex(page, "C4807");
  await page.locator('[data-unit="supply:axis:2"] input.tick').check();
  await controls.locator('[data-order="air-lift"]').click();
  await clickHex(page, "C4507");
  await expect(page.getByTestId("toast")).toContainText("this Game-Turn");
});

test("phase C: a supply unit flies back to the Tripoli box, where the 2nd CCNN Division's HQ waits (32.37)", async ({ page }) => {
  await openFixture(page, "set-up-done");
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await clickHex(page, "C4807");
  await page.locator('[data-unit="supply:axis:1"] input.tick').check();
  await controls.locator('[data-order="air-lift"]').click();
  await controls.locator('[data-order="air-lift-box"][data-box="tripoli"]').click();
  await expect(log(page).filter({ hasText: "flown C4807 → the tripoli box" })).toHaveCount(1);
  const s = await page.evaluate(() => window.__cna.view().pieces["supply:axis:1"]);
  expect(s.location).toEqual({ box: "tripoli" });
});

test("phase C: a foot battalion takes a motorisation point per TOE point and moves at 20 (32.54, R-123)", async ({ page }) => {
  // The set-up, with 5 points beside the I Libyan Motorised Battalion (CPA 10+, 5 TOE points) at C3617.
  const save = JSON.parse(readFileSync(here("fixtures/set-up-done.json"), "utf8"));
  save.motorisation.axis.C3617 = 5;
  await openStart(page);
  await page.evaluate((json) => window.__cna.load(json), JSON.stringify(save));
  await page.locator("[data-stack]").first().waitFor();
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await clickHex(page, "C3617");
  const checked = page.locator('[data-testid="stack-panel"] input.tick:checked');
  while ((await checked.count()) > 0) await checked.first().uncheck();
  // A tank battalion has its own transport.
  await page.locator('[data-unit="unit:it:iim-tank-bn"] input.tick').check();
  await controls.locator('[data-order="attach-motorisation"]').click();
  await expect(page.getByTestId("toast")).toContainText("own transport");
  await page.locator('[data-unit="unit:it:iim-tank-bn"] input.tick').uncheck();
  await page.locator('[data-unit="unit:it:i-libyan-mtr-bn"] input.tick').check();
  await controls.locator('[data-order="attach-motorisation"]').click();
  await expect(log(page).filter({ hasText: "takes 5 motorisation points." })).toHaveCount(1);
  const bn = await page.evaluate(() => window.__cna.view().pieces["unit:it:i-libyan-mtr-bn"]);
  expect(bn.motorised).toBe(5);
  await expect(controls.locator('[data-order="detach-motorisation"]')).toContainText("Dismount");
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

test("a supply unit takes its points in phase C and moves with a combat unit in G (32.31)", async ({ page }) => {
  // The set-up, with 30 motorisation points at Bardia (C4321) to attach.
  const save = JSON.parse(readFileSync(here("fixtures/set-up-done.json"), "utf8"));
  save.motorisation.axis.C4321 = 30;
  await openStart(page);
  await page.evaluate((json) => window.__cna.load(json), JSON.stringify(save));
  await page.locator("[data-stack]").first().waitFor();
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await clickHex(page, "C4321");
  await page.locator('[data-unit="supply:axis:4"] input.tick').check();
  await controls.locator('[data-order="attach-motorisation"]').click();
  await expect(page.getByTestId("log-entries").locator("li.event", { hasText: "takes 30 motorisation points." })).toHaveCount(1);
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "cw");
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "axis");
  await controls.locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).toContainText("movement");
  // G: a tank battalion takes the supply unit along; nothing else moves.
  await clickHex(page, "C4321");
  const checked = page.locator('[data-testid="stack-panel"] input.tick:checked');
  while ((await checked.count()) > 0) await checked.first().uncheck();
  await page.locator('[data-unit="unit:it:lxl-tank-bn"] input.tick').check();
  await page.locator('[data-unit="supply:axis:4"] input.tick').check();
  await expect(page.getByTestId("reach-count")).toBeVisible();
  const target = await page.evaluate(() => {
    const v = window.__cna.view();
    const held = new Set((Object.values(v.pieces) as Piece[]).map((p) => (typeof p.location === "object" && "hex" in p.location ? p.location.hex : "")));
    // The nearest free hex on screen.
    const centre = (e: Element) => {
      const b = (e as SVGGraphicsElement).getBBox();
      return [b.x + b.width / 2, b.y + b.height / 2];
    };
    const [x0, y0] = centre(document.querySelector('polygon.hex[data-hex="C4321"]')!);
    const free = [...document.querySelectorAll("polygon.reach")].filter((e) => !held.has(e.getAttribute("data-hex")!));
    free.sort((a, b) => Math.hypot(centre(a)[0] - x0, centre(a)[1] - y0) - Math.hypot(centre(b)[0] - x0, centre(b)[1] - y0));
    return free[0]?.getAttribute("data-hex") ?? null;
  });
  expect(target).not.toBeNull();
  await page.locator(`polygon.hex[data-hex="${target}"]`).click();
  const at = await page.evaluate(() => ["unit:it:lxl-tank-bn", "supply:axis:4"].map((id) => window.__cna.view().pieces[id].location));
  expect(at).toEqual([{ hex: target }, { hex: target }]);
});

test("phase K: field repair of broken-down tanks at Alexandria, one roll a type (22.2x)", async ({ page }) => {
  await openFixture(page, "repair");
  await expect(page.getByTestId("clock")).toContainText("repair");
  await expect(page.getByTestId("repair-hint")).toBeVisible();
  await expect(page.locator('[data-testid="marks"][data-kind="wreck"]')).toHaveAttribute("data-count", "1");
  await clickHex(page, "E3613");
  await expect(page.getByTestId("wreck")).toContainText("4 Mark VI Light");
  const repair = page.getByTestId("phase-controls").locator('[data-order="repair"][data-weapon="Mark VI Light"]');
  await repair.click();
  await expect(page.getByTestId("log-entries").locator("li.event", { hasText: "Field repair at E3613, Mark VI Light" })).toHaveCount(1);
  const after = await page.evaluate(() => {
    const v = window.__cna.view();
    const left = (v.broken_down?.["E3613"] ?? []).reduce((n: number, w: { points: number }) => n + w.points, 0);
    return left + (v.repaired?.cw?.["Mark VI Light"] ?? 0);
  });
  expect(after).toBe(4);
  // One roll per type per hex this phase: a second is the engine's refusal.
  if (await repair.isVisible()) {
    await repair.click();
    await expect(page.getByTestId("toast")).toContainText("22.23");
  }
  await page.getByTestId("phase-controls").locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).not.toContainText("repair");
});
