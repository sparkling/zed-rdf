//! Diagnostic builder for the shadow parser.
//!
//! Collects parse errors and warnings, then produces the
//! [`rdf_diff::Diagnostics`] value expected by the trait contract.

use rdf_diff::Diagnostics;

/// Accumulates diagnostic messages for a single parse run.
#[derive(Debug, Default)]
pub struct DiagnosticsBuilder {
    messages: Vec<String>,
    fatal: bool,
}

impl DiagnosticsBuilder {
    /// Create an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a fatal error message. Also sets the fatal flag.
    pub fn error(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
        self.fatal = true;
    }

    /// Push a non-fatal warning message.
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.messages.push(msg.into());
    }

    /// Whether any fatal error has been recorded.
    #[must_use]
    pub const fn has_fatal(&self) -> bool {
        self.fatal
    }

    /// Consume the builder and produce a [`Diagnostics`] value.
    #[must_use]
    pub fn build(self) -> Diagnostics {
        Diagnostics {
            messages: self.messages,
            fatal: self.fatal,
        }
    }
}
