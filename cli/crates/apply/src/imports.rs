//! The allowed-import rule for TypeScript and JavaScript outputs (SPEC.md, Verification ->
//! Conformance): code owned by a declaration may import only its own outputs, the exported
//! outputs of what its edges reach, packages its edges reach, and its kind's exact ambient
//! imports. Resolution is to file level; export-level checks are left to the typechecker.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use intlang_graph::loader::Project;
use intlang_plan::materialise::{kind_body, Materialisation};
use intlang_syntax::ast::{Expr, MemberValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What one declaration's code may import.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPolicy {
    pub own: BTreeSet<String>,
    /// Exported outputs of edge targets.
    pub granted: BTreeSet<String>,
    pub ambient: BTreeSet<String>,
    /// npm package names reachable over edges.
    pub packages: BTreeSet<String>,
    /// Path aliases from tsconfig.json, e.g. `("@/", "src/")`.
    pub aliases: Vec<(String, String)>,
}

pub const CODE_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs", "mts", "cts"];

/// Import specifiers in source text: `import ... from "x"`, `import "x"`, `export ... from
/// "x"`, `import("x")` and `require("x")`. Comments and other strings are skipped.
pub fn specifiers(text: &str) -> Vec<String> {
    #[derive(PartialEq)]
    enum Tok {
        Word(String),
        Str(String),
        Punct(char),
    }
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

fn npm_package(specifier: &str) -> String {
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

/// The policy of `address`: its own outputs, what its edges grant and its kind's ambient
/// imports.
pub fn policy_for(
    project: &Project,
    graph: &Value,
    mats: &BTreeMap<String, Materialisation>,
    own: BTreeSet<String>,
    address: &str,
    dir: &Path,
) -> ImportPolicy {
    let decls: BTreeMap<&str, &Value> = graph["declarations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| Some((d["address"].as_str()?, d)))
        .collect();
    let mut p = ImportPolicy {
        own,
        aliases: aliases(dir),
        ..ImportPolicy::default()
    };
    let Some(decl) = decls.get(address) else {
        return p;
    };
    if let Some(body) = kind_body(project, decl) {
        for m in &body.members {
            if let (true, MemberValue::Section(s)) = (m.name.text == "ambient", &m.value) {
                for f in &s.members {
                    if let MemberValue::Field(field) = &f.value {
                        if let Expr::List { items, .. } = &field.value {
                            for i in items {
                                if let Expr::String(lit) = i {
                                    p.ambient.insert(intlang_graph::scope::lit_text(lit));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for e in decl["edges"].as_array().into_iter().flatten() {
        if e["relation"] == "mentions" {
            continue;
        }
        let Some(target) = e["target"].as_str() else {
            continue;
        };
        if let Some((package, _)) = target.split_once("::") {
            if let Some(name) = package.strip_prefix("npm:") {
                p.packages.insert(name.to_string());
            }
            continue;
        }
        let Some(t) = decls.get(target) else { continue };
        if t["kind"] == "core:use" {
            if let Some(name) = t["fields"]["package"].as_str() {
                p.packages.insert(name.to_string());
            }
            continue;
        }
        let Some(m) = mats.get(target) else { continue };
        if t["kind"] == "core:lib" {
            p.granted.extend(m.slots.get("main").cloned());
        }
        if let Some(body) = kind_body(project, t) {
            for member in &body.members {
                if let (true, MemberValue::Section(s)) =
                    (member.name.text == "exports", &member.value)
                {
                    for entry in &s.members {
                        let MemberValue::Section(es) = &entry.value else {
                            continue;
                        };
                        for f in &es.members {
                            if let (true, MemberValue::Field(x)) =
                                (f.name.text == "output", &f.value)
                            {
                                if let Expr::String(lit) = &x.value {
                                    p.granted.extend(
                                        m.slots.get(&intlang_graph::scope::lit_text(lit)).cloned(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    p
}

/// Checks the imports of `files` (repository-relative) in `dir` against `policy`, returning
/// one message per violation.
pub fn check_imports(policy: &ImportPolicy, dir: &Path, files: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for file in files {
        let ext = file.rsplit('.').next().unwrap_or_default();
        if !CODE_EXTENSIONS.contains(&ext) {
            continue;
        }
        let Ok(text) = fs::read_to_string(dir.join(file)) else {
            continue;
        };
        let base = file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
        for spec in specifiers(&text) {
            let local = if spec.starts_with("./") || spec.starts_with("../") {
                normalise(&format!("{base}/{spec}"))
            } else if let Some((prefix, target)) = policy
                .aliases
                .iter()
                .find(|(p, _)| spec.starts_with(p.as_str()))
            {
                normalise(&format!("{target}{}", &spec[prefix.len()..]))
            } else if spec.starts_with('/') {
                None
            } else {
                if !(policy.ambient.contains(&spec)
                    || policy.packages.contains(&npm_package(&spec)))
                {
                    out.push(format!(
                        "{file} imports '{spec}', which no edge or ambient import grants"
                    ));
                }
                continue;
            };
            let Some(local) = local else {
                out.push(format!(
                    "{file} imports '{spec}', which leaves the repository"
                ));
                continue;
            };
            match resolve_file(dir, &local) {
                Some(resolved) if policy.own.contains(&resolved) || policy.granted.contains(&resolved) => {}
                Some(resolved) => out.push(format!("{file} imports '{spec}' ({resolved}), which is neither its own output nor exported by an edge target")),
                None => out.push(format!("{file} imports '{spec}', which does not resolve to a file")),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn rules() {
        let dir = std::env::temp_dir().join(format!("intlang-imports-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for f in [
            "src/app/login/page.tsx",
            "src/app/login/_parts/Form.tsx",
            "src/server/auth/sign_in.ts",
            "src/lib/secret.ts",
        ] {
            fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
            fs::write(dir.join(f), "").unwrap();
        }
        fs::write(
            dir.join("src/app/login/page.tsx"),
            r#"import Link from "next/link";
               import { Form } from "./_parts/Form";
               import signIn from "@/server/auth/sign_in";
               import { Button } from "ui-sdk";
               import secret from "../../lib/secret";
               import fs from "node:fs";"#,
        )
        .unwrap();
        let policy = ImportPolicy {
            own: ["src/app/login/page.tsx", "src/app/login/_parts/Form.tsx"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            granted: ["src/server/auth/sign_in.ts".to_string()].into(),
            ambient: ["next/link".to_string()].into(),
            packages: ["ui-sdk".to_string()].into(),
            aliases: vec![("@/".into(), "src/".into())],
        };
        let v = check_imports(&policy, &dir, &["src/app/login/page.tsx".into()]);
        assert_eq!(v.len(), 2, "{v:?}");
        assert!(
            v[0].contains("../../lib/secret") && v[1].contains("node:fs"),
            "{v:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
