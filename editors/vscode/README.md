# sdlang for VS Code

Highlighting and language support for sdlang specs (`.sd` files).

- **Highlighting** from a TextMate grammar, refined by the server's semantic tokens (which
  names are kinds, types, declarations, modules). Fenced ` ```spec ` and ` ```sdlang `
  blocks in Markdown are highlighted too.
- **Diagnostics**: syntax errors as you type, and the compiler's first error for the
  project (an unbound name, an unknown field, a cycle).
- **Navigation**: go to definition across modules, packages and `@{mentions}` in prose;
  find references; highlight a name's uses; document outline; workspace symbol search.
- **Hover**: a declaration's kind, address, summary and source; what a field, keyword,
  builtin type or annotation means; a requirement's address and the test tag that covers it.
- **Completion**: a kind's fields at the start of a body line, the names in scope in a
  value, what follows `module.` or `package.`, annotations after `@`, and snippets.
- **Rename** a declaration everywhere it is used, plus a `moved` statement so it keeps
  its identity.

The language features come from `sdc lsp`, the CLI's language server; this extension
starts it.

## Setup

The extension needs `sdc`. Install it in the project, which pins the version:

```bash
npm install --save-dev @sdlang/cli
```

The extension looks for the server in this order: the `sdlang.server.path` setting, the
project's `@sdlang/cli` in `node_modules`, `cli/target/release/sdc` or
`cli/target/debug/sdc` in the workspace (for working on sdlang itself), then `sdc` on
`PATH`. **sdlang: Restart Language Server** restarts it after an upgrade or rebuild.

## From source

```bash
cd cli && cargo build --release
```

```bash
cd editors/vscode && npm install && npm run package
```

```bash
code --install-extension editors/vscode/sdlang-0.1.0.vsix
```

To work on the extension, open `editors/vscode` in VS Code and press F5; the development
window opens the repository. `npm test` checks the grammar's scopes on a sample; `npm run test:e2e` opens the repository in VS Code (an installed copy, in a throwaway profile) and checks definition, hover, references, symbols, semantic tokens and diagnostics through the client.
