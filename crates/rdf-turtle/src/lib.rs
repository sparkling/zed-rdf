//! Main Turtle 1.1 and TriG parser.
//!
//! This crate is the Phase-A **main** implementation referenced by
//! ADR-0017 §4 and the sweep-integration contract in ADR-0020 §1.4. Its
//! shadow peer lives at `crates/syntax/rdf-turtle-shadow`; both sides
//! implement [`rdf_diff::Parser`] and are compared by the diff harness.
//!
//! # Public surface
//!
//! - [`TurtleParser`] — Turtle-only parser. Rejects TriG `{…}` graph
//!   blocks.
//! - [`TriGParser`] — accepts Turtle plus TriG named-graph blocks
//!   (§2.2).
//! - [`DiagnosticCode`] — structured error-code enum, keyed to the
//!   spec-readings pins under `docs/spec-readings/turtle/`.
//!
//! # Pinned readings
//!
//! - `TTL-LITESC-001` (literal escapes — short vs long strings, UCHAR
//!   decode, full ECHAR table).
//! - `TTL-BNPFX-001` (blank-node labels are document-scope; `@prefix`
//!   redefinitions and TriG graph blocks do **not** rescope).
//!
//! # Non-goals (verification-v1 scope)
//!
//! - Full RFC 3987 IRI normalisation (swap for `rdf-iri::Iri` later).
//! - IDNA / Punycode host normalisation.
//! - Streaming / incremental parse APIs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Pedantic lint carve-outs local to this crate. Keep narrow.
#![allow(
    clippy::doc_markdown,
    clippy::redundant_pub_crate,
    clippy::missing_const_for_fn,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::struct_field_names,
    clippy::needless_pass_by_ref_mut,
    clippy::option_if_let_else,
    clippy::manual_is_ascii_check,
    clippy::manual_strip,
    clippy::unnecessary_map_or,
    clippy::assigning_clones,
    clippy::redundant_closure,
    clippy::map_unwrap_or,
    clippy::needless_continue,
    clippy::redundant_closure_for_method_calls,
)]

mod diag;
mod grammar;
mod iri;
mod lexer;

pub use diag::{Diag, DiagnosticCode};

use rdf_diff::{Diagnostics, Facts, ParseOutcome};

use grammar::{Dialect, Parser as Inner};

/// Main Turtle 1.1 parser.
///
/// Stateless — construct with [`TurtleParser::new`] (or `default()`) and
/// reuse across inputs.
#[derive(Debug, Default, Clone, Copy)]
pub struct TurtleParser;

impl TurtleParser {
    /// Construct a fresh Turtle parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Parse `input` with an externally-supplied base IRI seeded into the
    /// parser's base-IRI slot before `@base` / `BASE` directives get a
    /// chance to override it.
    ///
    /// Intended for harness callers that know the retrieval URL of a
    /// fixture (e.g. the W3C manifests' `mf:assumedTestBase`) when the
    /// fixture itself omits a `@base` / `BASE` directive. Directives
    /// inside the input still replace the seeded base per Turtle §6.5
    /// (`base` / `sparqlBase` productions). This method is additive —
    /// the frozen [`rdf_diff::Parser::parse`] contract keeps
    /// "no external base" semantics.
    ///
    /// # Errors
    ///
    /// Propagates the same diagnostics as [`rdf_diff::Parser::parse`]
    /// ([`Diagnostics`]).
    pub fn parse_with_base(
        &self,
        input: &[u8],
        base: &str,
    ) -> Result<ParseOutcome, Diagnostics> {
        parse_with(input, Dialect::Turtle, TURTLE_ID, Some(base))
    }
}

const TURTLE_ID: &str = "rdf-turtle";

impl rdf_diff::Parser for TurtleParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        parse_with(input, Dialect::Turtle, TURTLE_ID, None)
    }

    fn id(&self) -> &'static str {
        TURTLE_ID
    }
}

/// Main TriG parser.
#[derive(Debug, Default, Clone, Copy)]
pub struct TriGParser;

impl TriGParser {
    /// Construct a fresh TriG parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// TriG analogue of [`TurtleParser::parse_with_base`] — seed a base
    /// IRI before the inner parser runs, so relative IRIs resolve even
    /// when the fixture omits a `@base` / `BASE` directive.
    ///
    /// # Errors
    ///
    /// Propagates the same diagnostics as [`rdf_diff::Parser::parse`]
    /// ([`Diagnostics`]).
    pub fn parse_with_base(
        &self,
        input: &[u8],
        base: &str,
    ) -> Result<ParseOutcome, Diagnostics> {
        parse_with(input, Dialect::TriG, TRIG_ID, Some(base))
    }
}

const TRIG_ID: &str = "rdf-trig";

impl rdf_diff::Parser for TriGParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        parse_with(input, Dialect::TriG, TRIG_ID, None)
    }

    fn id(&self) -> &'static str {
        TRIG_ID
    }
}

fn parse_with(
    input: &[u8],
    dialect: Dialect,
    parser_id: &'static str,
    base: Option<&str>,
) -> Result<ParseOutcome, Diagnostics> {
    let mut inner = Inner::new(input, dialect, parser_id);
    if let Some(b) = base {
        inner.set_initial_base(b);
    }
    if let Err(diag) = inner.parse_document() {
        return Err(Diagnostics {
            messages: vec![diag.render()],
            fatal: true,
        });
    }
    let (raw, prefixes) = inner.finish();
    let facts = Facts::canonicalise(raw, prefixes.into_iter().collect());
    Ok(ParseOutcome {
        facts,
        warnings: Diagnostics {
            messages: Vec::new(),
            fatal: false,
        },
    })
}
