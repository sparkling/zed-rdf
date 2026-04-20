//! Diagnostics handler — parses a document and converts `rdf_diagnostics`
//! errors into LSP `PublishDiagnosticsParams`.
//!
//! This module is consumed by the protocol layer (`dispatch.rs`) after every
//! `didOpen` and `didChange` notification.  Phase F positions every diagnostic
//! at `(0, 0)` because byte-offset-to-line mapping is deferred to Phase G.

use lsp_types::{
    Diagnostic, DiagnosticSeverity, Position, PublishDiagnosticsParams, Range, Url,
};

use rdf_diff::Parser as ParserTrait;

use crate::Language;

/// Parse `text` as `lang` and return `PublishDiagnosticsParams` ready for the
/// `textDocument/publishDiagnostics` notification.
///
/// All diagnostics are anchored at `(0, 0)` in Phase F (byte-offset mapping
/// is a Phase G deliverable).
#[must_use]
pub fn handle_diagnostics(
    text: &str,
    lang: Language,
    uri: &Url,
) -> PublishDiagnosticsParams {
    let diagnostics = collect_diagnostics(text, lang);
    PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    }
}

// ---------------------------------------------------------------------------
// Diagnostic collection helpers
// ---------------------------------------------------------------------------

/// Zero-range at the origin — Phase F placeholder for all diagnostic spans.
const fn origin_range() -> Range {
    let origin = Position { line: 0, character: 0 };
    Range { start: origin, end: origin }
}

fn make_diag(message: String, severity: DiagnosticSeverity) -> Diagnostic {
    Diagnostic {
        range: origin_range(),
        severity: Some(severity),
        source: Some("rdf-lsp".to_owned()),
        message,
        ..Default::default()
    }
}

/// Run the appropriate parser and collect any fatal errors and warnings.
fn collect_diagnostics(text: &str, lang: Language) -> Vec<Diagnostic> {
    match lang {
        Language::NTriples => parse_diagnostics(&rdf_ntriples::NTriplesParser, text),
        Language::NQuads => parse_diagnostics(&rdf_ntriples::NQuadsParser, text),
        Language::Turtle => parse_diagnostics(&rdf_turtle::TurtleParser, text),
        Language::TriG => parse_diagnostics(&rdf_turtle::TriGParser, text),
        // Languages without a Phase-F parser: no diagnostics reported.
        Language::RdfXml
        | Language::JsonLd
        | Language::TriX
        | Language::N3
        | Language::Sparql
        | Language::ShEx
        | Language::Datalog => vec![],
    }
}

/// Generic helper that drives any `rdf_diff::Parser` implementer.
fn parse_diagnostics<P: ParserTrait>(parser: &P, text: &str) -> Vec<Diagnostic> {
    match parser.parse(text.as_bytes()) {
        Err(fatal) => {
            // fatal is rdf_diff::Diagnostics { messages, fatal: true }
            let msg = if fatal.messages.is_empty() {
                "parse error".to_owned()
            } else {
                fatal.messages.join("; ")
            };
            vec![make_diag(msg, DiagnosticSeverity::ERROR)]
        }
        Ok(outcome) => {
            if outcome.warnings.messages.is_empty() {
                vec![]
            } else {
                let msg = outcome.warnings.messages.join("; ");
                vec![make_diag(msg, DiagnosticSeverity::WARNING)]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ntriples_yields_no_diagnostics() {
        let uri = Url::parse("file:///test.nt").unwrap();
        let text = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
        let params = handle_diagnostics(text, Language::NTriples, &uri);
        assert!(params.diagnostics.is_empty(), "valid NT should produce no diagnostics");
    }

    #[test]
    fn invalid_ntriples_yields_error_diagnostic() {
        let uri = Url::parse("file:///test.nt").unwrap();
        let text = "this is not valid ntriples\n";
        let params = handle_diagnostics(text, Language::NTriples, &uri);
        assert!(!params.diagnostics.is_empty(), "invalid NT should produce diagnostics");
        assert_eq!(
            params.diagnostics[0].severity,
            Some(DiagnosticSeverity::ERROR)
        );
    }

    #[test]
    fn sparql_yields_no_diagnostics() {
        let uri = Url::parse("file:///test.sparql").unwrap();
        let params =
            handle_diagnostics("SELECT ?s WHERE { ?s ?p ?o }", Language::Sparql, &uri);
        // SPARQL parser not available in Phase F; returns empty.
        assert!(params.diagnostics.is_empty());
    }
}
