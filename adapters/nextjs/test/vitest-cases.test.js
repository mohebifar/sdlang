"use strict";
const test = require("node:test");
const assert = require("node:assert");
const crypto = require("node:crypto");
const path = require("node:path");
const { toCheckResult } = require("../checks/vitest-cases");

const input = {
  check: { id: "nextjs.test", definition_hash: "sha256:" + "0".repeat(64) },
  snapshot: "sha256:" + "1".repeat(64),
  outputs: [{ member: "auth.login#outputs.tests", path: "tests/app/login/page.test.ts" }],
  workdir: "/work",
  inputs: "sha256:" + "2".repeat(64),
  profile: "local",
};

const report = {
  testResults: [
    {
      name: "/work/tests/app/login/page.test.ts",
      assertionResults: [
        { title: "redirects on success [case:valid_credentials]", status: "passed" },
        { title: "unknown email [case:generic_error:unknown_email]", status: "passed" },
        { title: "wrong password [case:generic_error:wrong_password]", status: "failed" },
        { title: "renders", status: "passed" },
        { title: "flaky [case:busy_button]", status: "passed", retryCount: 1 },
      ],
    },
    { name: "/work/tests/other.test.ts", assertionResults: [{ title: "[case:elsewhere]", status: "passed" }] },
  ],
};

test("tags become cases and instances; untagged tests are counted", () => {
  const r = toCheckResult(input, report);
  assert.deepStrictEqual(r.cases.map((c) => c.case), ["busy_button", "generic_error", "valid_credentials"]);
  const generic = r.cases.find((c) => c.case === "generic_error");
  assert.deepStrictEqual(generic.instances, [
    { key: "unknown_email", attempts: ["passed"] },
    { key: "wrong_password", attempts: ["failed"] },
  ]);
  assert.deepStrictEqual(r.cases[0].instances[0].attempts, ["failed", "passed"], "a retried pass keeps the failed attempt");
  assert.deepStrictEqual(r.untagged, { passed: 1, failed: 0 });
  assert.strictEqual(r.output, "auth.login#outputs.tests");
  assert.strictEqual(r.inputs, input.inputs);
});

test("the inventory digest covers the sorted case and key pairs", () => {
  const r = toCheckResult(input, report);
  const pairs = '[["busy_button","default"],["generic_error","unknown_email"],["generic_error","wrong_password"],["valid_credentials","default"]]';
  assert.strictEqual(r.inventory, "sha256:" + crypto.createHash("sha256").update(pairs).digest("hex"));
});
