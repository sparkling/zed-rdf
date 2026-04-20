//! Property / robustness tests for the `shex-syntax` crate.
//!
//! **Phase D stub state:** the parser currently returns `Err(Diagnostics)`
//! for every input, which is valid stub behaviour. The tests below are
//! structured so they pass against the stub and continue to pass (with
//! stronger assertions) when the real implementation merges.
//!
//! ## Invariants
//!
//! - **PSX1 no-panic.** Feeding any byte slice to `ShExParser::parse` must
//!   never panic. The result may be `Ok` or `Err`, but panicking is always
//!   a bug. This is verified here with a hardcoded set of known-valid ShEx
//!   fragments (wrapped in `std::panic::catch_unwind`).
//!
//! ## Future work
//!
//! // TODO: expand with proptest when implementation lands (Phase D merge).
//! // Suggested invariants:
//! //   PSX2 — canonical-form idempotence: Facts produced by ShExParser are
//! //           already in canonical form; running them through
//! //           `Facts::canonicalise` a second time is a no-op.
//! //   PSX3 — round-trip whitespace tolerance: shape declarations separated
//! //           by different amounts of whitespace/newlines yield identical
//! //           fact sets.
//! //   PSX4 — prefix-declaration independence: shape constraints expressed
//! //           with and without prefix declarations yield the same canonical
//! //           facts after IRI expansion.
//!
//! ## ADR references
//!
//! - ADR-0023 §2 — Phase D tester deliverables (property skeleton).
//! - ADR-0006 — testing strategy (property tests in the pyramid).

use rdf_diff::Parser as _;
use shex_syntax::ShExParser;

// ---------------------------------------------------------------------------
// Known-valid ShEx compact-syntax fragments used by PSX1.
// ---------------------------------------------------------------------------
//
// These are minimal, well-formed ShExC expressions that the real parser must
// accept. Against the stub they will return Err, which is fine — what we
// assert is the *absence of a panic*.
const WELL_FORMED_FRAGMENTS: &[&str] = &[
    // Empty schema — valid per ShEx 2.x §2.
    "",
    // Single prefix declaration only.
    "PREFIX ex: <http://example.org/>",
    // One shape with a required IRI property.
    "PREFIX ex: <http://example.org/>\n\
     ex:PersonShape { ex:name xsd:string }",
    // Shape with cardinality constraint (one-or-more).
    "PREFIX ex: <http://example.org/>\n\
     PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
     ex:S { ex:p xsd:integer+ }",
    // Shape with optional property (zero-or-one).
    "PREFIX ex: <http://example.org/>\n\
     PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
     ex:S { ex:q xsd:string? }",
    // Shape with a closed constraint.
    "PREFIX ex: <http://example.org/>\n\
     PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
     ex:Closed CLOSED { ex:p xsd:string }",
];

// ---------------------------------------------------------------------------
// PSX1 — no-panic on well-formed inputs
// ---------------------------------------------------------------------------

/// PSX1: `ShExParser::parse` must not panic on any well-formed ShEx fragment.
///
/// The result may be `Ok(ParseOutcome)` or `Err(Diagnostics)` — both are
/// acceptable. A panic is always a bug.
///
/// Each fragment is tested in its own `catch_unwind` so a failure on one
/// does not mask failures on the others.
#[test]
fn parse_well_formed_fragments_does_not_panic() {
    let parser = ShExParser::new();

    for fragment in WELL_FORMED_FRAGMENTS {
        let input = fragment.as_bytes();
        let result = std::panic::catch_unwind(|| {
            // ShExParser is stateless (no captured mutable state), so
            // calling parse from a closure is safe even across unwind.
            let p = ShExParser::new();
            let _ = p.parse(input);
        });

        assert!(
            result.is_ok(),
            "ShExParser::parse panicked on fragment: {fragment:?}",
        );

        // Additionally call through the already-constructed parser to
        // ensure the same parser instance can be reused (stateless contract).
        let _ = parser.parse(input);
    }
}

// ---------------------------------------------------------------------------
// PSX1b — no-panic on adversarial byte sequences
// ---------------------------------------------------------------------------

/// PSX1b: `ShExParser::parse` must not panic on adversarial byte sequences.
///
/// These inputs are intentionally malformed or edge-case byte patterns that
/// could trigger panics in a naive implementation (e.g. empty input, lone
/// byte, high-bit bytes, embedded NUL).
#[test]
fn parse_adversarial_bytes_does_not_panic() {
    let adversarial: &[&[u8]] = &[
        b"",
        b"\x00",
        b"\xff\xfe",
        b"{ unclosed",
        b"PREFIX :",  // incomplete prefix decl
        b"@@@",
        &[0x80, 0x81, 0x82], // invalid UTF-8 continuation bytes
    ];

    for input in adversarial {
        let result = std::panic::catch_unwind(|| {
            let p = ShExParser::new();
            let _ = p.parse(input);
        });

        assert!(
            result.is_ok(),
            "ShExParser::parse panicked on adversarial input: {input:?}",
        );
    }
}
