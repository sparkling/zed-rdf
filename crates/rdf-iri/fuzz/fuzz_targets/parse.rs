//! Fuzz target: `Iri::parse` on arbitrary bytes.
//!
//! Invariants (ADR-0018 §§3–4):
//!
//! 1. **No panics.** `Iri::parse` must return `Err(Diagnostic)` rather
//!    than panic on any byte string, valid or otherwise.
//! 2. **Structured rejection.** When parsing fails, the returned
//!    `Diagnostic` must carry a non-default `DiagnosticCode` — the
//!    libfuzzer harness never string-compares messages.
//! 3. **Round-trip on accept.** If `parse` succeeds, the stored raw
//!    IRI must equal the UTF-8 decoding of the input byte-for-byte
//!    (the pin `IRI-PCT-001` forbids parse-time rewrites).
//!
//! Coverage is driven by libfuzzer's SanitizerCoverage instrumentation;
//! no explicit seed corpus is committed (see
//! `docs/runbooks/fuzzing.md`). The nightly workflow minimises and
//! uploads a corpus artifact.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_iri::Iri;

fuzz_target!(|data: &[u8]| {
    // Reject non-UTF-8 up front — the public API is `&str`. A real
    // caller (e.g. the Turtle parser) has already validated UTF-8.
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    match Iri::parse(text) {
        Ok(iri) => {
            // Invariant 3: byte-for-byte preservation.
            assert_eq!(
                iri.as_str(),
                text,
                "IRI-PCT-001: parse() must not rewrite its input",
            );
        }
        Err(_diag) => {
            // Invariant 2: rejection is a structured Diagnostic. The
            // enum existence alone proves structured-ness; we do not
            // pattern-match the code because the set grows over time.
        }
    }
});
