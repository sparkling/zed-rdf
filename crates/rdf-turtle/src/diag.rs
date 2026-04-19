//! Diagnostic codes emitted by the Turtle / TriG parser.
//!
//! Codes map to the pins under `docs/spec-readings/turtle/` and to the
//! general `TTL-*` grammar family. Only the codes we actively emit are
//! enumerated; adding a new code requires a spec-reading pin.

use std::fmt;

/// Structured diagnostic code. Stable across the verification-v1 sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticCode {
    /// Pin `TTL-LITESC-001`: literal escape / forbidden raw-char error in
    /// short-quoted strings; unknown `\ch` escape; surrogate `UCHAR`.
    LitEsc,
    /// Pin `TTL-BNPFX-001`: purely informational; surfaces when the
    /// document-scope bnode rule was exercised (never fatal).
    BnPfx,
    /// Generic syntax error not otherwise classified.
    Syntax,
    /// Unexpected end-of-input during a production.
    UnexpectedEof,
    /// Unclosed IRI / string / collection / graph block.
    Unterminated,
    /// Percent / `PN_LOCAL_ESC` shape error inside a prefixed name.
    LocalEscape,
    /// Numeric literal lexically valid but outside the advertised
    /// category; currently not emitted (reserved).
    NumericShape,
    /// An `@prefix` / `@base` directive was missing its terminator `.`.
    DirectiveTerminator,
    /// A relative IRI was used with no base established.
    NoBase,
    /// An undeclared prefix was used in a prefixed name.
    UndeclaredPrefix,
}

impl DiagnosticCode {
    /// Short code string (e.g. `"TTL-LITESC-001"`).
    #[must_use]
    pub const fn as_code(self) -> &'static str {
        match self {
            Self::LitEsc => "TTL-LITESC-001",
            Self::BnPfx => "TTL-BNPFX-001",
            Self::Syntax => "TTL-SYNTAX-001",
            Self::UnexpectedEof => "TTL-EOF-001",
            Self::Unterminated => "TTL-UNTERM-001",
            Self::LocalEscape => "TTL-PNLOC-001",
            Self::NumericShape => "TTL-NUM-001",
            Self::DirectiveTerminator => "TTL-DIR-001",
            Self::NoBase => "TTL-BASE-001",
            Self::UndeclaredPrefix => "TTL-PFX-001",
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
