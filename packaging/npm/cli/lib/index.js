"use strict";
// Finds the native `sdc` binary that npm installed as an optional dependency for this
// platform. The editor extension uses the same lookup, so editor and CLI agree on the version.

const fs = require("node:fs");

/** `${process.platform}-${process.arch}` to the package that carries its binary. */
const PACKAGES = {
  "darwin-arm64": "@sdlang/cli-darwin-arm64",
  "darwin-x64": "@sdlang/cli-darwin-x64",
  "linux-arm64": "@sdlang/cli-linux-arm64",
  "linux-x64": "@sdlang/cli-linux-x64",
  "win32-arm64": "@sdlang/cli-win32-arm64",
  "win32-x64": "@sdlang/cli-win32-x64",
};

function binaryPath() {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PACKAGES[key];
  if (!pkg) {
    throw new Error(
      `sdc: no prebuilt binary for ${key}. Build it from source instead: cargo install sdlang-cli`,
    );
  }
  const exe = process.platform === "win32" ? "sdc.exe" : "sdc";
  let bin;
  try {
    bin = require.resolve(`${pkg}/bin/${exe}`);
  } catch {
    throw new Error(
      `sdc: ${pkg} is not installed. It is an optional dependency of @sdlang/cli; ` +
        "reinstall without --omit=optional or --no-optional.",
    );
  }
  if (process.platform !== "win32") {
    try {
      fs.accessSync(bin, fs.constants.X_OK);
    } catch {
      try {
        fs.chmodSync(bin, 0o755);
      } catch {
        // Read-only install: spawning reports the real error.
      }
    }
  }
  return bin;
}

module.exports = { binaryPath, PACKAGES };
