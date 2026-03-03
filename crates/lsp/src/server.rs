//! LSP server main loop with request/notification dispatch.
//!
//! Uses `lsp-server` (synchronous, crossbeam-based) for the transport.
//! No async runtime needed -- matches the project's synchronous architecture.

use lsp_server::{Connection, Message, Notification, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Notification as _, PublishDiagnostics,
};
use lsp_types::request::{
    Completion, DocumentSymbolRequest, GotoDefinition, HoverRequest, References,
    SemanticTokensFullRequest,
};
use lsp_types::{
    CompletionOptions, CompletionResponse, DocumentSymbolResponse, GotoDefinitionResponse,
    HoverProviderCapability, OneOf, PublishDiagnosticsParams, SaveOptions, SemanticTokens,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensResult,
    ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Uri,
};
use std::path::{Path, PathBuf};

use crate::agent_capabilities;
use crate::completion;
use crate::diagnostics;
use crate::document::DocumentState;
use crate::hover;
use crate::navigation::{self, ProjectIndex};
use crate::semantic_tokens;

/// Run the LSP server over stdio until shutdown.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, io_threads) = Connection::stdio();

    // ── Initialize handshake ──────────────────────────────────────────
    let server_capabilities = build_capabilities();
    let init_json = serde_json::to_value(&server_capabilities)?;
    let init_params: lsp_types::InitializeParams =
        serde_json::from_value(connection.initialize(init_json)?)?;

    // ── Build initial project index from workspace root ──────────────
    let mut workspace_root = extract_workspace_root(&init_params);
    let mut project_index = if let Some(root) = &workspace_root {
        navigation::build_project_index(root)
    } else {
        ProjectIndex::new()
    };

    // ── Main loop ─────────────────────────────────────────────────────
    let mut doc_state = DocumentState::new();

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                handle_request(&connection, &doc_state, &project_index, req)?;
            }
            Message::Notification(not) => {
                handle_notification(
                    &connection,
                    &mut doc_state,
                    &mut project_index,
                    &mut workspace_root,
                    not,
                )?;
            }
            Message::Response(_) => {
                // Ignore responses (we don't send requests to the client)
            }
        }
    }

    io_threads.join()?;
    Ok(())
}

/// Extract workspace root path from InitializeParams.
#[allow(deprecated)] // root_path/root_uri are deprecated but needed for backwards compat
fn extract_workspace_root(params: &lsp_types::InitializeParams) -> Option<PathBuf> {
    // Try workspace_folders first (modern LSP)
    if let Some(folders) = &params.workspace_folders {
        if let Some(folder) = folders.first() {
            return Some(uri_to_path_from_str(folder.uri.as_str()));
        }
    }
    // Fall back to root_uri
    if let Some(root_uri) = &params.root_uri {
        return Some(uri_to_path_from_str(root_uri.as_str()));
    }
    // Fall back to root_path (deprecated but still sent by VS Code)
    if let Some(root_path) = &params.root_path {
        if !root_path.is_empty() {
            return Some(PathBuf::from(root_path));
        }
    }
    None
}

/// Convert a URI string to a file system path.
fn uri_to_path_from_str(uri_str: &str) -> PathBuf {
    if let Some(path) = uri_str.strip_prefix("file://") {
        let decoded = percent_decode(path);
        #[cfg(windows)]
        {
            let decoded = decoded.strip_prefix('/').unwrap_or(&decoded);
            PathBuf::from(decoded)
        }
        #[cfg(not(windows))]
        {
            PathBuf::from(decoded)
        }
    } else {
        PathBuf::from(uri_str)
    }
}

fn build_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(false),
                })),
                ..Default::default()
            },
        )),
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                SemanticTokensOptions {
                    full: Some(SemanticTokensFullOptions::Delta { delta: Some(false) }),
                    legend: SemanticTokensLegend {
                        token_types: semantic_tokens::TOKEN_TYPES.to_vec(),
                        token_modifiers: semantic_tokens::TOKEN_MODIFIERS.to_vec(),
                    },
                    ..Default::default()
                },
            ),
        ),
        definition_provider: Some(OneOf::Left(true)),
        references_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![":".into(), " ".into()]),
            resolve_provider: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn handle_request(
    connection: &Connection,
    doc_state: &DocumentState,
    project_index: &ProjectIndex,
    req: lsp_server::Request,
) -> Result<(), Box<dyn std::error::Error>> {
    use lsp_types::request::Request as _;

    if req.method == SemanticTokensFullRequest::METHOD {
        let params: lsp_types::SemanticTokensParams = serde_json::from_value(req.params.clone())?;
        let uri_str = params.text_document.uri.as_str().to_string();

        let result = if let Some(doc) = doc_state.get(&uri_str) {
            semantic_tokens::compute_semantic_tokens(&doc.path, &doc.content)
                .map(|tokens| {
                    SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: tokens,
                    })
                })
                .unwrap_or_else(|| {
                    SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: Vec::new(),
                    })
                })
        } else {
            // Document not tracked -- try from file path
            let path = uri_to_path(&params.text_document.uri);
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            semantic_tokens::compute_semantic_tokens(&path, &content)
                .map(|tokens| {
                    SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: tokens,
                    })
                })
                .unwrap_or_else(|| {
                    SemanticTokensResult::Tokens(SemanticTokens {
                        result_id: None,
                        data: Vec::new(),
                    })
                })
        };

        let resp = Response::new_ok(req.id, serde_json::to_value(result)?);
        connection.sender.send(Message::Response(resp))?;
    } else if req.method == GotoDefinition::METHOD {
        let params: lsp_types::GotoDefinitionParams = serde_json::from_value(req.params.clone())?;
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let content = get_document_content(doc_state, uri);
        let result = navigation::goto_definition(project_index, uri, position, &content)
            .map(GotoDefinitionResponse::Scalar);
        let resp = Response::new_ok(req.id, serde_json::to_value(result)?);
        connection.sender.send(Message::Response(resp))?;
    } else if req.method == References::METHOD {
        let params: lsp_types::ReferenceParams = serde_json::from_value(req.params.clone())?;
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let content = get_document_content(doc_state, uri);
        let refs = navigation::find_references(project_index, uri, position, &content);
        let result: Option<Vec<lsp_types::Location>> =
            if refs.is_empty() { None } else { Some(refs) };
        let resp = Response::new_ok(req.id, serde_json::to_value(result)?);
        connection.sender.send(Message::Response(resp))?;
    } else if req.method == DocumentSymbolRequest::METHOD {
        let params: lsp_types::DocumentSymbolParams = serde_json::from_value(req.params.clone())?;
        let uri = &params.text_document.uri;
        let syms = navigation::document_symbols(project_index, uri);
        let result: Option<DocumentSymbolResponse> = if syms.is_empty() {
            None
        } else {
            Some(DocumentSymbolResponse::Nested(syms))
        };
        let resp = Response::new_ok(req.id, serde_json::to_value(result)?);
        connection.sender.send(Message::Response(resp))?;
    } else if req.method == HoverRequest::METHOD {
        let params: lsp_types::HoverParams = serde_json::from_value(req.params.clone())?;
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let content = get_document_content(doc_state, uri);
        let result = hover::compute_hover(project_index, position, &content);
        let resp = Response::new_ok(req.id, serde_json::to_value(result)?);
        connection.sender.send(Message::Response(resp))?;
    } else if req.method == Completion::METHOD {
        let params: lsp_types::CompletionParams = serde_json::from_value(req.params.clone())?;
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let content = get_document_content(doc_state, uri);
        let items = completion::compute_completions(project_index, position, &content);
        let result = CompletionResponse::Array(items);
        let resp = Response::new_ok(req.id, serde_json::to_value(result)?);
        connection.sender.send(Message::Response(resp))?;
    } else if req.method == "tenor/agentCapabilities" {
        // Custom request: compute agent capabilities for a document
        let params: lsp_types::TextDocumentIdentifier = serde_json::from_value(req.params.clone())?;
        let path = uri_to_path(&params.uri);
        let caps = agent_capabilities::compute_agent_capabilities(&path);
        let resp = Response::new_ok(req.id, serde_json::to_value(caps)?);
        connection.sender.send(Message::Response(resp))?;
    } else {
        // Unknown request -- method not found
        let resp = Response::new_err(
            req.id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("method not found: {}", req.method),
        );
        connection.sender.send(Message::Response(resp))?;
    }
    Ok(())
}

/// Get document content either from open documents or from disk.
fn get_document_content(doc_state: &DocumentState, uri: &Uri) -> String {
    let uri_str = uri.as_str().to_string();
    if let Some(doc) = doc_state.get(&uri_str) {
        doc.content.clone()
    } else {
        let path = uri_to_path(uri);
        std::fs::read_to_string(&path).unwrap_or_default()
    }
}

fn handle_notification(
    connection: &Connection,
    doc_state: &mut DocumentState,
    project_index: &mut ProjectIndex,
    workspace_root: &mut Option<PathBuf>,
    not: Notification,
) -> Result<(), Box<dyn std::error::Error>> {
    match not.method.as_str() {
        m if m == DidOpenTextDocument::METHOD => {
            let params: lsp_types::DidOpenTextDocumentParams = serde_json::from_value(not.params)?;
            let uri_str = params.text_document.uri.as_str().to_string();
            let path = uri_to_path(&params.text_document.uri);
            doc_state.open(
                &uri_str,
                path.clone(),
                params.text_document.version,
                params.text_document.text,
            );

            // If we have no workspace root yet, infer it from the opened file.
            // This handles editors that don't send rootUri/workspaceFolders.
            if workspace_root.is_none() {
                if let Some(root) = infer_workspace_root(&path) {
                    *project_index = navigation::build_project_index(&root);
                    *workspace_root = Some(root);
                }
            }

            let diags = diagnostics::compute_diagnostics(&path);
            publish_diagnostics(connection, params.text_document.uri, diags)?;
        }
        m if m == DidChangeTextDocument::METHOD => {
            let params: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(not.params)?;
            let uri_str = params.text_document.uri.as_str().to_string();
            // FULL sync: last content change has the entire document
            if let Some(change) = params.content_changes.into_iter().last() {
                doc_state.change(&uri_str, params.text_document.version, change.text);
            }
        }
        m if m == DidSaveTextDocument::METHOD => {
            let params: lsp_types::DidSaveTextDocumentParams = serde_json::from_value(not.params)?;
            let path = uri_to_path(&params.text_document.uri);
            let diags = diagnostics::compute_diagnostics(&path);
            publish_diagnostics(connection, params.text_document.uri.clone(), diags)?;

            // Send updated agent capabilities after save
            let caps = agent_capabilities::compute_agent_capabilities(&path);
            let caps_notification = Notification::new(
                "tenor/agentCapabilitiesUpdated".to_string(),
                serde_json::json!({
                    "uri": params.text_document.uri.as_str(),
                    "capabilities": caps,
                }),
            );
            connection
                .sender
                .send(Message::Notification(caps_notification))?;

            // Rebuild project index on save to pick up changes
            if let Some(root) = workspace_root {
                *project_index = navigation::build_project_index(root);
            }
        }
        m if m == DidCloseTextDocument::METHOD => {
            let params: lsp_types::DidCloseTextDocumentParams = serde_json::from_value(not.params)?;
            let uri_str = params.text_document.uri.as_str().to_string();
            doc_state.close(&uri_str);
            // Clear diagnostics for closed file
            publish_diagnostics(connection, params.text_document.uri, Vec::new())?;
        }
        _ => {
            // Unknown notification -- ignore
        }
    }
    Ok(())
}

/// Send `textDocument/publishDiagnostics` notification to the client.
fn publish_diagnostics(
    connection: &Connection,
    uri: Uri,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<(), Box<dyn std::error::Error>> {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    let not = Notification::new(PublishDiagnostics::METHOD.to_string(), params);
    connection.sender.send(Message::Notification(not))?;
    Ok(())
}

/// Infer workspace root by walking up from a file path looking for project markers.
fn infer_workspace_root(file_path: &Path) -> Option<PathBuf> {
    let mut dir = if file_path.is_file() {
        file_path.parent()?.to_path_buf()
    } else {
        file_path.to_path_buf()
    };
    loop {
        // Look for common project root markers
        if dir.join("Cargo.toml").exists()
            || dir.join(".git").exists()
            || dir.join("package.json").exists()
        {
            return Some(dir);
        }
        if !dir.pop() {
            break;
        }
    }
    // Fall back to the file's parent directory
    file_path.parent().map(|p| p.to_path_buf())
}

/// Convert an LSP URI to a file system path.
///
/// Handles `file:///path/to/file` URIs by stripping the scheme and authority
/// and percent-decoding (e.g. `%3A` → `:`).
fn uri_to_path(uri: &Uri) -> PathBuf {
    let s = uri.as_str();
    if let Some(path) = s.strip_prefix("file://") {
        let decoded = percent_decode(path);
        // On Unix: file:///foo/bar -> /foo/bar
        // On Windows: file:///C:/foo -> C:/foo (strip leading /)
        #[cfg(windows)]
        {
            let decoded = decoded.strip_prefix('/').unwrap_or(&decoded);
            PathBuf::from(decoded)
        }
        #[cfg(not(windows))]
        {
            PathBuf::from(decoded)
        }
    } else {
        PathBuf::from(s)
    }
}

/// Decode percent-encoded characters in a URI path (e.g. `%3A` → `:`).
fn percent_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(hi), Some(lo)) = (hi, lo) {
                if let (Some(h), Some(l)) = (hex_val(hi), hex_val(lo)) {
                    result.push((h << 4 | l) as char);
                    continue;
                }
                // Malformed percent encoding -- pass through
                result.push('%');
                result.push(hi as char);
                result.push(lo as char);
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
