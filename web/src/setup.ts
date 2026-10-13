// The set-up in words: what a pending piece's placement allows, for the
// tray (spec, *Placements*). The hexes themselves come from the engine's
// `placements`; this only says the rule in English.
import type { Placement } from "./types";

const COUNTRY: Record<string, string> = { libya: "Libya", egypt: "Egypt" };

function list(items: string[]): string {
  if (items.length <= 1) return items.join("");
  return `${items.slice(0, -1).join(", ")} or ${items[items.length - 1]}`;
}

/** "any hex of sheet C, not within 4 hexes of an enemy unit". */
export function placementWords(p: Placement): string {
  let where: string;
  if (p.hexes) where = p.hexes.length === 1 ? p.hexes[0] : `any of ${list(p.hexes)}`;
  else if (p.within) where = `any hex within ${p.within.hexes} of ${p.within.of}`;
  else if (p.region) where = `any hex in ${COUNTRY[p.region] ?? p.region}`;
  else if (p.sheets) where = p.sheets.length === 1 ? `any hex of sheet ${p.sheets[0]}` : `any hex of sheets ${list(p.sheets)}`;
  else if (p.place) where = p.place;
  else if (p.box) where = `the ${p.box} box`;
  else if (p.air_facility) where = "any air facility";
  else where = "nowhere";
  if (p.not_within) {
    const of = p.not_within.of === "enemy-unit" ? "an enemy unit" : p.not_within.of;
    where += `, not within ${p.not_within.hexes} hexes of ${of}`;
  }
  return where;
}
