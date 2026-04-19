//! Bridge test: `Diagnostic` → `lsp_types::Diagnostic`.  Only compiled
//! when the `lsp` feature is enabled.  Per the module-level docs on
//! `src/lsp.rs` the bridge emits a placeholder `Range` (byte offsets
//! substituted for UTF-16 columns); the LSP crate post-processes.

#![cfg(feature = "lsp")]

use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString};
use rdf_diagnostics::{Diagnostic, Severity, Span};

#[test]
fn severity_maps_per_lsp_spec() {
    assert_eq!(
        DiagnosticSeverity::from(Severity::Error),
        DiagnosticSeverity::ERROR
    );
    assert_eq!(
        DiagnosticSeverity::from(Severity::Warning),
        DiagnosticSeverity::WARNING
    );
    assert_eq!(
        DiagnosticSeverity::from(Severity::Info),
        DiagnosticSeverity::INFORMATION
    );
    assert_eq!(
        DiagnosticSeverity::from(Severity::Hint),
        DiagnosticSeverity::HINT
    );
}

#[test]
fn diagnostic_bridges_fields() {
    let d = Diagnostic::error("NT-LITESC-001", "surrogate escape", Span::new(5, 11))
        .with_hint("use a non-surrogate scalar value");
    let l: LspDiagnostic = d.into();

    assert_eq!(l.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(
        l.code,
        Some(NumberOrString::String("NT-LITESC-001".to_owned()))
    );
    assert_eq!(l.source.as_deref(), Some("rdf"));
    assert!(l.message.starts_with("surrogate escape\n"));
    assert!(l.message.contains("hint: use a non-surrogate scalar value"));
    // Placeholder range: byte offsets in the `character` field, line=0.
    assert_eq!(l.range.start.line, 0);
    assert_eq!(l.range.start.character, 5);
    assert_eq!(l.range.end.line, 0);
    assert_eq!(l.range.end.character, 11);
    // Related info is deliberately dropped at this layer.
    assert!(l.related_information.is_none());
}
