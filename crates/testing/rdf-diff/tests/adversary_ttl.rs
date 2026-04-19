//! Integration tests for the adversary Turtle / TriG fixture corpus.
//!
//! Tracked in `docs/verification/tests/catalogue.md` under the `adversary-ttl`
//! section (rows `AT1` through `AT13`). Each test corresponds to a fixture
//! in `tests/adversary-ttl/` and documents the divergence hypothesis from
//! `docs/verification/adversary-findings/ttl.md`.
//!
//! ## Lifecycle
//!
//! With `rdf-turtle` (main) landed, each test now runs the fixture through
//! the main parser and asserts the fact set matches the fixture-header
//! expectation. Cross-parser diff (main vs shadow vs oxttl) lives in
//! `xtask verify`; these tests are the in-process smoke check of the
//! main parser on the adversary corpus.
//!
//! ## ADR references
//!
//! - ADR-0019 §4 — adversary-corpus responsibilities.
//! - ADR-0020 §6.5 — cohort-B path claims; adversary paths never overlap
//!   non-adversary test paths by construction.

#![allow(
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::map_unwrap_or,
)]

use std::path::{Path, PathBuf};

use rdf_diff::{Fact, Parser as _};
use rdf_turtle::{TriGParser, TurtleParser};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn adversary_ttl_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("tests").join("adversary-ttl")
}

fn collect_fixtures() -> Vec<PathBuf> {
    let root = adversary_ttl_root();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .map(|e| e == "ttl" || e == "trig")
                    .unwrap_or(false)
        })
        .collect();
    out.sort();
    out
}

fn fixture_path(name: &str) -> PathBuf {
    adversary_ttl_root().join(name)
}

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name))
        .unwrap_or_else(|e| panic!("could not read fixture {name}: {e}"))
}

fn parse_ttl_expect_ok(name: &str) -> Vec<Fact> {
    let src = read_fixture(name);
    match TurtleParser::new().parse(src.as_bytes()) {
        Ok(out) => out.facts.set.keys().cloned().collect(),
        Err(d) => panic!("expected {name} to parse; got {d:?}"),
    }
}

fn parse_trig_expect_ok(name: &str) -> Vec<Fact> {
    let src = read_fixture(name);
    match TriGParser::new().parse(src.as_bytes()) {
        Ok(out) => out.facts.set.keys().cloned().collect(),
        Err(d) => panic!("expected trig {name} to parse; got {d:?}"),
    }
}

// ---------------------------------------------------------------------------
// AT0 — Fixture discovery
// ---------------------------------------------------------------------------

/// **AT0 — Adversary TTL/TriG fixtures are present and sorted.**
///
/// Always-on: ensures the `adversary-ttl/` directory exists and contains
/// the expected minimum number of fixtures. Fails early if cohort-B forgot
/// to commit the directory or a file went missing.
#[test]
fn at0_fixture_discovery_present_and_sorted() {
    let root = adversary_ttl_root();
    assert!(
        root.exists(),
        "adversary-ttl fixture directory missing at {}",
        root.display()
    );

    let fixtures = collect_fixtures();
    assert!(
        fixtures.len() >= 9,
        "expected at least 9 adversary-ttl fixtures (one per finding), found {}",
        fixtures.len()
    );

    for w in fixtures.windows(2) {
        assert!(w[0] <= w[1], "fixture list is not sorted: {:?} > {:?}", w[0], w[1]);
    }
}

/// **AT0b — Every expected fixture file exists on disk.**
#[test]
fn at0b_all_expected_fixtures_present() {
    let expected = [
        "fm1-leading-digit-local.ttl",
        "fm1-prefix-redefinition.ttl",
        "fm2-percent-encoding-local.ttl",
        "fm3-keyword-scope.ttl",
        "fm4-empty-collection.ttl",
        "fm4-nested-collection.ttl",
        "fm5-long-string-newline.ttl",
        "fm5-short-string-newline-invalid.ttl",
        "fm6-base-directive-replacement.ttl",
        "fm6-chained-base.ttl",
        "fm7-trailing-semicolon.ttl",
        "fm8-trig-bnode-scope.trig",
        "fm9-numeric-literal-types.ttl",
    ];
    for name in expected {
        let p = fixture_path(name);
        assert!(p.exists(), "missing adversary-ttl fixture: {name} at {}", p.display());
    }
}

// ---------------------------------------------------------------------------
// AT1 — FM1: Leading digit in PN_LOCAL
// ---------------------------------------------------------------------------

#[test]
fn at1_fm1_leading_digit_local() {
    let f = parse_ttl_expect_ok("fm1-leading-digit-local.ttl");
    // Three triples: ex:123, ex:0start, ex:9end — all as subjects.
    assert_eq!(f.len(), 3);
    let subjects: Vec<_> = f.iter().map(|t| t.subject.clone()).collect();
    assert!(subjects.contains(&"<http://example/123>".to_owned()));
    assert!(subjects.contains(&"<http://example/0start>".to_owned()));
    assert!(subjects.contains(&"<http://example/9end>".to_owned()));
}

// ---------------------------------------------------------------------------
// AT2 — FM1b: @prefix redefinition
// ---------------------------------------------------------------------------

#[test]
fn at2_fm1b_prefix_redefinition() {
    let f = parse_ttl_expect_ok("fm1-prefix-redefinition.ttl");
    let subjects: std::collections::BTreeSet<_> =
        f.iter().map(|t| t.subject.clone()).collect();
    assert!(subjects.contains("<http://example/a/s>"));
    assert!(subjects.contains("<http://example/b/s>"));
}

// ---------------------------------------------------------------------------
// AT3 — FM2: Percent-encoding in local part
// ---------------------------------------------------------------------------

#[test]
fn at3_fm2_percent_encoding_not_decoded() {
    let f = parse_ttl_expect_ok("fm2-percent-encoding-local.ttl");
    let subjects: std::collections::BTreeSet<_> =
        f.iter().map(|t| t.subject.clone()).collect();
    // Must pass the percent-encoding through unchanged (pin IRI-PCT-001).
    assert!(subjects.contains("<http://example/caf%C3%A9>"));
    assert!(subjects.contains("<http://example/spa%20ce>"));
}

// ---------------------------------------------------------------------------
// AT4 — FM3: Keyword scope
// ---------------------------------------------------------------------------

#[test]
fn at4_fm3_keyword_scope_position_sensitive() {
    let f = parse_ttl_expect_ok("fm3-keyword-scope.ttl");
    assert_eq!(f.len(), 4);
    // `a` in predicate position is rdf:type.
    assert!(
        f.iter()
            .any(|t| t.predicate == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"),
    );
    // `<http://example/a>` in subject position is an IRI, NOT the keyword.
    assert!(f.iter().any(|t| t.subject == "<http://example/a>"));
    // Booleans in object position.
    assert!(
        f.iter()
            .any(|t| t.object == "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"),
    );
    assert!(
        f.iter()
            .any(|t| t.object == "\"false\"^^<http://www.w3.org/2001/XMLSchema#boolean>"),
    );
}

// ---------------------------------------------------------------------------
// AT5 — FM4a: Empty collection = rdf:nil
// ---------------------------------------------------------------------------

#[test]
fn at5_fm4a_empty_collection_is_rdf_nil() {
    let f = parse_ttl_expect_ok("fm4-empty-collection.ttl");
    assert_eq!(f.len(), 1);
    assert_eq!(
        f[0].object,
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>",
    );
}

// ---------------------------------------------------------------------------
// AT6 — FM4b: Nested collections
// ---------------------------------------------------------------------------

#[test]
fn at6_fm4b_nested_collection_not_flattened() {
    let f = parse_ttl_expect_ok("fm4-nested-collection.ttl");
    // Expected triples:
    //   (1):    3 facts (head->first=1, head->rest=nil, s->p1=head)
    //   ((1)):  5 facts (outer head->first=inner head, outer->rest=nil,
    //           inner->first=1, inner->rest=nil, s->p2=outer)
    //   (1 2):  5 facts (head→first=1, head→rest=tail,
    //           tail→first=2, tail→rest=nil, s→p3=head)
    // Total = 13 facts.
    assert_eq!(f.len(), 13, "got {f:?}");
    // The outer rdf:first for ((1)) must be a bnode, not the integer 1.
    let first_preds: Vec<_> = f
        .iter()
        .filter(|t| {
            t.predicate == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#first>"
        })
        .collect();
    // At least one rdf:first points to a bnode (the nested-head case).
    assert!(first_preds.iter().any(|t| t.object.starts_with("_:")));
}

// ---------------------------------------------------------------------------
// AT7 — FM5: Long string with raw newline (positive case)
// ---------------------------------------------------------------------------

#[test]
fn at7_fm5_long_string_raw_newline_accepted() {
    let f = parse_ttl_expect_ok("fm5-long-string-newline.ttl");
    assert_eq!(f.len(), 2);
    assert!(f.iter().all(|t| t.object.contains('\n')));
}

// ---------------------------------------------------------------------------
// AT8 — FM5: Short string with raw newline (negative case)
// ---------------------------------------------------------------------------

#[test]
fn at8_fm5_short_string_raw_newline_rejected() {
    let src = read_fixture("fm5-short-string-newline-invalid.ttl");
    let res = TurtleParser::new().parse(src.as_bytes());
    let d = res.expect_err("short string with raw newline must be rejected");
    assert!(d.fatal, "diagnostic must be fatal");
    assert!(
        d.messages.iter().any(|m| m.starts_with("TTL-LITESC-001")),
        "expected TTL-LITESC-001, got {:?}",
        d.messages,
    );
}

// ---------------------------------------------------------------------------
// AT9 — FM6: BASE directive replacement
// ---------------------------------------------------------------------------

#[test]
fn at9_fm6_base_directive_replacement() {
    let f = parse_ttl_expect_ok("fm6-base-directive-replacement.ttl");
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].subject, "<http://example/b/rel>");
    assert_eq!(f[0].predicate, "<http://example/b/p>");
    assert_eq!(f[0].object, "<http://example/b/o>");
}

// ---------------------------------------------------------------------------
// AT10 — FM6b: Chained @base
// ---------------------------------------------------------------------------

#[test]
fn at10_fm6b_chained_base_resolution() {
    let f = parse_ttl_expect_ok("fm6-chained-base.ttl");
    let subjects: std::collections::BTreeSet<_> =
        f.iter().map(|t| t.subject.clone()).collect();
    assert!(subjects.contains("<http://example/a/r1>"));
    assert!(subjects.contains("<http://example/b/r2>"));
    assert!(subjects.contains("<http://example/c/r3>"));
}

// ---------------------------------------------------------------------------
// AT11 — FM7: Trailing semicolon
// ---------------------------------------------------------------------------

#[test]
fn at11_fm7_trailing_semicolon_accepted() {
    let f = parse_ttl_expect_ok("fm7-trailing-semicolon.ttl");
    // Fixture asserts exactly three triples (1 + 2 on the second line).
    assert_eq!(f.len(), 3);
}

// ---------------------------------------------------------------------------
// AT12 — FM8: TriG blank-node scope (DIVERGENCE per TTL-BNPFX-001)
// ---------------------------------------------------------------------------

/// **AT12 — FM8: TriG blank-node labels are document-scope.**
///
/// The fixture's original hypothesis (per-graph-block scope) is
/// **explicitly rejected** by pin `TTL-BNPFX-001`. The pin chooses
/// document-scope. This test asserts the pinned reading: `_:b` across
/// the default graph and the two named graphs is the SAME blank node.
/// Recorded as a divergence finding in
/// `crate/rdf-turtle/divergence-findings` because `oxttl` (the ADR-0019
/// §1 oracle) also implements document-scope, so the pin matches the
/// oracle on this input while the fixture text hypothesises the
/// opposite. The fixture stays committed as a regression gate for the
/// pinned reading.
#[test]
fn at12_fm8_trig_bnode_scope_per_graph() {
    let f = parse_trig_expect_ok("fm8-trig-bnode-scope.trig");
    let subjects: std::collections::BTreeSet<_> =
        f.iter().map(|t| t.subject.clone()).collect();
    // All four quads share a subject `_:b` that canonicalises to one
    // canonical bnode label after `Facts::canonicalise`.
    assert_eq!(
        subjects.len(),
        1,
        "TTL-BNPFX-001: _:b is document-scope across TriG graph blocks; \
         got distinct subjects {subjects:?}",
    );
}

// ---------------------------------------------------------------------------
// AT13 — FM9: Numeric literal datatype selection
// ---------------------------------------------------------------------------

#[test]
fn at13_fm9_numeric_literal_type_selection() {
    let f = parse_ttl_expect_ok("fm9-numeric-literal-types.ttl");
    // Each fixture triple has a distinct predicate; we build a map
    // predicate -> object for assertions.
    use std::collections::BTreeMap;
    let mut by_pred: BTreeMap<&str, &str> = BTreeMap::new();
    for t in &f {
        by_pred.insert(t.predicate.as_str(), t.object.as_str());
    }
    assert_eq!(
        by_pred["<http://example/int1>"],
        "\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>",
    );
    assert_eq!(
        by_pred["<http://example/dec1>"],
        "\"1.0\"^^<http://www.w3.org/2001/XMLSchema#decimal>",
    );
    assert_eq!(
        by_pred["<http://example/dbl1>"],
        "\"1.0e0\"^^<http://www.w3.org/2001/XMLSchema#double>",
    );
    assert_eq!(
        by_pred["<http://example/int_pos>"],
        "\"+1\"^^<http://www.w3.org/2001/XMLSchema#integer>",
    );
    assert_eq!(
        by_pred["<http://example/int_neg>"],
        "\"-0\"^^<http://www.w3.org/2001/XMLSchema#integer>",
    );
    assert_eq!(
        by_pred["<http://example/dbl2>"],
        "\"1.5e2\"^^<http://www.w3.org/2001/XMLSchema#double>",
    );
    assert_eq!(
        by_pred["<http://example/dec2>"],
        "\"-1.5\"^^<http://www.w3.org/2001/XMLSchema#decimal>",
    );
}
