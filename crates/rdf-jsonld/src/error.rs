//! Error type for the JSON-LD parser.

use rdf_diff::Diagnostics;

/// A JSON-LD parse error, convertible to [`rdf_diff::Diagnostics`].
#[derive(Debug)]
pub struct JsonLdError {
    /// Human-readable description.
    pub message: String,
}

impl From<JsonLdError> for Diagnostics {
    fn from(e: JsonLdError) -> Self {
        Self {
            messages: vec![e.message],
            fatal: true,
        }
    }
}

/// Convenience alias for fallible JSON-LD operations.
pub type Result<T> = std::result::Result<T, JsonLdError>;

/// Create a fatal JSON-LD error.
pub fn jsonld_err(msg: impl Into<String>) -> JsonLdError {
    JsonLdError { message: msg.into() }
}
