// The set-up in words: what a pending piece's placement allows, for the
// tray (spec, *Placements*). The hexes themselves come from the engine's
// `placements`; this only says the rule in English.
import type { Placement } from "./types";

const COUNTRY: Record<string, string> = { libya: "Libya", egypt: "Egypt" };

function list(items: string[]): string {
  if (items.length <= 1) return items.join("");
  return `${items.slice(0, -1).join(", ")} or ${items[items.length - 1]}`;
}

/** "any hex of sheet C, not within 4 hexes of an enemy unit"; several
 * locators read as one area: "any hex in Libya on sheet C". */
export function placementWords(p: Placement): string {
  const parts: string[] = [];
  if (p.hexes) parts.push(p.hexes.length === 1 ? p.hexes[0] : `any of ${list(p.hexes)}`);
  if (p.within) parts.push(`any hex within ${p.within.hexes} of ${p.within.of}`);
  if (p.region) parts.push(`any hex in ${COUNTRY[p.region] ?? p.region}`);
  if (p.sheets) {
    const sheets = p.sheets.length === 1 ? `sheet ${p.sheets[0]}` : `sheets ${list(p.sheets)}`;
    parts.push(parts.length ? `on ${sheets}` : `any hex of ${sheets}`);
  }
  if (p.west_of) parts.push(parts.length ? `west of ${p.west_of}` : `any hex west of ${p.west_of}`);
  if (p.place) parts.push(typeof p.place === "string" ? p.place : list(p.place));
  if (p.box) parts.push(`the ${p.box} box`);
  if (p.air_facility) parts.push("any air facility");
  let where = parts.length ? parts.join(" ") : "nowhere";
  if (p.not_within) {
    const of = p.not_within.of === "enemy-unit" ? "an enemy unit" : p.not_within.of;
    where += `, not within ${p.not_within.hexes} hexes of ${of}`;
  }
  if (p.together) where += ", all in one hex";
  return where;
}
