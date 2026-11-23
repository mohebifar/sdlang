#!/usr/bin/env node
// Fills in the installers and the Homebrew formula for one release.
//
// node packaging/render.mjs --tag v0.1.0 --repository owner/name --archives <dir> --out <dir>
//
// <dir> holds the release archives and their .sha256 files.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = path.dirname(fileURLToPath(import.meta.url));
const { values: args } = parseArgs({
  options: {
    tag: { type: "string" },
    repository: { type: "string", default: process.env.GITHUB_REPOSITORY ?? "" },
    archives: { type: "string" },
    out: { type: "string" },
  },
});
for (const name of ["tag", "repository", "archives", "out"]) {
  if (!args[name]) {
    console.error(`missing --${name}`);
    process.exit(2);
  }
}

const sums = {};
for (const file of fs.readdirSync(args.archives)) {
  const m = /^sdc-(.+)\.(tar\.gz|zip)\.sha256$/.exec(file);
  if (m) {
    sums[m[1]] = fs.readFileSync(path.join(args.archives, file), "utf8").split(/\s+/)[0];
  }
}

function fill(text) {
  const out = text
    .replaceAll("__REPO__", args.repository)
    .replaceAll("__TAG__", args.tag)
    .replaceAll("__VERSION__", args.tag.replace(/^v/, ""))
    .replace(/__SHA256:([a-z0-9_-]+)__/g, (_, target) => {
      if (!sums[target]) {
        throw new Error(`no checksum for ${target} in ${args.archives}`);
      }
      return sums[target];
    });
  const left = /__[A-Z0-9_:a-z-]+__/.exec(out);
  if (left) {
    throw new Error(`unfilled placeholder ${left[0]}`);
  }
  return out;
}

fs.mkdirSync(args.out, { recursive: true });
for (const [from, to] of [
  ["install.sh", "sdc-installer.sh"],
  ["install.ps1", "sdc-installer.ps1"],
  ["sdc.rb", "sdc.rb"],
]) {
  fs.writeFileSync(path.join(args.out, to), fill(fs.readFileSync(path.join(here, from), "utf8")));
}
console.log(`rendered installers and formula for ${args.tag} in ${args.out}`);
