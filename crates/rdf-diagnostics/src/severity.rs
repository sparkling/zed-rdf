//! Diagnostic severity levels.

/// Severity of a [`Diagnostic`](crate::Diagnostic).
///
/// Ordering matches the LSP convention: `Error` is the most severe,
/// `Hint` the least.  The ordering is used by
/// [`DiagnosticBag::is_fatal`](crate::DiagnosticBag::is_fatal) — only
/// [`Severity::Error`] is fatal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    /// A problem that prevents a valid parse / a valid document.  The
    /// parser may still emit partial facts after error recovery, but
    /// the document is not conformant.
    Error,
    /// A problem that does not prevent parsing but that a reasonable
    /// author would want to fix (e.g. deprecated syntax, ambiguous
    /// escape case).
    Warning,
    /// A purely informational note (e.g. "decoded `\u00E9` as `é`" for
    /// diagnostic-verbose mode).
    Info,
    /// A low-priority suggestion (e.g. "consider adding a `@prefix`
    /// declaration here").  Typically not rendered in CLI output.
    Hint,
}

impl Severity {
    /// `true` iff this severity is fatal — currently only
    /// [`Severity::Error`].
    #[must_use]
    pub const fn is_fatal(self) -> bool {
        matches!(self, Self::Error)
    }

    /// Short, lowercase human-readable label (`"error"`, `"warning"`,
    /// `"info"`, `"hint"`).  Used by the default [`render`](crate::render)
    /// output; snapshot tests depend on these exact strings.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}
