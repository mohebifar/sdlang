# Editor support

`sdc lsp` is a language server for sdlang: run it, speak the [language server
protocol](https://microsoft.github.io/language-server-protocol/) on stdin and stdout.
Any editor with an LSP client can use it. It provides:

| Feature | LSP |
| --- | --- |
| Syntax errors and the compiler's first error for the project | `publishDiagnostics` |
| Go to definition, across modules, packages, capabilities and `@{mentions}` | `definition` |
| Find references, highlight uses | `references`, `documentHighlight` |
| Hover: declarations, fields, keywords, builtin types, annotations, requirements | `hover` |
| Completion: fields, names in scope, `module.` and `package.` members, annotations | `completion` |
| Rename a declaration, adding a `moved` statement | `prepareRename`, `rename` |
| Outline and workspace search | `documentSymbol`, `workspace/symbol` |
| Folding of blocks, records, lists, prose, imports and comment runs | `foldingRange` |
| Kinds, types, declarations and modules told apart | `semanticTokens/full` |

Build it with `cargo build --release` in `cli/`; the binary is `cli/target/release/sdc`.
A project is the nearest directory above a file that has a `spec/` directory and is not
itself a package. A file outside any project gets syntax errors, an outline and folding.
While a buffer does not parse, the rest of the project keeps navigating from its last
text that did.

## VS Code

See [vscode/README.md](vscode/README.md): a grammar, the language configuration and a
client that starts `sdc lsp`.

## Neovim (0.11 or later)

```lua
vim.filetype.add({ extension = { sd = "sdlang" } })
vim.lsp.config("sdlang", {
  cmd = { "sdc", "lsp" },
  filetypes = { "sdlang" },
  root_markers = { "sd.lock", "spec" },
})
vim.lsp.enable("sdlang")
```

Neovim applies the server's semantic tokens, which highlight names; there is no
tree-sitter grammar yet, so strings, comments and keywords are not highlighted.

## Helix

In `languages.toml`:

```toml
[language-server.sdc]
command = "sdc"
args = ["lsp"]

[[language]]
name = "sdlang"
scope = "source.sdlang"
file-types = ["sd"]
roots = ["sd.lock"]
comment-token = "//"
indent = { tab-width = 2, unit = "  " }
language-servers = ["sdc"]
```

Helix highlights with tree-sitter only, so you get the language features without colours.

## Emacs (eglot)

```elisp
(define-derived-mode sdlang-mode prog-mode "sdlang"
  (setq-local comment-start "// "))
(add-to-list 'auto-mode-alist '("\\.sd\\'" . sdlang-mode))
(add-to-list 'eglot-server-programs '(sdlang-mode "sdc" "lsp"))
```

## Sublime Text (LSP package)

In LSP's settings, with a syntax that sets `source.sdlang` (or convert
`vscode/syntaxes/sdlang.tmLanguage.json` with PackageDev):

```json
{
  "clients": {
    "sdlang": {
      "enabled": true,
      "command": ["sdc", "lsp"],
      "selector": "source.sdlang"
    }
  }
}
```

The TextMate grammar in `vscode/syntaxes/` also works in other editors that read
TextMate grammars.
