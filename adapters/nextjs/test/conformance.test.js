"use strict";
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { run } = require("../checks/conformance");

function project(files) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "nextjs-conformance-"));
  for (const [f, text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(dir, f)), { recursive: true });
    fs.writeFileSync(path.join(dir, f), text);
  }
  return dir;
}

function judge(dir, rules, outputs) {
  return run({
    check: { id: "nextjs.conformance", definition_hash: "sha256:" + "0".repeat(64) },
    snapshot: "sha256:" + "1".repeat(64),
    subject: "auth.login",
    rules,
    workdir: dir,
    outputs: outputs.map(([member, p]) => ({ member: `auth.login#outputs.${member}`, path: p })),
  }).results;
}

test("a page with a default component export satisfies page_default_export", () => {
  const dir = project({ "src/app/login/page.tsx": "// export default nothing\nexport default function Login() { return null; }\n" });
  assert.deepStrictEqual(judge(dir, ["page_default_export"], [["page", "src/app/login/page.tsx"]])[0].status, "satisfied");
});

test("a page without a default export violates page_default_export", () => {
  const dir = project({ "src/app/login/page.tsx": "// export default function Old() {}\nexport const x = 1;\n" });
  assert.strictEqual(judge(dir, ["page_default_export"], [["page", "src/app/login/page.tsx"]])[0].status, "violated");
});

test("client code reading a private env variable violates no_client_secrets", () => {
  const dir = project({
    "src/app/login/page.tsx": 'import { Form } from "./_parts/Form";\nexport default function P() { return Form; }\n',
    "src/app/login/_parts/Form.tsx": '"use client";\nimport { helper } from "./helper";\nexport const Form = helper;\n',
    "src/app/login/_parts/helper.ts": "export const helper = process.env.API_SECRET;\nexport const ok = process.env.NEXT_PUBLIC_URL;\n",
  });
  const r = judge(dir, ["no_client_secrets"], [["page", "src/app/login/page.tsx"], ["form", "src/app/login/_parts/Form.tsx"]])[0];
  assert.strictEqual(r.status, "violated");
  assert.match(r.message, /API_SECRET/);
  assert.doesNotMatch(r.message, /NEXT_PUBLIC_URL/);
});

test("server code may read secrets, and client code may not import server-only modules", () => {
  const server = project({ "src/app/login/page.tsx": "const k = process.env.API_SECRET;\nexport default function P() { return k; }\n" });
  assert.strictEqual(judge(server, ["no_client_secrets"], [["page", "src/app/login/page.tsx"]])[0].status, "satisfied");
  const leaky = project({
    "src/app/login/page.tsx": '"use client";\nimport signIn from "@/server/auth/sign_in";\nexport default function P() { return signIn; }\n',
    "src/server/auth/sign_in.ts": 'import "server-only";\nexport default async function signIn() {}\n',
  });
  assert.strictEqual(judge(leaky, ["no_client_secrets"], [["page", "src/app/login/page.tsx"]])[0].status, "violated");
});

test("a computed env access is unsupported", () => {
  const dir = project({ "src/app/x/page.tsx": '"use client";\nconst k = "A";\nexport default function P() { return process.env[k]; }\n' });
  assert.strictEqual(judge(dir, ["no_client_secrets"], [["page", "src/app/x/page.tsx"]])[0].status, "unsupported");
});
