//! Fuzz target: `TurtleParser::parse` on arbitrary bytes.
//!
//! Invariants (ADR-0018 §§3–4):
//!
//! 1. **No panics** on any byte string.
//! 2. **Structured rejection:** `Err(Diagnostics)` with `fatal: true`
//!    and a non-empty message vector. The harness never inspects the
//!    textual content of the messages — only their existence and
//!    shape.
//! 3. **Linear fact bound** on the accept path: the number of canonical
//!    facts does not exceed `input.len() + 1` (guards against
//!    accidental exponential blow-ups in canonicalisation).

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_diff::Parser;
use rdf_turtle::TurtleParser;

fuzz_target!(|data: &[u8]| {
    match TurtleParser.parse(data) {
        Ok(outcome) => {
            assert!(
                outcome.facts.set.len() <= data.len().saturating_add(1),
                "fact count exceeded O(input) bound",
            );
            assert!(!outcome.warnings.fatal);
        }
        Err(diag) => {
            assert!(diag.fatal);
            assert!(!diag.messages.is_empty());
        }
    }
});
