// Shared helpers for the adapter's test check: reading protocol input and digests.
"use strict";
const crypto = require("node:crypto");
const fs = require("node:fs");

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

module.exports = { readStdin, sha256, canonical };
