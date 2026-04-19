//! Fuzz target: `Iri::normalise` idempotence.
//!
//! The pin `IRI-PCT-001` permits a narrow set of normalisations
//! (scheme case, host case, path dot-segment removal for absolute
//! IRIs). Whatever that set is, running it twice must be a fixed
//! point.
//!
//! Invariants:
//!
//! 1. **No panics** on any IRI returned by `Iri::parse`.
//! 2. **Idempotence:** `normalise(normalise(x)) == normalise(x)` for
//!    byte-for-byte equality of the raw IRI string.
//! 3. **Normalised output round-trips through `parse`** — the narrow
//!    normalisation must not produce something the parser would
//!    reject.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_iri::Iri;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(iri) = Iri::parse(text) else {
        return;
    };

    let once = iri.normalise();
    let twice = once.normalise();

    // Invariant 2.
    assert_eq!(
        once.as_str(),
        twice.as_str(),
        "IRI-PCT-001: normalise() must be idempotent",
    );

    // Invariant 3 — the normaliser never produces a string the parser
    // would reject.
    Iri::parse(once.as_str()).expect("normalise() output must parse");
});
