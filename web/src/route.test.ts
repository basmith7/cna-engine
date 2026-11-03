import { describe, expect, it } from "vitest";
import { parseRoute } from "./route";

describe("parseRoute", () => {
  it("reads a game page", () => {
    expect(parseRoute("/game/12", "")).toEqual({ page: "game", id: "12" });
    expect(parseRoute("/game/12/", "")).toEqual({ page: "game", id: "12" });
  });

  it("reads an invite, with or without its secret", () => {
    expect(parseRoute("/join/3", "?invite=a%2Bb")).toEqual({ page: "join", id: "3", invite: "a+b" });
    expect(parseRoute("/join/3", "")).toEqual({ page: "join", id: "3", invite: null });
    expect(parseRoute("/join/3", "?invite=")).toEqual({ page: "join", id: "3", invite: null });
  });

  it("sends anything else to the start screen", () => {
    for (const p of ["/", "", "/game", "/game/", "/game/abc", "/game/1/x", "/join/-1", "/lobby"]) {
      expect(parseRoute(p, "")).toEqual({ page: "start" });
    }
  });
});
