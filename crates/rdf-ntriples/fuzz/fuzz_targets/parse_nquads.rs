//! Fuzz target: `NQuadsParser::parse` on arbitrary bytes.
//!
//! Same invariants as the `parse_ntriples` target; additionally we
//! assert that the graph projection is consistent with the grammar
//! (accepted inputs either all lack a graph name or it is an
//! absolute-IRI-shaped string).

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_diff::Parser;
use rdf_ntriples::NQuadsParser;

fuzz_target!(|data: &[u8]| {
    match NQuadsParser.parse(data) {
        Ok(outcome) => {
            assert!(
                outcome.facts.set.len() <= data.len().saturating_add(1),
                "fact count exceeded O(input) bound",
            );
            assert!(!outcome.warnings.fatal);
            for fact in outcome.facts.set.keys() {
                if let Some(g) = fact.graph.as_deref() {
                    // Graph terms are either IRIs (<...>) or blank
                    // nodes (_:...). Anything else would be a grammar
                    // violation the parser was supposed to reject.
                    assert!(
                        g.starts_with('<') || g.starts_with("_:"),
                        "illegal graph shape {g:?} surfaced from Ok() path",
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
