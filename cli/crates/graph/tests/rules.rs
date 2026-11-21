//! Resolution and lowering rules, on small throwaway projects.

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

/// Writes `files` (paths under spec/) into a fresh project and lowers it.
fn build(name: &str, files: &[(&str, &str)]) -> Result<Value, String> {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("rules")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    for (path, text) in files {
        let path = root.join("spec").join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fs::create_dir_all(root.join("spec")).unwrap();
    intlang_graph::build(&root).map_err(|e| e.to_string())
}

fn decl<'a>(graph: &'a Value, address: &str) -> &'a Value {
    graph["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["address"] == address)
        .unwrap()
}

fn fails(name: &str, files: &[(&str, &str)], needle: &str) {
    match build(name, files) {
        Ok(_) => panic!("{name}: expected an error containing {needle:?}"),
        Err(e) => assert!(e.contains(needle), "{name}: {e}"),
    }
}

#[test]
fn modules_and_file_order() {
    let g = build(
        "modules",
        &[
            ("b.int", "import a\n"),
            ("a/x.int", "lib one {\n}\n"),
            ("app.int", "import a\n"),
        ],
    )
    .unwrap();
    let paths: Vec<&str> = g["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths, ["", "a"]);
    assert_eq!(
        g["modules"][0]["imports"],
        json!([{ "alias": "a", "source": "module:a" }]),
        "repeated imports coalesce"
    );
}

#[test]
fn collisions() {
    fails(
        "dup",
        &[("a.int", "lib x {\n}\n"), ("b.int", "lib x {\n}\n")],
        "declared twice",
    );
    fails(
        "import_vs_decl",
        &[
            ("a.int", "import m\nlib m {\n}\n"),
            ("m/x.int", "lib y {\n}\n"),
        ],
        "already bound",
    );
    fails(
        "capability",
        &[("app.int", "use \"zod\" for x\nlib x {\n}\n")],
        "already bound as a capability",
    );
    fails("prelude", &[("app.int", "lib string {\n}\n")], "core name");
}

#[test]
fn walking_and_visibility() {
    let base = ("m/x.int", "lib hidden {\n}\nexport lib shown {\n}\n");
    build(
        "visible",
        &[
            base,
            ("app.int", "import m\nlib user {\n  uses: [m.shown]\n}\n"),
        ],
    )
    .unwrap();
    fails(
        "hidden",
        &[
            base,
            ("app.int", "import m\nlib user {\n  uses: [m.hidden]\n}\n"),
        ],
        "not exported",
    );
    fails(
        "into_decl",
        &[
            base,
            ("app.int", "import m\nlib user {\n  uses: [m.shown.x]\n}\n"),
        ],
        "declarations are not records",
    );
    fails(
        "unbound",
        &[("app.int", "lib user {\n  uses: [nowhere]\n}\n")],
        "not bound here",
    );
    fails(
        "no_module",
        &[("app.int", "import ghost\n")],
        "no local module or locked package",
    );
}

#[test]
fn capabilities_without_a_package_expose_only_their_name() {
    let g = build(
        "bare",
        &[(
            "app.int",
            "use \"pdf-lib\" for pdf\nlib render {\n  uses: [pdf]\n}\n",
        )],
    )
    .unwrap();
    assert_eq!(
        decl(&g, "render")["fields"]["uses"],
        json!([{ "$ref": "pdf" }])
    );
    fails(
        "bare_member",
        &[(
            "app.int",
            "use \"pdf-lib\" for pdf\nlib render {\n  uses: [pdf.page]\n}\n",
        )],
        "exposes only its own name",
    );
}

#[test]
fn closed_schemas() {
    fails(
        "unknown_field",
        &[("a.int", "lib x {\n  colour: \"red\"\n}\n")],
        "has no field 'colour'",
    );
    fails(
        "unknown_section",
        &[("a.int", "lib x {\n  errors {\n    e: \"x\"\n  }\n}\n")],
        "has no section 'errors'",
    );
    fails(
        "wrong_type",
        &[("a.int", "action x {\n  summary: 1\n}\n")],
        "expected a string",
    );
}

#[test]
fn short_forms_lower_like_long_forms() {
    let short = build("short", &[("a.int", "action x {\n  errors {\n    taken: \"Name taken\"\n  }\n  expect {\n    unique: \"no two share a name\"\n  }\n}\n")]).unwrap();
    let long = build("long", &[("a.int", "action x {\n  errors {\n    taken {\n      summary: \"Name taken\"\n      code: \"taken\"\n    }\n  }\n  expect {\n    unique {\n      text: \"no two share a name\"\n    }\n  }\n}\n")]).unwrap();
    assert_eq!(decl(&short, "x")["members"], decl(&long, "x")["members"]);
    assert_eq!(
        decl(&short, "x")["members"]["errors"]["taken"]["code"],
        "taken",
        "code defaults to the key"
    );
}

#[test]
fn prose_and_mentions() {
    let g = build("prose", &[("a.int", "lib x {\n  prompt: \"\"\"\n    See @{y}.   \n    Then @{self}.\n  \"\"\"\n}\nlib y {\n}\n")]).unwrap();
    let x = decl(&g, "x");
    assert_eq!(
        x["fields"]["prompt"],
        json!({ "$prose": "See @{y}.\nThen @{x}.\n" })
    );
    assert_eq!(
        x["edges"],
        json!([
            { "relation": "mentions", "target": "x", "via": "prompt" },
            { "relation": "mentions", "target": "y", "via": "prompt" }
        ])
    );
}

#[test]
fn types_give_edges() {
    let g = build(
        "types",
        &[(
            "a.int",
            "type Name = string\naction x {\n  input: { name: Name, tags: [string]? }\n}\n",
        )],
    )
    .unwrap();
    let x = decl(&g, "x");
    assert_eq!(
        x["edges"],
        json!([{ "relation": "types", "target": "Name", "via": "input" }])
    );
    assert_eq!(
        x["fields"]["input"],
        json!({ "$type": "record", "fields": {
            "name": { "$type": "named", "name": "Name" },
            "tags": { "$type": "optional", "of": { "$type": "list", "of": { "$type": "builtin", "name": "string" } } }
        }})
    );
}

#[test]
fn rules_attach_within_their_module() {
    let files = [
        ("app.int", "import m\n"),
        ("m/rules.int", "rule only_m {\n  applies_to: select lib where meta.name matches \"a*\"\n}\nlib apple {\n}\nlib berry {\n}\n"),
        ("n/x.int", "lib avocado {\n}\n"),
    ];
    let g = build("rules", &files).unwrap();
    assert_eq!(decl(&g, "m.apple")["applies"], json!(["m.only_m"]));
    assert!(decl(&g, "m.berry").get("applies").is_none());
    assert!(
        decl(&g, "n.avocado").get("applies").is_none(),
        "outside the rule's module"
    );
    assert_eq!(
        decl(&g, "m.only_m")["fields"]["applies_to"],
        json!({ "$select": { "kinds": ["core:lib"], "where": "meta.name matches \"a*\"" } })
    );
}
