//! ShEx 2.x compact syntax parser (Phase D, ADR-0023 §2).
//!
//! Grammar-only — parses ShEx compact syntax (ShExC) and emits structural
//! facts. Does **not** perform ShEx validation (checking whether RDF graphs
//! conform to a schema). That is explicitly out of scope.
//!
//! # Fact vocabulary
//!
//! All emitted predicates use the `<urn:x-shex-syntax:*>` namespace.
//! The top-level schema subject is `<urn:x-shex-syntax:schema>`.
//!
//! # Grammar decisions
//!
//! See [`parser`] module for detailed grammar notes. In brief:
//! - `PREFIX name: <iri>` and `BASE <iri>` (SPARQL-style, case-insensitive).
//! - `@prefix name: <iri>` and `@base <iri>` (Turtle-style).
//! - Shape declarations: `<label> { tripleConstraint* }`.
//! - Node constraints: `xsd:string`, `IRI`, `LITERAL`, `NONLITERAL`, `BNODE`.
//! - Triple constraints with cardinality: `*`, `+`, `?`, `{n}`, `{n,m}`, `{n,}`.
//! - Shape references: `@ex:OtherShape`.
//! - `AND`, `OR`, `NOT` shape operators.
//! - Comments `#`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::doc_markdown,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::redundant_pub_crate,
    clippy::missing_const_for_fn,
)]

mod ast;
mod encode;
mod lexer;
mod parser;

use rdf_diff::{Diagnostics, Facts, ParseOutcome, Parser};

const PARSER_ID: &str = "shex-syntax";

/// Stateless ShEx compact syntax parser handle.
///
/// Construct once with [`ShExParser::new`] (or `default()`) and reuse across
/// inputs. The parser is thread-safe — all state is per-`parse` call.
#[derive(Debug, Default, Clone, Copy)]
pub struct ShExParser;

impl ShExParser {
    /// Create a new parser instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Parser for ShExParser {
    fn id(&self) -> &'static str {
        PARSER_ID
    }

    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let mut p = parser::Parser::new(input);
        let schema = p.parse_schema().map_err(|e| Diagnostics {
            fatal: true,
            messages: vec![format!(
                "shex-syntax: parse error at byte {}: {}",
                e.offset, e.message
            )],
        })?;

        let raw = encode::encode(&schema, PARSER_ID).map_err(|e| Diagnostics {
            fatal: true,
            messages: vec![format!(
                "shex-syntax: encode error at byte {}: {}",
                e.offset, e.message
            )],
        })?;

        let prefix_map = schema
            .prefixes
            .iter()
            .map(|(p, iri)| (p.clone(), iri.clone()))
            .collect();

        let facts = Facts::canonicalise(raw, prefix_map);
        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                messages: Vec::new(),
                fatal: false,
            },
        })
    }
}
