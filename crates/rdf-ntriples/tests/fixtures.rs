//! Fixture-driven acceptance tests for the main N-Triples / N-Quads
//! parser.
//!
//! Two corpora are exercised:
//!
//! 1. `external/fact-oracles/fixtures/smoke/nt/**` and `.../nq/**` — the
//!    smoke corpus the `xtask verify` PR gate runs before any vendored
//!    W3C test-suite lands.
//! 2. `crates/testing/rdf-diff/tests/adversary-nt/**` — cohort-B
//!    adversary fixtures. Each `.nt` / `.nq` has a paired `.expected`
//!    side-car; we pick the `outcome:` field (`accept` / `reject`) and
//!    assert the main parser agrees.
//!
//! Parser-correctness details per-fixture are documented in the
//! fixture's `.expected` file.

use std::path::{Path, PathBuf};

use rdf_diff::Parser as _;
use rdf_ntriples::{NQuadsParser, NTriplesParser};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at crates/rdf-ntriples")
        .to_path_buf()
}

fn read_expected_outcome(expected: &Path) -> String {
    let text = std::fs::read_to_string(expected)
        .unwrap_or_else(|e| panic!("read {} failed: {e}", expected.display()));
    for line in text.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("outcome:") {
            return rest.trim().to_ascii_lowercase();
        }
    }
    panic!("no `outcome:` line in {}", expected.display());
}

// ---------------------------------------------------------------------------
// Smoke-corpus fixtures (external/fact-oracles/fixtures/smoke/**)
// ---------------------------------------------------------------------------

#[test]
fn smoke_nt_accept_01_parses() {
    let path = workspace_root()
        .join("external/fact-oracles/fixtures/smoke/nt/accept-01.nt");
    let bytes = std::fs::read(&path).expect("read smoke accept-01.nt");
    let outcome = NTriplesParser
        .parse(&bytes)
        .unwrap_or_else(|d| panic!("accept: {:?}", d.messages));
    assert_eq!(outcome.facts.set.len(), 3);
}

#[test]
fn smoke_nt_reject_01_rejects() {
    let path = workspace_root()
        .join("external/fact-oracles/fixtures/smoke/nt/reject-01.nt");
    let bytes = std::fs::read(&path).expect("read smoke reject-01.nt");
    let err = NTriplesParser
        .parse(&bytes)
        .expect_err("smoke reject-01.nt must be rejected");
    assert!(err.fatal);
}

#[test]
fn smoke_nt_full_smoke_parses() {
    let path = workspace_root()
        .join("external/fact-oracles/fixtures/smoke/nt/smoke.nt");
    let bytes = std::fs::read(&path).expect("read smoke.nt");
    let outcome = NTriplesParser
        .parse(&bytes)
        .unwrap_or_else(|d| panic!("accept: {:?}", d.messages));
    // 5 lines of facts in the curated fixture.
    assert_eq!(outcome.facts.set.len(), 5);
}

#[test]
fn smoke_nq_accept_01_parses() {
    let path = workspace_root()
        .join("external/fact-oracles/fixtures/smoke/nq/accept-01.nq");
    let bytes = std::fs::read(&path).expect("read smoke accept-01.nq");
    let outcome = NQuadsParser
        .parse(&bytes)
        .unwrap_or_else(|d| panic!("accept: {:?}", d.messages));
    assert_eq!(outcome.facts.set.len(), 2);
    for fact in outcome.facts.set.keys() {
        assert_eq!(fact.graph.as_deref(), Some("<http://example.org/g>"));
    }
}

// ---------------------------------------------------------------------------
// Adversary-NT fixtures (cohort B — each pairs a .expected with .nt/.nq)
// ---------------------------------------------------------------------------

fn adversary_dir() -> PathBuf {
    workspace_root()
        .join("crates/testing/rdf-diff/tests/adversary-nt")
}

fn run_adversary_case(stem: &str, suffix: &str) {
    let base = adversary_dir();
    let input_path = base.join(format!("{stem}.{suffix}"));
    let expected_path = base.join(format!("{stem}.expected"));
    let bytes = std::fs::read(&input_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", input_path.display()));
    let outcome = read_expected_outcome(&expected_path);

    let result = if suffix == "nt" {
        NTriplesParser.parse(&bytes)
    } else {
        NQuadsParser.parse(&bytes)
    };

    match outcome.as_str() {
        "accept" | "accept-with-warnings" => {
            result.unwrap_or_else(|d| {
                panic!(
                    "{}: expected accept, got reject: {:?}",
                    input_path.display(),
                    d.messages
                )
            });
        }
        "reject" => {
            let err = result.err().unwrap_or_else(|| {
                panic!(
                    "{}: expected reject, got accept",
                    input_path.display()
                )
            });
            assert!(err.fatal, "{}: reject must be fatal", input_path.display());
        }
        other => panic!("unknown outcome {other:?} in {}", expected_path.display()),
    }
}

#[test]
fn adversary_fm1_eol_bare_cr() {
    run_adversary_case("fm1-eol-bare-cr", "nt");
}

#[test]
fn adversary_fm1_eol_crlf() {
    run_adversary_case("fm1-eol-crlf", "nt");
}

#[test]
fn adversary_fm2_relative_iri_predicate() {
    run_adversary_case("fm2-relative-iri-predicate", "nt");
}

#[test]
fn adversary_fm2_relative_iri_graph() {
    run_adversary_case("fm2-relative-iri-graph", "nq");
}

#[test]
fn adversary_fm3_unicode_escape_upper() {
    run_adversary_case("fm3-unicode-escape-upper", "nt");
}

#[test]
fn adversary_fm3_unicode_escape_lower() {
    run_adversary_case("fm3-unicode-escape-lower", "nt");
}

#[test]
fn adversary_fm4_bnode_dot_middle() {
    run_adversary_case("fm4-bnode-dot-middle", "nt");
}

#[test]
fn adversary_fm4_bnode_trailing_dot() {
    run_adversary_case("fm4-bnode-trailing-dot", "nt");
}

#[test]
fn adversary_fm5_datatype_relative_iri() {
    run_adversary_case("fm5-datatype-relative-iri", "nt");
}

#[test]
fn adversary_fm6_langtag_uppercase() {
    run_adversary_case("fm6-langtag-uppercase", "nt");
}

#[test]
fn adversary_fm6_langtag_lowercase() {
    run_adversary_case("fm6-langtag-lowercase", "nt");
}

#[test]
fn adversary_fm7_comment_no_final_newline() {
    run_adversary_case("fm7-comment-no-final-newline", "nt");
}

#[test]
fn fm3_escapes_decode_to_same_literal() {
    // Cross-check both FM3 fixtures produce the same canonical literal.
    let up = adversary_dir().join("fm3-unicode-escape-upper.nt");
    let lo = adversary_dir().join("fm3-unicode-escape-lower.nt");
    let up = NTriplesParser
        .parse(&std::fs::read(up).unwrap())
        .expect("accept upper");
    let lo = NTriplesParser
        .parse(&std::fs::read(lo).unwrap())
        .expect("accept lower");
    assert_eq!(up.facts, lo.facts, "NT-LITESC-001 decode divergence");
}
