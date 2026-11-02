// Save, load and autosave (spec, *Save and load*): the state JSON
// `save()` writes is the save format, so this is naming, the
// `localStorage` slot and the seed field. Pure functions; the app does
// the downloading and the storage.
import { clockLine } from "./clock";
import type { View } from "./types";

/** The `localStorage` key the autosave sits under. */
export const AUTOSAVE_KEY = "cna:autosave";

/** The file a Save downloads: `<scenario>-gt<N>-os<M>.json`, the clock at the moment of saving (`graziani` for Graziani's Offensive, else the slug). */
export function saveName(view: Pick<View, "clock" | "scenario">): string {
  const prefix = view.scenario === "grazianis-offensive" ? "graziani" : view.scenario;
  return `${prefix}-gt${view.clock.game_turn}-os${view.clock.stage}.json`;
}

/** A seed for a new game: a whole number under 2^32, shown in the field so the game can be repeated. */
export function randomSeed(): number {
  return Math.floor(Math.random() * 2 ** 32);
}

/**
 * The seed typed into the field, or null when it is not a whole number
 * the engine takes (a `u64`; JavaScript keeps whole numbers exact to 2^53).
 */
export function parseSeed(text: string): number | null {
  const t = text.trim();
  if (!/^\d+$/.test(t)) return null;
  const n = Number(t);
  return Number.isSafeInteger(n) ? n : null;
}

/**
 * The clock line of a save, for the Continue button, or null when the
 * text is not a save at all; the engine judges the rest when it loads.
 */
export function describeSave(json: string): string | null {
  try {
    const v = JSON.parse(json) as Partial<View>;
    if (!v || typeof v !== "object" || !v.clock || typeof v.clock.game_turn !== "number") return null;
    return clockLine({ clock: v.clock, pending: v.pending ?? null } as View);
  } catch {
    return null;
  }
}
