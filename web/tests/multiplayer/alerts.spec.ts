import { type BrowserContext, expect, type Page, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import type { Logged } from "../drive.ts";
import { here } from "../helpers.ts";

// Turn alerts (plan, slice 5, Task 4): when the move passes to bob while
// his game page is closed, the bot DMs him once with the game's link;
// with his alerts off, it does not. Orders go by the API here: playing
// them by clicks is `two-players.spec.ts`'s job.

const golden = JSON.parse(readFileSync(here("../../tests/games/graziani-m5.json"), "utf8")) as { seed: number; orders: Logged[] };
const FAKE = `http://localhost:${process.env.CNA_PW_FAKE_PORT ?? 3261}`;
/** Bob's Discord id in the fake, and so his DM channel. */
const BOB_DM = "dm-1002";

async function signIn(context: BrowserContext, who: "alice" | "bob"): Promise<Page> {
  const page = await context.newPage();
  await page.goto("/auth/discord");
  await page.getByRole("link", { name: who, exact: true }).click();
  await expect(page.getByTestId("play-online")).toBeVisible();
  return page;
}

function send(page: Page, method: "POST" | "PATCH", url: string, data: unknown) {
  return page.request.fetch(url, { method, data, headers: { Origin: new URL(page.url()).origin } });
}

/** Alice makes a private Axis game, bob joins it by the API, and alice plays until the move is bob's. */
async function toBobsMove(alice: Page, bob: Page): Promise<number> {
  const made = await send(alice, "POST", "/api/games", { side: "axis", open: false });
  expect(made.ok()).toBe(true);
  const { id, invite } = (await made.json()) as { id: number; invite: string };
  expect((await send(bob, "POST", `/api/games/${id}/join`, { side: "cw", invite })).ok()).toBe(true);
  for (let seq = 0; ; seq++) {
    const o = golden.orders[seq];
    expect(o.player, `the move passes to the Commonwealth before order ${seq}`).toBe("axis");
    const { version } = (await (await alice.request.get(`/api/games/${id}`)).json()) as { version: string };
    let r = await send(alice, "POST", `/api/games/${id}/orders`, { side: "axis", order: o.order, version });
    // The server limits orders per account; a second game in a row meets it.
    while (r.status() === 429) {
      await alice.waitForTimeout(1_000);
      r = await send(alice, "POST", `/api/games/${id}/orders`, { side: "axis", order: o.order, version });
    }
    expect(r.ok(), await r.text()).toBe(true);
    const g = (await (await alice.request.get(`/api/games/${id}`)).json()) as { view: { to_act: string | null } };
    if (g.view.to_act === "cw") return id;
  }
}

async function dmsToBob(page: Page): Promise<string[]> {
  const sent = (await (await page.request.get(`${FAKE}/_sent`)).json()) as { channel: string; content: string }[];
  return sent.filter((m) => m.channel === BOB_DM).map((m) => m.content);
}

test("bob is told once by DM that it is his move, and not at all with his alerts off", async ({ browser }) => {
  test.setTimeout(300_000);
  const alice = await signIn(await browser.newContext(), "alice");
  const bob = await signIn(await browser.newContext(), "bob");
  // Bob is signed in but not on the game's page: no stream is open.
  await bob.goto("about:blank");
  await bob.goto(new URL("/lobby", alice.url()).href);
  const before = (await dmsToBob(bob)).length;

  const id = await toBobsMove(alice, bob);
  await expect.poll(async () => (await dmsToBob(bob)).length, { timeout: 20_000 }).toBe(before + 1);
  const [dm] = (await dmsToBob(bob)).slice(before);
  expect(dm).toContain(`/game/${id}`);
  expect(dm).toContain("Your move");

  // Alerts off: a second game reaching bob's move sends nothing.
  const off = await send(bob, "PATCH", "/api/me", { alerts: false });
  expect(((await off.json()) as { alerts: boolean }).alerts).toBe(false);
  await toBobsMove(alice, bob);
  // The sender runs every 5 s: two rounds pass with nothing for bob.
  await bob.waitForTimeout(11_000);
  expect(await dmsToBob(bob)).toHaveLength(before + 1);
});
