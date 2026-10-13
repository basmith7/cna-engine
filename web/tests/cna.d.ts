// What test builds hang on `window` (App.svelte, `import.meta.env.MODE === "test"`).
import type { View } from "../src/types";

declare global {
  interface Window {
    __cna: { view(): View; save(): string; load(json: string): void };
  }
}

export {};
