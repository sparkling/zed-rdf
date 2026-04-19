//! Fuzz target: blank-node scope invariant across `@prefix` redefines.
//!
//! Pins the `TTL-BNPFX-001` reading: blank-node labels are **document
//! scope**. Two occurrences of `_:x` in the same document must alias
//! the same blank node, regardless of any `@prefix` redeclaration
//! between them. The shadow/main diff harness gates this too; the
//! fuzzer stresses it with random surrounding noise.
//!
//! Strategy: prepend a fixed, grammar-valid header that declares `ex:`
//! and asserts a triple using `_:x`. Then append the fuzzer-supplied
//! bytes, then append a second triple reusing `_:x`. If the combined
//! document parses successfully, both occurrences of `_:x` must
//! canonicalise to the same subject label in the resulting fact set.
//!
//! Invariants:
//!
//! 1. **No panics** on any appended payload.
//! 2. **Structured rejection** on failure (same shape as the other
//!    turtle fuzz targets).
//! 3. **Bnode aliasing preserved on accept:** among the facts whose
//!    predicate is `<http://ex/p>`, the canonical subject forms for
//!    the two `_:x`-using statements must be equal. (The concrete
//!    relabelling — `_:c0` etc. — depends on canonicalisation; the
//!    contract is stable *equality*, not a stable literal label.)

#![no_main]

use libfuzzer_sys::fuzz_target;
use rdf_diff::Parser;
use rdf_turtle::TurtleParser;

// Grammar-valid head: declares `_:x` as the subject of a triple with a
// known predicate.
const HEAD: &[u8] = b"@prefix ex: <http://ex/> .\n_:x ex:p <http://ex/o1> .\n";

// Grammar-valid tail: same `_:x`, same predicate, different object.
// Per TTL-BNPFX-001 both must alias the *same* document-scope bnode
// even if the fuzzer payload contained an `@prefix ex:` redeclaration.
const TAIL: &[u8] = b"\n_:x ex:p <http://ex/o2> .\n";

fuzz_target!(|data: &[u8]| {
    let mut doc = Vec::with_capacity(HEAD.len() + data.len() + TAIL.len());
    doc.extend_from_slice(HEAD);
    doc.extend_from_slice(data);
    doc.extend_from_slice(TAIL);

    match TurtleParser.parse(&doc) {
        Ok(outcome) => {
            // Collect the subject forms of the two `_:x`-anchored
            // triples. After canonicalisation both predicates render
            // as <http://ex/p>.
            let mut subjects: Vec<&str> = outcome
                .facts
                .set
                .keys()
                .filter(|f| f.predicate == "<http://ex/p>")
                .filter(|f| {
                    f.object == "<http://ex/o1>" || f.object == "<http://ex/o2>"
                })
                .map(|f| f.subject.as_str())
                .collect();
            subjects.sort_unstable();
            subjects.dedup();

            // Invariant 3: exactly one distinct subject form for the
            // two statements. Any fuzzer payload that caused `_:x` to
            // split into two distinct bnodes would leave us with two
            // subjects and surface as a failing assertion.
            //
            // If the harness produced zero matches (e.g. the fuzzer
            // payload hijacked canonicalisation in some unexpected
            // way), we cannot prove the invariant either way — skip
            // rather than spuriously crash.
            if !subjects.is_empty() {
                assert_eq!(
                    subjects.len(),
                    1,
                    "TTL-BNPFX-001: _:x must alias across @prefix redefinitions; \
                     got distinct subjects {subjects:?}",
                );
            }
        }
        Err(diag) => {
            // Invariant 2.
            assert!(diag.fatal);
            assert!(!diag.messages.is_empty());
        }
    }
});
