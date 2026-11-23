#!/usr/bin/env node
// Assembles the npm packages for one release from the built binaries:
//   <out>/cli-<platform>-<arch>/  one package per platform, holding bin/sdc[.exe]
//   <out>/cli/                    @sdlang/cli, which depends on all of them as optional
//
// node packaging/npm/build.mjs --version 0.1.0 --binaries <dir> --out <dir> [--repository owner/name] [--allow-missing]
//
// <dir> holds one folder per Rust target, each with the sdc binary: <dir>/aarch64-apple-darwin/sdc.
// The repository defaults to $GITHUB_REPOSITORY; npm provenance needs it to match the
// repository that built the release.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = path.dirname(fileURLToPath(import.meta.url));

/** Rust target to npm's os and cpu. */
const TARGETS = {
  "aarch64-apple-darwin": { os: "darwin", cpu: "arm64" },
  "x86_64-apple-darwin": { os: "darwin", cpu: "x64" },
  "aarch64-unknown-linux-musl": { os: "linux", cpu: "arm64" },
  "x86_64-unknown-linux-musl": { os: "linux", cpu: "x64" },
  "aarch64-pc-windows-msvc": { os: "win32", cpu: "arm64" },
  "x86_64-pc-windows-msvc": { os: "win32", cpu: "x64" },
};

const { values: args } = parseArgs({
  options: {
    version: { type: "string" },
    binaries: { type: "string" },
    out: { type: "string" },
    repository: { type: "string", default: process.env.GITHUB_REPOSITORY ?? "" },
    "allow-missing": { type: "boolean", default: false },
  },
});
for (const name of ["version", "binaries", "out"]) {
  if (!args[name]) {
    console.error(`missing --${name}`);
    process.exit(2);
  }
}
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(args.version)) {
  console.error(`not a version: ${args.version}`);
  process.exit(2);
}

const common = {
  version: args.version,
  license: "MIT OR Apache-2.0",
  ...(args.repository && {
    homepage: `https://github.com/${args.repository}#readme`,
    repository: { type: "git", url: `git+https://github.com/${args.repository}.git` },
    bugs: { url: `https://github.com/${args.repository}/issues` },
  }),
};

fs.rmSync(args.out, { recursive: true, force: true });
const optional = {};
for (const [target, { os, cpu }] of Object.entries(TARGETS)) {
  const exe = os === "win32" ? "sdc.exe" : "sdc";
  const from = path.join(args.binaries, target, exe);
  if (!fs.existsSync(from)) {
    if (args["allow-missing"]) {
      console.warn(`skipping ${target}: no ${from}`);
      continue;
    }
    console.error(`missing ${from}`);
    process.exit(1);
  }
  const name = `@sdlang/cli-${os}-${cpu}`;
  const dir = path.join(args.out, `cli-${os}-${cpu}`);
  fs.mkdirSync(path.join(dir, "bin"), { recursive: true });
  fs.copyFileSync(from, path.join(dir, "bin", exe));
  fs.chmodSync(path.join(dir, "bin", exe), 0o755);
  const manifest = {
    name,
    description: `The sdc binary for ${os} ${cpu}; install @sdlang/cli instead`,
    ...common,
    os: [os],
    cpu: [cpu],
    files: ["bin/"],
    preferUnplugged: true,
  };
  fs.writeFileSync(path.join(dir, "package.json"), JSON.stringify(manifest, null, 2) + "\n");
  fs.writeFileSync(path.join(dir, "README.md"), `# ${name}\n\nThe \`sdc\` binary for ${os} ${cpu}. Install [@sdlang/cli](https://www.npmjs.com/package/@sdlang/cli) instead.\n`);
  optional[name] = args.version;
}

const cli = path.join(args.out, "cli");
fs.cpSync(path.join(here, "cli"), cli, { recursive: true });
const manifest = JSON.parse(fs.readFileSync(path.join(cli, "package.json"), "utf8"));
Object.assign(manifest, common, { optionalDependencies: optional });
fs.writeFileSync(path.join(cli, "package.json"), JSON.stringify(manifest, null, 2) + "\n");
const readme = path.join(cli, "README.md");
const home = args.repository ? `https://github.com/${args.repository}` : "https://www.npmjs.com/package/@sdlang/cli";
fs.writeFileSync(readme, fs.readFileSync(readme, "utf8").replaceAll("{{homepage}}", home));
for (const licence of ["LICENSE-MIT", "LICENSE-APACHE"]) {
  for (const dir of fs.readdirSync(args.out)) {
    fs.copyFileSync(path.join(here, "..", "..", licence), path.join(args.out, dir, licence));
  }
}
console.log(`built ${Object.keys(optional).length} platform packages and @sdlang/cli ${args.version} in ${args.out}`);
