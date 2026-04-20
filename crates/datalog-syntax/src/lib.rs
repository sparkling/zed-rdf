//! Datalog syntax parser (Phase D, ADR-0023).
//!
//! Parses Datalog programs (recursive rules of the form `Head :- Body.`) and
//! emits structural facts. Does **not** execute programs.
//!
//! # Grammar
//!
//! ```text
//! program    ::= statement*
//! statement  ::= rule | fact | comment
//! fact       ::= atom '.'
//! rule       ::= atom ':-' body '.'
//! body       ::= literal (',' literal)*
//! literal    ::= atom | 'not' atom
//! atom       ::= relname '(' arglist? ')'
//! arglist    ::= arg (',' arg)*
//! arg        ::= variable | constant
//! variable   ::= [A-Z][a-zA-Z0-9_]*
//! constant   ::= [a-z][a-zA-Z0-9_]* | '"' [^"]* '"'
//! relname    ::= [a-z][a-zA-Z0-9_]*
//! comment    ::= '%' [^\n]* '\n'
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ast;
pub mod encode;
pub mod lexer;
pub mod parser;

use rdf_diff::{Diagnostics, ParseOutcome, Parser};

/// Stateless Datalog syntax parser handle.
pub struct DatalogParser;

impl DatalogParser {
    /// Create a new parser instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for DatalogParser {
    fn default() -> Self {
        Self
    }
}

impl Parser for DatalogParser {
    fn id(&self) -> &'static str {
        "datalog-syntax"
    }

    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        // Lex.
        let tokens = lexer::tokenise(input).map_err(|e| Diagnostics {
            fatal: true,
            messages: vec![format!("lex error at offset {}: {}", e.offset, e.message)],
        })?;

        // Parse.
        let program = parser::parse(&tokens).map_err(|e| Diagnostics {
            fatal: true,
            messages: vec![format!(
                "parse error at offset {}: {}",
                e.offset, e.message
            )],
        })?;

        // Encode AST → facts.
        let facts = encode::encode(&program);

        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                fatal: false,
                messages: vec![],
            },
        })
    }
}
