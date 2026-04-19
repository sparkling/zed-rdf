//! [`rdf_diff::Parser`] adapter over `oxjsonld`.
//!
//! Wraps oxigraph's `oxjsonld::JsonLdParser`. JSON-LD decodes to a
//! stream of `oxrdf::Quad` (JSON-LD supports named graphs natively);
//! each quad is rendered in its inline canonical form and handed to
//! [`Facts::canonicalise`].
//!
//! Per ADR-0019 §1, `oxjsonld` is a `[dev-dependency]` only; this
//! module is gated behind `#[cfg(all(test, feature = "oracle-oxjsonld"))]`.

use oxjsonld::JsonLdParser;
use oxrdf::{GraphName, NamedOrBlankNode, Quad, Term};

use crate::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

/// Adapter type for `oxjsonld::JsonLdParser`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Adapter;

impl Adapter {
    /// Construct a fresh `oxjsonld` JSON-LD adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

const PARSER_ID: &str = "oxjsonld-oracle";

impl Parser for Adapter {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let mut parser = JsonLdParser::new().for_slice(input);
        let mut raw: Vec<(Fact, FactProvenance)> = Vec::new();

        while let Some(step) = parser.next() {
            match step {
                Ok(quad) => raw.push(quad_to_fact(&quad)),
                Err(err) => return Err(crate::fatal(PARSER_ID, err)),
            }
        }

        let facts = Facts::canonicalise(raw, std::collections::BTreeMap::new());
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

/// Translate an `oxrdf::Quad` into a `(Fact, Provenance)` pair. JSON-LD
/// emits default-graph triples with `GraphName::DefaultGraph`, which
/// maps to `None` in the sweep's `Fact::graph`.
fn quad_to_fact(q: &Quad) -> (Fact, FactProvenance) {
    (
        Fact {
            subject: subject_to_string(&q.subject),
            predicate: q.predicate.as_str().to_string(),
            object: term_to_string(&q.object),
            graph: graph_to_string(&q.graph_name),
        },
        FactProvenance {
            offset: None,
            parser: PARSER_ID.to_string(),
        },
    )
}

fn subject_to_string(s: &NamedOrBlankNode) -> String {
    s.to_string()
}

fn term_to_string(t: &Term) -> String {
    t.to_string()
}

/// `GraphName::DefaultGraph` → `None`; named graphs render in their
/// N-Triples inline form via `Display`.
fn graph_to_string(g: &GraphName) -> Option<String> {
    match g {
        GraphName::DefaultGraph => None,
        other => Some(other.to_string()),
    }
}
