//! Integration tests for the adversary Turtle / TriG fixture corpus.
//!
//! Tracked in `docs/verification/tests/catalogue.md` under the `adversary-ttl`
//! section (rows `AT1` through `AT13`). Each test corresponds to a fixture
//! in `tests/adversary-ttl/` and documents the divergence hypothesis from
//! `docs/verification/adversary-findings/ttl.md`.
//!
//! ## Lifecycle
//!
//! Shadow parsers (`rdf-turtle-shadow`) and the main parser (`rdf-turtle`)
//! are claimed by `v1-shadow-ttl` and later Phase-A work. Until both exist
//! the per-fixture parse-and-diff bodies cannot be wired — tests are
//! `#[ignore]`-gated and document the expected divergence so the
//! `v1-reviewer` has a concrete unignore checklist.
//!
//! The fixture-discovery test (`AT0`) is **not** ignored: it runs on every
//! `cargo test --workspace` and verifies that the fixture directory is
//! present and sorted deterministically.
//!
//! ## Integration with `xtask verify`
//!
//! When `v1-ci-wiring` lands the `xtask verify adversary-ttl` sub-command,
//! the same fixtures are fed to the parser ensemble via
//! `cargo xtask verify --adversary-ttl`. The Rust tests here are the
//! pre-landing in-process mirror of that check.
//!
//! ## ADR references
//!
//! - ADR-0019 §4 — adversary-corpus responsibilities.
//! - ADR-0020 §6.5 — cohort-B path claims; adversary paths never overlap
//!   non-adversary test paths by construction.

#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};

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

    // Deterministic sort invariant.
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

/// **AT1 — FM1: PN_LOCAL may start with a digit.**
///
/// `ex:123` is syntactically valid Turtle. Parsers that apply XML NCName
/// rules will reject the input (AcceptRejectSplit); conformant parsers
/// accept it.
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT1)"]
fn at1_fm1_leading_digit_local() {
    let _input = read_fixture("fm1-leading-digit-local.ttl");
    // Wiring (to fill when parsers land):
    //   let main   = rdf_turtle::Parser::default().parse(_input.as_bytes());
    //   let shadow = rdf_turtle_shadow::Parser::default().parse(_input.as_bytes());
    //   // Both should accept; check agreement on the produced triples.
    //   let (main_ok, shadow_ok) = (main.is_ok(), shadow.is_ok());
    //   assert!(main_ok && shadow_ok,
    //       "FM1: expected both parsers to accept; main={main_ok} shadow={shadow_ok}");
    //   let report = diff(&main.unwrap().facts, &shadow.unwrap().facts)
    //       .expect("diff should not be NonCanonical");
    //   assert!(report.is_clean(), "FM1 diff: {:?}", report.divergences);
    todo!("wire AT1 when parsers land")
}

// ---------------------------------------------------------------------------
// AT2 — FM1b: @prefix redefinition
// ---------------------------------------------------------------------------

/// **AT2 — FM1b: @prefix redefinition replaces earlier binding.**
///
/// After a second `@prefix ex:` the first binding must be forgotten.
/// Stale-prefix parsers produce wrong absolute IRIs (FactOnlyIn).
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT2)"]
fn at2_fm1b_prefix_redefinition() {
    let _input = read_fixture("fm1-prefix-redefinition.ttl");
    todo!("wire AT2 when parsers land")
}

// ---------------------------------------------------------------------------
// AT3 — FM2: Percent-encoding in local part
// ---------------------------------------------------------------------------

/// **AT3 — FM2: Percent-encoded local part is NOT decoded.**
///
/// `ex:caf%C3%A9` must produce `<http://example/caf%C3%A9>` — the
/// percent-encoding must not be decoded before IRI concatenation.
/// A decoding parser produces `<http://example/café>`, a different IRI
/// (ObjectMismatch / FactOnlyIn).
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT3)"]
fn at3_fm2_percent_encoding_not_decoded() {
    let _input = read_fixture("fm2-percent-encoding-local.ttl");
    todo!("wire AT3 when parsers land")
}

// ---------------------------------------------------------------------------
// AT4 — FM3: Keyword scope
// ---------------------------------------------------------------------------

/// **AT4 — FM3: `a`/`true`/`false` are position-sensitive.**
///
/// `a` is rdf:type ONLY in predicate position. `true`/`false` are boolean
/// literals ONLY in object position. Over-eager keyword scanners produce
/// parse errors or wrong node types.
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT4)"]
fn at4_fm3_keyword_scope_position_sensitive() {
    let _input = read_fixture("fm3-keyword-scope.ttl");
    todo!("wire AT4 when parsers land")
}

// ---------------------------------------------------------------------------
// AT5 — FM4a: Empty collection = rdf:nil
// ---------------------------------------------------------------------------

/// **AT5 — FM4a: Empty collection `()` resolves to `rdf:nil`.**
///
/// A conformant parser must emit the triple with `rdf:nil` as the object,
/// not a fresh blank node. Divergence: FactOnlyIn on the blank-node variant.
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT5)"]
fn at5_fm4a_empty_collection_is_rdf_nil() {
    let _input = read_fixture("fm4-empty-collection.ttl");
    todo!("wire AT5 when parsers land")
}

// ---------------------------------------------------------------------------
// AT6 — FM4b: Nested collections
// ---------------------------------------------------------------------------

/// **AT6 — FM4b: Nested list `((1))` must not be flattened.**
///
/// The outer `rdf:first` must point to the blank node heading the inner
/// list, not directly to the integer `1`. Flattening produces a different
/// graph (FactOnlyIn / ObjectMismatch).
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT6)"]
fn at6_fm4b_nested_collection_not_flattened() {
    let _input = read_fixture("fm4-nested-collection.ttl");
    todo!("wire AT6 when parsers land")
}

// ---------------------------------------------------------------------------
// AT7 — FM5: Long string with raw newline (positive case)
// ---------------------------------------------------------------------------

/// **AT7 — FM5: Raw newline in `"""…"""` is valid content.**
///
/// A parser that applies short-string rules inside `"""…"""` will reject
/// this valid input (AcceptRejectSplit). The accepted form must carry the
/// newline character in the literal value.
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT7)"]
fn at7_fm5_long_string_raw_newline_accepted() {
    let _input = read_fixture("fm5-long-string-newline.ttl");
    todo!("wire AT7 when parsers land")
}

// ---------------------------------------------------------------------------
// AT8 — FM5: Short string with raw newline (negative case)
// ---------------------------------------------------------------------------

/// **AT8 — FM5: Raw newline in `"…"` is a parse error.**
///
/// A parser that shares a lexer path with long strings will accept this
/// invalid input. Conformant parsers must reject it. Divergence:
/// AcceptRejectSplit.
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT8)"]
fn at8_fm5_short_string_raw_newline_rejected() {
    let _input = read_fixture("fm5-short-string-newline-invalid.ttl");
    // Expected: at least one parser rejects (Diagnostics { fatal: true }).
    // Divergence = AcceptRejectSplit if parsers disagree on accept/reject.
    todo!("wire AT8 when parsers land")
}

// ---------------------------------------------------------------------------
// AT9 — FM6: BASE directive replacement
// ---------------------------------------------------------------------------

/// **AT9 — FM6: SPARQL-style `BASE` replaces active base IRI.**
///
/// After `@base <http://example/a/>` then `BASE <http://example/b/>`,
/// the relative IRI `<rel>` must resolve to `<http://example/b/rel>`.
/// A parser that ignores `BASE` will resolve against the wrong base.
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT9)"]
fn at9_fm6_base_directive_replacement() {
    let _input = read_fixture("fm6-base-directive-replacement.ttl");
    todo!("wire AT9 when parsers land")
}

// ---------------------------------------------------------------------------
// AT10 — FM6b: Chained @base
// ---------------------------------------------------------------------------

/// **AT10 — FM6b: Each `@base` replaces the previous; IRI resolution uses
/// the base in scope at that point in the document.**
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT10)"]
fn at10_fm6b_chained_base_resolution() {
    let _input = read_fixture("fm6-chained-base.ttl");
    todo!("wire AT10 when parsers land")
}

// ---------------------------------------------------------------------------
// AT11 — FM7: Trailing semicolon
// ---------------------------------------------------------------------------

/// **AT11 — FM7: Trailing `;` before `.` is valid Turtle.**
///
/// The grammar's optional `(verb objectList)?` after `;` means a bare
/// trailing semicolon is valid. Strict parsers that require a predicate
/// after every `;` will reject this input (AcceptRejectSplit).
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT11)"]
fn at11_fm7_trailing_semicolon_accepted() {
    let _input = read_fixture("fm7-trailing-semicolon.ttl");
    todo!("wire AT11 when parsers land")
}

// ---------------------------------------------------------------------------
// AT12 — FM8: TriG blank-node scope per graph
// ---------------------------------------------------------------------------

/// **AT12 — FM8: TriG blank-node labels are scoped per graph block.**
///
/// `_:b` in the default graph and `_:b` in `<http://example/g1>` are
/// distinct blank nodes. A document-level blank-node table incorrectly
/// unifies them (FactOnlyIn divergence after canonicalisation).
///
/// Unignore when a TriG parser implementing the `rdf_diff::Parser` trait
/// lands.
#[test]
#[ignore = "unignore when trig parser lands (AT12)"]
fn at12_fm8_trig_bnode_scope_per_graph() {
    let _input = read_fixture("fm8-trig-bnode-scope.trig");
    todo!("wire AT12 when trig parser lands")
}

// ---------------------------------------------------------------------------
// AT13 — FM9: Numeric literal datatype selection
// ---------------------------------------------------------------------------

/// **AT13 — FM9: Numeric token shape determines XSD datatype.**
///
/// `1` → `xsd:integer`; `1.0` → `xsd:decimal`; `1.0e0` → `xsd:double`.
/// Edge cases: `-0` and `+1` must be `xsd:integer`. A parser that strips
/// fractional zeros or misidentifies sign-prefixed tokens assigns the wrong
/// datatype (ObjectMismatch on the datatype IRI).
///
/// Unignore when `rdf-turtle` + `rdf-turtle-shadow` land.
#[test]
#[ignore = "unignore when rdf-turtle + rdf-turtle-shadow land (AT13)"]
fn at13_fm9_numeric_literal_type_selection() {
    let _input = read_fixture("fm9-numeric-literal-types.ttl");
    todo!("wire AT13 when parsers land")
}
