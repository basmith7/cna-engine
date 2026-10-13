// What test builds hang on `window` (App.svelte, `import.meta.env.MODE === "test"`).
import type { SealedOptionsResult, View } from "../src/types";

declare global {
  interface Window {
    __cna: { view(): View; save(): string; orders(): number; load(json: string): void; sealedOptions(): SealedOptionsResult };
  }
}

export {};
