// Test runner configuration (unmanaged): tests use Vitest's globals, so a test file needs
// no import its declaration's edges do not grant.
import { defineConfig } from "vitest/config";

export default defineConfig({
  esbuild: { jsx: "automatic" },
  test: {
    globals: true,
    environment: "node",
    include: ["tests/**/*.test.ts"],
  },
});
