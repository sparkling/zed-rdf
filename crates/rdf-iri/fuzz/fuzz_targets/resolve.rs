//! Fuzz target: `Iri::resolve` on (base, reference) pairs.
//!
//! Splits the input into two halves at a cursor-derived offset and
//! tries to parse each as an IRI. If both parse and the base is
//! absolute, `resolve` is exercised. The invariants we gate on:
//!
//! 1. **No panics.** `Iri::resolve` must never panic for a base that
//!    has passed the `is_absolute()` check — RFC 3986 §5.1 is the only
//!    documented panic condition.
//! 2. **Total function.** The returned `Iri` must itself parse (round
//!    trip through `Iri::parse`) — resolve is a projection of the
//!    reference into the base's scheme, not a bytes-producing
//!    primitive.
//! 3. **Idempotent re-resolve.** `r.resolve(base).resolve(base)` must
//!    equal `r.resolve(base)` — reference resolution is idempotent
//!    once the first pass has produced an absolute IRI.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_iri::Iri;

fuzz_target!(|data: &[u8]| {
    // Need at least two bytes to have any hope of a non-degenerate
    // split; reject tiny inputs outright.
    if data.len() < 2 {
        return;
    }
    let split = (data[0] as usize) % data.len();
    let (base_bytes, ref_bytes) = data[1..].split_at(split.min(data.len() - 1));

    let Ok(base_text) = std::str::from_utf8(base_bytes) else {
        return;
    };
    let Ok(ref_text) = std::str::from_utf8(ref_bytes) else {
        return;
    };

    let Ok(base) = Iri::parse(base_text) else {
        return;
    };
    // RFC 3986 §5.1 requires the base to be absolute; the API panics
    // otherwise. Honour that precondition.
    if !base.is_absolute() {
        return;
    }
    let Ok(reference) = Iri::parse(ref_text) else {
        return;
    };

    // Invariant 1: must not panic.
    let resolved = reference.resolve(&base);

    // Invariant 2: output is a parseable IRI.
    let reparsed = Iri::parse(resolved.as_str())
        .expect("resolve() output must round-trip through parse()");

    // Invariant 3: idempotence of resolution against the same base.
    let resolved_again = reparsed.resolve(&base);
    assert_eq!(
        resolved.as_str(),
        resolved_again.as_str(),
        "resolve() must be idempotent on an absolute result",
    );
});
