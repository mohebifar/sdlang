//! A language server for sdlang, over stdio (`sdc lsp`).
//!
//! Diagnostics (syntax errors, and the compiler's first error for the project), go to
//! definition, find references, highlights, hover, completion, rename (with a `moved`
//! statement), document and workspace symbols, folding and semantic tokens. Any editor
//! with an LSP client can use it; `editors/vscode` is the VS Code extension.

#![forbid(unsafe_code)]

pub mod analysis;
pub mod docs;
pub mod features;
mod server;
pub mod text;

/// Serves the protocol on `connection` until the client shuts the server down.
pub fn serve(
    connection: &lsp_server::Connection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    server::run(connection)
}

/// Serves the protocol on stdin and stdout until the client shuts the server down.
pub fn run_stdio() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (connection, io) = lsp_server::Connection::stdio();
    server::run(&connection)?;
    drop(connection);
    io.join()?;
    Ok(())
}
