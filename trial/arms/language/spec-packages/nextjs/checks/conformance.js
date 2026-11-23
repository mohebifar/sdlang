// Conformance check for @sdlang/nextjs (check protocol, purpose "conformance").
//
// page_default_export: the page slot's module has a default export that is a React
// component (a function, or an identifier or class whose name starts with a capital).
//
// no_client_secrets: no module in the subject's client bundle (a file marked "use client"
// and every local module it imports, transitively) reads a process.env variable without
// the NEXT_PUBLIC_ prefix or imports "server-only" or a module that does. A computed
// process.env access is reported unsupported. Judged statically on the candidate.
"use strict";
const fs = require("node:fs");
const path = require("node:path");
const { readStdin, stripComments, imports, resolveLocal } = require("./lib");

function read(workdir, file) {
  try {
    return fs.readFileSync(path.join(workdir, file), "utf8");
  } catch {
    return null;
  }
}

function pageDefaultExport(input) {
  const page = input.outputs.find((o) => o.member.endsWith("#outputs.page"));
  if (!page) return { status: "error", inputs: [], message: "the subject has no page slot" };
  const text = read(input.workdir, page.path);
  if (text === null) return { status: "violated", inputs: [page.path], message: `${page.path} does not exist` };
  const code = stripComments(text);
  const ok =
    /export\s+default\s+(async\s+)?function\b/.test(code) ||
    /export\s+default\s+class\s+[A-Z]/.test(code) ||
    /export\s+default\s+[A-Z][\w$]*\s*;?/.test(code) ||
    /export\s+default\s+(async\s*)?\([^)]*\)\s*=>/.test(code) ||
    /export\s*\{[^}]*\b[A-Z][\w$]*\s+as\s+default\b[^}]*\}/.test(code);
  return ok
    ? { status: "satisfied", inputs: [page.path] }
    : { status: "violated", inputs: [page.path], message: `${page.path} has no default export of a component` };
}

function isClient(text) {
  return /^\s*(["'])use client\1/.test(stripComments(text).replace(/^\s*(["'])use strict\1;?/, ""));
}

function noClientSecrets(input) {
  const workdir = input.workdir;
  // Every local module reachable from the subject's outputs.
  const reach = new Map();
  const todo = input.outputs.map((o) => o.path).filter((p) => /\.(tsx?|jsx?|mjs|cjs)$/.test(p));
  while (todo.length) {
    const file = todo.pop();
    if (reach.has(file)) continue;
    const text = read(workdir, file);
    if (text === null) continue;
    const specs = imports(text);
    reach.set(file, { text, specs });
    for (const s of specs) {
      const r = resolveLocal(workdir, file, s);
      if (r) todo.push(r);
    }
  }
  // The client bundle: client roots and everything they import.
  const bundle = new Set();
  const queue = [...reach.keys()].filter((f) => isClient(reach.get(f).text));
  while (queue.length) {
    const file = queue.pop();
    if (bundle.has(file)) continue;
    bundle.add(file);
    for (const s of reach.get(file)?.specs ?? []) {
      const r = resolveLocal(workdir, file, s);
      if (r && reach.has(r)) queue.push(r);
    }
  }
  const problems = [];
  let unsupported = null;
  for (const file of bundle) {
    const { text, specs } = reach.get(file);
    const code = stripComments(text);
    for (const m of code.matchAll(/process\.env\.([A-Za-z_$][\w$]*)/g)) {
      if (!m[1].startsWith("NEXT_PUBLIC_")) problems.push(`${file} reads process.env.${m[1]} in the client bundle`);
    }
    if (/process\.env\s*\[/.test(code) || /\{[^}]*\}\s*=\s*process\.env\b/.test(code)) {
      unsupported = `${file} reads process.env with a computed or destructured name`;
    }
    for (const s of specs) {
      if (s === "server-only") problems.push(`${file} imports "server-only" in the client bundle`);
      const r = resolveLocal(workdir, file, s);
      if (r && reach.get(r) && imports(reach.get(r).text).includes("server-only")) {
        problems.push(`${file} imports ${r}, which is marked "server-only"`);
      }
    }
  }
  const inputs = [...reach.keys()].sort();
  if (problems.length) return { status: "violated", inputs, message: problems.join("; ").slice(0, 2000) };
  if (unsupported) return { status: "unsupported", inputs, message: unsupported };
  return { status: "satisfied", inputs };
}

const RULES = { page_default_export: pageDefaultExport, no_client_secrets: noClientSecrets };

function run(input) {
  const results = (input.rules ?? []).map((rule) => {
    const judge = RULES[rule];
    const r = judge ? judge(input) : { status: "error", inputs: [], message: `unknown rule ${rule}` };
    return { rule, subject: input.subject, ...r };
  });
  return { protocol: "sdlang.check/1", check: input.check, snapshot: input.snapshot, results };
}

if (require.main === module) {
  process.stdout.write(JSON.stringify(run(readStdin())) + "\n");
}

module.exports = { run };
