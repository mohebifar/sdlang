// Maps Vitest's JSON report to the check protocol's test result.
//
// A test names its case with a tag in its title: "[case:valid_credentials]", or
// "[case:generic_error:wrong_password]" for one instance of a parameterised case. Untagged
// tests still run and can fail the check, but never provide coverage.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const { sha256, canonical } = require("./lib");

const TAG = /\[case:([a-z_][a-z0-9_]*)(?::([A-Za-z0-9_.-]+))?\]/;
const STATUS = { passed: "passed", failed: "failed", pending: "skipped", skipped: "skipped", todo: "skipped", disabled: "skipped" };

// The oracle digest is coarse: the test file plus the runner configuration.
function oracle(workdir, testPath) {
  const parts = [testPath, "vitest.config.ts", "vitest.config.mts", "vitest.config.js", "vitest.setup.ts"]
    .map((f) => {
      try {
        return f + "\0" + fs.readFileSync(path.join(workdir, f)).toString("base64");
      } catch {
        return "";
      }
    })
    .join("\0");
  return sha256(parts);
}

function toCheckResult(input, report) {
  const output = input.outputs[0];
  const digest = oracle(input.workdir, output.path);
  const cases = new Map();
  const untagged = { passed: 0, failed: 0 };
  for (const file of report.testResults ?? []) {
    const name = path.relative(input.workdir, file.name ?? "").split(path.sep).join("/");
    if (name !== output.path) continue;
    for (const t of file.assertionResults ?? []) {
      const status = STATUS[t.status] ?? "error";
      const m = TAG.exec(t.title ?? "") ?? TAG.exec(t.fullName ?? "");
      if (!m) {
        if (status === "failed" || status === "error") untagged.failed++;
        else if (status === "passed") untagged.passed++;
        continue;
      }
      const [, tag, key = "default"] = m;
      const attempts = [];
      for (let i = 0; i < (t.retryCount ?? 0); i++) attempts.push("failed");
      attempts.push(status);
      if (!cases.has(tag)) cases.set(tag, []);
      cases.get(tag).push({ key, attempts });
    }
  }
  const caseList = [...cases.entries()]
    .sort(([a], [b]) => (a < b ? -1 : 1))
    .map(([tag, instances]) => ({ case: tag, oracle: digest, instances }));
  const pairs = caseList.flatMap((c) => c.instances.map((i) => [c.case, i.key])).sort((a, b) => canonical(a) < canonical(b) ? -1 : 1);
  return {
    protocol: "sdlang.check/1",
    check: input.check,
    snapshot: input.snapshot,
    output: output.member,
    selection: "complete",
    cases: caseList,
    untagged,
    inventory: sha256(canonical(pairs)),
    inputs: input.inputs,
    profile: input.profile,
  };
}

module.exports = { toCheckResult, TAG };
