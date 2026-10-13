import { describe, expect, it } from "vitest";
import { withEvents, withIllegal, type LogEntry } from "./log";

describe("the log panel's entries", () => {
  it("writes a clock header once for a run of events under the same clock", () => {
    let log: LogEntry[] = [];
    log = withEvents(log, "Game-Turn 1 · set-up", ["a placed", "b placed"]);
    log = withEvents(log, "Game-Turn 1 · set-up", ["c placed"]);
    expect(log).toEqual([
      { kind: "header", text: "Game-Turn 1 · set-up" },
      { kind: "event", text: "a placed" },
      { kind: "event", text: "b placed" },
      { kind: "event", text: "c placed" },
    ]);
  });

  it("writes a new header when the clock moves on, even after a refusal", () => {
    let log = withEvents([], "Game-Turn 1 · set-up", ["a placed"]);
    log = withIllegal(log, "Illegal (59.2): no");
    log = withEvents(log, "Game-Turn 1 · set-up", ["b placed"]);
    log = withEvents(log, "Game-Turn 1 · OpStage 1 · A · initiative", ["Axis declares itself Player A."]);
    expect(log.map((e) => e.kind)).toEqual(["header", "event", "illegal", "event", "header", "event"]);
    expect(log[4]).toEqual({ kind: "header", text: "Game-Turn 1 · OpStage 1 · A · initiative" });
  });

  it("writes no header for an empty batch", () => {
    expect(withEvents([], "x", [])).toEqual([]);
  });
});
