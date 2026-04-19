//! The core [`Diagnostic`] value type.

use crate::{Severity, Span};

/// A single structured parser diagnostic.
///
/// `Diagnostic` values are the boundary type between parser crates and
/// their consumers (CLI, LSP, snapshot harness).  Parsers build them
/// via the [`Diagnostic::new`] constructor and the builder-style
/// [`Diagnostic::with_hint`] / [`Diagnostic::with_related`] helpers,
/// then push them into a [`DiagnosticBag`](crate::DiagnosticBag).
///
/// # Fields
///
/// - `severity` — see [`Severity`].
/// - `code` — a stable identifier tied to a spec-reading or rule
///   (e.g. `"NT-LITESC-001"` per
///   `docs/spec-readings/ntriples/literal-escapes.md`).  Consumers
///   rely on this for triage; once minted a code is immutable.
/// - `message` — one-line, human-readable description.  Not
///   localised — English, present tense.
/// - `span` — the primary byte range the diagnostic is about.
/// - `hint` — optional follow-up sentence suggesting a fix.
/// - `related` — optional secondary [`(Span, String)`] notes (see
///   [`Related`]).
///
/// # Equality
///
/// Two `Diagnostic` values are equal iff every field is equal.
/// Snapshot tests rely on this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity of this diagnostic — see [`Severity`].
    pub severity: Severity,
    /// Stable diagnostic code (e.g. `"NT-LITESC-001"`).  Parsers mint
    /// codes per their own convention; the registry of minted codes
    /// lives under `docs/spec-readings/` per ADR-0018.
    pub code: &'static str,
    /// One-line English description of the problem.
    pub message: String,
    /// Primary source span.
    pub span: Span,
    /// Optional follow-up suggesting a fix.  Rendered on its own line
    /// after the caret; LSP consumers surface it via `relatedInformation`.
    pub hint: Option<String>,
    /// Secondary notes (e.g. "the matching `(` is here").  May be
    /// empty; never `None` to keep the field match-exhaustive.
    pub related: Vec<Related>,
}

/// A secondary span + message note attached to a primary [`Diagnostic`].
///
/// Equivalent to LSP `DiagnosticRelatedInformation` minus the file
/// location (this crate is single-file by design; multi-file notes
/// are added at the LSP layer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Related {
    /// Source span the note is about.
    pub span: Span,
    /// Note text.
    pub message: String,
}

impl Diagnostic {
    /// Construct a new diagnostic with the four required fields.
    /// `hint` is `None`, `related` is empty.
    #[must_use]
    pub fn new(
        severity: Severity,
        code: &'static str,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            span,
            hint: None,
            related: Vec::new(),
        }
    }

    /// Shortcut for `Diagnostic::new(Severity::Error, …)`.
    #[must_use]
    pub fn error(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self::new(Severity::Error, code, message, span)
    }

    /// Shortcut for `Diagnostic::new(Severity::Warning, …)`.
    #[must_use]
    pub fn warning(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self::new(Severity::Warning, code, message, span)
    }

    /// Builder: attach a `hint` sentence.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Builder: append a [`Related`] note.
    #[must_use]
    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.related.push(Related {
            span,
            message: message.into(),
        });
        self
    }

    /// `true` iff `self.severity.is_fatal()`.  Convenience shortcut.
    #[must_use]
    pub const fn is_fatal(&self) -> bool {
        self.severity.is_fatal()
    }
}
