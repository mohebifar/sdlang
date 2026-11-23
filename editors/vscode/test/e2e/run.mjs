// Runs test/e2e/suite.cjs inside VS Code with this extension loaded, against the
// repository: `npm run test:e2e`. Uses an installed VS Code if VSCODE_PATH is set (or the
// macOS default exists), in a throwaway profile, so your own settings are not touched.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { runTests } from "@vscode/test-electron";

const here = path.dirname(fileURLToPath(import.meta.url));
const ext = path.resolve(here, "../..");
const repo = path.resolve(ext, "../..");
const profile = fs.mkdtempSync(path.join(os.tmpdir(), "sdlang-vscode-"));
const mac = "/Applications/Visual Studio Code.app/Contents/MacOS/Code";
const vscodeExecutablePath = process.env.VSCODE_PATH || (fs.existsSync(mac) ? mac : undefined);

try {
  await runTests({
    vscodeExecutablePath,
    extensionDevelopmentPath: ext,
    extensionTestsPath: path.join(here, "suite.cjs"),
    launchArgs: [repo, `--user-data-dir=${profile}/data`, `--extensions-dir=${profile}/ext`, "--disable-extensions", "--disable-workspace-trust"],
  });
} finally {
  fs.rmSync(profile, { recursive: true, force: true });
}
