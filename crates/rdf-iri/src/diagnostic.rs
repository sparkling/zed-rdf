//! Crate-local diagnostic stub.
//!
//! `rdf-diagnostics` has not yet landed at the time this crate was
//! written; the types defined here are deliberately small and
//! swap-compatible. A follow-up PR replaces this module with a re-export
//! from `rdf-diagnostics`.
//!
//! Codes follow the template pinned in
//! `docs/spec-readings/iri/percent-encoding-3986-vs-3987.md`:
//!
//! - `IRI-PCT-001` — percent-encoding / equality pin violation.
//! - `IRI-SYNTAX-001` — general RFC 3987 syntax rejection.
//! - `IRI-SYNTAX-002` — authority / host syntax rejection.
//! - `IRI-PORT-001` — port subcomponent syntax rejection.
//! - `IRI-SCHEME-001` — scheme subcomponent syntax rejection.
//! - `IRI-URI-001` — IRI → URI mapping rejection (e.g., control char).

use std::fmt;

use thiserror::Error;

/// A structured diagnostic code. Stable identifiers suitable for
/// triage hints in `rdf-diff::DiffReport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// IRI equality / percent-encoding pin violation.
    PercentEncoding,
    /// General RFC 3987 grammar rejection.
    Syntax,
    /// Authority or host subcomponent rejection.
    Authority,
    /// Port subcomponent rejection.
    Port,
    /// Scheme subcomponent rejection.
    Scheme,
    /// IRI → URI mapping rejection.
    UriMapping,
}

impl DiagnosticCode {
    /// Stable string identifier, used in diagnostic messages and in
    /// diff-harness triage hints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PercentEncoding => "IRI-PCT-001",
            Self::Syntax => "IRI-SYNTAX-001",
            Self::Authority => "IRI-SYNTAX-002",
            Self::Port => "IRI-PORT-001",
            Self::Scheme => "IRI-SCHEME-001",
            Self::UriMapping => "IRI-URI-001",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A parse-time or conversion-time diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{code}: {message}")]
pub struct Diagnostic {
    /// Stable code for triage.
    pub code: DiagnosticCode,
    /// Human-readable detail.
    pub message: String,
    /// 0-indexed byte offset into the original input, when known.
    pub offset: Option<usize>,
}

impl Diagnostic {
    /// Construct a new diagnostic.
    #[must_use]
    pub fn new(
        code: DiagnosticCode,
        message: impl Into<String>,
        offset: Option<usize>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            offset,
        }
    }

    /// Convenience constructor for the `IRI-PCT-001` pin code.
    #[must_use]
    pub fn percent_encoding(message: impl Into<String>, offset: Option<usize>) -> Self {
        Self::new(DiagnosticCode::PercentEncoding, message, offset)
    }
}
