#!/usr/bin/env node
"use strict";
// Runs the native binary with our arguments, stdio and exit status.

const { spawnSync } = require("node:child_process");
const { binaryPath } = require("../lib/index.js");

let bin;
try {
  bin = binaryPath();
} catch (err) {
  console.error(err.message);
  process.exit(1);
}
const result = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  console.error(`sdc: cannot run ${bin}: ${result.error.message}`);
  process.exit(1);
}
if (result.signal) {
  process.kill(process.pid, result.signal);
}
process.exit(result.status ?? 1);
