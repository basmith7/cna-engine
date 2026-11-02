import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { clickHex, expectDomMatchesView, here, openFixture, shoot as shootInto, shotsDir, throughCurtain } from "./helpers.ts";

// Slice 4's proving screenshots.
const shots = shotsDir("slice-4");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

/** `cna-play replay` of golden orders 0–64, as the SHA-256 of the state JSON (pinned by `cna-web`'s `the_e2e_fixtures_are_current`). */
const movementDone = readFileSync(here("fixtures/movement-done.sha256"), "utf8").trim();
const sha256 = (s: string) => createHash("sha256").update(s).digest("hex");

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");
const lastLine = (page: Page) => log(page).last();

/** The hex of a piece, from the engine's view. */
async function hexOf(page: Page, id: string): Promise<string | null> {
  return page.evaluate((id) => {
    const l = window.__cna.view().pieces[id].location;
    return typeof l === "object" && "hex" in l ? l.hex : null;
  }, id);
}

test("golden orders 59–64 by clicks: initiative, the Libyans into Sollum, a reaction refused for want of fuel, Fort Capuzzo, end segment", async ({ page }) => {
  test.setTimeout(120_000);
  await openFixture(page, "set-up-done");
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · OpStage 1 · A · initiative");
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");

  // 59: the Axis, holding the initiative, declares itself Player A.
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="declare-initiative"][data-side="axis"]').click();
  await expect(lastLine(page)).toContainText("C · organisation");
  await expect(log(page).filter({ hasText: "Axis declares itself Player A." })).toHaveCount(1);
  // Phase C (abstract supply): the Axis, then the Commonwealth, organise
  // nothing; the curtain hands over each way.
  await expect(page.getByTestId("organise-hint")).toBeVisible();
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "cw");
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "axis");
  await expect(lastLine(page)).toContainText("F · reserve designation");
  // 60: nothing in reserve.
  await controls.locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · OpStage 1 · G · movement and combat · Player A · movement");
  await expect(page.getByTestId("curtain")).toHaveCount(0);

  // 61: the 1st Libyan Division's stack at Fort Capuzzo (C4020): its own
  // eleven pieces tick themselves, the overlay shades what the engine
  // allows, Sollum (C4021) at the pinned 1 CP. The proving screenshot,
  // the path to C4021 under the pointer.
  await clickHex(page, "C4020");
  const panel = page.getByTestId("stack-panel");
  await expect(panel.locator("input.tick:checked")).toHaveCount(11);
  await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "move");
  await expect(page.getByTestId("reach-count")).toContainText(/^\d+ hexes reachable, up to 15 CP\./);
  const sollum = page.locator('polygon.reach[data-hex="C4021"]');
  await expect(sollum).toHaveAttribute("data-cp", "1");
  await expect(sollum).toHaveAttribute("data-dp", "0");
  await expect(page.locator('text.cp[data-hex="C4021"]')).toHaveText("1");
  await page.locator('polygon.hex[data-hex="C4021"]').hover();
  await expect(page.getByTestId("hover-path")).toBeAttached();
  await shoot(page, "reachable.png");
  await page.getByTestId("assault").check();
  await page.locator('polygon.hex[data-hex="C4021"]').click();
  await expect(log(page).filter({ hasText: "C4020 → C4021 for 1 CP." })).toHaveCount(1);
  await expect(lastLine(page)).toContainText("Reaction offered to 4th Ryl Horse Arty, 7th Medium Artillery Regt against C4021.");
  expect(await hexOf(page, "unit:it:1st-libyan-infantry-hq")).toBe("C4021");
  await expectDomMatchesView(page);
  await throughCurtain(page, "cw");

  // The Commonwealth tries to react with 4 RHA, from the reaction panel
  // to a shaded hex; the hexes next to the Libyans are not offered
  // (8.5x). It owes 1 fuel and no supply unit in range can pay, so the
  // engine refuses and nothing changes (32.22, R-120).
  const pending = page.getByTestId("pending");
  await expect(pending).toHaveAttribute("data-kind", "reaction");
  await expect(pending).toContainText("Reaction · Commonwealth");
  await pending.locator('[data-unit="unit:cw:4-rha"] input').check();
  await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "react");
  await expect(page.locator('polygon.reach[data-hex="C3923"]')).toBeAttached();
  for (const h of ["C3921", "C4022", "C4121", "C4122"]) await expect(page.locator(`polygon.reach[data-hex="${h}"]`)).toHaveCount(0);
  const offered = await page.evaluate(() => window.__cna.save());
  await page.locator('polygon.hex[data-hex="C3923"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (32.22)");
  expect(await page.evaluate(() => window.__cna.save())).toBe(offered);
  await page.getByTestId("toast").getByLabel("Dismiss").click();
  expect(await hexOf(page, "unit:cw:4-rha")).toBe("C3922");
  // 62: the Commonwealth passes: no reaction.
  await expect(page.getByTestId("to-act")).toHaveText("Commonwealth to act");
  await pending.locator('[data-order="pass"]').click();
  await throughCurtain(page, "axis");
  await expect(page.getByTestId("pending")).toHaveCount(0);

  // A custom path up the escarpment from Sollum: shift-clicks build it,
  // Enter sends it, the engine refuses it and nothing changes.
  const before = await page.evaluate(() => window.__cna.save());
  await clickHex(page, "C4021");
  await expect(panel.locator("input.tick:checked")).toHaveCount(11);
  await clickHex(page, "C3921", ["Shift"]);
  await expect(page.getByTestId("custom")).toContainText("Custom path: C3921");
  await expect(page.getByTestId("custom-path")).toBeAttached();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("toast")).toContainText("Illegal (8.4x)");
  expect(await page.evaluate(() => window.__cna.save())).toBe(before);
  await page.getByTestId("toast").getByLabel("Dismiss").click();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("custom-path")).toHaveCount(0);

  // 63: the 2nd Libyan Division from C3920 into Fort Capuzzo, now empty.
  // C3920 is reachable for the Libyans still ticked at C4021: a click
  // on a hex of one's own selects it and never moves onto it.
  await expect(page.locator('polygon.reach[data-hex="C3920"]')).toBeAttached();
  await clickHex(page, "C3920");
  await expect(panel.locator('[data-unit="unit:it:2nd-libyan-infantry-hq"] input.tick')).toBeChecked();
  await expect(panel.locator("input.tick:checked")).toHaveCount(11);
  expect(await hexOf(page, "unit:it:1st-libyan-infantry-hq")).toBe("C4021");
  await expect(page.locator('polygon.reach[data-hex="C4020"]')).toBeAttached();
  await page.locator('polygon.hex[data-hex="C4020"]').click();
  await expect(log(page).filter({ hasText: "C3920 → C4020" })).toHaveCount(1);
  expect(await hexOf(page, "unit:it:2nd-libyan-infantry-hq")).toBe("C4020");
  await expectDomMatchesView(page);

  // 64: end the segment: gun positions are sealed first, the Axis to act
  // (slice 5's panel); the state is `replay`'s.
  await controls.locator('[data-order="end-segment"]').click();
  await expect(page.getByTestId("pending")).toHaveAttribute("data-kind", "sealed");
  await expect(page.getByTestId("pending")).toContainText("Sealed gun positions · Axis");
  await expect(page.getByTestId("sealed-panel")).toBeAttached();
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  await expect(page.getByTestId("phase-controls")).toHaveCount(0);
  await expect(page.getByTestId("curtain")).toHaveCount(0);
  expect(sha256(await page.evaluate(() => window.__cna.save()))).toBe(movementDone);
});

test("a ZOC declaration is answered from its panel", async ({ page }) => {
  await openFixture(page, "declare-zoc");
  const pending = page.getByTestId("pending");
  await expect(pending).toHaveAttribute("data-kind", "declare-zoc");
  await expect(pending).toContainText("An enemy stack is about to move next to you");
  await expect(page.locator('[data-testid="marks"][data-kind="zoc"]')).toHaveAttribute("data-count", "1");
  await expect(page.locator('polygon.mark[data-hex="C3721"]')).toBeAttached();
  await expect(pending.locator('input[data-hex="C3721"]')).toBeChecked();
  await expect(page.getByTestId("phase-controls")).toHaveCount(0);
  await pending.locator('[data-order="declare-zoc"]').click();
  await expect(log(page).filter({ hasText: "C3721 exerts a ZOC." })).toHaveCount(1);
  await throughCurtain(page, "axis");
  await expect(page.getByTestId("pending")).toHaveCount(0);
  await expect(page.locator('[data-testid="marks"][data-kind="zoc"]')).toHaveAttribute("data-count", "0");
  expect((await page.evaluate(() => window.__cna.view())).zoc).toEqual({ C3721: true });
});

test("a stack in the Tripoli box moves off-map by its buttons; Gabes is refused", async ({ page }) => {
  await openFixture(page, "off-map");
  const box = page.locator('[data-where="box-tripoli"]');
  await expect(box.locator("[data-unit]")).toHaveCount(30);
  // Two battalions on their own: the legions' are attached to their HQs.
  await box.locator('[data-unit="unit:it:203rd-engineer-bn"]').click();
  await box.locator('[data-unit="unit:it:203rd-machinegun-bn"]').click();
  await expect(box.locator('[aria-pressed="true"]')).toHaveCount(2);
  const moves = page.getByTestId("off-map-moves");
  await expect(moves.locator("button")).toHaveCount(5);
  await moves.locator('[data-to="gabes"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (8.8x)");
  await page.getByTestId("toast").getByLabel("Dismiss").click();
  await moves.locator('[data-to="tripolitania"]').click();
  await expect(log(page).filter({ hasText: "moves off-map" })).toHaveCount(2);
  await expect(box.locator("[data-unit]")).toHaveCount(28);
  await expectDomMatchesView(page);
});

test("reserves are designated in phase F and released in the release segment", async ({ page }) => {
  await openFixture(page, "reserve");
  await expect(page.getByTestId("clock")).toContainText("F · reserve designation");
  const controls = page.getByTestId("phase-controls");
  await clickHex(page, "C4020");
  await expect(controls.locator('[data-order="designate-reserve"]')).toHaveText("Designate reserve (11 ticked)");
  await controls.locator('[data-order="designate-reserve"]').click();
  await expect(log(page).filter({ hasText: "designated reserve." })).toHaveCount(1);
  await expect(page.locator('[data-stack="C4020"] .marker.reserve-I')).toHaveCount(1);
  await controls.locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).toContainText("Player A · movement");
  // The combat steps between are slice 5's: the fixture has them drained.
  await openFixture(page, "reserve-release");
  await expect(page.getByTestId("clock")).toContainText("Player A · reserve release");
  await expect(page.locator('[data-stack="C4020"] .marker.reserve-I')).toHaveCount(1);
  await clickHex(page, "C4020");
  await expect(controls.locator('[data-order="release-reserve"]')).toHaveText("Release reserve (11 ticked)");
  await controls.locator('[data-order="release-reserve"]').click();
  await expect(log(page).filter({ hasText: "released from Reserve I." })).toHaveCount(1);
  await expect(page.locator('[data-stack="C4020"] .marker.reserve-I')).toHaveCount(0);
  await expectDomMatchesView(page);
});

test("stays usable at a phone width with the overlay up", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page, "off-map");
  await clickHex(page, "C4020");
  await expect(page.getByTestId("reachable")).toHaveAttribute("data-count", /^[1-9]\d*$/);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.getByTestId("move-panel").scrollIntoViewIfNeeded();
  await expect(page.getByTestId("move-panel")).toBeInViewport();
  await page.evaluate(() => window.scrollTo(0, 0));
  await shoot(page, "reachable-390.png");
});
