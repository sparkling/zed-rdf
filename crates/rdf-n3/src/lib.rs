//! Main N3 (Notation3) parser — Phase B implementation.
//!
//! N3 is a superset of Turtle 1.1. This parser extends Turtle with the
//! following N3-specific productions:
//!
//! - **`@keywords` directive**: `@keywords a, is, of, has.` — enables bare
//!   keywords (without `@` prefix). Handled as a lexer-mode flag.
//! - **Quoted formulas** `{ … }`: a set of triples enclosed in braces.
//!   Treated as a named graph identified by a fresh blank-node IRI
//!   (e.g. `_:formula_N`). Triples inside emit with `Fact::graph = Some(…)`.
//! - **Logical implication** `=>` (entails): emitted as a triple with
//!   predicate `<http://www.w3.org/2000/10/swap/log#implies>`.
//! - **Reverse property path** `is P of O`: emits `(O, P, subject)` — i.e.
//!   subject and object swap relative to normal `subject P O`.
//! - **`@forAll` / `@forSome`**: parsed and skipped; a warning is appended
//!   to `ParseOutcome::warnings`.
//!
//! # Design
//!
//! The parser is a self-contained recursive-descent implementation that
//! covers the full Turtle grammar plus the N3 extensions above. It does not
//! depend on `rdf-turtle`'s internal modules (those are `pub(crate)`), but
//! it re-uses the same canonical wire format understood by `rdf-diff`.
//!
//! W3C N3 Team Submission (2011-03-28):
//! <https://www.w3.org/TeamSubmission/n3/>

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::doc_markdown,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_ref_mut,
    clippy::missing_const_for_fn,
    clippy::redundant_pub_crate,
    clippy::option_if_let_else,
    clippy::assigning_clones,
    clippy::manual_strip,
    clippy::map_unwrap_or,
    clippy::redundant_closure_for_method_calls,
    clippy::manual_is_ascii_check,
    clippy::type_complexity,
    clippy::unnecessary_map_or,
    clippy::unused_self,
    clippy::needless_continue,
)]

mod iri;
mod lexer;
mod parser;

use rdf_diff::{Diagnostics, Facts, ParseOutcome, Parser};

const N3_ID: &str = "rdf-n3";

/// Main N3 parser.
///
/// Stateless — construct with [`N3Parser::new`] and reuse across inputs.
#[derive(Debug, Default, Clone, Copy)]
pub struct N3Parser;

impl N3Parser {
    /// Construct a fresh N3 parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Parser for N3Parser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let mut inner = parser::N3GrammarParser::new(input, N3_ID);
        if let Err(msg) = inner.parse_document() {
            return Err(Diagnostics {
                messages: vec![msg],
                fatal: true,
            });
        }
        let (raw, prefixes, warnings) = inner.finish();
        let facts = Facts::canonicalise(raw, prefixes.into_iter().collect());
        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                messages: warnings,
                fatal: false,
            },
        })
    }

    fn id(&self) -> &'static str {
        N3_ID
    }
}
