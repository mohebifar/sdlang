//! The server end to end, over an in-memory connection, on a small project written to a
//! temporary directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use lsp_types::Url;
use serde_json::{json, Value};

const ACTIONS: &str = r#"export action add_to_cart {
  summary: "Puts a product in the cart"
  input: { id: string }
  uses: [cart]
  prompt: "Keep @{cart} in sync."
  expect {
    adds_item: "cart has the product"
  }
}

export lib cart {
  summary: "Keeps cart items"
}
"#;

const CHECKOUT: &str = r#"import module shop

action checkout {
  uses: [shop.cart]
}
"#;

struct Client {
    conn: Connection,
    next: i32,
    notes: Vec<Notification>,
    server: Option<thread::JoinHandle<()>>,
}

impl Client {
    fn start(root: &Path) -> Client {
        let (server, conn) = Connection::memory();
        let handle = thread::spawn(move || sdlang_lsp::serve(&server).unwrap());
        let mut c = Client {
            conn,
            next: 0,
            notes: Vec::new(),
            server: Some(handle),
        };
        let root_uri = Url::from_file_path(root).unwrap();
        c.request(
            "initialize",
            json!({ "processId": null, "rootUri": root_uri, "capabilities": {} }),
        );
        c.notify("initialized", json!({}));
        c
    }

    fn notify(&self, method: &str, params: Value) {
        self.conn
            .sender
            .send(Message::Notification(Notification::new(
                method.into(),
                params,
            )))
            .unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = RequestId::from(self.next);
        self.conn
            .sender
            .send(Message::Request(Request::new(
                id.clone(),
                method.into(),
                params,
            )))
            .unwrap();
        loop {
            match self.conn.receiver.recv().unwrap() {
                Message::Response(r) if r.id == id => {
                    return match (r.result, r.error) {
                        (Some(v), _) => v,
                        (None, Some(e)) => json!({ "error": e.message }),
                        _ => Value::Null,
                    };
                }
                Message::Notification(n) => self.notes.push(n),
                _ => {}
            }
        }
    }

    fn open(&self, path: &Path, text: &str) {
        let uri = Url::from_file_path(path).unwrap();
        self.notify("textDocument/didOpen", json!({ "textDocument": { "uri": uri, "languageId": "sdlang", "version": 1, "text": text } }));
    }

    fn change(&self, path: &Path, text: &str) {
        let uri = Url::from_file_path(path).unwrap();
        self.notify("textDocument/didChange", json!({ "textDocument": { "uri": uri, "version": 2 }, "contentChanges": [{ "text": text }] }));
    }

    /// The diagnostics last published for `path`, after the server has caught up.
    fn diagnostics(&mut self, path: &Path) -> Vec<Value> {
        self.request("workspace/symbol", json!({ "query": "" }));
        let uri = Url::from_file_path(path).unwrap().to_string();
        self.notes
            .iter()
            .rev()
            .find(|n| {
                n.method == "textDocument/publishDiagnostics" && n.params["uri"] == uri.as_str()
            })
            .map(|n| {
                n.params["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        if let Some(h) = self.server.take() {
            h.join().unwrap();
        }
    }
}

fn project(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("spec/shop")).unwrap();
    fs::create_dir_all(root.join("spec/checkout")).unwrap();
    let actions = root.join("spec/shop/actions.sd");
    let checkout = root.join("spec/checkout/actions.sd");
    fs::write(&actions, ACTIONS).unwrap();
    fs::write(&checkout, CHECKOUT).unwrap();
    let root = fs::canonicalize(root).unwrap();
    (
        root.clone(),
        root.join("spec/shop/actions.sd"),
        root.join("spec/checkout/actions.sd"),
    )
}

/// Where `needle` (plus `delta` characters) is in `text`, as an LSP position.
fn pos(text: &str, needle: &str, delta: usize) -> Value {
    let i = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle} not in text"))
        + delta;
    let line = text[..i].matches('\n').count();
    let col = i - text[..i].rfind('\n').map_or(0, |k| k + 1);
    json!({ "line": line, "character": col })
}

fn at(path: &Path, text: &str, needle: &str, delta: usize) -> Value {
    json!({ "textDocument": { "uri": Url::from_file_path(path).unwrap() }, "position": pos(text, needle, delta) })
}

#[test]
fn hover_shows_the_declaration_and_its_summary() {
    let (root, actions, _) = project("hover");
    let mut c = Client::start(&root);
    c.open(&actions, ACTIONS);
    let h = c.request("textDocument/hover", at(&actions, ACTIONS, "[cart]", 2));
    let text = h["contents"]["value"].as_str().unwrap();
    assert!(text.contains("**lib** `shop.cart`"), "{text}");
    assert!(text.contains("Keeps cart items"), "{text}");
    let h = c.request("textDocument/hover", at(&actions, ACTIONS, "adds_item", 1));
    assert!(h["contents"]["value"]
        .as_str()
        .unwrap()
        .contains("shop.add_to_cart#expect.adds_item"));
    let h = c.request("textDocument/hover", at(&actions, ACTIONS, "string", 1));
    assert!(h["contents"]["value"]
        .as_str()
        .unwrap()
        .contains("**builtin** `string`"));
}

#[test]
fn definition_crosses_modules_and_mentions() {
    let (root, actions, checkout) = project("definition");
    let mut c = Client::start(&root);
    c.open(&checkout, CHECKOUT);
    let d = c.request(
        "textDocument/definition",
        at(&checkout, CHECKOUT, "shop.cart", 6),
    );
    assert_eq!(d["uri"], Url::from_file_path(&actions).unwrap().as_str());
    assert_eq!(d["range"]["start"], pos(ACTIONS, "lib cart", 4));
    let d = c.request(
        "textDocument/definition",
        at(&actions, ACTIONS, "@{cart}", 3),
    );
    assert_eq!(d["range"]["start"], pos(ACTIONS, "lib cart", 4));
}

#[test]
fn references_and_rename_cover_every_use_and_record_the_move() {
    let (root, actions, checkout) = project("rename");
    let mut c = Client::start(&root);
    c.open(&actions, ACTIONS);
    let refs = c.request("textDocument/references", json!({ "textDocument": { "uri": Url::from_file_path(&actions).unwrap() }, "position": pos(ACTIONS, "lib cart", 4), "context": { "includeDeclaration": true } }));
    assert_eq!(refs.as_array().unwrap().len(), 4, "{refs}"); // definition, uses, mention, checkout

    let edit = c.request("textDocument/rename", json!({ "textDocument": { "uri": Url::from_file_path(&actions).unwrap() }, "position": pos(ACTIONS, "[cart]", 1), "newName": "basket" }));
    let changes = &edit["changes"];
    let here = changes[Url::from_file_path(&actions).unwrap().as_str()]
        .as_array()
        .unwrap();
    let there = changes[Url::from_file_path(&checkout).unwrap().as_str()]
        .as_array()
        .unwrap();
    assert_eq!(there.len(), 1);
    assert_eq!(here.iter().filter(|e| e["newText"] == "basket").count(), 3);
    assert!(here.iter().any(|e| e["newText"]
        .as_str()
        .unwrap()
        .contains("moved shop.cart -> shop.basket")));

    let refused = c.request("textDocument/rename", json!({ "textDocument": { "uri": Url::from_file_path(&actions).unwrap() }, "position": pos(ACTIONS, "[cart]", 1), "newName": "add_to_cart" }));
    assert!(refused["error"]
        .as_str()
        .unwrap()
        .contains("already exists"));
}

#[test]
fn completion_offers_the_kinds_fields_and_names_in_scope() {
    let (root, actions, checkout) = project("completion");
    let mut c = Client::start(&root);
    let edited = ACTIONS.replace("  uses: [cart]\n", "  uses: [cart]\n  ou\n");
    c.open(&actions, &edited);
    let items = c.request(
        "textDocument/completion",
        at(&actions, &edited, "  ou\n", 4),
    );
    let labels: Vec<&str> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["label"].as_str().unwrap())
        .collect();
    assert!(
        labels.contains(&"output") && labels.contains(&"errors"),
        "{labels:?}"
    );
    assert!(
        !labels.contains(&"summary"),
        "summary is already there: {labels:?}"
    );

    c.change(&actions, ACTIONS);
    let edited = CHECKOUT.replace("[shop.cart]", "[shop.]");
    c.open(&checkout, &edited);
    let items = c.request(
        "textDocument/completion",
        at(&checkout, &edited, "[shop.]", 6),
    );
    let labels: Vec<&str> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["label"].as_str().unwrap())
        .collect();
    assert!(
        labels.contains(&"cart") && labels.contains(&"add_to_cart"),
        "{labels:?}"
    );
}

#[test]
fn diagnostics_report_syntax_and_compile_errors_and_clear() {
    let (root, _, checkout) = project("diagnostics");
    let mut c = Client::start(&root);
    c.open(&checkout, &CHECKOUT.replace("shop.cart", "shop.nope"));
    let d = c.diagnostics(&checkout);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0]["message"].as_str().unwrap().contains("nope"), "{d:?}");

    c.change(
        &checkout,
        &CHECKOUT.replace("uses: [shop.cart]", "uses: [shop.cart] x"),
    );
    let d = c.diagnostics(&checkout);
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0]["range"]["start"]["line"], 3);
    // While the buffer does not parse, the rest of the project still navigates from its
    // last good text.
    let s = c.request("workspace/symbol", json!({ "query": "checkout" }));
    assert_eq!(s.as_array().unwrap().len(), 1, "{s}");

    c.change(&checkout, CHECKOUT);
    assert!(c.diagnostics(&checkout).is_empty());
}

#[test]
fn symbols_folding_and_tokens() {
    let (root, actions, _) = project("symbols");
    let mut c = Client::start(&root);
    c.open(&actions, ACTIONS);
    let doc = json!({ "textDocument": { "uri": Url::from_file_path(&actions).unwrap() } });
    let symbols = c.request("textDocument/documentSymbol", doc.clone());
    let names: Vec<&str> = symbols
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["add_to_cart", "cart"]);
    assert_eq!(symbols[0]["detail"], "action");
    let folds = c.request("textDocument/foldingRange", doc.clone());
    assert!(
        folds
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["startLine"] == 0 && f["endLine"] == 7),
        "{folds}"
    );
    let tokens = c.request("textDocument/semanticTokens/full", doc);
    assert!(
        tokens["data"].as_array().unwrap().len() >= 5 * 8,
        "{tokens}"
    );
}
