//! Property / robustness tests for the `datalog-syntax` crate.
//!
//! **Phase D stub state:** the parser currently returns `Err(Diagnostics)`
//! for every input, which is valid stub behaviour. The tests below are
//! structured so they pass against the stub and continue to pass (with
//! stronger assertions) when the real implementation merges.
//!
//! ## Invariants
//!
//! - **PDL1 no-panic.** Feeding any byte slice to `DatalogParser::parse`
//!   must never panic. The result may be `Ok` or `Err`, but panicking is
//!   always a bug. This is verified here with a hardcoded set of known-valid
//!   Datalog programs (wrapped in `std::panic::catch_unwind`).
//!
//! ## Future work
//!
//! // TODO: expand with proptest when implementation lands (Phase D merge).
//! // Suggested invariants:
//! //   PDL2 — canonical-form idempotence: Facts produced by DatalogParser
//! //           are already in canonical form; running them through
//! //           `Facts::canonicalise` a second time is a no-op.
//! //   PDL3 — whitespace tolerance: rules separated by different amounts
//! //           of whitespace / blank lines yield identical fact sets.
//! //   PDL4 — comment stripping: programs with and without `%` or `//`
//! //           comment lines yield the same facts for the non-comment
//! //           portion.
//!
//! ## ADR references
//!
//! - ADR-0023 §2 — Phase D tester deliverables (property skeleton).
//! - ADR-0006 — testing strategy (property tests in the pyramid).

use datalog_syntax::DatalogParser;
use rdf_diff::Parser as _;

// ---------------------------------------------------------------------------
// Known-valid Datalog programs used by PDL1.
// ---------------------------------------------------------------------------
//
// These are minimal, well-formed Datalog programs (recursive rules of the
// form `Head :- Body.`). Against the stub they return Err — what we assert
// is the *absence of a panic*.
const WELL_FORMED_PROGRAMS: &[&str] = &[
    // Empty program — valid; zero rules.
    "",
    // Single EDB fact.
    "parent(tom, bob).",
    // Single rule, no recursion.
    "ancestor(X, Y) :- parent(X, Y).",
    // Recursive rule.
    "ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z).",
    // Multiple clauses, recursive ancestry.
    "parent(tom, bob).\n\
     parent(bob, ann).\n\
     ancestor(X, Y) :- parent(X, Y).\n\
     ancestor(X, Z) :- parent(X, Y), ancestor(Y, Z).",
    // Rule with multiple body literals.
    "sibling(X, Y) :- parent(Z, X), parent(Z, Y).",
    // Rule using an IRI-shaped constant (RDF-flavoured Datalog).
    "triple(<http://ex/s>, <http://ex/p>, <http://ex/o>).",
];

// ---------------------------------------------------------------------------
// PDL1 — no-panic on well-formed inputs
// ---------------------------------------------------------------------------

/// PDL1: `DatalogParser::parse` must not panic on any well-formed Datalog
/// program.
///
/// The result may be `Ok(ParseOutcome)` or `Err(Diagnostics)` — both are
/// acceptable. A panic is always a bug.
///
/// Each program is tested in its own `catch_unwind` so a failure on one
/// does not mask failures on the others.
#[test]
fn parse_well_formed_programs_does_not_panic() {
    let parser = DatalogParser::new();

    for program in WELL_FORMED_PROGRAMS {
        let input = program.as_bytes();
        let result = std::panic::catch_unwind(|| {
            // DatalogParser is stateless (no captured mutable state), so
            // calling parse from a closure is safe even across unwind.
            let p = DatalogParser::new();
            let _ = p.parse(input);
        });

        assert!(
            result.is_ok(),
            "DatalogParser::parse panicked on program: {program:?}",
        );

        // Additionally call through the already-constructed parser to
        // ensure the same parser instance can be reused (stateless contract).
        let _ = parser.parse(input);
    }
}

// ---------------------------------------------------------------------------
// PDL1b — no-panic on adversarial byte sequences
// ---------------------------------------------------------------------------

/// PDL1b: `DatalogParser::parse` must not panic on adversarial byte sequences.
///
/// These inputs are intentionally malformed or edge-case byte patterns that
/// could trigger panics in a naive implementation.
#[test]
fn parse_adversarial_bytes_does_not_panic() {
    let adversarial: &[&[u8]] = &[
        b"",
        b"\x00",
        b"\xff\xfe",
        b":-",          // naked neck without head
        b"head :- .",   // empty body
        b".()",
        &[0x80, 0x81, 0x82], // invalid UTF-8 continuation bytes
    ];

    for input in adversarial {
        let result = std::panic::catch_unwind(|| {
            let p = DatalogParser::new();
            let _ = p.parse(input);
        });

        assert!(
            result.is_ok(),
            "DatalogParser::parse panicked on adversarial input: {input:?}",
        );
    }
}
