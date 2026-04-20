//! LSP message dispatch and server lifecycle.
//!
//! This module owns:
//! - The `Connection` lifecycle (initialize → main loop → shutdown).
//! - Document state (`ServerState`).
//! - Dispatch from raw `lsp_server::Message` values to feature handlers.
//! - The `publishDiagnostics` pipeline.
//!
//! Feature handlers (`handle_hover`, `handle_completion`, etc.) live in
//! `crate::features` and are called as pure functions; they do not touch
//! the connection or document store.

use std::collections::HashMap;

use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{
    CompletionOptions, CompletionResponse, Diagnostic, DiagnosticSeverity,
    DocumentSymbolResponse, GotoDefinitionResponse, HoverProviderCapability, OneOf, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
        PublishDiagnostics,
    },
    request::{
        Completion, DocumentSymbolRequest, Formatting, GotoDefinition, HoverRequest, Request as _,
    },
};

use crate::Language;

// ---------------------------------------------------------------------------
// Document state
// ---------------------------------------------------------------------------

struct ServerState {
    documents: HashMap<Url, String>,
}

impl ServerState {
    fn new() -> Self {
        Self { documents: HashMap::new() }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the LSP server synchronously over stdin/stdout.
///
/// Blocks until the client completes the shutdown/exit sequence or when an
/// unrecoverable protocol error occurs.
///
/// # Errors
///
/// Returns any error that prevents the server from completing its lifecycle
/// (e.g. broken pipe, serialisation failure, protocol violation).
pub fn run_server() -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let caps = server_capabilities();
    let caps_value = serde_json::to_value(&caps)?;
    connection.initialize(caps_value)?;

    let mut state = ServerState::new();
    main_loop(&connection, &mut state)?;

    io_threads.join()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Capabilities
// ---------------------------------------------------------------------------

fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![
                ":".to_string(),
                "@".to_string(),
                "<".to_string(),
                "\"".to_string(),
            ]),
            ..Default::default()
        }),
        definition_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

fn main_loop(
    connection: &Connection,
    state: &mut ServerState,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    break;
                }
                dispatch_request(connection, req, state);
            }
            Message::Response(_resp) => {
                // Client responses to server-initiated requests: ignored in Phase F.
            }
            Message::Notification(notif) => {
                dispatch_notification(connection, notif, state);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Notification dispatch
// ---------------------------------------------------------------------------

fn dispatch_notification(
    connection: &Connection,
    notif: Notification,
    state: &mut ServerState,
) {
    match notif.method.as_str() {
        DidOpenTextDocument::METHOD => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidOpenTextDocumentParams>(notif.params)
            {
                let uri = params.text_document.uri;
                let text = params.text_document.text;
                state.documents.insert(uri.clone(), text.clone());
                let lang = Language::from_uri(&uri);
                publish_diagnostics(connection, &uri, &text, lang);
            }
        }

        DidChangeTextDocument::METHOD => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidChangeTextDocumentParams>(notif.params)
            {
                let uri = params.text_document.uri;
                // Full-sync: exactly one change covering the whole document.
                if let Some(change) = params.content_changes.into_iter().next() {
                    let text = change.text;
                    state.documents.insert(uri.clone(), text.clone());
                    let lang = Language::from_uri(&uri);
                    publish_diagnostics(connection, &uri, &text, lang);
                }
            }
        }

        DidCloseTextDocument::METHOD => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidCloseTextDocumentParams>(notif.params)
            {
                state.documents.remove(&params.text_document.uri);
            }
        }

        _ => {
            // Unknown notifications are silently ignored per LSP spec §3.18.
        }
    }
}

// ---------------------------------------------------------------------------
// Request dispatch
// ---------------------------------------------------------------------------

fn dispatch_request(connection: &Connection, req: Request, state: &ServerState) {
    match req.method.as_str() {
        HoverRequest::METHOD => handle_hover_request(connection, req, state),
        Completion::METHOD => handle_completion_request(connection, req, state),
        GotoDefinition::METHOD => handle_goto_definition_request(connection, req, state),
        DocumentSymbolRequest::METHOD => handle_document_symbol_request(connection, req, state),
        Formatting::METHOD => handle_formatting_request(connection, req, state),
        _ => send_error(connection, req.id, ErrorCode::MethodNotFound, "method not implemented"),
    }
}

// ---------------------------------------------------------------------------
// Individual request handlers
// ---------------------------------------------------------------------------

fn handle_hover_request(connection: &Connection, req: Request, state: &ServerState) {
    let (id, params) = match cast_request::<HoverRequest>(req) {
        Ok(pair) => pair,
        Err(id) => return send_null_ok(connection, id),
    };

    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let lang = Language::from_uri(uri);

    let result = state
        .documents
        .get(uri)
        .and_then(|text| lang.and_then(|l| crate::features::hover::handle_hover(text, l, pos)));

    let json = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    send_ok(connection, id, json);
}

fn handle_completion_request(connection: &Connection, req: Request, state: &ServerState) {
    let (id, params) = match cast_request::<Completion>(req) {
        Ok(pair) => pair,
        Err(id) => return send_null_ok(connection, id),
    };

    let uri = &params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let lang = Language::from_uri(uri);

    let result: Option<CompletionResponse> = state.documents.get(uri).and_then(|text| {
        lang.map(|l| {
            CompletionResponse::List(crate::features::completion::handle_completion(text, l, pos))
        })
    });

    let json = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    send_ok(connection, id, json);
}

fn handle_goto_definition_request(
    connection: &Connection,
    req: Request,
    state: &ServerState,
) {
    let (id, params) = match cast_request::<GotoDefinition>(req) {
        Ok(pair) => pair,
        Err(id) => return send_null_ok(connection, id),
    };

    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let lang = Language::from_uri(uri);

    let result: Option<GotoDefinitionResponse> = state.documents.get(uri).and_then(|text| {
        lang.and_then(|l| {
            crate::features::goto_definition::handle_goto_definition(text, l, pos)
                .map(GotoDefinitionResponse::Scalar)
        })
    });

    let json = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    send_ok(connection, id, json);
}

fn handle_document_symbol_request(
    connection: &Connection,
    req: Request,
    state: &ServerState,
) {
    let (id, params) = match cast_request::<DocumentSymbolRequest>(req) {
        Ok(pair) => pair,
        Err(id) => return send_null_ok(connection, id),
    };

    let uri = &params.text_document.uri;
    let lang = Language::from_uri(uri);

    let symbols = state
        .documents
        .get(uri)
        .map(|text| {
            lang.map_or_else(Vec::new, |l| {
                crate::features::document_symbols::handle_document_symbols(text, l)
            })
        })
        .unwrap_or_default();

    let result = Some(DocumentSymbolResponse::Nested(symbols));
    let json = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    send_ok(connection, id, json);
}

fn handle_formatting_request(connection: &Connection, req: Request, state: &ServerState) {
    let (id, params) = match cast_request::<Formatting>(req) {
        Ok(pair) => pair,
        Err(id) => return send_null_ok(connection, id),
    };

    let uri = &params.text_document.uri;
    let lang = Language::from_uri(uri);

    // `handle_formatting` returns `None` for languages whose Phase E formatter
    // was a stretch goal (RDF/XML, JSON-LD). The protocol sends an empty
    // success response (null result) rather than an error.
    let result: Option<Vec<lsp_types::TextEdit>> = state.documents.get(uri).and_then(|text| {
        lang.and_then(|l| crate::features::formatting::handle_formatting(text, l))
    });

    let json = serde_json::to_value(result).unwrap_or(serde_json::Value::Null);
    send_ok(connection, id, json);
}

// ---------------------------------------------------------------------------
// Diagnostic pipeline
// ---------------------------------------------------------------------------

fn publish_diagnostics(
    connection: &Connection,
    uri: &Url,
    text: &str,
    lang: Option<Language>,
) {
    let diagnostics = lang.map_or_else(Vec::new, |l| parse_document(text, l));

    let params = PublishDiagnosticsParams { uri: uri.clone(), diagnostics, version: None };
    let notif = Notification::new(PublishDiagnostics::METHOD.to_owned(), params);

    if let Err(e) = connection.sender.send(Message::Notification(notif)) {
        eprintln!("rdf-lsp: failed to send publishDiagnostics: {e}");
    }
}

/// Parse `text` using the appropriate parser for `lang`.
///
/// Converts `rdf_diff::Diagnostics` into `Vec<lsp_types::Diagnostic>`.
/// Fatal parse errors map to `ERROR` severity; non-fatal warnings map to
/// `WARNING` severity. All positions default to `(0, 0)` in Phase F.
fn parse_document(text: &str, lang: Language) -> Vec<Diagnostic> {
    use rdf_diff::Parser as _;

    let input = text.as_bytes();

    let (fatal, messages) = match lang {
        Language::NTriples => match rdf_ntriples::NTriplesParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::NQuads => match rdf_ntriples::NQuadsParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::Turtle => match rdf_turtle::TurtleParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::TriG => match rdf_turtle::TriGParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::RdfXml => match rdf_xml::RdfXmlParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::JsonLd => match rdf_jsonld::JsonLdParser::default().parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::TriX => match rdf_trix::TriXParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::N3 => match rdf_n3::N3Parser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::Sparql => match sparql_syntax::SparqlParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::ShEx => match shex_syntax::ShExParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
        Language::Datalog => match datalog_syntax::DatalogParser.parse(input) {
            Ok(outcome) => (false, outcome.warnings.messages),
            Err(diag) => (diag.fatal, diag.messages),
        },
    };

    convert_diagnostics(messages, fatal)
}

/// Convert raw diagnostic messages into `lsp_types::Diagnostic` values.
///
/// Phase F maps all positions to `(0, 0)` — byte-offset mapping is deferred
/// to Phase G. The `source` field is `"rdf-lsp"` on every diagnostic.
fn convert_diagnostics(messages: Vec<String>, fatal: bool) -> Vec<Diagnostic> {
    let severity = if fatal {
        DiagnosticSeverity::ERROR
    } else {
        DiagnosticSeverity::WARNING
    };

    messages
        .into_iter()
        .map(|msg| Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            severity: Some(severity),
            source: Some("rdf-lsp".to_string()),
            message: msg,
            ..Default::default()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

fn send_ok(connection: &Connection, id: RequestId, value: serde_json::Value) {
    let resp = Response { id, result: Some(value), error: None };
    if let Err(e) = connection.sender.send(Message::Response(resp)) {
        eprintln!("rdf-lsp: failed to send response: {e}");
    }
}

fn send_null_ok(connection: &Connection, id: RequestId) {
    send_ok(connection, id, serde_json::Value::Null);
}

fn send_error(connection: &Connection, id: RequestId, code: ErrorCode, message: &str) {
    let resp = Response {
        id,
        result: None,
        error: Some(lsp_server::ResponseError {
            code: code as i32,
            message: message.to_string(),
            data: None,
        }),
    };
    if let Err(e) = connection.sender.send(Message::Response(resp)) {
        eprintln!("rdf-lsp: failed to send error response: {e}");
    }
}

/// Cast a generic `Request` to a typed LSP request `R`.
///
/// Returns `Ok((id, params))` on success, `Err(id)` if params cannot be
/// deserialised (which should not happen with a well-behaved client).
fn cast_request<R>(req: Request) -> Result<(RequestId, R::Params), RequestId>
where
    R: lsp_types::request::Request,
{
    let id = req.id.clone();
    match serde_json::from_value::<R::Params>(req.params) {
        Ok(params) => Ok((id, params)),
        Err(_) => Err(id),
    }
}
