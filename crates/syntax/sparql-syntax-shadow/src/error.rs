//! Parse error type for `sparql-syntax-shadow`.

use thiserror::Error;

/// Errors produced by the shadow SPARQL parser.
#[derive(Debug, Error)]
pub enum ParseError {
    /// The lexer encountered an unrecognised character or invalid token.
    #[error("lex error at byte {offset}: {message}")]
    Lex {
        /// Byte offset (0-indexed) where the error occurred.
        offset: usize,
        /// Description of the lex error.
        message: String,
    },

    /// The token stream does not match the SPARQL 1.1 grammar.
    #[error("syntax error near '{near}': {message}")]
    Syntax {
        /// The token text near the error site.
        near: String,
        /// Human-readable description of what was expected.
        message: String,
    },

    /// An IRI reference is malformed (not a valid IRI).
    #[error("malformed IRI <{iri}>: {message}")]
    BadIri {
        /// The IRI text that was rejected.
        iri: String,
        /// Reason the IRI was rejected.
        message: String,
    },
}

impl ParseError {
    /// Construct a [`ParseError::Lex`].
    pub(crate) fn lex(offset: usize, message: impl Into<String>) -> Self {
        Self::Lex {
            offset,
            message: message.into(),
        }
    }

    /// Construct a [`ParseError::Syntax`] at a given token.
    pub(crate) fn syntax(near: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Syntax {
            near: near.into(),
            message: message.into(),
        }
    }
}
