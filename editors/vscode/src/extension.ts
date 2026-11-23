// Starts `sdc lsp` for .sd files. Everything language-aware lives in the server; this
// file only finds the binary and wires the client.

import * as fs from "fs";
import { createRequire } from "module";
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

/**
 * The native binary that the project's `@sdlang/cli` installed for this platform, so the
 * editor runs the same version as `npx sdc`. Resolving from the real location of
 * `@sdlang/cli` finds the platform package under npm, pnpm and Yarn's node_modules layout.
 */
function projectBinary(folder: string, exe: string): string | undefined {
  try {
    const fromProject = createRequire(path.join(folder, "package.json"));
    const cli = fs.realpathSync(fromProject.resolve("@sdlang/cli/package.json"));
    const pkg = `@sdlang/cli-${process.platform}-${process.arch}`;
    const bin = createRequire(cli).resolve(`${pkg}/bin/${exe}`);
    return fs.existsSync(bin) ? bin : undefined;
  } catch {
    return undefined;
  }
}

/**
 * The `sdc` binary: the setting, the project's `@sdlang/cli`, a build of this repository's
 * CLI in the workspace, then `sdc` on PATH.
 */
function serverCommand(): string {
  const configured = vscode.workspace.getConfiguration("sdlang").get<string>("server.path");
  if (configured) {
    return configured;
  }
  const exe = process.platform === "win32" ? "sdc.exe" : "sdc";
  const folders = (vscode.workspace.workspaceFolders ?? []).map((f) => f.uri.fsPath);
  for (const folder of folders) {
    const bin = projectBinary(folder, exe);
    if (bin) {
      return bin;
    }
  }
  for (const folder of folders) {
    for (const profile of ["release", "debug"]) {
      const candidate = path.join(folder, "cli", "target", profile, exe);
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  }
  return exe;
}

async function start(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration("sdlang");
  const command = serverCommand();
  const serverOptions: ServerOptions = {
    command,
    args: config.get<string[]>("server.args") ?? ["lsp"],
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "sdlang" },
      { scheme: "untitled", language: "sdlang" },
    ],
  };
  client = new LanguageClient("sdlang", "sdlang", serverOptions, clientOptions);
  try {
    await client.start();
  } catch (err) {
    client = undefined;
    const choice = await vscode.window.showErrorMessage(
      `sdlang: could not start the language server (${command} lsp). Install it in the project with \`npm install --save-dev @sdlang/cli\`, put \`sdc\` on PATH, or set sdlang.server.path.`,
      "Open Settings",
    );
    if (choice === "Open Settings") {
      await vscode.commands.executeCommand("workbench.action.openSettings", "sdlang.server");
    }
  }
}

async function stop(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("sdlang.restartServer", async () => {
      await stop();
      await start(context);
    }),
    vscode.workspace.onDidChangeConfiguration(async (e) => {
      if (e.affectsConfiguration("sdlang.server")) {
        await stop();
        await start(context);
      }
    }),
  );
  await start(context);
}

export async function deactivate(): Promise<void> {
  await stop();
}
