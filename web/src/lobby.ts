// The lobby's side of the server's HTTP API (`crates/cna-server/src/api.rs`):
// who is signed in, the games to list, and creating, joining and leaving
// one. Sign-in itself is a page visit to `/auth/discord`.
import type { Clock, Side } from "./types";

/** `GET /api/me`. */
export type Account = { id: number; discord_id: string; name: string; avatar: string | null; alerts: boolean; admin: boolean };

/** A game's `status` column. */
export type GameStatus = "open" | "active" | "over" | "abandoned" | "stale";
export type Seats = { axis: string | null; cw: string | null };

/** An entry of `GET /api/games?mine`. */
export type MyGame = {
  id: number;
  scenario: string;
  status: GameStatus;
  to_act: Side | null;
  clock: Clock | null;
  seats: Seats;
  /** `null` for an "either" creator still waiting to be seated. */
  my_side: Side | null;
  my_move: boolean;
  opponent: string | null;
};

/** An entry of `GET /api/games?open`. */
export type OpenGame = {
  id: number;
  scenario: string;
  status: GameStatus;
  to_act: Side | null;
  clock: Clock | null;
  seats: Seats;
  creator: string | null;
  creator_side: Side | "either";
};

export type CreateSide = Side | "either";
/** `POST /api/games`: `open` lists it among the open games; `open_information` plays with nothing hidden. */
export type CreateOptions = { side: CreateSide; open: boolean; open_information: boolean; scenario?: string };
/** What creating answers: the invite is shown this once. */
export type Created = { id: number; invite: string };

export const SIGN_IN_URL = "/auth/discord";

/** A refusal from the server, with its message. */
export class LobbyError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

async function call<T>(method: string, url: string, body?: unknown): Promise<T> {
  const r = await fetch(url, {
    method,
    credentials: "same-origin",
    headers: body === undefined ? { Accept: "application/json" } : { Accept: "application/json", "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await r.text();
  let json: unknown = null;
  try {
    json = text ? JSON.parse(text) : null;
  } catch {
    json = null;
  }
  if (!r.ok) {
    const e = (json as { error?: unknown } | null)?.error;
    throw new LobbyError(r.status, typeof e === "string" ? e : `the server answered ${r.status}`);
  }
  return json as T;
}

/** The signed-in account, or `null` when nobody is signed in (401) or there is no lobby server. */
export async function me(): Promise<Account | null> {
  try {
    // A server without the API (the hot-seat build's) answers its page, not an account.
    const a = await call<Account | null>("GET", "/api/me");
    return a && typeof a.name === "string" ? a : null;
  } catch (e) {
    if (e instanceof LobbyError && e.status === 401) return null;
    throw e;
  }
}

/** The caller's games, those awaiting the caller's move first (the server's order kept otherwise). */
export async function myGames(): Promise<MyGame[]> {
  return sortMine(await call<MyGame[]>("GET", "/api/games?mine"));
}

export function sortMine(games: MyGame[]): MyGame[] {
  return [...games.filter((g) => g.my_move), ...games.filter((g) => !g.my_move)];
}

/** Public games waiting for a player that the caller is not in. */
export async function openGames(): Promise<OpenGame[]> {
  return call<OpenGame[]>("GET", "/api/games?open");
}

export async function createGame(o: CreateOptions): Promise<Created> {
  return call<Created>("POST", "/api/games", o);
}

/** Takes `side` in game `id`; a private game needs its `invite`. */
export async function joinGame(id: number, side: Side, invite?: string): Promise<void> {
  await call("POST", `/api/games/${id}/join`, invite === undefined ? { side } : { side, invite });
}

/** Leaves game `id`, which is then abandoned. */
export async function leaveGame(id: number): Promise<void> {
  await call("POST", `/api/games/${id}/leave`, {});
}

export async function signOut(): Promise<void> {
  await call("POST", "/auth/sign-out", {});
}

/** The seats a joiner of `g` may take: the one the creator left free, or either when the creator would take either. */
export function freeSides(g: OpenGame): Side[] {
  if (g.creator_side === "either") return ["axis", "cw"];
  return [g.creator_side === "axis" ? "cw" : "axis"];
}

/** The link that lets the opponent join a game, private or not. */
export function inviteLink(origin: string, c: Created): string {
  return `${origin}/join/${c.id}?invite=${encodeURIComponent(c.invite)}`;
}

/** An entry of `GET /admin/accounts`. */
export type AdminAccount = { id: number; discord_id: string; name: string; admin: boolean; suspended: boolean };

/** Every account, oldest first; admins only (403 otherwise). */
export async function accounts(): Promise<AdminAccount[]> {
  return call<AdminAccount[]>("GET", "/admin/accounts");
}

/** Suspends account `id` (its sessions and streams end) or restores it. */
export async function setSuspended(id: number, suspended: boolean): Promise<void> {
  await call("POST", `/admin/accounts/${id}/${suspended ? "suspend" : "restore"}`, {});
}
