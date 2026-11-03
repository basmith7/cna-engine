// The page's path, as the online client reads it: a game (`/game/{id}`),
// an invite (`/join/{id}?invite=…`), or the start screen for anything
// else. The server answers each of these paths with the client.

export type Route = { page: "start" } | { page: "game"; id: string } | { page: "join"; id: string; invite: string | null };

/** The route for `pathname` and `search` (`location`'s). */
export function parseRoute(pathname: string, search: string): Route {
  const m = /^\/(game|join)\/(\d+)\/?$/.exec(pathname);
  if (!m) return { page: "start" };
  if (m[1] === "game") return { page: "game", id: m[2] };
  const invite = new URLSearchParams(search).get("invite");
  return { page: "join", id: m[2], invite: invite || null };
}

/** Where sign-in should come back to: the server's callback lands on `/`, so an invite followed while signed out waits here. */
export const RETURN_KEY = "cna-return-to";
