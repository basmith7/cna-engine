import { expect, test, type Locator, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Assignment, Fire, HexId, Order, PieceId, Side } from "../src/types.ts";
import { clickHex, expectDomMatchesView, here, openStart, shoot as shootInto, shotsDir } from "./helpers.ts";

// Slice 6's proving screenshot: the end dialog after the golden game
// played by clicks (spec, *What "done" means* 8).
const shots = shotsDir("slice-6");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

type Logged = { player: Side; order: Order };
type GoldenLog = { seed: number; orders: Logged[] };

/** M5's golden game (abstract supply on): 458 orders, seed 7, ending at the start of GT7 with neither side at a level. */
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

/** Ticks exactly `pieces` in the stack panel, whatever it ticked by itself. */
async function tickExactly(page: Page, pieces: PieceId[]) {
  const panel = page.getByTestId("stack-panel");
  const want = new Set(pieces);
  for (const li of await panel.locator("li[data-unit]").all()) {
    const id = (await li.getAttribute("data-unit"))!;
    const tick = li.locator("input.tick");
    if ((await tick.count()) === 0) {
      expect(want.has(id), `${id} is not tickable`).toBe(false);
      continue;
    }
    if (want.has(id)) await tick.check();
    else await tick.uncheck();
  }
  await expect(panel.locator("input.tick:checked")).toHaveCount(pieces.length);
}

/**
 * Sends a stack along `path` by clicks: a one-hex path onto an empty hex
 * by a click on it (the engine's path to a neighbour is that hex); any
 * other path hex by hex with Shift, then Enter, so the path is the log's
 * and not the overlay's cheapest. A one-hex path onto a hex with a stack
 * takes the Shift route too: a plain click there selects (own pieces) or
 * is intercepted by the stack's counter (the stack is drawn over the hex
 * polygon), and the app's way of moving onto own pieces is Shift + Enter.
 */
async function walk(page: Page, path: HexId[]) {
  if (path.length === 1 && (await page.locator(`[data-stack="${path[0]}"]`).count()) === 0) {
    await expect(page.locator(`polygon.reach[data-hex="${path[0]}"]`)).toBeAttached();
    await page.locator(`polygon.hex[data-hex="${path[0]}"]`).click();
    return;
  }
  for (const h of path) await clickHex(page, h, ["Shift"]);
  await expect(page.getByTestId("custom")).toContainText(`Custom path: ${path.join(" → ")}`);
  await page.keyboard.press("Enter");
}

/** Adds a barrage fire row as the panel offers it, then types the log's points and target. */
async function addFire(sealed: Locator, f: Fire) {
  await sealed.locator(`[data-add-fire="${f.piece}"]`).click();
  const row = sealed.locator(`[data-unit="${f.piece}"] [data-fire]`).last();
  await row.locator("[data-points]").fill(String(f.points));
  await row.locator("[data-target]").selectOption(f.target);
}

/** Adds the rows of one unit's assignment: a row per allocation in each role. */
async function addAssignment(sealed: Locator, a: Assignment) {
  for (const role of ["anti-armour", "close-assault"] as const) {
    const allocs = role === "anti-armour" ? (a.anti_armour ?? []) : (a.close_assault ?? []);
    for (const alloc of allocs) {
      await sealed.locator(`[data-add-role="${a.piece}:${role}"]`).click();
      const row = sealed.locator(`[data-unit="${a.piece}"] [data-role][data-kind="${role}"]`).last();
      await row.locator("[data-points]").fill(String(alloc.points));
      if (alloc.target !== undefined) await row.locator("[data-target]").selectOption(alloc.target);
      else await expect(row.locator("[data-target]")).toHaveCount(0);
    }
  }
}

/** Ticks a gun in the positions panel, opening the fold of far guns if it is in there. */
async function tickGun(sealed: Locator, id: PieceId) {
  const box = sealed.locator(`[data-gun="${id}"]`);
  if (!(await box.isVisible())) await sealed.locator("details.far summary").click();
  await box.check();
}

/**
 * The clicks that produce one logged order, or a failure for an order
 * the UI cannot express by clicks: nothing is sent to the engine by
 * any other route.
 */
async function perform(page: Page, i: number, { order }: Logged) {
  const controls = page.getByTestId("phase-controls");
  const pending = page.getByTestId("pending");
  const sealed = page.getByTestId("sealed-panel");
  if (order === "pass") {
    // Pass belongs to the open interrupt's panel while one is open.
    if ((await pending.count()) > 0) await pending.locator('[data-order="pass"]').click();
    else await controls.locator('[data-order="pass"]').click();
    return;
  }
  if (order === "end-segment") {
    await controls.locator('[data-order="end-segment"]').click();
    return;
  }
  if ("place" in order) {
    const { piece, hex } = order.place;
    const button = page.locator(`[data-where^="tray-"] [data-unit="${piece}"]`);
    if ((await button.getAttribute("aria-pressed")) !== "true") await button.click();
    await expect(page.locator(`polygon.placement[data-hex="${hex}"]`)).toBeAttached();
    await clickHex(page, hex);
    return;
  }
  if ("declare-initiative" in order) {
    await controls.locator(`[data-order="declare-initiative"][data-side="${order["declare-initiative"].player_a}"]`).click();
    return;
  }
  if ("move" in order) {
    const { pieces, path, assault } = order.move;
    await clickHex(page, await hexOf(page, pieces[0]));
    await tickExactly(page, pieces);
    await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "move");
    await page.getByTestId("assault").setChecked(assault ?? false);
    await walk(page, path);
    return;
  }
  if ("react" in order) {
    const { pieces, path } = order.react;
    await expect(pending).toHaveAttribute("data-kind", "reaction");
    for (const id of pieces) await pending.locator(`[data-unit="${id}"] input`).check();
    await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "react");
    await walk(page, path);
    return;
  }
  if ("gun-positions" in order) {
    await expect(pending).toHaveAttribute("data-kind", "sealed");
    for (const id of order["gun-positions"].forward) await tickGun(sealed, id);
    await sealed.locator('[data-order="gun-positions"]').click();
    return;
  }
  if ("barrage" in order) {
    await expect(pending).toHaveAttribute("data-kind", "sealed");
    for (const f of order.barrage.fires) await addFire(sealed, f);
    await sealed.locator('[data-order="barrage"]').click();
    return;
  }
  if ("assign-forces" in order) {
    await expect(pending).toHaveAttribute("data-kind", "sealed");
    for (const a of order["assign-forces"].assignments) await addAssignment(sealed, a);
    await sealed.locator('[data-order="assign-forces"]').click();
    return;
  }
  if ("assault" in order) {
    await expect(page.locator(`polygon.mark[data-hex="${order.assault.target}"]`)).toBeAttached();
    await clickHex(page, order.assault.target);
    return;
  }
  if ("march-prisoners" in order) {
    const { from, path, points } = order["march-prisoners"];
    const panel = page.getByTestId("march-panel");
    await panel.getByLabel(`Points to march from ${from}`).fill(String(points));
    await panel.getByLabel(`Points to march from ${from}`).locator("xpath=ancestor::li").locator('[data-testid="march-arm"]').click();
    for (const h of path) await clickHex(page, h);
    await panel.getByTestId("march-send").click();
    return;
  }
  if ("detail-guards" in order) {
    const { piece, points } = order["detail-guards"];
    await clickHex(page, await hexOf(page, piece));
    const row = page.getByTestId("guard-controls");
    await row.getByLabel("Unit for guards").selectOption(piece);
    await row.getByLabel("Guard points").fill(String(points));
    await row.locator('[data-order="detail-guards"]').click();
    return;
  }
  // declare-zoc, move-off-map, retreat-before-assault, retreat,
  // designate-reserve, release-reserve: not in the golden game (spec,
  // *Scripted games*); each has its own fixture e2e. Reaching one here
  // is a failure, never a skip.
  throw new Error(`order ${i} cannot be expressed by clicks by this driver: ${JSON.stringify(order)}`);
}

test("the golden game, all 458 orders by clicks, with a save, reload and load in the middle, ends with replay's state and the report open", async ({ page }) => {
  test.setTimeout(900_000);
  // A stuck click (a counter over the hex, a curtain over a button) fails
  // in seconds with the order named, not at the test's timeout.
  page.setDefaultTimeout(15_000);
  expect(golden.orders).toHaveLength(458);
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
    await perform(page, i, logged);
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
