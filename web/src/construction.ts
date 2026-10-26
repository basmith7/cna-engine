// Construction projects (24.1x): what each item is called and how many
// stages of work it takes (the Construction Chart, 24.17).
import type { BuildItem } from "./types";

const NAMES: Record<BuildItem, string> = {
  fortification: "fortification",
  "real-minefield": "minefield",
  "dummy-minefield": "dummy minefield",
};

const STAGES: Record<BuildItem, number> = {
  fortification: 3,
  "real-minefield": 1,
  "dummy-minefield": 1,
};

export const itemName = (item: BuildItem): string => NAMES[item];
export const itemStages = (item: BuildItem): number => STAGES[item];
