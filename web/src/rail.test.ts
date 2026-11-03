import { describe, expect, it } from "vitest";
import { aboardAt, addStop, stopsOn, toggleAtStop } from "./rail";

describe("rail stops (R-005)", () => {
  const two = addStop(addStop([], "E3412"), "E3211");

  it("adds a hex once", () => {
    expect(addStop(two, "E3412")).toEqual(two);
    expect(two.map((s) => s.at)).toEqual(["E3412", "E3211"]);
  });

  it("follows who is aboard stop by stop", () => {
    let stops = toggleAtStop(two, 0, "off", "a");
    stops = toggleAtStop(stops, 0, "on", "s");
    expect(aboardAt(["a", "b"], stops, 0)).toEqual(["a", "b"]);
    expect(aboardAt(["a", "b"], stops, 1)).toEqual(["b", "s"]);
    // A second toggle drops the name again.
    expect(toggleAtStop(stops, 0, "on", "s")[0].on).toEqual([]);
  });

  it("sends only the stops the path passes before its end, in its order, with someone getting off or on", () => {
    const stops = toggleAtStop(toggleAtStop(two, 1, "off", "a"), 0, "off", "b");
    expect(stopsOn(stops, ["E3513", "E3412", "E3312", "E3211", "E3210"]).map((s) => s.at)).toEqual(["E3412", "E3211"]);
    // A stop at the end, or past it, is not one; an empty stop is dropped.
    expect(stopsOn(stops, ["E3513", "E3412"])).toEqual([]);
    expect(stopsOn(two, ["E3513", "E3412", "E3312", "E3211", "E3210"])).toEqual([]);
  });
});
