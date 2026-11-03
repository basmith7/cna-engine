import { defineConfig, devices } from "@playwright/test";

// The two-player e2e (`tests/multiplayer/`) against cna-server with the
// `test-auth` feature, signed in through the fake Discord; `serve.sh`
// starts both and a Postgres. Its own config, not a project beside the
// hot-seat one: Playwright starts every `webServer` whichever project runs,
// and the hot-seat specs need neither cargo nor Docker.
// `npx playwright test -c playwright.multiplayer.config.ts`.
const port = Number(process.env.CNA_PW_PORT ?? 3260);
export default defineConfig({
  testDir: "./tests/multiplayer",
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: `http://localhost:${port}`,
    trace: "retain-on-failure",
  },
  projects: [{ name: "multiplayer", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } }],
  webServer: {
    command: "tests/multiplayer/serve.sh",
    // Up once the server answers and its database does.
    url: `http://localhost:${port}/healthz`,
    reuseExistingServer: false,
    // A cold cargo build of the server comes first.
    timeout: 900_000,
    stdout: "pipe",
    gracefulShutdown: { signal: "SIGTERM", timeout: 10_000 },
  },
});
