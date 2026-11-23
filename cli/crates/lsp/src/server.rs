//! The protocol loop: buffers, one analysis per project root, requests and diagnostics.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    DidSaveTextDocument, Notification as _, PublishDiagnostics,
};
use lsp_types::request::{
    Completion, DocumentHighlightRequest, DocumentSymbolRequest, FoldingRangeRequest,
    GotoDefinition, HoverRequest, PrepareRenameRequest, References, RegisterCapability, Rename,
    Request as _, SemanticTokensFullRequest, WorkspaceSymbolRequest,
};
use lsp_types::{
    CompletionOptions, CompletionResponse, Diagnostic, DiagnosticSeverity,
    DidChangeWatchedFilesRegistrationOptions, DocumentSymbolResponse, FileSystemWatcher,
    GlobPattern, GotoDefinitionResponse, InitializeParams, OneOf, PrepareRenameResponse,
    PublishDiagnosticsParams, Registration, RegistrationParams, RenameOptions, SemanticTokens,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensResult,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, Url,
    WorkDoneProgressOptions, WorkspaceEdit, WorkspaceSymbolResponse,
};
use serde_json::Value;

use crate::analysis::{self, Analysis, Buffer};
use crate::features;
use crate::text::{path_to_url, url_to_path, LineIndex};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn run(connection: &Connection) -> Result<(), Error> {
    let (id, params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(params)?;
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        folding_range_provider: Some(lsp_types::FoldingRangeProviderCapability::Simple(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".into(), "@".into()]),
            ..Default::default()
        }),
        semantic_tokens_provider: Some(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: features::TOKEN_TYPES.to_vec(),
                    token_modifiers: features::TOKEN_MODIFIERS.to_vec(),
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                range: None,
                work_done_progress_options: WorkDoneProgressOptions::default(),
            }
            .into(),
        ),
        ..Default::default()
    };
    let result = serde_json::json!({
        "capabilities": capabilities,
        "serverInfo": ServerInfo { name: "sdc lsp".into(), version: Some(env!("CARGO_PKG_VERSION").into()) },
    });
    connection.initialize_finish(id, result)?;

    let mut server = Server {
        conn: connection,
        buffers: HashMap::new(),
        analyses: HashMap::new(),
        published: HashMap::new(),
        folders: workspace_folders(&params),
        next_id: 0,
    };
    let watch = params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|w| w.did_change_watched_files)
        .and_then(|d| d.dynamic_registration)
        .unwrap_or(false);
    if watch {
        server.register_watchers()?;
    }
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                let response = server.request(req);
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(n) => server.notification(n)?,
            Message::Response(_) => {}
        }
    }
    Ok(())
}

fn workspace_folders(params: &InitializeParams) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = params
        .workspace_folders
        .iter()
        .flatten()
        .filter_map(|f| url_to_path(&f.uri))
        .collect();
    #[allow(deprecated)]
    if out.is_empty() {
        out.extend(params.root_uri.as_ref().and_then(url_to_path));
    }
    out
}

/// A key for an analysis: a project root, or the path of a file outside any project.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Key {
    Root(PathBuf),
    Alone(PathBuf),
}

struct Server<'c> {
    conn: &'c Connection,
    buffers: HashMap<PathBuf, Buffer>,
    analyses: HashMap<Key, Analysis>,
    /// Files each analysis last published diagnostics for, so they can be cleared.
    published: HashMap<Key, HashSet<PathBuf>>,
    folders: Vec<PathBuf>,
    next_id: i32,
}

/// `dir` is a project root: it has `spec/` and is not itself a package.
fn is_root(dir: &Path) -> bool {
    dir.join("spec").is_dir() && !dir.join("package.sd").is_file()
}

/// Project roots in a folder, a few levels down.
fn roots_in(folder: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if is_root(folder) {
        out.push(folder.to_path_buf());
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir()
            && !name.starts_with('.')
            && !matches!(
                name.as_str(),
                "node_modules" | "target" | "spec" | "spec-packages"
            )
        {
            roots_in(&p, depth - 1, out);
        }
    }
}

impl Server<'_> {
    fn send(&self, msg: Message) -> Result<(), Error> {
        self.conn.sender.send(msg)?;
        Ok(())
    }

    fn register_watchers(&mut self) -> Result<(), Error> {
        let watchers = ["**/*.sd", "**/sd.lock"]
            .iter()
            .map(|g| FileSystemWatcher {
                glob_pattern: GlobPattern::String(g.to_string()),
                kind: None,
            })
            .collect();
        let params = RegistrationParams {
            registrations: vec![Registration {
                id: "sdlang-watch".into(),
                method: DidChangeWatchedFiles::METHOD.into(),
                register_options: Some(serde_json::to_value(
                    DidChangeWatchedFilesRegistrationOptions { watchers },
                )?),
            }],
        };
        self.next_id += 1;
        let req = Request::new(
            RequestId::from(self.next_id),
            RegisterCapability::METHOD.into(),
            params,
        );
        self.send(Message::Request(req))
    }

    // -----------------------------------------------------------------------------------
    // Analyses.

    fn analysis(&mut self, key: &Key) -> &Analysis {
        if !self.analyses.contains_key(key) {
            let a = match key {
                Key::Root(root) => analysis::build(root, &self.buffers),
                Key::Alone(path) => analysis::standalone(path, self.buffers.get(path)),
            };
            self.analyses.insert(key.clone(), a);
        }
        &self.analyses[key]
    }

    /// The analysis that covers `path`.
    fn key_for(&mut self, path: &Path) -> Key {
        if let Some((k, _)) = self
            .analyses
            .iter()
            .find(|(k, a)| matches!(k, Key::Root(_)) && a.by_path.contains_key(path))
        {
            return k.clone();
        }
        for dir in path.ancestors().skip(1) {
            if !is_root(dir) {
                continue;
            }
            let key = Key::Root(dir.to_path_buf());
            if path.starts_with(dir.join("spec")) || self.analysis(&key).by_path.contains_key(path)
            {
                return key;
            }
        }
        Key::Alone(path.to_path_buf())
    }

    /// Drops every analysis `path` may affect.
    fn invalidate(&mut self, path: &Path) -> Vec<Key> {
        let mut keys: Vec<Key> = self
            .analyses
            .iter()
            .filter(|(k, a)| match k {
                Key::Root(root) => a.by_path.contains_key(path) || path.starts_with(root),
                Key::Alone(p) => p == path,
            })
            .map(|(k, _)| k.clone())
            .collect();
        let own = self.key_for_uncached(path);
        if !keys.contains(&own) {
            keys.push(own);
        }
        for k in &keys {
            self.analyses.remove(k);
        }
        keys
    }

    fn key_for_uncached(&self, path: &Path) -> Key {
        path.ancestors()
            .skip(1)
            .find(|d| is_root(d) && path.starts_with(d.join("spec")))
            .map(|d| Key::Root(d.to_path_buf()))
            .unwrap_or_else(|| Key::Alone(path.to_path_buf()))
    }

    fn refresh(&mut self, keys: Vec<Key>) -> Result<(), Error> {
        for key in keys {
            if let Key::Alone(p) = &key {
                if !self.buffers.contains_key(p) {
                    self.publish_clear(&key)?;
                    continue;
                }
            }
            self.analysis(&key);
            self.publish(&key)?;
        }
        Ok(())
    }

    fn publish_clear(&mut self, key: &Key) -> Result<(), Error> {
        for path in self.published.remove(key).unwrap_or_default() {
            self.publish_one(&path, Vec::new())?;
        }
        Ok(())
    }

    fn publish(&mut self, key: &Key) -> Result<(), Error> {
        let a = &self.analyses[key];
        let mut now: HashMap<PathBuf, Vec<Diagnostic>> = HashMap::new();
        for (path, diags) in &a.diags {
            // Ranges against what the editor shows: the buffer if open, else the file.
            let text = match self.buffers.get(path) {
                Some(b) => sdlang_syntax::lexer::normalize(&b.text).into_owned(),
                None => match a.file(path) {
                    Some(f) => a.files[f].text.clone(),
                    None => std::fs::read_to_string(path).unwrap_or_default(),
                },
            };
            let index = LineIndex::new(&text);
            now.insert(
                path.clone(),
                diags
                    .iter()
                    .map(|d| Diagnostic {
                        range: index.range(&text, d.span),
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("sdlang".into()),
                        message: d.message.clone(),
                        ..Default::default()
                    })
                    .collect(),
            );
        }
        let before = self.published.remove(key).unwrap_or_default();
        for path in before.iter().filter(|p| !now.contains_key(*p)) {
            self.publish_one(path, Vec::new())?;
        }
        let paths: HashSet<PathBuf> = now.keys().cloned().collect();
        for (path, diags) in now {
            self.publish_one(&path, diags)?;
        }
        self.published.insert(key.clone(), paths);
        Ok(())
    }

    fn publish_one(&self, path: &Path, diagnostics: Vec<Diagnostic>) -> Result<(), Error> {
        let Some(uri) = path_to_url(path) else {
            return Ok(());
        };
        let params = PublishDiagnosticsParams {
            uri,
            diagnostics,
            version: None,
        };
        self.send(Message::Notification(Notification::new(
            PublishDiagnostics::METHOD.into(),
            params,
        )))
    }

    // -----------------------------------------------------------------------------------
    // Notifications.

    fn notification(&mut self, n: Notification) -> Result<(), Error> {
        match n.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let p: lsp_types::DidOpenTextDocumentParams = serde_json::from_value(n.params)?;
                if let Some(path) = url_to_path(&p.text_document.uri) {
                    self.set_text(&path, p.text_document.text)?;
                }
            }
            DidChangeTextDocument::METHOD => {
                let p: lsp_types::DidChangeTextDocumentParams = serde_json::from_value(n.params)?;
                if let (Some(path), Some(change)) = (
                    url_to_path(&p.text_document.uri),
                    p.content_changes.into_iter().last(),
                ) {
                    self.set_text(&path, change.text)?;
                }
            }
            DidCloseTextDocument::METHOD => {
                let p: lsp_types::DidCloseTextDocumentParams = serde_json::from_value(n.params)?;
                if let Some(path) = url_to_path(&p.text_document.uri) {
                    self.buffers.remove(&path);
                    let keys = self.invalidate(&path);
                    self.refresh(keys)?;
                }
            }
            DidSaveTextDocument::METHOD => {}
            DidChangeWatchedFiles::METHOD => {
                let p: lsp_types::DidChangeWatchedFilesParams = serde_json::from_value(n.params)?;
                let mut keys = Vec::new();
                for change in p.changes {
                    if let Ok(path) = change.uri.to_file_path() {
                        for k in self.invalidate(&path) {
                            if !keys.contains(&k) {
                                keys.push(k);
                            }
                        }
                    }
                }
                keys.retain(|k| {
                    matches!(k, Key::Root(_))
                        || matches!(k, Key::Alone(p) if self.buffers.contains_key(p))
                });
                self.refresh(keys)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn set_text(&mut self, path: &Path, text: String) -> Result<(), Error> {
        let buffer = self.buffers.entry(path.to_path_buf()).or_default();
        if sdlang_syntax::parse_source(&text).is_ok() {
            buffer.last_good = Some(text.clone());
        }
        buffer.text = text;
        let keys = self.invalidate(path);
        self.refresh(keys)
    }

    // -----------------------------------------------------------------------------------
    // Requests.

    fn request(&mut self, req: Request) -> Response {
        let id = req.id.clone();
        let result = self.dispatch(req.method.as_str(), req.params);
        match result {
            Ok(Some(v)) => Response::new_ok(id, v),
            Ok(None) => Response::new_ok(id, Value::Null),
            Err(Failure::Unknown) => Response::new_err(
                id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("unhandled method {}", req.method),
            ),
            Err(Failure::Message(m)) => {
                Response::new_err(id, lsp_server::ErrorCode::RequestFailed as i32, m)
            }
        }
    }

    fn dispatch(&mut self, method: &str, params: Value) -> Result<Option<Value>, Failure> {
        fn parse<P: serde::de::DeserializeOwned>(v: Value) -> Result<P, Failure> {
            serde_json::from_value(v).map_err(|e| Failure::Message(e.to_string()))
        }
        fn json<T: serde::Serialize>(v: T) -> Result<Option<Value>, Failure> {
            serde_json::to_value(v)
                .map(Some)
                .map_err(|e| Failure::Message(e.to_string()))
        }
        match method {
            HoverRequest::METHOD => {
                let p: lsp_types::HoverParams = parse(params)?;
                self.at(&p.text_document_position_params, |a, f, o| {
                    json(features::hover(a, f, o))
                })
            }
            GotoDefinition::METHOD => {
                let p: lsp_types::GotoDefinitionParams = parse(params)?;
                self.at(&p.text_document_position_params, |a, f, o| {
                    json(features::definition(a, f, o).map(GotoDefinitionResponse::Scalar))
                })
            }
            References::METHOD => {
                let p: lsp_types::ReferenceParams = parse(params)?;
                let include = p.context.include_declaration;
                self.at(&p.text_document_position, |a, f, o| {
                    json(features::references(a, f, o, include))
                })
            }
            DocumentHighlightRequest::METHOD => {
                let p: lsp_types::DocumentHighlightParams = parse(params)?;
                self.at(&p.text_document_position_params, |a, f, o| {
                    json(features::highlights(a, f, o))
                })
            }
            PrepareRenameRequest::METHOD => {
                let p: lsp_types::TextDocumentPositionParams = parse(params)?;
                self.at(&p, |a, f, o| match features::prepare_rename(a, f, o) {
                    Ok((range, placeholder)) => {
                        json(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder })
                    }
                    Err(m) => Err(Failure::Message(m)),
                })
            }
            Rename::METHOD => {
                let p: lsp_types::RenameParams = parse(params)?;
                let new_name = p.new_name.clone();
                self.at(&p.text_document_position, |a, f, o| match features::rename(
                    a, f, o, &new_name,
                ) {
                    Ok(changes) => json(WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    }),
                    Err(m) => Err(Failure::Message(m)),
                })
            }
            Completion::METHOD => {
                let p: lsp_types::CompletionParams = parse(params)?;
                let tdp = &p.text_document_position;
                let Some(path) = url_to_path(&tdp.text_document.uri) else {
                    return Ok(None);
                };
                let key = self.key_for(&path);
                let text = match self.buffers.get(&path) {
                    Some(b) => sdlang_syntax::lexer::normalize(&b.text).into_owned(),
                    None => std::fs::read_to_string(&path).unwrap_or_default(),
                };
                let offset = LineIndex::new(&text).offset(&text, tdp.position);
                let a = self.analysis(&key);
                let module = a.module_of(&path).unwrap_or_default();
                json(CompletionResponse::Array(features::complete(
                    a, &module, &text, offset,
                )))
            }
            DocumentSymbolRequest::METHOD => {
                let p: lsp_types::DocumentSymbolParams = parse(params)?;
                self.document(&p.text_document.uri, |a, f| {
                    json(DocumentSymbolResponse::Nested(features::document_symbols(
                        a, f,
                    )))
                })
            }
            FoldingRangeRequest::METHOD => {
                let p: lsp_types::FoldingRangeParams = parse(params)?;
                self.document(&p.text_document.uri, |a, f| json(features::folding(a, f)))
            }
            SemanticTokensFullRequest::METHOD => {
                let p: lsp_types::SemanticTokensParams = parse(params)?;
                self.document(&p.text_document.uri, |a, f| {
                    json(SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: features::semantic_tokens(a, f),
                    }))
                })
            }
            WorkspaceSymbolRequest::METHOD => {
                let p: lsp_types::WorkspaceSymbolParams = parse(params)?;
                let mut roots = Vec::new();
                for folder in self.folders.clone() {
                    roots_in(&folder, 3, &mut roots);
                }
                for root in roots {
                    self.analysis(&Key::Root(root));
                }
                let all: Vec<&Analysis> = self
                    .analyses
                    .iter()
                    .filter(|(k, _)| matches!(k, Key::Root(_)))
                    .map(|(_, a)| a)
                    .collect();
                json(WorkspaceSymbolResponse::Flat(features::workspace_symbols(
                    &all, &p.query,
                )))
            }
            _ => Err(Failure::Unknown),
        }
    }

    /// Resolves a document's analysis and file; `None` for a file that is not analysed or
    /// whose buffer does not parse (its positions would be wrong).
    fn locate(&mut self, uri: &Url) -> Option<(Key, usize)> {
        let path = url_to_path(uri)?;
        let key = self.key_for(&path);
        let a = self.analysis(&key);
        let file = a.file(&path)?;
        (!a.files[file].stale).then_some((key, file))
    }

    fn at(
        &mut self,
        p: &lsp_types::TextDocumentPositionParams,
        f: impl FnOnce(&Analysis, usize, usize) -> Result<Option<Value>, Failure>,
    ) -> Result<Option<Value>, Failure> {
        let Some((key, file)) = self.locate(&p.text_document.uri) else {
            return Ok(None);
        };
        let a = &self.analyses[&key];
        let fd = &a.files[file];
        let offset = fd.index.offset(&fd.text, p.position);
        f(a, file, offset)
    }

    fn document(
        &mut self,
        uri: &Url,
        f: impl FnOnce(&Analysis, usize) -> Result<Option<Value>, Failure>,
    ) -> Result<Option<Value>, Failure> {
        let Some((key, file)) = self.locate(uri) else {
            return Ok(None);
        };
        f(&self.analyses[&key], file)
    }
}

enum Failure {
    Unknown,
    Message(String),
}
