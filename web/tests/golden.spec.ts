import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { HexId, PieceId, Side } from "../src/types.ts";
import { type Logged, perform } from "./drive.ts";
import { expectDomMatchesView, here, openStart, shoot as shootInto, shotsDir } from "./helpers.ts";

// Slice 6's proving screenshot: the end dialog after the golden game
// played by clicks (spec, *What "done" means* 8).
const shots = shotsDir("slice-6");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

type GoldenLog = { seed: number; orders: Logged[] };

/** M5's golden game (abstract supply on): 487 orders, seed 7, ending at the start of GT7 with neither side at a level. */
const golden: GoldenLog = JSON.parse(readFileSync(here("../../tests/games/graziani-m5.json"), "utf8")) as GoldenLog;
/** `cna-play replay` of the whole log, as the SHA-256 of the state JSON (pinned by `cna-web`'s `the_e2e_fixtures_are_current` and `cna-game::golden`). */
const goldenDone = readFileSync(here("fixtures/golden-done.sha256"), "utf8").trim();
const sha256 = (s: string) => createHash("sha256").update(s).digest("hex");

/** The orders after which the game is saved, the page reloaded and the file loaded: the scripted fighting is done (plan, task 5). */
const SAVE_AFTER = 84;

const sideName = (s: Side) => (s === "axis" ? "Axis" : "Commonwealth");

/** The hex a piece stands on, from the engine's view. */
async function hexOf(page: Page, id: PieceId): Promise<HexId> {
  const h = await page.evaluate((id) => {
    const l = window.__cna.view().pieces[id].location;
    return typeof l === "object" && "hex" in l ? l.hex : null;
  }, id);
  expect(h, `${id} is on a hex`).not.toBeNull();
  return h!;
}

test("the golden game, all 487 orders by clicks, with a save, reload and load in the middle, ends with replay's state and the report open", async ({ page }) => {
  test.setTimeout(900_000);
  // A stuck click (a counter over the hex, a curtain over a button) fails
  // in seconds with the order named, not at the test's timeout.
  page.setDefaultTimeout(15_000);
  expect(golden.orders).toHaveLength(487);
  await openStart(page);
  await page.getByTestId("seed").fill(String(golden.seed));
  await page.getByTestId("new-game").click();
  await page.locator("[data-stack]").first().waitFor();

  const curtain = page.getByTestId("curtain");
  const illegal = page.getByTestId("log-entries").locator("li.illegal");
  for (const [i, logged] of golden.orders.entries()) {
    // The hot-seat hand-over: the curtain is down for exactly the side
    // whose order comes next.
    if ((await curtain.count()) > 0) {
      await expect(curtain).toHaveAttribute("data-side", logged.player);
      await curtain.click();
      await expect(curtain).toHaveCount(0);
    }
    if (i === SAVE_AFTER) {
      // Save, reload the page, load the file, play on. This comes after
      // the curtain is lifted: order 84 hands over, and the curtain would
      // intercept the Save click.
      const download = page.waitForEvent("download");
      await page.getByTestId("save").click();
      const file = await download;
      const path = join(test.info().outputPath(), file.suggestedFilename());
      await file.saveAs(path);
      const saved = readFileSync(path, "utf8");
      expect(saved).toBe(await page.evaluate(() => window.__cna.save()));
      await page.reload();
      await expect(page.getByTestId("start")).toBeVisible();
      await page.getByTestId("start").getByLabel("Load a save").setInputFiles(path);
      await page.locator("[data-stack]").first().waitFor();
      expect(await page.evaluate(() => window.__cna.save())).toBe(saved);
      expect(await page.evaluate(() => window.__cna.orders())).toBe(SAVE_AFTER);
      // A loaded game opens behind the curtain of the side to act.
      await expect(curtain).toHaveAttribute("data-side", logged.player);
      await curtain.click();
      await expect(curtain).toHaveCount(0);
      await expectDomMatchesView(page);
    }
    await expect(page.getByTestId("to-act")).toHaveText(`${sideName(logged.player)} to act`);
    await perform(page, i, logged, hexOf);
    // The engine accepted it: one more order in its log, no refusal.
    await expect.poll(() => page.evaluate(() => window.__cna.orders()), { message: `order ${i} accepted` }).toBe(i + 1);
    await expect(illegal).toHaveCount(0);
    // The DOM's counters are the view's after every order.
    await expectDomMatchesView(page);
  }

  // The end: the report open, no level for either side, every order
  // control gone, and the state byte-equal to replay's.
  await expect(page.getByTestId("to-act")).toHaveText("Game over");
  const dialog = page.getByTestId("end-dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator('[data-level="axis"]')).toContainText("No level");
  await expect(dialog.locator('[data-level="cw"]')).toContainText("No level");
  await expect(dialog).toContainText("The clock stopped at Game-Turn 7 · OpStage 1 · A · initiative.");
  await expect(page.getByTestId("phase-controls")).toHaveCount(0);
  await expect(page.getByTestId("pending")).toHaveCount(0);
  const view = await page.evaluate(() => window.__cna.view());
  expect(view.over).not.toBeNull();
  expect(view.over!.axis).toBeNull();
  expect(view.over!.cw).toBeNull();
  expect(sha256(await page.evaluate(() => window.__cna.save()))).toBe(goldenDone);
  await shoot(page, "golden-end.png");
  await page.setViewportSize({ width: 390, height: 844 });
  // The dialog body scrolls at this height; the close button is reachable.
  await dialog.getByTestId("end-close").scrollIntoViewIfNeeded();
  await expect(dialog.getByTestId("end-close")).toBeInViewport();
  await shoot(page, "golden-end-390.png");
});
