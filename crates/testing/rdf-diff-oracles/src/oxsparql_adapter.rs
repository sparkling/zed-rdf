//! [`rdf_diff::Parser`] adapter over oxigraph's SPARQL syntax crate
//! (`spargebra`).
//!
//! ADR-0019 §1 names this oracle role `oxsparql-syntax`. That crate is
//! unpublished; oxigraph ships its SPARQL 1.1 parser as
//! [`spargebra`](https://crates.io/crates/spargebra) (same source tree,
//! same maintainer, MIT/Apache-2.0). The ADR governs the *role*, so
//! we pin `spargebra` and document the rename here.
//!
//! ## Semantics for the diff harness
//!
//! SPARQL queries and updates are **not** fact-producing in the same
//! sense as RDF parsers: they describe operations, not an RDF graph.
//! The sweep's interest is `accept/reject` parity — whether our
//! `sparql-syntax` crate accepts exactly the same SPARQL 1.1 surface
//! as a reference parser. This adapter therefore returns an empty
//! [`Facts`] set on success and a fatal [`Diagnostics`] on syntax
//! error; [`crate::Divergence::AcceptRejectSplit`] is the sole
//! divergence category it can trigger.
//!
//! Per ADR-0019 §1, `spargebra` is a `[dev-dependency]` only; this
//! module is gated behind `#[cfg(all(test, feature = "oracle-oxsparql"))]`.

use spargebra::SparqlParser;

use crate::{Diagnostics, Facts, ParseOutcome, Parser};

/// Adapter type for `spargebra`.
///
/// The adapter probes SPARQL **query** syntax first; on failure, it
/// falls back to **update** syntax. This mirrors how a combined parser
/// would dispatch and keeps the oracle symmetric with our shadow's
/// accept surface.
#[derive(Debug, Default, Clone, Copy)]
pub struct Adapter;

impl Adapter {
    /// Construct a fresh `spargebra` SPARQL adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

const PARSER_ID: &str = "spargebra-oracle";

impl Parser for Adapter {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        // SPARQL is text-only. Reject non-UTF8 input up front; a
        // lossy conversion would mask accept/reject divergences.
        let Ok(text) = std::str::from_utf8(input) else {
            return Err(crate::fatal(PARSER_ID, "input is not valid UTF-8"));
        };

        let query_err = match SparqlParser::new().parse_query(text) {
            Ok(_) => {
                return Ok(ParseOutcome {
                    facts: Facts::default(),
                    warnings: Diagnostics {
                        messages: Vec::new(),
                        fatal: false,
                    },
                });
            }
            Err(err) => err,
        };

        // Not a query — try update. Only if both fail is the input
        // rejected; messages from both attempts are surfaced so
        // triage can see why each side failed.
        match SparqlParser::new().parse_update(text) {
            Ok(_) => Ok(ParseOutcome {
                facts: Facts::default(),
                warnings: Diagnostics {
                    messages: Vec::new(),
                    fatal: false,
                },
            }),
            Err(update_err) => Err(Diagnostics {
                messages: vec![
                    format!("{PARSER_ID}: not a valid query: {query_err}"),
                    format!("{PARSER_ID}: not a valid update: {update_err}"),
                ],
                fatal: true,
            }),
        }
    }

    fn id(&self) -> &'static str {
        PARSER_ID
    }
}
