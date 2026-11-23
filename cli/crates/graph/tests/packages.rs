//! Packages: a directory with a `package.sd` header, found through `sd.lock`.

use std::fs;
use std::path::{Path, PathBuf};

use sdlang_graph::loader;

/// Writes `files` (paths relative to the project root) into a fresh project.
fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("packages")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    root
}

const LOCK: &str = r#"{ "packages": [{ "id": "npm:ui-sdk", "path": "pkgs/ui" }] }"#;
const ENTRY: &str = "export type Button = string\n";

fn load_error(root: &Path) -> String {
    loader::load(root).unwrap_err().to_string()
}

#[test]
fn header_names_the_package_and_its_files() {
    let root = project(
        "header",
        &[
            ("spec/app.sd", "import ui_sdk\n"),
            ("sd.lock", LOCK),
            (
                "pkgs/ui/package.sd",
                "package ui_sdk {\n  entry: \"./src/main.sd\"\n  skills {\n    style: \"./skills/style.md\"\n  }\n  provides {\n    ui {\n      button: Button\n    }\n  }\n}\n",
            ),
            ("pkgs/ui/src/main.sd", ENTRY),
        ],
    );
    let project = loader::load(&root).unwrap();
    let p = &project.packages[0];
    assert_eq!(p.id, "npm:ui-sdk");
    assert_eq!(p.source_name, "ui-sdk");
    assert_eq!(p.import_name, "ui_sdk");
    assert_eq!(p.skills["style"], "./skills/style.md");
    assert_eq!(p.provides["ui"]["button"], "Button");
    assert!(p.files[0].path.ends_with("src/main.sd"));
}

#[test]
fn entry_defaults_to_spec_index() {
    let root = project(
        "default-entry",
        &[
            ("spec/app.sd", "import ui_sdk\n"),
            ("sd.lock", LOCK),
            ("pkgs/ui/package.sd", "package ui_sdk {}\n"),
            ("pkgs/ui/spec/index.sd", ENTRY),
        ],
    );
    let project = loader::load(&root).unwrap();
    assert!(project.packages[0].files[0].path.ends_with("spec/index.sd"));
}

#[test]
fn package_sd_holds_only_the_header() {
    let root = project(
        "extra-statement",
        &[
            ("spec/app.sd", "\n"),
            ("sd.lock", LOCK),
            (
                "pkgs/ui/package.sd",
                "package ui_sdk {}\ntype Extra = string\n",
            ),
            ("pkgs/ui/spec/index.sd", ENTRY),
        ],
    );
    assert!(load_error(&root).contains("one package header and nothing else"));
}

#[test]
fn unknown_header_fields_are_errors() {
    let root = project(
        "unknown-field",
        &[
            ("spec/app.sd", "\n"),
            ("sd.lock", LOCK),
            (
                "pkgs/ui/package.sd",
                "package ui_sdk {\n  version: \"1.0.0\"\n}\n",
            ),
            ("pkgs/ui/spec/index.sd", ENTRY),
        ],
    );
    assert!(load_error(&root).contains("unknown package field `version`"));
}

#[test]
fn a_missing_package_sd_is_reported() {
    let root = project(
        "missing",
        &[
            ("spec/app.sd", "\n"),
            ("sd.lock", LOCK),
            ("pkgs/ui/spec/index.sd", ENTRY),
        ],
    );
    assert!(load_error(&root).contains("package.sd"));
}

#[test]
fn a_package_header_under_spec_is_rejected() {
    let root = project("under-spec", &[("spec/app.sd", "package oops {}\n")]);
    let err = sdlang_graph::build(&root).unwrap_err().to_string();
    assert!(err.contains("belongs in a package's package.sd"), "{err}");
}
