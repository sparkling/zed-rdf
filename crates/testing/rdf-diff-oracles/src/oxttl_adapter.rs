//! [`rdf_diff::Parser`] adapter over `oxttl`.
//!
//! Wraps oxigraph's `oxttl::TurtleParser` and exposes it through the
//! frozen [`Parser`] trait from `rdf-diff`. Accepts any input the
//! upstream parser accepts; translates each yielded `oxrdf::Triple`
//! into a [`Fact`] in the crate-local "inline" canonical form, then
//! hands the batch to [`Facts::canonicalise`] for sweep-wide
//! canonicalisation per ADR-0019 §2.
//!
//! ## Scope
//!
//! `oxttl` covers `N-Triples`, `N-Quads`, `Turtle`, `TriG`, and `N3`. This adapter
//! targets **Turtle** (the load-bearing format for the cohort-A
//! shadow cohort). Additional format adapters can be added as
//! sibling modules when the sweep requires them without changing this
//! file.
//!
//! ## Test-scope
//!
//! Per ADR-0019 §1 `oxttl` is a strict `[dev-dependency]`; this module
//! is therefore gated behind `#[cfg(all(test, feature = "oracle-oxttl"))]`
//! at the crate root.

use oxrdf::{NamedOrBlankNode, Term, Triple};
use oxttl::TurtleParser;

use crate::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

/// Adapter type for `oxttl::TurtleParser`.
///
/// Construct with [`Adapter::new`]; the adapter is stateless and can
/// be reused across inputs.
#[derive(Debug, Default, Clone, Copy)]
pub struct Adapter;

impl Adapter {
    /// Construct a fresh oxttl Turtle adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// Stable identifier for diff reports. Must match the sweep's agreed
/// parser id naming (see ADR-0020 §1.4 and `rdf-diff` docs).
const PARSER_ID: &str = "oxttl-oracle";

impl Parser for Adapter {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let mut parser = TurtleParser::new().for_slice(input);
        let mut raw: Vec<(Fact, FactProvenance)> = Vec::new();

        // Drain the parser. Any fatal error short-circuits; the `oxttl`
        // iterator reports each error exactly once (see oxttl docs).
        while let Some(step) = parser.next() {
            match step {
                Ok(triple) => raw.push(triple_to_fact(&triple)),
                Err(err) => return Err(crate::fatal(PARSER_ID, err)),
            }
        }

        // Translate parser-reported prefixes for provenance only; they
        // are not part of the diff. See `rdf-diff` `Facts::prefixes`.
        let prefixes = parser
            .prefixes()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let facts = Facts::canonicalise(raw, prefixes);
        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                messages: Vec::new(),
                fatal: false,
            },
        })
    }

    fn id(&self) -> &'static str {
        PARSER_ID
    }
}

/// Translate an `oxrdf::Triple` into the sweep's `(Fact, Provenance)`
/// pair. Offsets are not reported by oxttl's high-level iterator, so
/// provenance records the parser id only.
fn triple_to_fact(t: &Triple) -> (Fact, FactProvenance) {
    (
        Fact {
            subject: subject_to_string(&t.subject),
            predicate: t.predicate.as_str().to_string(),
            object: term_to_string(&t.object),
            graph: None, // Turtle is single-graph; TriG support would populate this.
        },
        FactProvenance {
            offset: None,
            parser: PARSER_ID.to_string(),
        },
    )
}

/// Render a subject in the inline pre-canonical form. The `Display`
/// impls on `oxrdf` produce N-Triples-style serialisation, which is
/// exactly the inline form `Facts::canonicalise` consumes.
fn subject_to_string(s: &NamedOrBlankNode) -> String {
    s.to_string()
}

/// Render an object in the inline pre-canonical form. See
/// [`subject_to_string`] for the rationale.
fn term_to_string(t: &Term) -> String {
    t.to_string()
}
