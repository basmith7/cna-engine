import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { clickHex, expectDomMatchesView, here, newGame, shoot as shootInto, shotsDir } from "./helpers.ts";

// Slice 3's proving screenshots.
const shots = shotsDir("slice-3");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

type Logged = { player: "axis" | "cw"; order: { place: { piece: string; hex: string } } };

/** The golden game's 59 free placements: orders 0–58 of `tests/games/graziani-m5.json`. */
const placements: Logged[] = (JSON.parse(readFileSync(here("../../tests/games/graziani-m5.json"), "utf8")) as { orders: Logged[] }).orders.slice(0, 59);
/** `cna-play replay` of those 59 orders, as the SHA-256 of the state JSON (pinned by `cna-web`'s `the_set_up_done_fixture_is_current`). */
const setUpDone = readFileSync(here("fixtures/set-up-done.sha256"), "utf8").trim();

const sha256 = (s: string) => createHash("sha256").update(s).digest("hex");

/** Picks a tray piece; a click on the one already picked would unpick it. */
async function pick(page: Page, side: string, piece: string) {
  const button = page.locator(`[data-where="tray-${side}"] [data-unit="${piece}"]`);
  if ((await button.getAttribute("aria-pressed")) !== "true") await button.click();
  await expect(page.getByTestId("setup").locator(`[data-unit="${piece}"]`)).toBeAttached();
}

test("both sides place their free pieces by clicking, through the curtain, to GT1 phase A", async ({ page }) => {
  test.setTimeout(240_000);
  await newGame(page);
  await expect(page.getByTestId("setup")).toContainText("Set-up · Axis 36 to place");
  await expect(page.getByTestId("placements")).toHaveAttribute("data-count", "0");

  // The first Axis piece picked: its legal hexes shaded, its placement in
  // words and its constraint prose under it. The proving screenshot.
  await pick(page, "axis", placements[0].order.place.piece);
  const count = Number(await page.getByTestId("placements").getAttribute("data-count"));
  expect(count).toBeGreaterThan(100);
  await expect(page.getByTestId("setup")).toContainText("any hex of sheet C, not within 4 hexes of an enemy unit");
  await expect(page.getByTestId("setup")).toContainText("also: Libyan part of sheet C; all units of the group in one hex");
  await expect(page.getByTestId("placement-count")).toHaveText(`${count} hexes allowed. Click one to place it.`);
  // The shaded hexes are the engine's, every one on sheet C.
  const shaded = await page.locator("polygon.placement").evaluateAll((ps) => ps.map((p) => p.getAttribute("data-hex")!));
  expect(shaded.length).toBe(count);
  expect(shaded.every((h) => h.startsWith("C"))).toBe(true);
  await shoot(page, "placements.png");

  // The other side's tray is not clickable yet.
  await expect(page.locator('[data-where="tray-cw"] [data-unit]').first()).toBeDisabled();

  // A deliberate illegal click: a hex outside the shading is sent anyway
  // and the engine refuses it; nothing changes.
  const before = await page.evaluate(() => window.__cna.save());
  await clickHex(page, "D2010");
  await expect(page.getByTestId("toast")).toContainText("Illegal (59.2)");
  await expect(page.getByTestId("log-entries").locator("li.illegal")).toHaveText(/^Illegal \(59\.2\): D2010 /);
  expect(await page.evaluate(() => window.__cna.save())).toBe(before);
  await expect(page.getByTestId("setup")).toContainText("36 to place");
  await page.getByTestId("toast").getByLabel("Dismiss").click();
  await expect(page.getByTestId("toast")).toHaveCount(0);

  let placed = 0;
  for (const [i, { player, order }] of placements.entries()) {
    const { piece, hex } = order.place;
    await expect(page.getByTestId("to-act")).toHaveText(`${player === "axis" ? "Axis" : "Commonwealth"} to act`);
    // The tray picks the next piece by itself after a placement; the
    // script picks its own, as a player choosing an order would.
    await pick(page, player, piece);
    await expect(page.getByTestId("placements")).toHaveAttribute("data-count", /^[1-9]\d*$/);
    await expect(page.locator(`polygon.placement[data-hex="${hex}"]`)).toBeAttached();
    await clickHex(page, hex);
    placed++;
    // One line per placement; the last one also starts play (set-up complete, the phase, the initiative).
    if (i < 58) await expect(page.getByTestId("log-entries").locator("li.event")).toHaveCount(placed, { timeout: 10_000 });
    else await expect(page.getByTestId("log-entries").locator("li.event").last()).not.toContainText("placed at", { timeout: 10_000 });
    const { pending } = await expectDomMatchesView(page);
    expect(pending.axis + pending.cw).toBe(59 - placed);

    if (i === 35) {
      // The Axis is done: the curtain drops for the Commonwealth.
      await expect(page.getByTestId("curtain")).toHaveAttribute("data-side", "cw");
      await expect(page.getByTestId("curtain")).toContainText("Commonwealth to play");
      await shoot(page, "curtain.png");
      await page.getByTestId("curtain").click();
      await expect(page.getByTestId("curtain")).toHaveCount(0);
      await expect(page.getByTestId("setup")).toContainText("Set-up · Commonwealth 23 to place");
    } else if (i < 58) {
      await expect(page.getByTestId("curtain")).toHaveCount(0);
    }
  }

  // Set-up complete: GT1 OpStage 1 phase A, the Axis to act. The
  // Commonwealth placed last, so the curtain is down for the Axis.
  await expect(page.getByTestId("curtain")).toHaveAttribute("data-side", "axis");
  await page.getByTestId("curtain").click();
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · OpStage 1 · A · initiative");
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  await expect(page.getByTestId("setup")).toHaveCount(0);
  await expect(page.locator("[data-where^=tray-]")).toHaveCount(0);
  const log = page.getByTestId("log-entries");
  await expect(log.locator("li.header").first()).toHaveText("Game-Turn 1 · set-up");
  await expect(log.locator("li.event", { hasText: "Set-up complete; play begins." })).toHaveCount(1);
  await expect(log.locator("li.event").last()).toContainText("Game-Turn 1 OpStage 1: A · initiative begins.");

  // The state is what `cna-play replay` makes of the same 59 orders.
  const view = await page.evaluate(() => window.__cna.view());
  expect(view.pending).toBeNull();
  expect(view.to_act).toBe("axis");
  expect(sha256(await page.evaluate(() => window.__cna.save()))).toBe(setUpDone);
});

test("stays usable at a phone width while placing", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await newGame(page);
  await page.getByTestId("off-map").scrollIntoViewIfNeeded();
  await pick(page, "axis", placements[0].order.place.piece);
  await expect(page.getByTestId("placements")).toHaveAttribute("data-count", /^[1-9]\d*$/);
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.evaluate(() => window.scrollTo(0, 0));
  await shoot(page, "placements-390.png");
});
