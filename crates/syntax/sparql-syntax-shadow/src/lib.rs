//! Independent shadow implementation of a SPARQL 1.1 grammar-only parser.
//!
//! This crate is a **disjoint** second implementation written for the
//! verification-v1 sweep (ADR-0020). It intentionally uses a different
//! internal structure from the main `sparql-syntax` crate so that the
//! diff-harness can catch bugs in either independently.
//!
//! The parser implements [`rdf_diff::Parser`] and is gated behind the
//! `shadow` feature flag. Without the `shadow` feature the crate compiles
//! as an empty shell, satisfying workspace membership at zero cost. It
//! emits a canonical AST-as-Facts encoding described in `README.md`.
//!
//! # Scope
//!
//! Grammar-only: the parser produces a structural fact set from the
//! SPARQL 1.1 Query Language grammar (W3C Rec 21 March 2013). No query
//! execution, no dataset evaluation, no variable binding.
//!
//! Covered constructs:
//! - SELECT / CONSTRUCT / ASK / DESCRIBE
//! - SPARQL 1.1 Update: DELETE DATA / INSERT DATA / DELETE/INSERT,
//!   LOAD, CLEAR, CREATE, DROP, COPY, MOVE, ADD
//! - SERVICE (federated query)
//! - Property paths (|, /, ^, ?, *, +, !)
//! - BIND, subquery projections, VALUES
//! - Literal lexical forms preserved exactly (no numeric normalisation)
//! - Language tags and datatype IRIs carried through
//!
//! # References
//!
//! - W3C SPARQL 1.1 Query Language: <https://www.w3.org/TR/sparql11-query/>
//! - W3C SPARQL 1.1 Update: <https://www.w3.org/TR/sparql11-update/>
//! - W3C SPARQL 1.1 Federation Extensions: <https://www.w3.org/TR/sparql11-federated-query/>

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Pedantic-level lints: these are style lints that fire heavily in a
// hand-written recursive-descent parser. They do not affect correctness.
#![allow(clippy::too_many_lines)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::while_let_loop)]
#![allow(clippy::branches_sharing_code)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::unused_self)]
#![allow(clippy::use_self)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::useless_format)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::single_match_else)]
#![allow(clippy::redundant_else)]
#![allow(clippy::unnested_or_patterns)]
#![allow(clippy::iter_with_drain)]
#![allow(clippy::needless_continue)]
#![allow(clippy::wildcard_in_or_patterns)]
#![allow(clippy::match_wildcard_for_single_variants)]

#[cfg(feature = "shadow")]
pub mod ast;
#[cfg(feature = "shadow")]
pub mod encode;
#[cfg(feature = "shadow")]
pub mod lexer;
#[cfg(feature = "shadow")]
pub mod parser;

#[cfg(feature = "shadow")]
mod error;

#[cfg(feature = "shadow")]
pub use error::ParseError;
#[cfg(feature = "shadow")]
pub use rdf_diff::{Diagnostics, Facts, ParseOutcome, Parser};

/// The shadow SPARQL 1.1 parser entry point.
///
/// Implements [`rdf_diff::Parser`] by:
/// 1. Lexing the input via [`lexer`].
/// 2. Parsing a SPARQL query or update via [`parser`].
/// 3. Encoding the AST as a fact set via [`encode`].
#[cfg(feature = "shadow")]
pub struct SparqlShadowParser;

#[cfg(feature = "shadow")]
impl Parser for SparqlShadowParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let src = match std::str::from_utf8(input) {
            Ok(s) => s,
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![format!("input is not valid UTF-8: {e}")],
                    fatal: true,
                });
            }
        };

        let tokens = match lexer::tokenise(src) {
            Ok(t) => t,
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![format!("lex error: {e}")],
                    fatal: true,
                });
            }
        };

        let (document, warnings) = match parser::parse_document(&tokens, src) {
            Ok(pair) => pair,
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![format!("parse error: {e}")],
                    fatal: true,
                });
            }
        };

        let facts = encode::encode_document(&document);

        let warn_diag = Diagnostics {
            messages: warnings,
            fatal: false,
        };

        Ok(ParseOutcome {
            facts,
            warnings: warn_diag,
        })
    }

    fn id(&self) -> &'static str {
        "sparql-syntax-shadow"
    }
}
