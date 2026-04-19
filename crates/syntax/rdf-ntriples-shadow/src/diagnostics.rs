//! Helper for accumulating structured parser diagnostics.

use rdf_diff::Diagnostics;

/// Accumulates diagnostic messages and tracks fatality.
#[derive(Debug, Default)]
pub struct DiagnosticsBuilder {
    messages: Vec<String>,
    fatal: bool,
}

impl DiagnosticsBuilder {
    /// Create a new, empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a warning (non-fatal).
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
    }

    /// Append an error and mark the parse as fatal.
    pub fn error(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
        self.fatal = true;
    }

    /// `true` if at least one fatal error has been recorded.
    #[must_use]
    pub const fn is_fatal(&self) -> bool {
        self.fatal
    }

    /// Consume the builder and produce a [`Diagnostics`] value.
    #[must_use]
    pub fn finish(self) -> Diagnostics {
        Diagnostics {
            messages: self.messages,
            fatal: self.fatal,
        }
    }

    /// Non-fatal warning-only diagnostics (all messages so far, `fatal:false`).
    #[must_use]
    pub fn as_warnings(&self) -> Diagnostics {
        Diagnostics {
            messages: self.messages.clone(),
            fatal: false,
        }
    }
}
