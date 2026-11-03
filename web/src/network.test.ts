import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MOVED_ON, NetworkEngine, STALE_CASE } from "./network";
import type { Order, View } from "./types";

// A view as the tests need it: only the fields `NetworkEngine` reads.
const view = (to_act: string | null, turn = 1) => ({ to_act, turn }) as unknown as View;

type Call = { url: string; init: RequestInit };
type Reply = { status: number; json: unknown };

let calls: Call[];
let replies: Map<string, Reply[]>;

/** Answers `METHOD path` (query included) with the replies queued for it, in order; the last one repeats. */
function reply(method: string, path: string, ...rs: Reply[]) {
  replies.set(`${method} ${path}`, rs);
}

class FakeEventSource {
  static last: FakeEventSource | undefined;
  onmessage: ((m: MessageEvent<string>) => void) | null = null;
  private named = new Map<string, (() => void)[]>();
  closed = false;
  constructor(
    readonly url: string,
    readonly init?: EventSourceInit,
  ) {
    FakeEventSource.last = this;
  }
  close() {
    this.closed = true;
  }
  addEventListener(name: string, f: () => void) {
    this.named.set(name, [...(this.named.get(name) ?? []), f]);
  }
  /** A named event, as the server sends `event: <name>`. */
  emitNamed(name: string) {
    for (const f of this.named.get(name) ?? []) f();
  }
  emit(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) } as MessageEvent<string>);
  }
}

const order = { "end-phase": null } as unknown as Order;

beforeEach(() => {
  calls = [];
  replies = new Map();
  FakeEventSource.last = undefined;
  vi.stubGlobal("EventSource", FakeEventSource);
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init: RequestInit = {}) => {
      calls.push({ url, init });
      const key = `${init.method ?? "GET"} ${url}`;
      const q = replies.get(key);
      if (!q) throw new Error(`unexpected ${key}`);
      const r = q.length > 1 ? q.shift()! : q[0];
      return new Response(JSON.stringify(r.json), { status: r.status, headers: { "Content-Type": "application/json" } });
    }),
  );
  reply("GET", "/api/games/7", { status: 200, json: { side: "axis", seq: 3, open: false, status: "active", view: view("axis") } });
  reply("GET", "/api/games?mine", { status: 200, json: [{ id: 7, opponent: "bob" }, { id: 8, opponent: "carol" }] });
});

afterEach(() => vi.unstubAllGlobals());

describe("NetworkEngine", () => {
  it("loads the seat, seq and view, and opens the stream", async () => {
    const e = await NetworkEngine.open("7");
    expect(e.side).toBe("axis");
    expect(e.seq).toBe(3);
    expect(await e.view()).toEqual(view("axis"));
    expect(e.turnText).toBe("Your move");
    expect(e.myMove).toBe(true);
    expect(FakeEventSource.last?.url).toBe("/api/games/7/stream");
    expect(calls.every((c) => c.init.credentials === "same-origin")).toBe(true);
  });

  it("names the opponent it waits for, and falls back when the lobby list fails", async () => {
    reply("GET", "/api/games/7", { status: 200, json: { side: "cw", seq: 0, open: false, view: view("axis") } });
    expect((await NetworkEngine.open("7")).turnText).toBe("Waiting for bob");
    reply("GET", "/api/games?mine", { status: 500, json: { error: "internal error" } });
    expect((await NetworkEngine.open("7")).turnText).toBe("Waiting for your opponent");
  });

  it("says the game is not in play, whoever to_act names, unless it is active", async () => {
    for (const [status, text] of [
      ["open", "Waiting for an opponent to join"],
      ["abandoned", "Abandoned: a player left"],
      ["stale", "Stopped: the rules changed under this game"],
    ]) {
      reply("GET", "/api/games/7", { status: 200, json: { side: "axis", seq: 0, open: false, status, view: view("axis") } });
      expect((await NetworkEngine.open("7")).turnText).toBe(text);
    }
  });

  it("sends the order with its seq as JSON and reloads the view after it lands", async () => {
    const e = await NetworkEngine.open("7");
    reply("POST", "/api/games/7/orders", { status: 200, json: { ok: [{ "phase-skipped": {} }] } });
    reply("GET", "/api/games/7", { status: 200, json: { side: "axis", seq: 4, open: false, view: view("cw", 2) } });
    const changes = vi.fn();
    e.onChange(changes);
    expect(await e.apply("axis", order)).toEqual({ ok: [{ "phase-skipped": {} }] });
    const post = calls.find((c) => c.init.method === "POST")!;
    expect(JSON.parse(post.init.body as string)).toEqual({ side: "axis", order, seq: 3 });
    expect((post.init.headers as Record<string, string>)["Content-Type"]).toBe("application/json");
    expect(e.seq).toBe(4);
    expect(await e.view()).toEqual(view("cw", 2));
    expect(changes).toHaveBeenCalled();
  });

  it("on a stale seq refetches the game and says it moved on", async () => {
    const e = await NetworkEngine.open("7");
    reply("POST", "/api/games/7/orders", { status: 409, json: { error: "stale seq: the game is at 5", seq: 5 } });
    reply("GET", "/api/games/7", { status: 200, json: { side: "axis", seq: 5, open: false, view: view("cw", 3) } });
    expect(await e.apply("axis", order)).toEqual({ illegal: { case: STALE_CASE, reason: MOVED_ON } });
    expect(e.seq).toBe(5);
    expect(await e.view()).toEqual(view("cw", 3));
  });

  it("answers an illegal order as the server words it, the view untouched", async () => {
    const e = await NetworkEngine.open("7");
    const illegal = { case: "not-your-turn", reason: "it is not your turn" };
    reply("POST", "/api/games/7/orders", { status: 422, json: { illegal } });
    const before = calls.length;
    expect(await e.apply("axis", order)).toEqual({ illegal });
    expect(calls.length).toBe(before + 1);
    expect(e.seq).toBe(3);
  });

  it("reports a refusal with no seq (game not in play) without refetching", async () => {
    const e = await NetworkEngine.open("7");
    reply("POST", "/api/games/7/orders", { status: 409, json: { error: "the game is not in play" } });
    expect(await e.apply("axis", order)).toEqual({ illegal: { case: "server", reason: "the game is not in play" } });
  });

  it("replaces the view whole on each stream message and logs its events", async () => {
    const e = await NetworkEngine.open("7");
    const changes = vi.fn();
    e.onChange(changes);
    const ev = { "die-rolled": { purpose: "weather", value: 4 } };
    FakeEventSource.last!.emit({ seq: 3, events: [ev], view: view("cw", 9) });
    expect(await e.view()).toEqual(view("cw", 9));
    expect(e.seq).toBe(4);
    expect(e.log).toEqual([ev]);
    expect(e.turnText).toBe("Waiting for bob");
    expect(changes).toHaveBeenCalledTimes(1);
    // A message older than the view held (the order's own reload got there first) changes nothing.
    FakeEventSource.last!.emit({ seq: 1, events: [ev], view: view("axis", 1) });
    expect(await e.view()).toEqual(view("cw", 9));
    expect(e.log).toEqual([ev]);
  });

  it("passes queries' illegal answers through and throws other errors", async () => {
    const e = await NetworkEngine.open("7");
    const illegal = { case: "no-move", reason: "out of supply" };
    reply("GET", `/api/games/7/reachable?pieces=${encodeURIComponent('["a1"]')}&purpose=move`, { status: 422, json: { illegal } });
    expect(await e.reachable("axis", ["a1"], "move")).toEqual({ illegal });
    reply("GET", `/api/games/7/fuel-sources?pieces=${encodeURIComponent('["a1"]')}`, { status: 200, json: { ok: ["s1"] } });
    expect(await e.fuelSources("axis", ["a1"])).toEqual(["s1"]);
    reply("GET", "/api/games/7/placements?piece=x", { status: 403, json: { error: "not your piece" } });
    await expect(e.placements("x")).rejects.toThrow("not your piece");
    reply("GET", "/api/games/7/save", { status: 409, json: { error: "the game is not over" } });
    await expect(e.save()).rejects.toThrow("the game is not over");
  });

  it("on a seats event reloads the status and the opponent's name", async () => {
    reply("GET", "/api/games/7", { status: 200, json: { side: "axis", seq: 0, open: false, status: "open", view: view("axis") } });
    reply("GET", "/api/games?mine", { status: 200, json: [{ id: 7, opponent: null }] });
    const e = await NetworkEngine.open("7");
    expect(e.turnText).toBe("Waiting for an opponent to join");
    const changes = vi.fn();
    e.onChange(changes);

    // Bob joins: the game goes active and he is to move.
    reply("GET", "/api/games/7", { status: 200, json: { side: "axis", seq: 0, open: false, status: "active", view: view("cw") } });
    reply("GET", "/api/games?mine", { status: 200, json: [{ id: 7, opponent: "bob" }] });
    FakeEventSource.last!.emitNamed("seats");
    await vi.waitFor(() => expect(e.turnText).toBe("Waiting for bob"));
    expect(e.opponent).toBe("bob");
    expect(changes).toHaveBeenCalled();

    // He leaves.
    reply("GET", "/api/games/7", { status: 200, json: { side: "axis", seq: 0, open: false, status: "abandoned", view: view("cw") } });
    FakeEventSource.last!.emitNamed("seats");
    await vi.waitFor(() => expect(e.turnText).toBe("Abandoned: a player left"));
  });

  it("close ends the stream and its subscriptions", async () => {
    const e = await NetworkEngine.open("7");
    const changes = vi.fn();
    e.onChange(changes);
    const es = FakeEventSource.last!;
    e.close();
    expect(es.closed).toBe(true);
    es.emit({ seq: 3, events: [], view: view("cw") });
    es.emitNamed("seats");
    await Promise.resolve();
    expect(changes).not.toHaveBeenCalled();
    expect(calls.filter((c) => c.url === "/api/games/7")).toHaveLength(1);
  });
});
