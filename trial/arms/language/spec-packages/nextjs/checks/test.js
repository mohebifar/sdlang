// Test check for @sdlang/nextjs (check protocol, purpose "test"): runs Vitest on the one
// test output in the input, in the candidate, and reports per case (see vitest-cases.js).
// SPEC_VITEST overrides the command (default: pnpm exec vitest).
"use strict";
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const { readStdin } = require("./lib");
const { toCheckResult } = require("./vitest-cases");

const input = readStdin();
const output = input.outputs[0];
const report = path.join(input.scratch, `vitest-${process.pid}.json`);
const command = (process.env.SPEC_VITEST ?? "pnpm exec vitest").split(" ");
const run = spawnSync(command[0], [...command.slice(1), "run", output.path, "--reporter=json", `--outputFile=${report}`], {
  cwd: input.workdir,
  encoding: "utf8",
  env: { ...process.env, CI: "1" },
});
if (!fs.existsSync(report)) {
  // No report at all is an infrastructure failure, not a failing test.
  process.stderr.write(`vitest wrote no report (exit ${run.status}):\n${run.stdout}\n${run.stderr}`);
  process.exit(1);
}
process.stdout.write(JSON.stringify(toCheckResult(input, JSON.parse(fs.readFileSync(report, "utf8")))) + "\n");
