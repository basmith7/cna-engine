import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const mapData = fileURLToPath(new URL("../vendor/cna/data/map", import.meta.url));

export default defineConfig({
  plugins: [svelte()],
  build: {
    // The 7 073-hex map JSON is in the main chunk by design (spec, Map drawing data).
    chunkSizeWarningLimit: 1500,
  },
  server: {
    // The map JSON is read straight from the submodule (spec, Decisions).
    fs: { allow: [".", mapData] },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
