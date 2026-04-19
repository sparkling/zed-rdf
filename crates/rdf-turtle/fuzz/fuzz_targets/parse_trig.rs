//! Fuzz target: `TriGParser::parse` on arbitrary bytes.
//!
//! Same invariant set as `parse_turtle`, with the addition that any
//! graph term surfaced on the Ok() path must be IRI- or bnode-shaped
//! — a defensive check that the TriG `GRAPH … { … }` block parser
//! cannot smuggle an unrecognised term through canonicalisation.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_diff::Parser;
use rdf_turtle::TriGParser;

fuzz_target!(|data: &[u8]| {
    match TriGParser.parse(data) {
        Ok(outcome) => {
            assert!(
                outcome.facts.set.len() <= data.len().saturating_add(1),
                "fact count exceeded O(input) bound",
            );
            assert!(!outcome.warnings.fatal);
            for fact in outcome.facts.set.keys() {
                if let Some(g) = fact.graph.as_deref() {
                    assert!(
                        g.starts_with('<') || g.starts_with("_:"),
                        "illegal graph shape {g:?} surfaced from TriG Ok() path",
                    );
                }
            }
        }
        Err(diag) => {
            assert!(diag.fatal);
            assert!(!diag.messages.is_empty());
        }
    }
});
