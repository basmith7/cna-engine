// The clicks that play one logged order, shared by the hot-seat golden
// game (`golden.spec.ts`) and the two-player one (`multiplayer/`).
import { expect, type Locator, type Page } from "@playwright/test";
import type { Assignment, Fire, HexId, Order, PieceId, Side } from "../src/types.ts";
import { clickHex } from "./helpers.ts";

export type Logged = { player: Side; order: Order };

/** Ticks exactly `pieces` in the stack panel, whatever it ticked by itself. */
export async function tickExactly(page: Page, pieces: PieceId[]) {
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
export async function walk(page: Page, path: HexId[]) {
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
export async function perform(page: Page, i: number, { order }: Logged, hexOf: (page: Page, id: PieceId) => Promise<HexId>) {
  const controls = page.getByTestId("phase-controls");
  const pending = page.getByTestId("pending");
  const sealed = page.getByTestId("sealed-panel");
  if (order === "pass") {
    // Pass belongs to the open interrupt's panel while one is open.
    if ((await pending.count()) > 0) await pending.locator('[data-order="pass"]').click();
    else await controls.locator('[data-order="pass"]').click();
    return;
  }
  if (order === "end-segment" || order === "plan-chariot-raid") {
    await controls.locator(`[data-order="${order}"]`).click();
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
