//! Property tests for the main `rdf-ntriples` crate.
//!
//! Invariants exercised (spec-level, not shadow-parity):
//!
//! - **PNT1 canonical-form idempotence.** `Facts` produced by
//!   `NTriplesParser` are already canonical: feeding them back through
//!   `rdf_diff::Facts::canonicalise` is a no-op (set-level equality with
//!   the original).  Grounded in `rdf-diff`'s canonicalisation contract
//!   (`Facts::canonicalise` is a closure — applying it twice produces
//!   the same set).  The main parser promises to emit facts already in
//!   that canonical form so downstream consumers can skip the pass.
//! - **PNT2 semantic newline/terminator tolerance.** For every generated
//!   document, parsing it with `\n`-separated statements vs the same
//!   document with `\r\n` separators yields the identical `Facts` set.
//!   Grounded in N-Triples §2 `EOL ::= [#xD#xA]+`.
//!
//! ### Skipped invariant
//!
//! The brief originally asked for a *pretty-printer* round-trip
//! (parse → pretty-print → parse → equal facts).  **Skipped**: this
//! crate does not expose a pretty-printer — the only serialisation
//! helpers (`escape_literal_lex`) are crate-private and there is no
//! public writer API.  The gap is recorded in
//! `crate/rdf-ntriples/proptest-invariants` for follow-up.
//!
//! Generators stay small (ASCII-only IRIs, short literals, ≤8 triples
//! per document) to keep the 50–100-case default well inside the 30 s
//! per-crate budget.

#![cfg(not(miri))]

use proptest::prelude::*;
use rdf_diff::{Facts, Parser as _};
use rdf_ntriples::NTriplesParser;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

fn iri_strategy() -> impl Strategy<Value = String> {
    // Absolute http IRI with a short path tail. The parser enforces
    // absolute IRIs at every position (NT §2).
    ("[a-z]{1,8}", "[A-Za-z0-9_~.-]{1,16}", 0u8..=16)
        .prop_map(|(host, tail, n)| format!("<http://{host}.example/{tail}/{n}>"))
}

fn bnode_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9]{0,6}".prop_map(|s| format!("_:{s}"))
}

fn subject_strategy() -> impl Strategy<Value = String> {
    prop_oneof![iri_strategy(), bnode_strategy()]
}

fn plain_lit_strategy() -> impl Strategy<Value = String> {
    // Safe lexical chars only — no `"` or `\` or CR/LF.
    "[A-Za-z0-9 _.-]{0,24}".prop_map(|s| format!("\"{s}\""))
}

fn lang_tag_strategy() -> impl Strategy<Value = String> {
    "[a-z]{2,3}(-[A-Z]{2})?".prop_map(|t| t.to_owned())
}

fn lang_lit_strategy() -> impl Strategy<Value = String> {
    ("[A-Za-z0-9 _.-]{0,24}", lang_tag_strategy())
        .prop_map(|(lex, tag)| format!("\"{lex}\"@{tag}"))
}

fn typed_lit_strategy() -> impl Strategy<Value = String> {
    ("[A-Za-z0-9 _.-]{0,24}", iri_strategy())
        .prop_map(|(lex, dt)| format!("\"{lex}\"^^{dt}"))
}

fn object_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        iri_strategy(),
        bnode_strategy(),
        plain_lit_strategy(),
        lang_lit_strategy(),
        typed_lit_strategy(),
    ]
}

fn triple_strategy() -> impl Strategy<Value = String> {
    (subject_strategy(), iri_strategy(), object_strategy())
        .prop_map(|(s, p, o)| format!("{s} {p} {o} ."))
}

fn document_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(triple_strategy(), 0..=8)
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

fn parse(src: &str) -> Option<Facts> {
    NTriplesParser.parse(src.as_bytes()).ok().map(|o| o.facts)
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    /// PNT1 — canonicalising the parser's output is a no-op.
    #[test]
    fn parser_output_is_canonicalisation_idempotent(triples in document_strategy()) {
        let doc: String = triples.iter().fold(String::new(), |mut acc, t| {
            acc.push_str(t);
            acc.push('\n');
            acc
        });
        let facts = parse(&doc).expect("generator emits valid N-Triples");

        // Feed the set back through the canonicaliser. The frozen
        // `rdf_diff::Facts::canonicalise` signature consumes
        // `(Fact, FactProvenance)` pairs — reconstruct from the map.
        let raw = facts
            .set
            .iter()
            .map(|(f, p)| (f.clone(), p.clone()))
            .collect::<Vec<_>>();
        let round_trip = Facts::canonicalise(raw, BTreeMap::new());

        prop_assert_eq!(
            facts.set.keys().cloned().collect::<Vec<_>>(),
            round_trip.set.keys().cloned().collect::<Vec<_>>(),
        );
    }

    /// PNT2 — `\n` vs `\r\n` line endings yield the same fact set.
    /// NT §2: `EOL ::= [#xD#xA]+`, both are legal statement separators.
    #[test]
    fn lf_and_crlf_produce_equal_facts(triples in document_strategy()) {
        let lf: String = triples.iter().fold(String::new(), |mut acc, t| {
            acc.push_str(t);
            acc.push('\n');
            acc
        });
        let crlf: String = triples.iter().fold(String::new(), |mut acc, t| {
            acc.push_str(t);
            acc.push_str("\r\n");
            acc
        });
        let a = parse(&lf).expect("valid NT (LF)");
        let b = parse(&crlf).expect("valid NT (CRLF)");
        prop_assert_eq!(
            a.set.keys().cloned().collect::<Vec<_>>(),
            b.set.keys().cloned().collect::<Vec<_>>(),
        );
    }
}
