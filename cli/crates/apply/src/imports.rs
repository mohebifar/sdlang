//! Import and export scanning for TypeScript and JavaScript files, used by `sdc import` to
//! propose edges from the imports adopted code already has. Imports are not restricted:
//! edges are hints for planning and navigation (docs/spec.md, Edges and relations).

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

pub const CODE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "mts", "cts"];

/// A token of JavaScript or TypeScript source, as far as import and export scanning
/// needs: comments are dropped and template literals with substitutions are skipped.
#[derive(PartialEq)]
enum Tok {
    Word(String),
    Str(String),
    Punct(char),
}

fn tokens(text: &str) -> Vec<Tok> {
    let chars: Vec<char> = text.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            let mut s = String::new();
            i += 1;
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1;
                }
                if i < chars.len() {
                    s.push(chars[i]);
                }
                i += 1;
            }
            i += 1;
            // Template literals with substitutions are never specifiers we can check.
            if quote != '`' || !s.contains("${") {
                toks.push(Tok::Str(s));
            }
        } else if c.is_alphanumeric() || c == '_' || c == '$' {
            let mut w = String::new();
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                w.push(chars[i]);
                i += 1;
            }
            toks.push(Tok::Word(w));
        } else {
            if !c.is_whitespace() {
                toks.push(Tok::Punct(c));
            }
            i += 1;
        }
    }
    toks
}

/// Import specifiers in source text: `import ... from "x"`, `import "x"`, `export ... from
/// "x"`, `import("x")` and `require("x")`. Comments and other strings are skipped.
pub fn specifiers(text: &str) -> Vec<String> {
    let toks = tokens(text);
    let mut out = Vec::new();
    for (k, t) in toks.iter().enumerate() {
        let word = |s: &str| matches!(t, Tok::Word(w) if w == s);
        let next = toks.get(k + 1);
        let after = toks.get(k + 2);
        // A member access such as `foo.import(` or `x.from "..."` is not a keyword use.
        let after_dot = k > 0 && toks[k - 1] == Tok::Punct('.');
        if after_dot {
            continue;
        }
        if word("from") || word("import") {
            if let Some(Tok::Str(s)) = next {
                out.push(s.clone());
                continue;
            }
        }
        if word("import") || word("require") {
            if let (Some(Tok::Punct('(')), Some(Tok::Str(s))) = (next, after) {
                out.push(s.clone());
            }
        }
    }
    out
}

/// Names a module exports: `export default` as `default`; `export function`, `class`,
/// `const`, `let`, `var`, `enum`, `type` and `interface` declarations; and the exported
/// names of `export { a, b as c }` lists, in source order. `export *` adds nothing it can
/// name, and a declaration list names only its first binding.
pub fn exports(text: &str) -> Vec<String> {
    let toks = tokens(text);
    let word = |k: usize| match toks.get(k) {
        Some(Tok::Word(w)) => Some(w.as_str()),
        _ => None,
    };
    let mut out = Vec::new();
    let mut k = 0;
    while k < toks.len() {
        let after_dot = k > 0 && toks[k - 1] == Tok::Punct('.');
        if after_dot || word(k) != Some("export") {
            k += 1;
            continue;
        }
        let mut j = k + 1;
        while matches!(word(j), Some("declare" | "async" | "abstract")) {
            j += 1;
        }
        if word(j) == Some("type") && toks.get(j + 1) == Some(&Tok::Punct('{')) {
            j += 1;
        }
        match (word(j), toks.get(j)) {
            (Some("default"), _) => out.push("default".to_string()),
            (
                Some(
                    "function" | "class" | "const" | "let" | "var" | "enum" | "type" | "interface"
                    | "namespace",
                ),
                _,
            ) => {
                let mut n = j + 1;
                if toks.get(n) == Some(&Tok::Punct('*')) {
                    n += 1;
                }
                if let Some(name) = word(n) {
                    out.push(name.to_string());
                }
            }
            (_, Some(Tok::Punct('{'))) => {
                let mut n = j + 1;
                let mut last: Option<&str> = None;
                while n < toks.len() && toks[n] != Tok::Punct('}') {
                    match &toks[n] {
                        Tok::Punct(',') => {
                            if let Some(l) = last.take() {
                                out.push(l.to_string());
                            }
                        }
                        Tok::Word(w) if w != "as" && w != "type" => last = Some(w),
                        _ => {}
                    }
                    n += 1;
                }
                if let Some(l) = last {
                    out.push(l.to_string());
                }
                j = n;
            }
            _ => {}
        }
        k = j + 1;
    }
    let mut seen = BTreeSet::new();
    out.retain(|n| seen.insert(n.clone()));
    out
}

/// Where a specifier in `file` points inside the repository: `None` for a bare package
/// import, `Some(Err)` when it leaves the repository or resolves to no file.
pub fn resolve_local(
    dir: &Path,
    aliases: &[(String, String)],
    file: &str,
    spec: &str,
) -> Option<Result<String, &'static str>> {
    let base = file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let local = if spec.starts_with("./") || spec.starts_with("../") {
        normalise(&format!("{base}/{spec}"))
    } else if let Some((prefix, target)) =
        aliases.iter().find(|(p, _)| spec.starts_with(p.as_str()))
    {
        normalise(&format!("{target}{}", &spec[prefix.len()..]))
    } else if spec.starts_with('/') {
        None
    } else {
        return None;
    };
    let Some(local) = local else {
        return Some(Err("leaves the repository"));
    };
    Some(resolve_file(dir, &local).ok_or("does not resolve to a file"))
}

/// The npm package a bare specifier names: `@scope/name` or `name`.
pub fn npm_package(specifier: &str) -> String {
    let parts: Vec<&str> = specifier.split('/').collect();
    if specifier.starts_with('@') && parts.len() > 1 {
        format!("{}/{}", parts[0], parts[1])
    } else {
        parts[0].to_string()
    }
}

fn normalise(path: &str) -> Option<String> {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            s => out.push(s),
        }
    }
    Some(out.join("/"))
}

/// Resolves a repository-relative module path to a file in `dir`, as TypeScript would.
fn resolve_file(dir: &Path, path: &str) -> Option<String> {
    let mut candidates = vec![path.to_string()];
    for ext in CODE_EXTENSIONS.iter().chain(["json"].iter()) {
        candidates.push(format!("{path}.{ext}"));
    }
    for ext in ["ts", "tsx", "js", "jsx"] {
        candidates.push(format!("{path}/index.{ext}"));
    }
    // `./x.js` written for `./x.ts` under ESM resolution.
    if let Some(stem) = path.strip_suffix(".js") {
        candidates.push(format!("{stem}.ts"));
        candidates.push(format!("{stem}.tsx"));
    }
    candidates.into_iter().find(|c| dir.join(c).is_file())
}

/// Path aliases from `tsconfig.json` (`compilerOptions.paths` entries ending in `/*`).
pub fn aliases(dir: &Path) -> Vec<(String, String)> {
    let Ok(text) = fs::read_to_string(dir.join("tsconfig.json")) else {
        return vec![];
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return vec![];
    };
    let mut out = Vec::new();
    for (k, targets) in v["compilerOptions"]["paths"]
        .as_object()
        .into_iter()
        .flatten()
    {
        let (Some(prefix), Some(target)) = (
            k.strip_suffix('*'),
            targets[0].as_str().and_then(|t| t.strip_suffix('*')),
        ) else {
            continue;
        };
        out.push((
            prefix.to_string(),
            normalise(target)
                .map(|t| if t.is_empty() { t } else { format!("{t}/") })
                .unwrap_or_default(),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_names() {
        let src = r#"
            export default function Page() {}
            export async function signIn() {}
            export const requestReset = 1, other = 2;
            export type Input = { a: string };
            export interface Shape {}
            export { a, b as c } from "./x";
            export type { T };
            // export function commented() {}
            const s = "export function inString() {}";
            obj.export = 1;
            export * from "./y";
        "#;
        assert_eq!(
            exports(src),
            [
                "default",
                "signIn",
                "requestReset",
                "Input",
                "Shape",
                "a",
                "c",
                "T"
            ]
        );
    }

    #[test]
    fn scanning() {
        let src = r#"
            import React from "react";
            import type { Row } from '@/db/schema/Workspace';
            import "./styles.css";
            export { x } from "../lib/x";
            const m = await import("next/navigation");
            const fs = require("fs");
            // import nope from "commented";
            /* import "block" */
            const s = "from 'not-an-import'";
            obj.import("member");
        "#;
        assert_eq!(
            specifiers(src),
            [
                "react",
                "@/db/schema/Workspace",
                "./styles.css",
                "../lib/x",
                "next/navigation",
                "fs"
            ]
        );
    }
}
