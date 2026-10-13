import { defineConfig, devices } from "@playwright/test";

// Headless Chromium against the production build served by `vite preview`,
// built in `test` mode so `window.__cna` exposes the engine's view and save.
export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  // One worker: the performance budget in `map.spec.ts` is a timing, and on
  // CI's two cores it fails when a second worker runs the set-up e2e beside it.
  workers: 1,
  retries: 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: "http://localhost:4173",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } }],
  webServer: {
    command: "npm run build -- --mode test && npm run preview -- --port 4173 --strictPort",
    url: "http://localhost:4173",
    // Never reuse a server on 4173: another worktree's preview or a non-test
    // build would have no `window.__cna`.
    reuseExistingServer: false,
    timeout: 120_000,
  },
});
