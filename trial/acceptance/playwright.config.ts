import { defineConfig } from "@playwright/test";

// One worker: every test file starts its own `next start` on the arm's build, so the
// in-memory store starts empty and files never share a server.
export default defineConfig({
  testDir: "ui",
  workers: 1,
  fullyParallel: false,
  retries: 0,
  timeout: 60_000,
  reporter: [["json", { outputFile: process.env.PW_REPORT ?? "test-results/ui.json" }], ["list"]],
  use: { headless: true },
});
