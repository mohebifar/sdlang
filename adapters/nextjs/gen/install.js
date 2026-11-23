// Generator for the stack's toolchain files (generator protocol). v0 does not install
// dependencies itself: an install needs the network and runs package scripts, which the
// generator protocol forbids. It reports the change as unsupported, so the plan is amended
// and a human runs the package manager.
"use strict";
const fs = require("node:fs");

const input = JSON.parse(fs.readFileSync(0, "utf8"));
process.stdout.write(
  JSON.stringify({
    protocol: "sdlang.generator/1",
    dispatch: input.dispatch,
    order_hash: input.binding.order_hash,
    status: "unsupported",
    reason: "dependency installation is not automated in v0; run the package manager and adopt package.json and the lockfile",
  }) + "\n",
);
