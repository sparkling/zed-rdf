//! [`rdf_diff::Parser`] adapter over `oxrdfxml`.
//!
//! Wraps oxigraph's `oxrdfxml::RdfXmlParser` for RDF/XML input.
//! Translation to the sweep's canonical [`Facts`] form is identical
//! to the `oxttl_adapter`: each emitted `oxrdf::Triple` is rendered
//! in its N-Triples inline form and passed through
//! [`Facts::canonicalise`].
//!
//! Per ADR-0019 §1, `oxrdfxml` is a `[dev-dependency]` only; this
//! module is gated behind `#[cfg(all(test, feature = "oracle-oxrdfxml"))]`.

use oxrdf::{NamedOrBlankNode, Term, Triple};
use oxrdfxml::RdfXmlParser;

use crate::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

/// Adapter type for `oxrdfxml::RdfXmlParser`.
#[derive(Debug, Default, Clone, Copy)]
pub struct Adapter;

impl Adapter {
    /// Construct a fresh `oxrdfxml` RDF/XML adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

const PARSER_ID: &str = "oxrdfxml-oracle";

impl Parser for Adapter {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let mut parser = RdfXmlParser::new().for_slice(input);
        let mut raw: Vec<(Fact, FactProvenance)> = Vec::new();

        while let Some(step) = parser.next() {
            match step {
                Ok(triple) => raw.push(triple_to_fact(&triple)),
                Err(err) => return Err(crate::fatal(PARSER_ID, err)),
            }
        }

        // RDF/XML does not carry Turtle-style `@prefix` declarations
        // in a form the parser surfaces; the upstream API omits them.
        // Prefixes are left empty.
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

/// Translate an `oxrdf::Triple` into a `(Fact, Provenance)` pair.
/// Duplicated by shape across ox* adapters by intent: each adapter
/// owns its upstream's idiosyncrasies (error types, iterator protocol)
/// and converges on the same inline form here.
fn triple_to_fact(t: &Triple) -> (Fact, FactProvenance) {
    (
        Fact {
            subject: subject_to_string(&t.subject),
            predicate: t.predicate.as_str().to_string(),
            object: term_to_string(&t.object),
            graph: None,
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
