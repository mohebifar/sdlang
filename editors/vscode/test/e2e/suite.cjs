// Inside VS Code: open a spec file and check that the server answers through the client.
const assert = require("node:assert/strict");
const path = require("node:path");
const vscode = require("vscode");

async function until(what, f, ms = 20000) {
  const end = Date.now() + ms;
  for (;;) {
    const v = await f();
    if (v) return v;
    if (Date.now() > end) throw new Error(`timed out waiting for ${what}`);
    await new Promise((r) => setTimeout(r, 200));
  }
}

exports.run = async function run() {
  const root = vscode.workspace.workspaceFolders[0].uri.fsPath;
  const file = vscode.Uri.file(path.join(root, "cli/spec/apply/module.sd"));
  const doc = await vscode.workspace.openTextDocument(file);
  await vscode.window.showTextDocument(doc);
  assert.equal(doc.languageId, "sdlang");
  const text = doc.getText();
  const at = (needle, delta) => doc.positionAt(text.indexOf(needle) + delta);

  const defs = await until("definition", async () => {
    const d = await vscode.commands.executeCommand("vscode.executeDefinitionProvider", file, at("[workspace,", 3));
    return d && d.length ? d : null;
  });
  const target = defs[0].targetUri ?? defs[0].uri;
  const range = defs[0].targetSelectionRange ?? defs[0].targetRange ?? defs[0].range;
  assert.equal(target.fsPath, file.fsPath);
  assert.equal(doc.getText(range), "workspace");

  const hovers = await vscode.commands.executeCommand("vscode.executeHoverProvider", file, at("[workspace,", 3));
  const hover = hovers.flatMap((h) => h.contents).map((c) => c.value ?? c).join("\n");
  assert.ok(hover.includes("apply.workspace"), hover);

  const refs = await vscode.commands.executeCommand("vscode.executeReferenceProvider", file, at("[workspace,", 3));
  assert.ok(refs.length >= 2, `references: ${refs.length}`);

  const symbols = await vscode.commands.executeCommand("vscode.executeDocumentSymbolProvider", file);
  assert.ok(symbols.some((s) => s.name === "engine"), symbols.map((s) => s.name).join(", "));

  const legend = await vscode.commands.executeCommand("vscode.provideDocumentSemanticTokensLegend", file);
  assert.ok(legend && legend.tokenTypes.includes("namespace"));

  // A broken edit shows a diagnostic; undoing it clears it.
  const editor = vscode.window.activeTextEditor;
  await editor.edit((e) => e.insert(at("uses: [workspace", 7), "nope_"));
  const diags = await until("a diagnostic", () => {
    const d = vscode.languages.getDiagnostics(file);
    return d.length ? d : null;
  });
  assert.ok(diags[0].message.includes("nope_workspace"), diags[0].message);
  await vscode.commands.executeCommand("undo");
  await until("diagnostics to clear", () => vscode.languages.getDiagnostics(file).length === 0);
  console.log("e2e: definition, hover, references, symbols, semantic tokens and diagnostics work in VS Code");
};
