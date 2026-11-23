// Runs the acceptance tests active at one edit against one arm's tree (docs/trial.md,
// Acceptance tests): builds the arm with `next build`, runs the active UI tests
// (Playwright, a fresh `next start` per file) and contract tests (Vitest), and writes a
// summary with one outcome per test id. Whatever the build generates in the arm's tree is
// removed afterwards, unless it was there before.
//
// Usage: node run.mjs <arm dir> <edit: baseline | E1 ... E10> [--out <summary.json>]
// Environment: PATCHED_ABOUT for A8.2 (see patches/e8.mjs).
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const outAt = args.indexOf("--out");
const out = outAt >= 0 ? args.splice(outAt, 2)[1] : null;
const [armArg, edit] = args;
const ORDER = ["baseline", "E1", "E2", "E3", "E4", "E5", "E6", "E7", "E8", "E9", "E10"];
if (!armArg || !ORDER.includes(edit)) {
  console.error("usage: node run.mjs <arm dir> <baseline | E1 ... E10> [--out <file>]");
  process.exit(2);
}
const arm = path.resolve(armArg);
const table = JSON.parse(fs.readFileSync(path.join(here, "tests.json"), "utf8"));
const at = ORDER.indexOf(edit);
const active = Object.entries(table.tests)
  .filter(([, t]) => ORDER.indexOf(t.from) <= at && (t.retired === null || ORDER.indexOf(t.retired) > at))
  .map(([id, t]) => ({ id, kind: t.kind }));
let routes = {};
for (const e of ORDER.slice(0, at + 1)) Object.assign(routes, table.routes[e] ?? {});

const results = Object.fromEntries(active.map((t) => [t.id, "not run"]));
const env = { ...process.env, ARM: arm, ROUTES: JSON.stringify(routes), CI: "1" };
const bin = (pkg, rel) => path.join(here, "node_modules", pkg, rel);
const run = (argv, cwd, extra = {}) =>
  spawnSync(process.execPath, argv, { cwd, env: { ...env, ...extra }, stdio: "inherit" });

// The build's output must not outlive the run: in the strong and plain arms the agent works
// in this tree, and a stale .next/types would show it type errors the run left behind.
const generated = [".next", "next-env.d.ts", "tsconfig.tsbuildinfo"].filter(
  (p) => !fs.existsSync(path.join(arm, p)),
);
const build = run([path.join(arm, "node_modules/next/dist/bin/next"), "build"], arm);
const report = path.join(here, "test-results");
fs.mkdirSync(report, { recursive: true });
const ui = active.filter((t) => t.kind === "ui").map((t) => `ui/${t.id}.spec.ts`);
const contract = active.filter((t) => t.kind === "contract").map((t) => `contract/${t.id}.test.ts`);
const idOf = (title) => title.split(" ")[0];

if (build.status !== 0) {
  for (const t of active.filter((t) => t.kind === "ui")) results[t.id] = "failed (build)";
} else if (ui.length) {
  const file = path.join(report, "ui.json");
  fs.rmSync(file, { force: true });
  run([bin("@playwright/test", "cli.js"), "test", ...ui], here, { PW_REPORT: file });
  const walk = (suite) => [
    ...(suite.specs ?? []).flatMap((s) => s.tests.map((t) => [s.title, t.results.at(-1)?.status ?? "not run"])),
    ...(suite.suites ?? []).flatMap(walk),
  ];
  if (fs.existsSync(file)) {
    for (const [title, status] of JSON.parse(fs.readFileSync(file, "utf8")).suites.flatMap(walk)) {
      results[idOf(title)] = status === "passed" ? "passed" : "failed";
    }
  }
}
if (contract.length) {
  const file = path.join(report, "contract.json");
  fs.rmSync(file, { force: true });
  run([bin("vitest", "vitest.mjs"), "run", ...contract], here, { VT_REPORT: file });
  if (fs.existsSync(file)) {
    for (const r of JSON.parse(fs.readFileSync(file, "utf8")).testResults) {
      for (const a of r.assertionResults) results[idOf(a.title)] = a.status === "passed" ? "passed" : "failed";
      // A file that failed to load has no assertions; every id it held failed.
      if (!r.assertionResults.length) results[idOf(path.basename(r.name, ".test.ts"))] = "failed (load)";
    }
  }
}

for (const p of generated) fs.rmSync(path.join(arm, p), { recursive: true, force: true });

const summary = { arm, edit, routes, build: build.status === 0 ? "passed" : "failed", results };
const text = JSON.stringify(summary, null, 2) + "\n";
if (out) fs.writeFileSync(out, text);
process.stdout.write(text);
process.exit(Object.values(results).every((r) => r === "passed") ? 0 : 1);
