import path from "node:path";
import { defineConfig } from "vitest/config";

const arm = process.env.ARM ?? "";

// Contract tests import an arm's action modules from their adapter paths. Each test file
// runs in its own process (isolate), so the in-memory store starts empty.
export default defineConfig({
  resolve: {
    alias: [
      { find: /^server-only$/, replacement: path.resolve("contract/server-only.ts") },
      { find: /^@\//, replacement: `${path.join(arm, "src")}/` },
    ],
  },
  test: {
    include: ["contract/**/*.test.ts"],
    pool: "forks",
    isolate: true,
    reporters: [["json", { outputFile: process.env.VT_REPORT ?? "test-results/contract.json" }], "default"],
  },
});
