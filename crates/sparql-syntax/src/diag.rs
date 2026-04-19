//! Diagnostic codes emitted by the SPARQL 1.1 grammar parser.
//!
//! Codes map to pins under `docs/spec-readings/sparql/`. Only the codes
//! this crate actively surfaces are enumerated; adding a new variant
//! requires a fresh pin per ADR-0018.

use std::fmt;

/// Structured diagnostic code. Stable across the verification-v1 sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// Pin `SPARQL-LITCMP-001`: literal comparison semantics — deferred to
    /// the evaluator. Non-fatal when emitted from syntax.
    LitCmp,
    /// Pin `SPARQL-PROLOGUE-001`: `BASE` / `PREFIX` appear only in the
    /// Prologue (§4.1); mid-query use is a parse error. Maps to
    /// adversary-brief Failure Mode 5.
    Prologue,
    /// Pin `SPARQL-BIND-001`: `BIND` introduces a variable; that variable
    /// MUST NOT already appear in the surrounding group graph pattern up
    /// to the point of BIND (§18.2.1). Maps to adversary-brief Failure
    /// Mode 11b.
    BindScope,
    /// Pin `SPARQL-UPDATE-001`: `INSERT DATA` / `DELETE DATA` forbid
    /// variables (§3.1.1 / §3.1.2). `DELETE DATA` additionally forbids
    /// blank nodes.
    UpdateDataForm,
    /// Pin `SPARQL-PATH-001`: property path precedence — `^` binds the
    /// whole PathElt, so `^!(p)` means `^(!(p))`, not `!(^p)`. Maps to
    /// adversary-brief Failure Mode 9.
    PathPrecedence,
    /// Pin `SPARQL-AGG-001`: aggregate scope — inside `GROUP BY` the
    /// expression must not contain an aggregate; `HAVING` may refer to
    /// `SELECT` aggregate aliases (§11.4).
    AggregateScope,
    /// Generic syntax error not otherwise classified.
    Syntax,
    /// Unexpected end-of-input during a production.
    UnexpectedEof,
    /// Unclosed IRI / string / block.
    Unterminated,
    /// Invalid UTF-8 in the source byte input.
    InvalidUtf8,
    /// IRI validation failed at the grammar boundary (delegated to
    /// `rdf-iri`).
    IriInvalid,
    /// A relative IRI was used with no `BASE` established.
    NoBase,
    /// An undeclared prefix was used in a prefixed name.
    UndeclaredPrefix,
    /// A literal escape / UCHAR error in a string literal.
    LitEsc,
}

impl DiagnosticCode {
    /// Short code string (e.g. `"SPARQL-LITCMP-001"`).
    #[must_use]
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::LitCmp => "SPARQL-LITCMP-001",
            Self::Prologue => "SPARQL-PROLOGUE-001",
            Self::BindScope => "SPARQL-BIND-001",
            Self::UpdateDataForm => "SPARQL-UPDATE-001",
            Self::PathPrecedence => "SPARQL-PATH-001",
            Self::AggregateScope => "SPARQL-AGG-001",
            Self::Syntax => "SPARQL-SYNTAX-001",
            Self::UnexpectedEof => "SPARQL-EOF-001",
            Self::Unterminated => "SPARQL-UNTERM-001",
            Self::InvalidUtf8 => "SPARQL-UTF8-001",
            Self::IriInvalid => "SPARQL-IRI-001",
            Self::NoBase => "SPARQL-BASE-001",
            Self::UndeclaredPrefix => "SPARQL-PFX-001",
            Self::LitEsc => "SPARQL-LITESC-001",
        }
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_code())
    }
}

/// A rendered diagnostic — code plus message plus byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    /// Structured code.
    pub code: DiagnosticCode,
    /// Human-readable message.
    pub message: String,
    /// 0-indexed byte offset into the input.
    pub offset: usize,
    /// Whether the diagnostic is fatal (rejects) or a warning.
    pub fatal: bool,
}

impl Diag {
    /// Build a fatal diagnostic.
    #[must_use]
    pub fn fatal(code: DiagnosticCode, message: impl Into<String>, offset: usize) -> Self {
        Self {
            code,
            message: message.into(),
            offset,
            fatal: true,
        }
    }

    /// Build a non-fatal diagnostic (warning).
    #[must_use]
    pub fn warn(code: DiagnosticCode, message: impl Into<String>, offset: usize) -> Self {
        Self {
            code,
            message: message.into(),
            offset,
            fatal: false,
        }
    }

    /// Render the diagnostic to the message template used by the pins.
    #[must_use]
    pub fn render(&self) -> String {
        format!(
            "{}: {} at byte {}",
            self.code.as_code(),
            self.message,
            self.offset,
        )
    }
}
