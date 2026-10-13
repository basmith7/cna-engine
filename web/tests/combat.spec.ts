import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import type { Condition, Gun, View } from "../src/types.ts";
import { clickHex, expectDomMatchesView, here, openFixture, shoot as shootInto, shotsDir, throughCurtain } from "./helpers.ts";

// Slice 5's proving screenshots.
const shots = shotsDir("slice-5");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

/** `cna-play replay` of golden orders 0–76, as the SHA-256 of the state JSON (pinned by `cna-web`'s `the_e2e_fixtures_are_current`). */
const combatDone = readFileSync(here("fixtures/combat-done.sha256"), "utf8").trim();
const sha256 = (s: string) => createHash("sha256").update(s).digest("hex");

const log = (page: Page) => page.getByTestId("log-entries").locator("li.event");
const pending = (page: Page) => page.getByTestId("pending");
const sealed = (page: Page) => page.getByTestId("sealed-panel");

const AXIS_GUN = "unit:it:1st-libyan-artillery-regt";

/** Nothing the Axis sealed is on the page: no typed row, no ticked gun, no revealed line in the log, nothing of it in the view. */
async function expectNoAxisAnswer(page: Page, step: "gun-positions" | "barrage" | "assign-forces") {
  await expect(sealed(page).locator("[data-fire], [data-role]")).toHaveCount(0);
  await expect(sealed(page).locator("[data-gun]:checked")).toHaveCount(0);
  await expect(log(page).filter({ hasText: /guns forward|barrage plotted|forces assigned/ })).toHaveCount(step === "gun-positions" ? 0 : step === "barrage" ? 2 : 4);
  const combat = (await page.evaluate(() => window.__cna.view())).combat;
  if (step === "gun-positions") expect(combat.forward ?? []).toEqual([]);
  if (step === "barrage") expect(combat.plots?.axis).toBeUndefined();
  if (step === "assign-forces") expect(combat.assignments?.axis).toBeUndefined();
  if (step === "gun-positions") await expect(page.locator(".marker.forward")).toHaveCount(0);
}

test("golden orders 66–76 by clicks: gun positions, the barrage on 1 KRRC, assignments, the assault on Halfaya, the passes", async ({ page }) => {
  test.setTimeout(180_000);
  await openFixture(page, "combat-start");
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · OpStage 1 · G · movement and combat · Player A · gun positions");
  await expect(page.getByTestId("to-act")).toHaveText("Axis to act");
  await expect(page.getByTestId("phase-controls")).toHaveCount(0);

  // 66: the Axis stands its Libyan artillery forward; the panel offers
  // only gun-class units, the ones next to the enemy first.
  await expect(pending(page)).toHaveAttribute("data-kind", "sealed");
  await expect(pending(page)).toContainText("Sealed gun positions · Axis");
  const options = await page.evaluate(() => window.__cna.sealedOptions());
  const guns: Gun[] = "ok" in options && "gun-positions" in options.ok ? options.ok["gun-positions"].guns : [];
  expect(guns.length).toBeGreaterThan(2);
  await expect(sealed(page).locator("[data-gun]")).toHaveCount(guns.length);
  await expect(sealed(page).locator(".near [data-gun]")).toHaveCount(guns.filter((g) => g.next_to_enemy).length);
  await expect(sealed(page).locator(`.near [data-gun="${AXIS_GUN}"]`)).toBeAttached();
  await sealed(page).locator(`[data-gun="${AXIS_GUN}"]`).check();
  await sealed(page).locator('[data-order="gun-positions"]').click();
  await expect(log(page).filter({ hasText: "Axis has sealed its gun positions." })).toHaveCount(1);
  await throughCurtain(page, "cw");
  await expectNoAxisAnswer(page, "gun-positions");

  // 67: the Commonwealth stands nobody forward; both positions are revealed.
  await expect(pending(page)).toContainText("Sealed gun positions · Commonwealth");
  await sealed(page).locator('[data-order="gun-positions"]').click();
  await expect(log(page).filter({ hasText: "Axis guns forward: 1st Libyan Artillery Regt." })).toHaveCount(1);
  await expect(log(page).filter({ hasText: "Commonwealth guns forward: nothing." })).toHaveCount(1);
  await throughCurtain(page, "axis");
  await clickHex(page, "C4021");
  await expect(page.getByTestId("stack-panel").locator(`[data-unit="${AXIS_GUN}"] .marker.forward`)).toHaveCount(1);

  // 68: the Axis barrage: 6 points from the artillery at 1 KRRC. The
  // proving screenshot, mid-entry.
  await expect(pending(page)).toContainText("Sealed barrage · Axis");
  await sealed(page).locator(`[data-add-fire="${AXIS_GUN}"]`).click();
  const fire = sealed(page).locator('[data-fire="0"]');
  await fire.locator("[data-points]").fill("6");
  await fire.locator("[data-target]").selectOption("unit:cw:1-krrc");
  await expect(sealed(page).locator('[data-order="barrage"]')).toHaveText("Seal barrage (1 fire)");
  await clickHex(page, "C3922");
  await shoot(page, "barrage-entry.png");
  // An over-strength fire is refused by the engine and the row stays.
  await fire.locator("[data-points]").fill("9");
  await sealed(page).locator('[data-order="barrage"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (12.2x)");
  await page.getByTestId("toast").getByLabel("Dismiss").click();
  await expect(fire).toBeAttached();
  // The refusal names the answer, so it is not in the log the other side reads next.
  await expect(page.getByTestId("log-entries").locator("li.illegal")).toHaveCount(0);
  await fire.locator("[data-points]").fill("6");
  await sealed(page).locator('[data-order="barrage"]').click();
  await expect(log(page).filter({ hasText: "Axis has sealed its barrage." })).toHaveCount(1);
  await throughCurtain(page, "cw");
  await expectNoAxisAnswer(page, "barrage");
  await expect(pending(page)).toContainText("Sealed barrage · Commonwealth");
  await shoot(page, "sealed-other-side.png");

  // 69: the Commonwealth fires nothing; the barrage is revealed and rolled.
  await sealed(page).locator('[data-order="barrage"]').click();
  await expect(log(page).filter({ hasText: "Axis barrage plotted: 1st Libyan Artillery Regt 6 at 1st KRRC." })).toHaveCount(1);
  await expect(log(page).filter({ hasText: /^Axis barrage on 1st KRRC by 1st Libyan Artillery Regt: \d+ points/ })).toHaveCount(1);
  // 70: retreat before assault is the Commonwealth's; it passes.
  await expect(page.getByTestId("curtain")).toHaveCount(0);
  await expect(pending(page)).toHaveAttribute("data-kind", "retreat-before-assault");
  await pending(page).locator('[data-order="pass"]').click();
  await throughCurtain(page, "axis");

  // 71: the Axis assigns four Libyan battalions, 5 points each, to
  // assault Halfaya (C3922); the phasing side names the hex.
  await expect(pending(page)).toContainText("Sealed force assignment · Axis");
  for (const bn of ["viii", "ix", "x", "xi"]) {
    const id = `unit:it:${bn}-libyan-bn`;
    await sealed(page).locator(`[data-add-role="${id}:close-assault"]`).click();
    const row = sealed(page).locator(`[data-unit="${id}"] [data-role]`);
    await row.locator("[data-points]").fill("5");
    await expect(row.locator("[data-target]")).toHaveValue("C3922");
  }
  await expect(sealed(page).locator('[data-order="assign-forces"]')).toHaveText("Seal assignment (4 units)");
  await sealed(page).locator('[data-order="assign-forces"]').click();
  await expect(log(page).filter({ hasText: "Axis has sealed its force assignment." })).toHaveCount(1);
  await throughCurtain(page, "cw");
  await expectNoAxisAnswer(page, "assign-forces");

  // 72: the Commonwealth commits 1 KRRC and 3 Coldstream Guards in full
  // to the defence; no hex is named. Revealed, anti-armour resolves,
  // and the close assault step waits on the Axis.
  await expect(pending(page)).toContainText("Sealed force assignment · Commonwealth");
  for (const id of ["unit:cw:1-krrc", "unit:cw:3-cldgds"]) {
    await sealed(page).locator(`[data-add-role="${id}:close-assault"]`).click();
    const row = sealed(page).locator(`[data-unit="${id}"] [data-role]`);
    await row.locator("[data-points]").fill("6");
    await expect(row.locator("[data-target]")).toHaveCount(0);
  }
  await sealed(page).locator('[data-order="assign-forces"]').click();
  await expect(log(page).filter({ hasText: /^Axis forces assigned: .*5 assault at C3922/ })).toHaveCount(1);
  await expect(log(page).filter({ hasText: "Commonwealth forces assigned: 1st KRRC: 6 assault; 3rd Coldstream Guards: 6 assault." })).toHaveCount(1);
  await throughCurtain(page, "axis");

  // 73: the assault itself: Halfaya is outlined, a click resolves it, the
  // result and its losses are in the log. Pass is refused while it waits.
  await expect(page.getByTestId("assault-panel")).toContainText("Close assault · 1 to resolve");
  await expect(page.locator('polygon.mark[data-hex="C3922"]')).toBeAttached();
  await page.getByTestId("phase-controls").locator('[data-order="pass"]').click();
  await expect(page.getByTestId("toast")).toContainText("Illegal (15.2x)");
  await page.getByTestId("toast").getByLabel("Dismiss").click();
  await clickHex(page, "C3922");
  await expect(log(page).filter({ hasText: /^Assault on C3922 by .*: \d+ v \d+ raw/ })).toHaveCount(1);
  await expect(log(page).filter({ hasText: /loses \d+ points, TOE now \d+\./ }).first()).toBeAttached();
  await expect(page.getByTestId("assault-panel")).toHaveCount(0);
  await expect(page.locator("polygon.mark")).toHaveCount(0);
  await expectDomMatchesView(page);

  // 74–76: the step and the Axis half end; the Commonwealth passes its
  // reserve designation. The state is `replay`'s.
  const controls = page.getByTestId("phase-controls");
  await controls.locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).toContainText("Player A · reserve release");
  await controls.locator('[data-order="pass"]').click();
  await throughCurtain(page, "cw");
  await expect(page.getByTestId("clock")).toContainText("F · reserve designation · Player B");
  await controls.locator('[data-order="pass"]').click();
  await expect(page.getByTestId("clock")).toContainText("Player B · movement");
  expect(sha256(await page.evaluate(() => window.__cna.save()))).toBe(combatDone);
});

test("a retreat before assault goes along a reachable path from the panel", async ({ page }) => {
  await openFixture(page, "retreat-before-assault");
  await expect(pending(page)).toHaveAttribute("data-kind", "retreat-before-assault");
  await expect(pending(page)).toContainText("Retreat before assault · Commonwealth");
  await expect(page.getByTestId("phase-controls")).toHaveCount(0);
  // 4 RHA at C3923, alone: the overlay shows where it may pull back to.
  await clickHex(page, "C3923");
  await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "retreat-before-assault");
  await expect(page.getByTestId("move-panel")).toContainText("Retreat before assault");
  await expect(page.getByTestId("assault")).toHaveCount(0);
  const reaches = page.locator("polygon.reach");
  expect(await reaches.count()).toBeGreaterThan(0);
  await expect(page.getByTestId("retreat-count")).toContainText(/^\d+ hexes to retreat to for the 1 ticked\./);
  const to = (await reaches.first().getAttribute("data-hex"))!;
  await page.locator(`polygon.hex[data-hex="${to}"]`).click();
  await expect(log(page).filter({ hasText: `4th Ryl Horse Arty retreat before assault to ${to}.` })).toHaveCount(1);
  await expect(page.getByTestId("curtain")).toHaveCount(0);
  await expect(pending(page)).toHaveAttribute("data-kind", "retreat-before-assault");
  await expectDomMatchesView(page);
  await pending(page).locator('[data-order="pass"]').click();
  await expect(pending(page)).toContainText("Sealed force assignment · Axis");
});

test("a forced retreat shows the order, says what standing costs as the engine then charges it, and takes a reachable hex", async ({ page }) => {
  await openFixture(page, "forced-retreat");
  await expect(pending(page)).toHaveAttribute("data-kind", "forced-retreat");
  await expect(page.getByTestId("retreat-order")).toHaveText("Commonwealth must retreat 3 hexes from C4021 for committing nothing.");
  await expect(pending(page).locator("li[data-unit]")).toHaveCount(3);
  // The ordered stack is ticked by itself, the map has come to it and the overlay is up.
  await expect(page.getByTestId("stack-panel").locator("input.tick:checked")).toHaveCount(3);
  await expect(page.getByTestId("hex")).toHaveText("hex C3922");
  await expect(page.locator('[data-stack="C3922"]')).toBeInViewport();
  await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "forced-retreat");
  // The cost of standing is the engine's preview of Pass, not a formula:
  // the stack is one hex from its attackers, so two short.
  const consequence = page.getByTestId("pass-consequence");
  const text = (await consequence.textContent())!;
  const said = text.match(/^Pass stands where it is, 2 hexes short of the order, and (?:(.+) loses (\d+) points? for it|loses (\d+) points for it: (.+))\.$/);
  expect(said, text).toBeTruthy();
  const [, one, oneN, manyN, many] = said!;
  const total = Number(one ? oneN : manyN);
  expect(total).toBeGreaterThan(0);
  const each: [string, number][] = one ? [[one, total]] : many.split(", ").map((part) => [part.replace(/ \d+$/, ""), Number(part.match(/(\d+)$/)![1])]);
  expect(each.reduce((a, [, n]) => a + n, 0)).toBe(total);
  await expect(pending(page).locator('[data-order="pass"]')).toHaveText("Pass: stand and pay");
  await shoot(page, "forced-retreat.png");
  // Pass does exactly what the line said: the log carries the shortfall
  // and each named loss, and the TOE went down by the total.
  const toe = (v: View) => Object.values(v.pieces).reduce((n, p) => n + p.toe, 0);
  const before = toe(await page.evaluate(() => window.__cna.view()));
  await pending(page).locator('[data-order="pass"]').click();
  await expect(log(page).filter({ hasText: "stand 2 hexes short of the ordered retreat and pay for it." })).toHaveCount(1);
  for (const [who, n] of each) {
    await expect(log(page).filter({ hasText: `${who} loses ${n} point` })).toHaveCount(1);
  }
  expect(before - toe(await page.evaluate(() => window.__cna.view()))).toBe(total);
  await expect(page.locator('[data-testid="pending"][data-kind="forced-retreat"]')).toHaveCount(0);
  await expectDomMatchesView(page);

  // Or the retreat is made: an empty hex the full distance away (a hex
  // of one's own would be selected, not retreated to).
  await openFixture(page, "forced-retreat");
  await expect(page.getByTestId("move-panel")).toHaveAttribute("data-purpose", "forced-retreat");
  const to = await page.evaluate(() =>
    [...document.querySelectorAll('polygon.reach[data-short="0"]')].map((p) => p.getAttribute("data-hex")!).find((h) => !document.querySelector(`[data-stack="${h}"]`)),
  );
  expect(to).toBeTruthy();
  await clickHex(page, to!);
  await expect(log(page).filter({ hasText: `→ ${to} for` })).toHaveCount(1);
  expect(await page.evaluate(() => window.__cna.view().pieces["unit:cw:1-krrc"].location)).toEqual({ hex: to });
  await expect(page.locator('[data-testid="pending"][data-kind="forced-retreat"]')).toHaveCount(0);
  await expectDomMatchesView(page);
});

test("the forced retreat panel reads whole at a phone width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page, "forced-retreat");
  await expect(pending(page)).toHaveAttribute("data-kind", "forced-retreat");
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await pending(page).scrollIntoViewIfNeeded();
  await expect(pending(page).locator('[data-order="pass"]')).toBeInViewport();
  await shoot(page, "forced-retreat-390.png");
});

test("the end of the game is the engine's report, with every order control gone", async ({ page }) => {
  await openFixture(page, "game-over");
  await expect(page.getByTestId("to-act")).toHaveText("Game over");
  const dialog = page.getByTestId("end-dialog");
  await expect(dialog).toBeVisible();
  const report = (await page.evaluate(() => window.__cna.view())).over!;
  await expect(dialog.locator('[data-level="axis"]')).toContainText(report.axis ?? "No level");
  await expect(dialog.locator('[data-level="cw"]')).toContainText(report.cw ?? "No level");
  await expect(dialog.getByTestId("conditions").locator("tbody tr")).toHaveCount(report.checked.length);
  await expect(dialog.getByTestId("conditions").locator('tr[data-met="true"]')).toHaveCount(report.checked.filter((c: Condition) => c.met).length);
  await expect(dialog).toContainText("The clock stopped at Game-Turn 7");
  await shoot(page, "end-dialog.png");
  await dialog.getByTestId("end-close").click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByTestId("phase-controls")).toHaveCount(0);
  await expect(page.getByTestId("pending")).toHaveCount(0);
  await expect(page.getByTestId("move-panel")).toHaveCount(0);
  await clickHex(page, "C4021");
  await expect(page.getByTestId("stack-panel").locator("input.tick")).toHaveCount(0);
  await page.getByTestId("show-report").click();
  await expect(page.getByTestId("end-dialog")).toBeVisible();
});

test("stays usable at a phone width with the barrage panel open", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openFixture(page, "combat-start");
  await sealed(page).locator(`[data-gun="${AXIS_GUN}"]`).check();
  await sealed(page).locator('[data-order="gun-positions"]').click();
  await throughCurtain(page, "cw");
  await sealed(page).locator('[data-order="gun-positions"]').click();
  await throughCurtain(page, "axis");
  await sealed(page).locator(`[data-add-fire="${AXIS_GUN}"]`).click();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await sealed(page).scrollIntoViewIfNeeded();
  await expect(sealed(page).locator('[data-order="barrage"]')).toBeInViewport();
  await shoot(page, "barrage-390.png");
});
