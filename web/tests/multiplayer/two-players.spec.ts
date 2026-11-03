import { type BrowserContext, expect, type Page, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import type { HexId, PieceId, Side } from "../../src/types.ts";
import { type Logged, perform } from "../drive.ts";
import { here } from "../helpers.ts";

// Two players over cna-server (plan, slice 4, Task 5): alice (Axis) and bob
// (Commonwealth) sign in through the fake Discord, alice makes a private
// game on the golden seed, bob joins by its invite, and they play the
// golden game's first 77 orders, each from the seat its side holds. Every
// response and stream message bob's browser receives is put to the leak
// predicate; the server's state at the end is the one `replay` gives.

type GoldenLog = { seed: number; orders: Logged[] };
const golden = JSON.parse(readFileSync(here("../../tests/games/graziani-m5.json"), "utf8")) as GoldenLog;
/** Pinned by `cna-server`'s `two_players` test: `replay` of these orders. */
const pinned = readFileSync(here("fixtures/two-players-done.sha256"), "utf8").trim();
const ORDERS = 77;

const sideName = (s: Side) => (s === "axis" ? "Axis" : "Commonwealth");

/** The hex a piece's counter is drawn on: the network game has no `window.__cna`. */
async function hexOf(page: Page, id: PieceId): Promise<HexId> {
  const h = await page.locator(`svg.map g[data-unit="${id}"]`).getAttribute("data-hex");
  expect(h, `${id} is on a hex`).not.toBeNull();
  return h!;
}

/** Signs `who` in through the fake Discord's consent page. */
async function signIn(context: BrowserContext, who: "alice" | "bob"): Promise<Page> {
  const page = await context.newPage();
  await page.goto("/auth/discord");
  await page.getByRole("link", { name: who, exact: true }).click();
  await expect(page.getByTestId("play-online")).toBeVisible();
  const me = await page.request.get("/api/me");
  expect(me.ok()).toBe(true);
  expect(((await me.json()) as { name: string }).name).toBe(who);
  return page;
}

/** A state change from `page`'s session, with the Origin the CSRF check wants. */
function post(page: Page, url: string, data: unknown) {
  return page.request.post(url, { data, headers: { Origin: new URL(page.url()).origin } });
}

test("alice and bob play the golden game's first 77 orders over the server, bob sees no leak, and the state is replay's", async ({ browser, baseURL }) => {
  test.setTimeout(600_000);
  const origin = new URL(baseURL!).origin;
  const aliceCtx = await browser.newContext();
  const bobCtx = await browser.newContext();
  // Bob's stream messages, as his page receives them.
  await bobCtx.addInitScript(() => {
    const w = window as unknown as { __sse: string[]; EventSource: typeof EventSource };
    w.__sse = [];
    const Base = w.EventSource;
    w.EventSource = class extends Base {
      constructor(url: string | URL, init?: EventSourceInit) {
        super(url, init);
        this.addEventListener("message", (m) => w.__sse.push((m as MessageEvent<string>).data));
      }
    };
  });
  const alice = await signIn(aliceCtx, "alice");
  const bob = await signIn(bobCtx, "bob");
  // A page that throws can keep drawing an old view: none may.
  const errors: string[] = [];
  for (const p of [alice, bob]) p.on("pageerror", (e) => errors.push(e.stack ?? e.message));
  alice.setDefaultTimeout(15_000);
  bob.setDefaultTimeout(15_000);

  // Alice makes a private game as Axis on the golden seed.
  expect((await post(alice, "/test/next-seed", { seed: golden.seed })).ok()).toBe(true);
  await alice.getByTestId("play-online").click();
  const lobby = alice.getByTestId("lobby");
  await lobby.getByLabel("Side").selectOption("axis");
  await lobby.getByLabel("Listed in the open games").uncheck();
  await lobby.getByRole("button", { name: "Create" }).click();
  const link = await alice.getByTestId("invite").getByLabel("Invite link").inputValue();
  const id = /\/join\/(\d+)\?invite=/.exec(link)?.[1];
  expect(id, link).toBeDefined();

  // Every JSON body bob's browser gets from the server, put to the
  // predicate about Axis once the game exists; each check is awaited
  // before the test ends.
  const leaks: string[] = [];
  const checks: Promise<void>[] = [];
  const check = (what: string, json: unknown) => {
    checks.push(
      (async () => {
        const r = await post(alice, `/test/games/${id}/leaks?about=axis`, json);
        expect(r.ok(), `leak check of ${what}: ${r.status()} ${await r.text()}`).toBe(true);
        for (const l of (await r.json()) as string[]) leaks.push(`${what}: ${l}`);
      })(),
    );
  };
  // Each response is read here before bob's page gets it (a body read
  // from a `response` event is gone once the page navigates); the
  // stream, which never ends, goes through and is read in the page.
  let responses = 0;
  await bobCtx.route(`${origin}/api/**`, async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname.endsWith("/stream")) return route.continue();
    const r = await route.fetch();
    if ((r.headers()["content-type"] ?? "").startsWith("application/json")) {
      responses++;
      check(`${route.request().method()} ${url.pathname}${url.search}`, await r.json());
    }
    await route.fulfill({ response: r });
  });
  let messages = 0;
  let streamed = 0;
  const drainStream = async () => {
    for (const data of await bob.evaluate(() => (window as unknown as { __sse: string[] }).__sse.splice(0))) {
      messages++;
      const json = JSON.parse(data) as { seq?: number };
      streamed = Math.max(streamed, json.seq ?? 0);
      check(`stream message ${messages}`, json);
    }
  };

  // Bob follows the invite and takes the Commonwealth seat.
  await bob.goto(link);
  await bob.getByTestId("join").getByRole("button", { name: "Join as Commonwealth" }).click();
  await bob.waitForURL(`**/game/${id}`);
  await bob.locator("[data-stack]").first().waitFor();
  await alice.goto(`/game/${id}`);
  await alice.locator("[data-stack]").first().waitFor();

  const seq = async () => ((await (await alice.request.get(`/api/games/${id}`)).json()) as { seq: number }).seq;
  for (const [i, logged] of golden.orders.slice(0, ORDERS).entries()) {
    const page = logged.player === "axis" ? alice : bob;
    await expect(page.getByTestId("to-act"), `order ${i}`).toHaveText(`${sideName(logged.player)} to act`);
    await perform(page, i, logged, hexOf);
    await expect.poll(seq, { message: `order ${i} accepted` }).toBe(i + 1);
    await expect(page.getByTestId("log-entries").locator("li.illegal")).toHaveCount(0);
    await drainStream();
  }

  // Bob's stream has the last order (seq counts from 0), and every check is in.
  await expect.poll(async () => (await drainStream(), streamed), { message: "bob's stream reached the last order" }).toBe(ORDERS - 1);
  await Promise.all(checks);
  expect(responses, "bob's browser fetched from the server").toBeGreaterThan(0);
  expect(messages, "bob's stream carried the orders").toBeGreaterThan(0);
  expect(leaks).toEqual([]);
  expect(errors).toEqual([]);

  const hash = await alice.request.get(`/test/games/${id}/hash`);
  expect(hash.ok()).toBe(true);
  expect(await hash.text()).toBe(pinned);
  await aliceCtx.close();
  await bobCtx.close();
});
