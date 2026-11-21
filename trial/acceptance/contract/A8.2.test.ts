import fs from "node:fs";
import path from "node:path";
import { expect, it } from "vitest";
import { armDir } from "./arm";

// PATCHED_ABOUT names the copy of the About page the operator saved right after the E8 hand
// patch (patches/e8.mjs writes it).
it("A8.2 the About page file is byte-identical to the hand patch", () => {
  const saved = process.env.PATCHED_ABOUT;
  expect(saved, "set PATCHED_ABOUT").toBeTruthy();
  const now = fs.readFileSync(path.join(armDir, "src/app/about/page.tsx"));
  expect(now.equals(fs.readFileSync(saved!))).toBe(true);
});
