# @sdlang/cli

`sdc`, the command-line tool for [sdlang]({{homepage}}): a spec language for code
written by AI agents. This package installs a prebuilt native binary for your platform
through an optional dependency (`@sdlang/cli-<platform>-<arch>`), so installing it per
project pins the version your plans and state were made with.

```bash
npm install --save-dev @sdlang/cli
```

```bash
npx sdc plan . --json
```

Prebuilt for macOS (arm64, x64), Linux (arm64, x64; static, so glibc or musl) and Windows
(arm64, x64). Elsewhere, build from source: `cargo install sdlang-cli`.

Early preview: the language and protocol will change. Dual-licensed under MIT or Apache 2.0.
