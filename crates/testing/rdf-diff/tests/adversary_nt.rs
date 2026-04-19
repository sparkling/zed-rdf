//! Integration tests for the adversary N-Triples / N-Quads fixture corpus.
//!
//! Cohort-B fixtures live under `tests/adversary-nt/`; this module is the
//! test-side catalogue that documents each failure mode (FM1..FM7 from
//! `docs/verification/adversary-findings/nt.md`) and registers one
//! `#[test]` per fixture.
//!
//! ## Lifecycle
//!
//! Shadow parsers (`rdf-ntriples-shadow`) are present. The main parser
//! (`rdf-ntriples`) did **not** land in the Phase-A scope surveyed by
//! `phaseA-tester` — the tracking ADR (ADR-0017 §4) carries it as a
//! follow-up. Until the main parser exists the per-fixture diff bodies
//! cannot be wired; each `#[test]` body loads the fixture + its
//! `.expected` sibling, asserts both are present and non-empty (so
//! accidental deletion is still caught on every `cargo test`), and
//! leaves a `todo!` with a concrete wiring sketch for the reviewer.
//!
//! The structural always-on tests (`ant0_*`, `ant0b_*`, `fm*_fixtures_present`)
//! run on every `cargo test --workspace` and protect the fixture corpus
//! from bit-rot even while the main parser is deferred.
//!
//! ## Integration with `xtask verify`
//!
//! `cargo run -p xtask -- verify` discovers `external/fact-oracles/nt/*.json`
//! plus the smoke fixtures under `external/fact-oracles/fixtures/smoke/nt/`
//! and produces a per-format `DiffReport` even when the main parser is
//! absent. The per-FM tests here are the in-process mirror of that
//! check; they become hard gates when `rdf-ntriples` lands.
//!
//! ## ADR references
//!
//! - ADR-0017 §4 — Phase-A scope and main-parser handoff list.
//! - ADR-0019 §4 — adversary-corpus responsibilities.
//! - ADR-0020 §6.5 — cohort-B path claims.

#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn adversary_nt_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("adversary-nt")
}

fn fixture_path(name: &str) -> PathBuf {
    adversary_nt_root().join(name)
}

fn assert_pair_present(input: &str, expected: &str) {
    for name in [input, expected] {
        let p = fixture_path(name);
        assert!(
            p.exists(),
            "adversary-nt fixture missing: {}",
            p.display()
        );
        let bytes = std::fs::read(&p)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()));
        assert!(!bytes.is_empty(), "adversary-nt fixture empty: {}", p.display());
    }
}

fn collect_fixtures() -> Vec<PathBuf> {
    let root = adversary_nt_root();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| matches!(e, "nt" | "nq" | "expected"))
                    .unwrap_or(false)
        })
        .collect();
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// ANT0 — Fixture discovery (always-on)
// ---------------------------------------------------------------------------

/// **ANT0 — adversary-nt fixture directory is present and sorted.**
#[test]
fn ant0_fixture_discovery_present_and_sorted() {
    let root = adversary_nt_root();
    assert!(
        root.exists(),
        "adversary-nt fixture directory missing at {}",
        root.display()
    );
    let fixtures = collect_fixtures();
    assert!(
        fixtures.len() >= 14,
        "expected ≥14 adversary-nt files (7 FMs × input+expected, \
         some with 2 variants); found {}",
        fixtures.len()
    );
    for w in fixtures.windows(2) {
        assert!(w[0] <= w[1], "fixture list not sorted: {:?} > {:?}", w[0], w[1]);
    }
}

/// **ANT0b — each expected `(input, expected)` pair is on disk.**
#[test]
fn ant0b_all_expected_fixtures_present() {
    let pairs: &[(&str, &str)] = &[
        ("fm1-eol-bare-cr.nt", "fm1-eol-bare-cr.expected"),
        ("fm1-eol-crlf.nt", "fm1-eol-crlf.expected"),
        ("fm2-relative-iri-predicate.nt", "fm2-relative-iri-predicate.expected"),
        ("fm2-relative-iri-graph.nq", "fm2-relative-iri-graph.expected"),
        ("fm3-unicode-escape-lower.nt", "fm3-unicode-escape-lower.expected"),
        ("fm3-unicode-escape-upper.nt", "fm3-unicode-escape-upper.expected"),
        ("fm4-bnode-dot-middle.nt", "fm4-bnode-dot-middle.expected"),
        ("fm4-bnode-trailing-dot.nt", "fm4-bnode-trailing-dot.expected"),
        ("fm5-datatype-relative-iri.nt", "fm5-datatype-relative-iri.expected"),
        ("fm6-langtag-lowercase.nt", "fm6-langtag-lowercase.expected"),
        ("fm6-langtag-uppercase.nt", "fm6-langtag-uppercase.expected"),
        ("fm7-comment-no-final-newline.nt", "fm7-comment-no-final-newline.expected"),
    ];
    for (input, expected) in pairs {
        assert_pair_present(input, expected);
    }
}

/// **ANT0c — README present.**
#[test]
fn ant0c_readme_present() {
    let r = adversary_nt_root().join("README.md");
    assert!(r.exists(), "adversary-nt README.md missing at {}", r.display());
}

// ---------------------------------------------------------------------------
// ANT1 — FM1: EOL handling (CR, LF, CRLF)
// ---------------------------------------------------------------------------

/// **ANT1a — FM1: bare CR `\r` as line terminator.**
///
/// RDF 1.2 draft §2 relaxes `EOL ::= [#xD#xA]+` to allow bare CR.
/// A parser rejecting `\r` produces `Diagnostics { fatal: true }` →
/// `AcceptRejectSplit` divergence against a permissive oracle.
///
/// Unignore when `rdf-ntriples` (main) lands.
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT1a)"]
fn ant1a_fm1_eol_bare_cr() {
    assert_pair_present("fm1-eol-bare-cr.nt", "fm1-eol-bare-cr.expected");
    // Wiring sketch (Phase-A main parser follow-up):
    //   let input = std::fs::read(fixture_path("fm1-eol-bare-cr.nt"))?;
    //   let main   = rdf_ntriples::NTriplesParser::default().parse(&input);
    //   let shadow = rdf_ntriples_shadow::Parser::default().parse(&input);
    //   match (main, shadow) {
    //       (Ok(m), Ok(s)) => assert!(diff(&m.facts, &s.facts)?.is_clean()),
    //       _ => { /* AcceptRejectSplit is the expected divergence */ }
    //   }
    todo!("wire ANT1a when rdf-ntriples main parser lands");
}

/// **ANT1b — FM1: CRLF `\r\n` as single EOL.**
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT1b)"]
fn ant1b_fm1_eol_crlf() {
    assert_pair_present("fm1-eol-crlf.nt", "fm1-eol-crlf.expected");
    todo!("wire ANT1b when rdf-ntriples main parser lands");
}

// ---------------------------------------------------------------------------
// ANT2 — FM2: Relative IRI prohibition
// ---------------------------------------------------------------------------

/// **ANT2a — FM2: relative IRI in predicate position must be rejected.**
///
/// NT §2.1 forbids relative IRIs (no base). A parser borrowing Turtle
/// resolution logic will silently resolve `<p>` against a synthesised
/// base → `AcceptRejectSplit` or `ObjectMismatch` divergence.
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT2a)"]
fn ant2a_fm2_relative_iri_predicate_rejected() {
    assert_pair_present("fm2-relative-iri-predicate.nt", "fm2-relative-iri-predicate.expected");
    todo!("wire ANT2a when rdf-ntriples main parser lands");
}

/// **ANT2b — FM2: relative IRI in N-Quads graph slot must be rejected.**
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT2b)"]
fn ant2b_fm2_relative_iri_graph_rejected() {
    assert_pair_present("fm2-relative-iri-graph.nq", "fm2-relative-iri-graph.expected");
    todo!("wire ANT2b when rdf-ntriples main parser lands");
}

// ---------------------------------------------------------------------------
// ANT3 — FM3: Unicode escape case-sensitivity
// ---------------------------------------------------------------------------

/// **ANT3a — FM3: `\u00e9` and `\u00E9` decode to the same code point.**
///
/// HEX is case-insensitive in the NT grammar. A parser that stores the
/// raw escape rather than decoding produces two distinct literals →
/// `ObjectMismatch`.
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT3a)"]
fn ant3a_fm3_unicode_escape_case_lower() {
    assert_pair_present("fm3-unicode-escape-lower.nt", "fm3-unicode-escape-lower.expected");
    todo!("wire ANT3a when rdf-ntriples main parser lands");
}

/// **ANT3b — FM3: uppercase hex digits decode identically.**
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT3b)"]
fn ant3b_fm3_unicode_escape_case_upper() {
    assert_pair_present("fm3-unicode-escape-upper.nt", "fm3-unicode-escape-upper.expected");
    todo!("wire ANT3b when rdf-ntriples main parser lands");
}

// ---------------------------------------------------------------------------
// ANT4 — FM4: Blank-node trailing-dot restriction
// ---------------------------------------------------------------------------

/// **ANT4a — FM4: `_:b.1` is valid (dot allowed in the middle).**
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT4a)"]
fn ant4a_fm4_bnode_dot_middle_accepted() {
    assert_pair_present("fm4-bnode-dot-middle.nt", "fm4-bnode-dot-middle.expected");
    todo!("wire ANT4a when rdf-ntriples main parser lands");
}

/// **ANT4b — FM4: `_:b1.` must be rejected (dot cannot be final).**
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT4b)"]
fn ant4b_fm4_bnode_trailing_dot_rejected() {
    assert_pair_present("fm4-bnode-trailing-dot.nt", "fm4-bnode-trailing-dot.expected");
    todo!("wire ANT4b when rdf-ntriples main parser lands");
}

// ---------------------------------------------------------------------------
// ANT5 — FM5: Datatype IRI absoluteness
// ---------------------------------------------------------------------------

/// **ANT5 — FM5: `"42"^^<integer>` must be rejected (relative datatype IRI).**
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT5)"]
fn ant5_fm5_datatype_relative_iri_rejected() {
    assert_pair_present("fm5-datatype-relative-iri.nt", "fm5-datatype-relative-iri.expected");
    todo!("wire ANT5 when rdf-ntriples main parser lands");
}

// ---------------------------------------------------------------------------
// ANT6 — FM6: Language tag case (RDF 1.1 vs 1.2)
// ---------------------------------------------------------------------------

/// **ANT6a — FM6: lowercase language tag.**
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT6a)"]
fn ant6a_fm6_langtag_lowercase() {
    assert_pair_present("fm6-langtag-lowercase.nt", "fm6-langtag-lowercase.expected");
    todo!("wire ANT6a when rdf-ntriples main parser lands");
}

/// **ANT6b — FM6: uppercase language tag normalisation (1.1 vs 1.2 split).**
///
/// Under RDF 1.1 `@EN` and `@en` are distinct; under RDF 1.2 they
/// canonicalise to `@en`. The diff harness's `canonicalise_term`
/// implements the RDF 1.2 behaviour (BCP-47 §2.1.1 case-fold), so a
/// 1.1-strict main parser would surface as `ObjectMismatch`.
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT6b)"]
fn ant6b_fm6_langtag_uppercase() {
    assert_pair_present("fm6-langtag-uppercase.nt", "fm6-langtag-uppercase.expected");
    todo!("wire ANT6b when rdf-ntriples main parser lands");
}

// ---------------------------------------------------------------------------
// ANT7 — FM7: Comment with no final newline
// ---------------------------------------------------------------------------

/// **ANT7 — FM7: file ending with `# comment` and no trailing newline.**
///
/// NT §2 `ntriplesDoc ::= triple? (EOL triple?)* EOL?` — trailing EOL
/// is optional. Line-based parsers that require a final `\n` reject a
/// valid file → `AcceptRejectSplit`.
#[test]
#[ignore = "unignore when rdf-ntriples main parser lands (ANT7)"]
fn ant7_fm7_comment_no_final_newline_accepted() {
    assert_pair_present("fm7-comment-no-final-newline.nt", "fm7-comment-no-final-newline.expected");
    todo!("wire ANT7 when rdf-ntriples main parser lands");
}
