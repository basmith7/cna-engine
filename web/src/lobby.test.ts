import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LobbyError, createGame, freeSides, inviteLink, joinGame, leaveGame, me, myGames, openGames, signOut, type MyGame, type OpenGame } from "./lobby";

type Call = { url: string; init: RequestInit };
let calls: Call[];
let replies: Map<string, { status: number; json?: unknown }>;

function reply(method: string, path: string, status: number, json?: unknown) {
  replies.set(`${method} ${path}`, { status, json });
}

beforeEach(() => {
  calls = [];
  replies = new Map();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (url: string, init: RequestInit = {}) => {
      calls.push({ url, init });
      const key = `${init.method ?? "GET"} ${url}`;
      const r = replies.get(key);
      if (!r) throw new Error(`unexpected ${key}`);
      return new Response(r.json === undefined ? null : JSON.stringify(r.json), { status: r.status });
    }),
  );
});

afterEach(() => vi.unstubAllGlobals());

const mine = (id: number, my_move: boolean) => ({ id, my_move }) as MyGame;

describe("lobby client", () => {
  it("reads the account, and nobody when the server answers 401", async () => {
    reply("GET", "/api/me", 200, { id: 1, name: "alice", admin: false });
    expect((await me())?.name).toBe("alice");
    reply("GET", "/api/me", 401, { error: "not signed in" });
    expect(await me()).toBeNull();
  });

  it("throws the server's message for other failures", async () => {
    reply("GET", "/api/me", 500, { error: "internal error" });
    await expect(me()).rejects.toThrow("internal error");
  });

  it("lists my games with my moves first, keeping the server's order otherwise", async () => {
    reply("GET", "/api/games?mine", 200, [mine(1, false), mine(2, true), mine(3, false), mine(4, true)]);
    expect((await myGames()).map((g) => g.id)).toEqual([2, 4, 1, 3]);
  });

  it("lists no games when there are none", async () => {
    reply("GET", "/api/games?mine", 200, []);
    reply("GET", "/api/games?open", 200, []);
    expect(await myGames()).toEqual([]);
    expect(await openGames()).toEqual([]);
  });

  it("creates a game with the options as JSON and returns its invite", async () => {
    reply("POST", "/api/games", 201, { id: 9, invite: "tok" });
    const c = await createGame({ side: "either", open: false, open_information: true });
    expect(c).toEqual({ id: 9, invite: "tok" });
    expect(JSON.parse(calls[0].init.body as string)).toEqual({ side: "either", open: false, open_information: true });
    expect(calls[0].init.credentials).toBe("same-origin");
  });

  it("refuses with the status and message when the server says no", async () => {
    reply("POST", "/api/games", 409, { error: "you have too many games waiting for a player" });
    const e = await createGame({ side: "axis", open: true, open_information: false }).catch((e) => e);
    expect(e).toBeInstanceOf(LobbyError);
    expect(e.status).toBe(409);
    expect(e.message).toBe("you have too many games waiting for a player");
  });

  it("joins with the invite only when given one", async () => {
    reply("POST", "/api/games/5/join", 200, { id: 5, side: "cw" });
    await joinGame(5, "cw");
    await joinGame(5, "cw", "tok");
    expect(calls.map((c) => JSON.parse(c.init.body as string))).toEqual([{ side: "cw" }, { side: "cw", invite: "tok" }]);
  });

  it("leaves and signs out, accepting an empty 204", async () => {
    reply("POST", "/api/games/5/leave", 200, { id: 5, status: "abandoned" });
    reply("POST", "/auth/sign-out", 204);
    await leaveGame(5);
    await signOut();
    expect(calls.map((c) => c.url)).toEqual(["/api/games/5/leave", "/auth/sign-out"]);
  });

  it("gives a joiner the seat the creator left free", () => {
    const g = (creator_side: OpenGame["creator_side"]) => ({ creator_side }) as OpenGame;
    expect(freeSides(g("axis"))).toEqual(["cw"]);
    expect(freeSides(g("cw"))).toEqual(["axis"]);
    expect(freeSides(g("either"))).toEqual(["axis", "cw"]);
  });

  it("builds the invite link with the token escaped", () => {
    expect(inviteLink("https://cna.example", { id: 3, invite: "a+b/c" })).toBe("https://cna.example/join/3?invite=a%2Bb%2Fc");
  });
});
