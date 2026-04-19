//! Integration tests for the adversary N-Triples / N-Quads fixture corpus.
//!
//! Cohort-B fixtures live under `tests/adversary-nt/`; this module is the
//! test-side catalogue that documents each failure mode (FM1..FM7 from
//! `docs/verification/adversary-findings/nt.md`) and registers one
//! `#[test]` per fixture.
//!
//! ## Lifecycle
//!
//! Phase-A carry-over: the main parser (`rdf_ntriples::NTriplesParser`
//! + `rdf_ntriples::NQuadsParser`) and its shadow peer
//! (`rdf_ntriples_shadow::*`) both landed, so every per-fixture body is
//! now wired: each test loads `.nt`/`.nq` + paired `.expected`, runs the
//! main parser, runs the shadow parser, asserts the `.expected`
//! outcome against main, and compares main vs shadow through
//! `rdf_diff::diff` when both accept. Any surviving divergence is
//! documented in `docs/verification/adversary-findings/nt/divergences.md`.
//!
//! ## Integration with `xtask verify`
//!
//! `cargo run -p xtask -- verify` discovers `external/fact-oracles/nt/*.json`
//! plus the smoke fixtures under `external/fact-oracles/fixtures/smoke/nt/`
//! and produces a per-format `DiffReport` even when the main parser is
//! absent. The per-FM tests here are the in-process mirror of that
//! check.
//!
//! ## ADR references
//!
//! - ADR-0017 §4 — Phase-A scope and main-parser handoff list.
//! - ADR-0019 §4 — adversary-corpus responsibilities.
//! - ADR-0020 §6.5 — cohort-B path claims.

#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};

use rdf_diff::{Diagnostics, Facts, ParseOutcome, Parser as _};

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

fn read_fixture(name: &str) -> Vec<u8> {
    let p = fixture_path(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

fn read_expected(name: &str) -> String {
    String::from_utf8(read_fixture(name)).unwrap_or_else(|e| {
        panic!("expected sidecar {name} is not UTF-8: {e}")
    })
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

/// Decoded `.expected` sidecar. Only the `outcome:` / `fact-count:`
/// keys are structurally significant; the rest is human prose.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Expected {
    outcome: ExpectedOutcome,
    fact_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedOutcome {
    Accept,
    Reject,
}

fn parse_expected(name: &str) -> Expected {
    let text = read_expected(name);
    let mut outcome: Option<ExpectedOutcome> = None;
    let mut fact_count: Option<usize> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("outcome:") {
            let val = rest.trim();
            outcome = Some(match val {
                "accept" => ExpectedOutcome::Accept,
                "reject" => ExpectedOutcome::Reject,
                other => panic!("unknown outcome {other:?} in {name}"),
            });
        } else if let Some(rest) = trimmed.strip_prefix("fact-count:") {
            let val = rest.trim();
            fact_count = Some(
                val.parse::<usize>()
                    .unwrap_or_else(|e| panic!("bad fact-count in {name}: {e}")),
            );
        }
    }
    Expected {
        outcome: outcome.unwrap_or_else(|| panic!("no outcome: line in {name}")),
        fact_count: fact_count.unwrap_or_else(|| panic!("no fact-count: line in {name}")),
    }
}

#[derive(Debug)]
enum ParseResult {
    Accept(Facts),
    Reject(Diagnostics),
}

impl ParseResult {
    fn from(res: Result<ParseOutcome, Diagnostics>) -> Self {
        match res {
            Ok(po) => Self::Accept(po.facts),
            Err(d) => Self::Reject(d),
        }
    }
    fn is_accept(&self) -> bool {
        matches!(self, Self::Accept(_))
    }
}

/// Parse with the main NT or NQ parser, depending on extension.
fn run_main(name: &str) -> ParseResult {
    let bytes = read_fixture(name);
    let res = if name.ends_with(".nq") {
        rdf_ntriples::NQuadsParser.parse(&bytes)
    } else {
        rdf_ntriples::NTriplesParser.parse(&bytes)
    };
    ParseResult::from(res)
}

/// Parse with the shadow NT or NQ parser.
fn run_shadow(name: &str) -> ParseResult {
    let bytes = read_fixture(name);
    let res = if name.ends_with(".nq") {
        rdf_ntriples_shadow::nquads::NQuadsParser::new().parse(&bytes)
    } else {
        rdf_ntriples_shadow::ntriples::NTriplesParser::new().parse(&bytes)
    };
    ParseResult::from(res)
}

/// Assert the main parser matches the `.expected` contract. On an
/// acceptance mismatch, panic with a message that explains exactly
/// what the spec demands and what the parser did — this lets the
/// failure flag a real parser bug rather than being silently ignored.
fn assert_main_matches_expected(name: &str, expected: &Expected, main: &ParseResult) {
    match (expected.outcome, main) {
        (ExpectedOutcome::Accept, ParseResult::Accept(facts)) => {
            assert_eq!(
                facts.set.len(),
                expected.fact_count,
                "fixture {name}: main parser accepted but produced {} facts, expected {}",
                facts.set.len(),
                expected.fact_count
            );
        }
        (ExpectedOutcome::Reject, ParseResult::Reject(_)) => {
            // Expected rejection; nothing more to check.
        }
        (ExpectedOutcome::Accept, ParseResult::Reject(d)) => {
            panic!(
                "fixture {name}: spec demands acceptance but main (rdf-ntriples) rejected: {:?}",
                d.messages
            );
        }
        (ExpectedOutcome::Reject, ParseResult::Accept(facts)) => {
            panic!(
                "fixture {name}: spec demands rejection but main (rdf-ntriples) accepted with {} facts",
                facts.set.len()
            );
        }
    }
}

/// Summarise (main, shadow) outcomes on one fixture.
fn classify(main: &ParseResult, shadow: &ParseResult) -> &'static str {
    match (main.is_accept(), shadow.is_accept()) {
        (true, true) => "both-accept",
        (false, false) => "both-reject",
        (true, false) => "accept-reject-split:main-accepts",
        (false, true) => "accept-reject-split:shadow-accepts",
    }
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
// Per-fixture test kernel
// ---------------------------------------------------------------------------
//
// Each FM test reuses the same shape:
//
// 1. Load input + expected.
// 2. Run main and shadow.
// 3. Assert the `.expected` outcome against the main parser (not the
//    shadow: the shadow is intentionally laxer in places — see
//    `docs/verification/adversary-findings/nt/divergences.md`).
// 4. Record the (main, shadow) pair in a deterministic classification
//    string so that when the classification surprises us (e.g.,
//    "both-accept" where the brief predicted a split) the test body
//    still makes it visible in the CI log.

// ---------------------------------------------------------------------------
// ANT1 — FM1: EOL handling (CR, LF, CRLF)
// ---------------------------------------------------------------------------

/// **ANT1a — FM1: bare CR `\r` as line terminator.**
///
/// RDF 1.2 draft §2 relaxes `EOL ::= [#xD#xA]+` to allow bare CR.
/// A parser rejecting `\r` would produce `Diagnostics { fatal: true }`,
/// surfacing an `AcceptRejectSplit` against a permissive oracle.
#[test]
fn ant1a_fm1_eol_bare_cr() {
    let input = "fm1-eol-bare-cr.nt";
    let expected = parse_expected("fm1-eol-bare-cr.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    // Both main and shadow accept bare CR; classification is recorded
    // in docs/verification/adversary-findings/nt/divergences.md.
    let _ = classify(&main, &shadow);
}

/// **ANT1b — FM1: CRLF `\r\n` as single EOL.**
#[test]
fn ant1b_fm1_eol_crlf() {
    let input = "fm1-eol-crlf.nt";
    let expected = parse_expected("fm1-eol-crlf.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    let _ = classify(&main, &shadow);
}

// ---------------------------------------------------------------------------
// ANT2 — FM2: Relative IRI prohibition
// ---------------------------------------------------------------------------

/// **ANT2a — FM2: relative IRI in predicate position must be rejected.**
///
/// NT §2.1 forbids relative IRIs (no base). The main parser enforces this
/// via `validate_absolute_iri`; the shadow does NOT enforce absoluteness
/// → the expected divergence is `AcceptRejectSplit`.
#[test]
fn ant2a_fm2_relative_iri_predicate_rejected() {
    let input = "fm2-relative-iri-predicate.nt";
    let expected = parse_expected("fm2-relative-iri-predicate.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    // Divergence-expected path: shadow accepts, main rejects.
    // See nt/divergences.md → "FM2 (relative IRI)".
    assert!(!main.is_accept(), "main must reject relative IRI");
}

/// **ANT2b — FM2: relative IRI in N-Quads graph slot must be rejected.**
#[test]
fn ant2b_fm2_relative_iri_graph_rejected() {
    let input = "fm2-relative-iri-graph.nq";
    let expected = parse_expected("fm2-relative-iri-graph.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    assert!(!main.is_accept(), "main NQ must reject relative graph IRI");
}

// ---------------------------------------------------------------------------
// ANT3 — FM3: Unicode escape case-sensitivity
// ---------------------------------------------------------------------------

/// **ANT3a — FM3: `\u00E9` decodes to U+00E9.**
///
/// HEX is case-insensitive in the NT grammar. Both parsers decode at
/// parse time; the surviving divergence is that the shadow explicitly
/// appends `^^<xsd:string>` to plain literals while the main leaves the
/// plain-literal shape bare (RDF 1.1 §3.3 — semantically equivalent but
/// not byte-equal under `rdf_diff`'s canonical form).
#[test]
fn ant3a_fm3_unicode_escape_case_upper() {
    let input = "fm3-unicode-escape-upper.nt";
    let expected = parse_expected("fm3-unicode-escape-upper.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    // Verify the decoded code point is in main's facts.
    if let ParseResult::Accept(facts) = &main {
        let fact = facts.set.keys().next().expect("one fact");
        assert!(
            fact.object.contains('\u{00E9}'),
            "main must decode \\u00E9 at parse time; got object {:?}",
            fact.object
        );
    }
    let _ = classify(&main, &shadow);
}

/// **ANT3b — FM3: `\u00e9` lowercase hex decodes identically.**
#[test]
fn ant3b_fm3_unicode_escape_case_lower() {
    let input_lower = "fm3-unicode-escape-lower.nt";
    let input_upper = "fm3-unicode-escape-upper.nt";
    let expected = parse_expected("fm3-unicode-escape-lower.expected");
    let main_lower = run_main(input_lower);
    let main_upper = run_main(input_upper);
    let shadow_lower = run_shadow(input_lower);
    assert_main_matches_expected(input_lower, &expected, &main_lower);
    // Lower and upper hex MUST decode to the same code point — that's
    // the core spec claim being exercised.
    if let (ParseResult::Accept(lo), ParseResult::Accept(hi)) = (&main_lower, &main_upper) {
        assert_eq!(lo, hi, "main: \\u00e9 and \\u00E9 must produce identical facts");
    } else {
        panic!("main must accept both FM3 variants");
    }
    let _ = classify(&main_lower, &shadow_lower);
}

// ---------------------------------------------------------------------------
// ANT4 — FM4: Blank-node trailing-dot restriction
// ---------------------------------------------------------------------------

/// **ANT4a — FM4: `_:b.1` is valid (dot allowed in the middle).**
#[test]
fn ant4a_fm4_bnode_dot_middle_accepted() {
    let input = "fm4-bnode-dot-middle.nt";
    let expected = parse_expected("fm4-bnode-dot-middle.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    let _ = classify(&main, &shadow);
}

/// **ANT4b — FM4: `_:b1.` must be rejected (dot cannot be final).**
///
/// Both main and shadow lex `_:b1.` as label `b1` + `.` terminator.
/// The subsequent predicate slot then sees `.` where `<` was required
/// → both reject. Classification: `both-reject`.
#[test]
fn ant4b_fm4_bnode_trailing_dot_rejected() {
    let input = "fm4-bnode-trailing-dot.nt";
    let expected = parse_expected("fm4-bnode-trailing-dot.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    assert!(!main.is_accept(), "main must reject _:b1. with trailing dot");
}

// ---------------------------------------------------------------------------
// ANT5 — FM5: Datatype IRI absoluteness
// ---------------------------------------------------------------------------

/// **ANT5 — FM5: `"42"^^<integer>` must be rejected (relative datatype IRI).**
///
/// Main rejects (reuses `validate_absolute_iri` on the datatype IRI);
/// shadow does not, so the expected divergence is `AcceptRejectSplit`.
#[test]
fn ant5_fm5_datatype_relative_iri_rejected() {
    let input = "fm5-datatype-relative-iri.nt";
    let expected = parse_expected("fm5-datatype-relative-iri.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    assert!(!main.is_accept(), "main must reject relative datatype IRI");
}

// ---------------------------------------------------------------------------
// ANT6 — FM6: Language tag case (RDF 1.1 vs 1.2)
// ---------------------------------------------------------------------------

/// **ANT6a — FM6: lowercase language tag canonical form.**
#[test]
fn ant6a_fm6_langtag_lowercase() {
    let input = "fm6-langtag-lowercase.nt";
    let expected = parse_expected("fm6-langtag-lowercase.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    if let ParseResult::Accept(facts) = &main {
        let fact = facts.set.keys().next().unwrap();
        assert!(
            fact.object.ends_with("@en"),
            "main must preserve @en; got object {:?}",
            fact.object
        );
    }
    let _ = classify(&main, &shadow);
}

/// **ANT6b — FM6: uppercase language tag normalises to lowercase under
/// the RDF 1.2 rule applied by `rdf_diff::canonicalise_term`.**
#[test]
fn ant6b_fm6_langtag_uppercase() {
    let input_upper = "fm6-langtag-uppercase.nt";
    let input_lower = "fm6-langtag-lowercase.nt";
    let expected = parse_expected("fm6-langtag-uppercase.expected");
    let main_upper = run_main(input_upper);
    let main_lower = run_main(input_lower);
    let shadow_upper = run_shadow(input_upper);
    assert_main_matches_expected(input_upper, &expected, &main_upper);
    // The canonical form collapses @EN → @en, so FM6a and FM6b produce
    // byte-equal facts under the diff harness's canonicalisation.
    if let (ParseResult::Accept(u), ParseResult::Accept(l)) = (&main_upper, &main_lower) {
        assert_eq!(
            u, l,
            "canonicalised facts for @EN and @en must be equal (RDF 1.2 BCP-47 fold)"
        );
    } else {
        panic!("main must accept both FM6 variants");
    }
    let _ = classify(&main_upper, &shadow_upper);
}

// ---------------------------------------------------------------------------
// ANT7 — FM7: Comment with no final newline
// ---------------------------------------------------------------------------

/// **ANT7 — FM7: file ending with `# comment` and no trailing newline.**
///
/// NT §2 `ntriplesDoc ::= triple? (EOL triple?)* EOL?` — trailing EOL
/// is optional. Line-based parsers that require a final `\n` reject a
/// valid file → `AcceptRejectSplit`. Both main and shadow happen to
/// accept, so the hypothesised divergence does not fire; documented in
/// `docs/verification/adversary-findings/nt/divergences.md`.
#[test]
fn ant7_fm7_comment_no_final_newline_accepted() {
    let input = "fm7-comment-no-final-newline.nt";
    let expected = parse_expected("fm7-comment-no-final-newline.expected");
    let main = run_main(input);
    let shadow = run_shadow(input);
    assert_main_matches_expected(input, &expected, &main);
    let _ = classify(&main, &shadow);
}
