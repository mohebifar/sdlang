// Shared helpers for the adapter's checks: reading protocol input, digests, source scanning.
"use strict";
const crypto = require("node:crypto");
const fs = require("node:fs");
const path = require("node:path");

function readStdin() {
  return JSON.parse(fs.readFileSync(0, "utf8"));
}

function sha256(bytes) {
  return "sha256:" + crypto.createHash("sha256").update(bytes).digest("hex");
}

// Canonical JSON for the values the checks hash (strings, numbers, arrays, plain objects):
// keys sorted by code point, no whitespace.
function canonical(value) {
  if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
  if (value && typeof value === "object") {
    const keys = Object.keys(value).sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    return "{" + keys.map((k) => JSON.stringify(k.normalize("NFC")) + ":" + canonical(value[k])).join(",") + "}";
  }
  if (typeof value === "string") return JSON.stringify(value.normalize("NFC"));
  return JSON.stringify(value);
}

// Source text with comments blanked out (string contents kept), so regexes do not match
// commented-out code.
function stripComments(text) {
  let out = "";
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    const n = text[i + 1];
    if (c === "/" && n === "/") {
      while (i < text.length && text[i] !== "\n") i++;
    } else if (c === "/" && n === "*") {
      i += 2;
      while (i < text.length && !(text[i] === "*" && text[i + 1] === "/")) i++;
      i += 2;
    } else if (c === '"' || c === "'" || c === "`") {
      const q = c;
      out += c;
      i++;
      while (i < text.length && text[i] !== q) {
        if (text[i] === "\\") {
          out += text[i];
          i++;
        }
        out += text[i] ?? "";
        i++;
      }
      out += q;
      i++;
    } else {
      out += c;
      i++;
    }
  }
  return out;
}

const IMPORT_RE = /(?:^|[^.\w$])(?:import|export)\s+(?:[^'"]*?\sfrom\s*)?["']([^"']+)["']|(?:^|[^.\w$])(?:import|require)\s*\(\s*["']([^"']+)["']\s*\)/g;

function imports(text) {
  const out = [];
  for (const m of stripComments(text).matchAll(IMPORT_RE)) out.push(m[1] ?? m[2]);
  return out;
}

// Resolves a relative or @/ specifier from a repository-relative file to an existing file.
function resolveLocal(workdir, fromFile, specifier) {
  let base;
  if (specifier.startsWith("./") || specifier.startsWith("../")) {
    base = path.posix.normalize(path.posix.join(path.posix.dirname(fromFile), specifier));
  } else if (specifier.startsWith("@/")) {
    base = "src/" + specifier.slice(2);
  } else {
    return null;
  }
  const candidates = [base];
  for (const ext of ["ts", "tsx", "js", "jsx", "mjs", "cjs"]) candidates.push(`${base}.${ext}`);
  for (const ext of ["ts", "tsx", "js", "jsx"]) candidates.push(`${base}/index.${ext}`);
  return candidates.find((c) => fs.existsSync(path.join(workdir, c)) && fs.statSync(path.join(workdir, c)).isFile()) ?? null;
}

module.exports = { readStdin, sha256, canonical, stripComments, imports, resolveLocal };
