import { expect, test, type Page } from "@playwright/test";
import type { Firer, View } from "../src/types.ts";
import { clickHex, expectDomMatchesView, openFixture, shoot as shootInto, shotsDir, throughCurtain } from "./helpers.ts";

// The Mediterranean Fleet (§30), plan Task 4's proving screenshots: the
// module is off for new games, so both fixtures turn it on by hand.
const shots = shotsDir("m5-fleet");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");

async function open(page: Page, fixture: string) {
  await openFixture(page, fixture);
  const curtain = page.getByTestId("curtain");
  if ((await curtain.count()) > 0) await throughCurtain(page, (await curtain.getAttribute("data-side")) as "axis" | "cw");
}

test("phase E: a ship in Alexandria is stationed by clicks, drawn at sea, and Pass ends the phase (30.1x)", async ({ page }) => {
  await open(page, "fleet");
  await expect(page.getByTestId("clock")).toContainText("E · CW fleet");
  const panel = page.getByTestId("fleet-panel");
  await expect(panel.getByTestId("ships-in-port").locator("li[data-unit]")).toHaveCount(11);
  await expect(page.getByTestId("ships")).toHaveAttribute("data-count", "0");
  const { ship, hexes, empty } = await page.evaluate(() => {
    const view: View = window.__cna.view();
    const o = view.station_options!;
    const held = new Set(Object.values(view.pieces).flatMap((p) => (typeof p.location === "object" && "hex" in p.location ? [p.location.hex] : [])));
    return { ship: "ship:cw:valiant", hexes: o.hexes.length, empty: o.hexes.find((h) => !held.has(h) && h.startsWith("E"))! };
  });

  // An inland hex is refused by the engine, and nothing is marked before a pick.
  await expect(page.locator('[data-testid="marks"][data-kind="station"]')).toHaveAttribute("data-count", "0");
  await panel.locator(`li[data-unit="${ship}"] input[type="radio"]`).check();
  await expect(page.locator('[data-testid="marks"][data-kind="station"]')).toHaveAttribute("data-count", String(hexes));
  await clickHex(page, "E1730");
  await expect(page.getByTestId("toast")).toContainText("Illegal (30.1x)");
  await page.getByTestId("toast").getByLabel("Dismiss").click();

  await clickHex(page, empty);
  await expect(log(page).filter({ hasText: `Valiant puts to sea at ${empty} (30.1x).` })).toHaveCount(1);
  await expect(log(page).filter({ hasText: "undefined" })).toHaveCount(0);
  await expect(page.locator(`svg.map g[data-ship="${ship}"][data-hex="${empty}"]`)).toHaveCount(1);
  await expect(panel.getByTestId("ships-at-sea").locator(`li[data-unit="${ship}"]`)).toContainText(`Valiant at ${empty}`);
  await expect(panel.getByTestId("ships-in-port").locator("li[data-unit]")).toHaveCount(10);
  await shoot(page, "ship-stationed.png");
  await expectDomMatchesView(page);

  await page.getByTestId("phase-controls").locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).not.toContainText("E · CW fleet");
  await expect(page.locator(`svg.map g[data-ship="${ship}"]`)).toHaveCount(1);
});

test("the barrage step offers a stationed ship its one point count, and it fires (30.2x)", async ({ page }) => {
  await open(page, "fleet-barrage");
  const pending = page.getByTestId("pending");
  await expect(pending).toContainText("Sealed barrage · Commonwealth");
  const ship = "ship:cw:valiant";
  const firer = await page.evaluate((ship) => {
    const o = window.__cna.sealedOptions() as { ok: { barrage: { firers: Firer[] } } };
    return o.ok.barrage.firers.find((f) => f.piece === ship)!;
  }, ship);
  const target = firer.targets[0];
  const points = firer.ship_points![target];
  await expect(page.locator(`svg.map g[data-ship="${ship}"]`)).toHaveCount(1);
  await pending.locator(`[data-add-fire="${ship}"]`).click();
  const fire = pending.locator('[data-fire="0"]');
  await expect(fire.locator("[data-points]")).toHaveValue(String(points));
  await expect(fire.locator("[data-points]")).toHaveAttribute("readonly", "");
  await shoot(page, "ship-barrage-entry.png");
  await pending.locator('[data-order="barrage"]').click();
  await expect(log(page).filter({ hasText: new RegExp(`^Commonwealth barrage on .+ by HMS Valiant: ${points} points`) })).toHaveCount(1);
  await expect(log(page).filter({ hasText: "undefined" })).toHaveCount(0);
  await expect(page.getByTestId("toast")).toHaveCount(0);
  expect(await page.evaluate((ship) => window.__cna.view().fleet?.[ship]?.fired, ship)).toBe(true);
  await shoot(page, "ship-fired.png");
});
