//! Fuzz target: `ShExParser::parse` on arbitrary bytes.
//!
//! Invariants (ADR-0018 §§3–4):
//!
//! 1. **No panics.** Any byte slice must produce either
//!    `Ok(ParseOutcome)` or `Err(Diagnostics)`. The parser's job is
//!    to emit a diagnostic, not to unwind.
//! 2. **Structured rejection.** `Err` must carry a non-empty
//!    `messages` vector with `fatal: true`. The harness never
//!    string-compares messages — the shape is the contract.
//! 3. **Acceptance implies bounded output.** Accepted inputs produce
//!    at most `O(input.len())` facts (guards against a pathological
//!    accidental-duplication explosion).

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_diff::Parser;
use shex_syntax::ShExParser;

fuzz_target!(|data: &[u8]| {
    match ShExParser::new().parse(data) {
        Ok(outcome) => {
            // Invariant 3: linear bound in input length.
            assert!(
                outcome.facts.set.len() <= data.len().saturating_add(1),
                "fact count ({}) exceeded O(input) bound for {} input bytes",
                outcome.facts.set.len(),
                data.len(),
            );
            // Warnings, if any, must carry `fatal: false`.
            assert!(
                !outcome.warnings.fatal,
                "Ok() path must never emit a fatal warning",
            );
        }
        Err(diag) => {
            // Invariant 2: structured diagnostic shape.
            assert!(diag.fatal, "Err() path must carry fatal: true");
            assert!(
                !diag.messages.is_empty(),
                "Err() path must carry at least one message",
            );
        }
    }
});
