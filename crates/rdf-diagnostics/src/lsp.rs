//! Optional LSP bridge — `impl From<Diagnostic> for lsp_types::Diagnostic`.
//!
//! Compiled only when the `lsp` feature is enabled.  The `rdf-lsp`
//! crate opts in; parser crates and the foundations layer stay
//! LSP-agnostic by default.
//!
//! ## Limitations
//!
//! - [`Span`](crate::Span) is byte-offset-based; LSP positions are
//!   `(line, UTF-16 column)`.  The bridge here cannot perform that
//!   conversion because it does not have the source text.  It emits
//!   a **placeholder** `Range` at `(0, span.start)`–`(0, span.end)`
//!   with byte offsets substituted for UTF-16 columns; the `rdf-lsp`
//!   crate is expected to post-process diagnostics and replace the
//!   placeholder with the real UTF-16 range it computes from the
//!   document it already holds.
//! - [`Related`](crate::Related) notes are dropped in the bridge
//!   because LSP's `DiagnosticRelatedInformation` requires a
//!   `Location` (URI + range), which this crate does not know.  The
//!   LSP layer attaches related info itself.
//!
//! These limitations keep `rdf-diagnostics` file- and URI-free, which
//! is why it can be a dependency of every parser without dragging LSP
//! types through the workspace.

use lsp_types::{
    Diagnostic as LspDiagnostic, DiagnosticSeverity, NumberOrString, Position, Range,
};

use crate::{Diagnostic, Severity};

impl From<Severity> for DiagnosticSeverity {
    fn from(value: Severity) -> Self {
        match value {
            Severity::Error => Self::ERROR,
            Severity::Warning => Self::WARNING,
            Severity::Info => Self::INFORMATION,
            Severity::Hint => Self::HINT,
        }
    }
}

impl From<Diagnostic> for LspDiagnostic {
    fn from(d: Diagnostic) -> Self {
        // Placeholder range; see module-level docs.  `u32::try_from`
        // saturates at `u32::MAX` because LSP positions are `u32`.
        let start = u32::try_from(d.span.start).unwrap_or(u32::MAX);
        let end = u32::try_from(d.span.end).unwrap_or(u32::MAX);
        let range = Range {
            start: Position { line: 0, character: start },
            end: Position { line: 0, character: end },
        };
        let message = match d.hint {
            Some(hint) => format!("{}\nhint: {}", d.message, hint),
            None => d.message,
        };
        Self {
            range,
            severity: Some(d.severity.into()),
            code: Some(NumberOrString::String(d.code.to_owned())),
            code_description: None,
            source: Some("rdf".to_owned()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }
}
