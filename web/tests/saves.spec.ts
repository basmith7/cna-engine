import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { clickHex, expectDomMatchesView, here, newGame, openStart, shoot as shootInto, shotsDir } from "./helpers.ts";

// Slice 6's proving screenshots.
const shots = shotsDir("slice-6");
const shoot = (page: Page, name: string) => shootInto(page, shots, name);

const save = (page: Page) => page.evaluate(() => window.__cna.save());

/** Hands `text` to the Load input as a file named `name`. */
async function loadText(page: Page, where: "start" | "game", name: string, text: string) {
  const input = page.getByTestId(where === "start" ? "start" : "game-panel").getByLabel("Load a save");
  await input.setInputFiles({ name, mimeType: "application/json", buffer: Buffer.from(text) });
}

test("a new game shows its seed; Save downloads the state as graziani-gt<N>-os<M>.json; the autosave is offered as Continue on the next visit", async ({ page }) => {
  await openStart(page);
  // A fresh browser context has no autosave to continue.
  await expect(page.getByTestId("continue")).toHaveCount(0);
  // The seed is random by default and shown, so a game can be repeated.
  await expect(page.getByTestId("seed")).toHaveValue(/^\d+$/);
  await shoot(page, "start.png");
  await page.getByTestId("seed").fill("seven");
  await expect(page.getByTestId("new-game")).toBeDisabled();
  await expect(page.getByTestId("start")).toContainText("a whole number, please");
  await page.getByTestId("seed").fill("7");
  await page.getByTestId("new-game").click();
  await page.locator("[data-stack]").first().waitFor();
  await expect(page.getByTestId("start")).toHaveCount(0);
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · set-up");

  // One placement, then Save: the file is `save()` byte for byte.
  await page.locator('[data-where="tray-axis"] [data-unit="supply:axis:16"]').click();
  await clickHex(page, "C0101");
  await expect(page.getByTestId("log-entries").locator("li.event")).toHaveCount(1);
  const download = page.waitForEvent("download");
  await page.getByTestId("save").click();
  const file = await download;
  expect(file.suggestedFilename()).toBe("graziani-gt1-os1.json");
  const path = join(test.info().outputPath(), file.suggestedFilename());
  await file.saveAs(path);
  const saved = readFileSync(path, "utf8");
  expect(saved).toBe(await save(page));
  expect(JSON.parse(saved).pieces["supply:axis:16"].location).toEqual({ hex: "C0101" });

  // The autosave: a reload offers Continue at the save's clock, and
  // Continue restores exactly that state.
  await page.reload();
  await expect(page.getByTestId("start")).toBeVisible();
  await expect(page.getByTestId("start")).toContainText("Autosave at Game-Turn 1 · set-up");
  await shoot(page, "start-continue.png");
  await page.getByTestId("continue").click();
  await page.locator("[data-stack]").first().waitFor();
  expect(await save(page)).toBe(saved);
  await expect(page.locator('[data-where="tray-axis"] [data-unit]')).toHaveCount(35);
  await expectDomMatchesView(page);
  // Play goes on from it, and the autosave follows.
  await page.getByTestId("phase-controls").waitFor({ state: "detached" });
  await page.locator('[data-where="tray-axis"] [data-unit="supply:axis:17"]').click();
  await clickHex(page, "C0101");
  await expect(page.getByTestId("log-entries").locator("li.event")).toHaveCount(1);
  const later = await save(page);
  expect(later).not.toBe(saved);
  expect(await page.evaluate(() => localStorage.getItem("cna:autosave"))).toBe(later);
});

test("a refused file is shown and the current game kept; a good one loads", async ({ page }) => {
  await newGame(page);
  await page.locator('[data-where="tray-axis"] [data-unit="supply:axis:16"]').click();
  await clickHex(page, "C0101");
  await expect(page.getByTestId("log-entries").locator("li.event")).toHaveCount(1);
  const before = await save(page);

  // A save from another vendor/cna commit. The proving screenshot.
  const commit = (JSON.parse(before) as { rules_commit: string }).rules_commit;
  const other = before.replace(commit, "0".repeat(40));
  expect(other).not.toBe(before);
  await loadText(page, "game", "graziani-elsewhere.json", other);
  const error = page.getByTestId("game-panel").getByTestId("load-error");
  await expect(error).toContainText("This file was not loaded:");
  await expect(error).toContainText("vendor/cna");
  expect(await save(page)).toBe(before);
  await expect(page.getByTestId("log-entries").locator("li.event")).toHaveCount(1);
  await page.getByRole("button", { name: "fit" }).click();
  await shoot(page, "load-error.png");

  // A malformed save: a sealed step with nobody waiting, which used to
  // kill the page; and plain garbage.
  const broken = JSON.parse(before) as { pending: unknown };
  broken.pending = { sealed: { step: "barrage", waiting: [] } };
  await loadText(page, "game", "broken.json", JSON.stringify(broken));
  await expect(error).toContainText("nobody waiting");
  expect(await save(page)).toBe(before);
  await loadText(page, "game", "garbage.json", "not json at all");
  await expect(error).toContainText("parsing the state");
  expect(await save(page)).toBe(before);

  // The game-over fixture loads: the error goes, the clock is the file's.
  await loadText(page, "game", "game-over.json", readFileSync(here("fixtures/game-over.json"), "utf8"));
  await expect(page.getByTestId("to-act")).toHaveText("Game over");
  await expect(page.getByTestId("end-dialog")).toBeVisible();
  await expect(error).toHaveCount(0);
  await expect(page.getByTestId("log-entries").locator("li.event")).toHaveCount(0);
  await expectDomMatchesView(page);
});

test("a save loads from the start screen, and a refused one is shown there", async ({ page }) => {
  await openStart(page);
  await loadText(page, "start", "garbage.json", "{}");
  await expect(page.getByTestId("start").getByTestId("load-error")).toContainText("This file was not loaded:");
  await expect(page.getByTestId("start")).toBeVisible();
  await loadText(page, "start", "combat-start.json", readFileSync(here("fixtures/combat-start.json"), "utf8"));
  await page.locator("[data-stack]").first().waitFor();
  await expect(page.getByTestId("clock")).toHaveText("Game-Turn 1 · OpStage 1 · G · movement and combat · Player A · gun positions");
  await expect(page.getByTestId("pending")).toContainText("Sealed gun positions · Axis");
  await expectDomMatchesView(page);
});

test("the start screen and the game panel read whole at a phone width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openStart(page);
  let overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await expect(page.getByTestId("new-game")).toBeInViewport();
  await shoot(page, "start-390.png");
  await page.getByTestId("seed").fill("7");
  await page.getByTestId("new-game").click();
  await page.locator("[data-stack]").first().waitFor();
  overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.getByTestId("game-panel").scrollIntoViewIfNeeded();
  await expect(page.getByTestId("save")).toBeInViewport();
});
