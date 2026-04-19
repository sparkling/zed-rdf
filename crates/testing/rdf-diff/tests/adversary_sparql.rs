//! Adversary SPARQL 1.1 grammar fixtures — integration with `xtask verify`.
//!
//! Agent: `v1-adv-sparql` (cohort B, adversary hive, verification-v1 sweep).
//! ADR references: ADR-0019 §4, ADR-0020 §1.4.
//!
//! Each `.sparql` file under `tests/adversary-sparql/` is a targeted
//! adversary fixture for one of the 9 failure modes documented in
//! `docs/verification/adversary-findings/sparql.md`. The fixtures exercise
//! grammar-level and specification-semantic divergences. Execution
//! semantics are deliberately out of scope.
//!
//! ## Test structure
//!
//! - `adversary_sparql_fixtures_present` — always-on; asserts the fixture
//!   directory exists and contains the expected 12 files (9 primary +
//!   3 extended). Fails fast if a fixture was accidentally deleted.
//!
//! - `adversary_sparql_fixture_metadata` — always-on; reads each fixture,
//!   checks it is non-empty and begins with a `#` comment carrying the
//!   fixture name. Cheap smoke test.
//!
//! - `adversary_sparql_fm5_must_be_reject_candidate` and
//!   `adversary_sparql_fm11b_must_be_reject_candidate` — always-on;
//!   assert that the fixtures annotated as "Reject (parse error)" are
//!   flagged in their header comment. These tests surface the real
//!   divergence (accept/reject split) once parsers land.
//!
//! - `adversary_sparql_shadow_vs_main_*` — `#[ignore]`-gated; one per
//!   fixture; wired up when `sparql-syntax` + `sparql-syntax-shadow` land.
//!   The `v1-reviewer` unignores them at handoff.
//!
//! ## `xtask verify` integration
//!
//! `xtask verify --adversary-sparql` runs `cargo test -p rdf-diff
//! --test adversary_sparql` and checks that `adversary_sparql_fixtures_present`
//! and `adversary_sparql_fixture_metadata` pass. The ignored parser
//! integration tests become hard gates once the shadow crates land.

#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};

/// Fixture filenames in lexicographic sort order (matches `collect_sparql_fixtures`).
const EXPECTED_FIXTURES: &[&str] = &[
    "fm1-optional-filter-unbound.sparql",
    "fm10-service-nesting.sparql",
    "fm11-bind-scoping.sparql",
    "fm11b-bind-scoping-violation.sparql",
    "fm12-subquery-projection.sparql",
    "fm2-minus-no-shared-variable.sparql",
    "fm3-construct-bnode-per-row.sparql",
    "fm4-having-select-alias.sparql",
    "fm5-base-mid-query.sparql",
    "fm6-graph-variable-default-graph.sparql",
    "fm7-filter-not-exists-vs-optional.sparql",
    "fm8-insert-data-bnode-scope.sparql",
    "fm9-inverse-negated-property-path.sparql",
];

fn adversary_sparql_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("tests").join("adversary-sparql")
}

fn collect_sparql_fixtures() -> Vec<PathBuf> {
    let root = adversary_sparql_root();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sparql"))
        .collect();
    out.sort();
    out
}

/// **A1 — All 13 adversary SPARQL fixtures are present and sorted.**
///
/// Fails immediately if any fixture file is missing or if new fixtures are
/// added without updating this catalogue. Kept always-on so `cargo test`
/// on any worktree that claims `adversary-sparql/**` is protected.
#[test]
fn adversary_sparql_fixtures_present() {
    let root = adversary_sparql_root();
    assert!(
        root.exists(),
        "adversary-sparql fixture directory missing at {}",
        root.display()
    );

    let fixtures = collect_sparql_fixtures();
    let names: Vec<&str> = fixtures
        .iter()
        .filter_map(|p| p.file_name()?.to_str())
        .collect();

    assert_eq!(
        names, EXPECTED_FIXTURES,
        "adversary-sparql fixture set mismatch.\n  found:    {names:?}\n  expected: {EXPECTED_FIXTURES:?}"
    );
}

/// **A2 — Every fixture is non-empty and starts with a `#` comment header.**
///
/// The comment header carries the fixture name and spec reference; its
/// presence is load-bearing for `v1-adv-veto`'s triage.
#[test]
fn adversary_sparql_fixture_metadata() {
    for path in collect_sparql_fixtures() {
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(
            !content.trim().is_empty(),
            "fixture is empty: {}",
            path.display()
        );
        assert!(
            content.trim_start().starts_with('#'),
            "fixture does not start with a # comment: {}",
            path.display()
        );
    }
}

/// **A3 — fm5 reject-candidate fixture contains the expected annotation.**
///
/// fm5 (BASE mid-query) must be rejected by a conformant parser. The fixture
/// header must contain the string "PARSE ERROR" so that automated triage can
/// classify it as an accept/reject divergence target.
#[test]
fn adversary_sparql_fm5_must_be_reject_candidate() {
    let path = adversary_sparql_root().join("fm5-base-mid-query.sparql");
    let content = std::fs::read_to_string(&path)
        .expect("fm5 fixture must exist");
    assert!(
        content.contains("PARSE ERROR"),
        "fm5 fixture must be annotated 'PARSE ERROR' to flag it as a reject candidate"
    );
}

/// **A4 — fm11b reject-candidate fixture contains the expected annotation.**
///
/// fm11b (BIND scoping violation) must be rejected. Header must say
/// "QUERY ERROR".
#[test]
fn adversary_sparql_fm11b_must_be_reject_candidate() {
    let path = adversary_sparql_root().join("fm11b-bind-scoping-violation.sparql");
    let content = std::fs::read_to_string(&path)
        .expect("fm11b fixture must exist");
    assert!(
        content.contains("QUERY ERROR"),
        "fm11b fixture must be annotated 'QUERY ERROR' to flag it as a reject candidate"
    );
}

/// **A5 — README is present.**
#[test]
fn adversary_sparql_readme_present() {
    let readme = adversary_sparql_root().join("README.md");
    assert!(
        readme.exists(),
        "adversary-sparql/README.md missing at {}",
        readme.display()
    );
}

// ---------------------------------------------------------------------------
// Parser integration stubs (ignored until shadow crates land).
// One test per primary finding. Unignore at v1-reviewer handoff.
// ---------------------------------------------------------------------------

/// **I-FM1 — fm1: shadow vs main agree on OPTIONAL+FILTER(unbound) query.**
///
/// Both parsers must accept. Post-parse: diff must be clean on the
/// *query tree* representation (grammar-level, not eval-level).
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm1_shadow_vs_main() {
    // Wiring (replace todo when shadow crates land):
    //   let src = include_str!("adversary-sparql/fm1-optional-filter-unbound.sparql");
    //   let main_result   = sparql_syntax::Parser::default().parse(src.as_bytes());
    //   let shadow_result = sparql_syntax_shadow::Parser::default().parse(src.as_bytes());
    //   assert!(main_result.is_ok(),   "main parser rejected fm1: {main_result:?}");
    //   assert!(shadow_result.is_ok(), "shadow parser rejected fm1: {shadow_result:?}");
    //   let report = rdf_diff::diff(&main_result.unwrap().facts,
    //                               &shadow_result.unwrap().facts).unwrap();
    //   assert!(report.is_clean(), "fm1 divergence: {:?}", report.divergences);
}

/// **I-FM2 — fm2: MINUS with disjoint variables.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm2_shadow_vs_main() {}

/// **I-FM3 — fm3: CONSTRUCT blank node per solution row.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm3_shadow_vs_main() {}

/// **I-FM4 — fm4: HAVING references SELECT aggregate alias.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm4_shadow_vs_main() {}

/// **I-FM5 — fm5: BASE mid-query must be REJECTED by all conformant parsers.**
///
/// Both parsers must produce `Err(Diagnostics { fatal: true, .. })`.
/// If either accepts, that is the real divergence this fixture is designed
/// to surface.
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm5_both_must_reject() {
    // Wiring:
    //   let src = include_str!("adversary-sparql/fm5-base-mid-query.sparql");
    //   let main_result   = sparql_syntax::Parser::default().parse(src.as_bytes());
    //   let shadow_result = sparql_syntax_shadow::Parser::default().parse(src.as_bytes());
    //   assert!(main_result.is_err(),   "main parser accepted invalid fm5 (BASE mid-query)");
    //   assert!(shadow_result.is_err(), "shadow parser accepted invalid fm5 (BASE mid-query)");
}

/// **I-FM6 — fm6: GRAPH ?g excludes default graph.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm6_shadow_vs_main() {}

/// **I-FM7 — fm7: FILTER NOT EXISTS vs OPTIONAL rewrite.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm7_shadow_vs_main() {}

/// **I-FM8 — fm8: INSERT DATA blank node scope.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm8_shadow_vs_main() {}

/// **I-FM9 — fm9: inverse negated property path precedence.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm9_shadow_vs_main() {}

/// **I-FM10 — fm10: nested SERVICE.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm10_shadow_vs_main() {}

/// **I-FM11 — fm11: BIND legal scoping.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm11_shadow_vs_main() {}

/// **I-FM11b — fm11b: BIND violation must be rejected.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm11b_both_must_reject() {}

/// **I-FM12 — fm12: subquery projection-list scope.**
#[test]
#[ignore = "unignore when sparql-syntax + sparql-syntax-shadow land (v1-reviewer)"]
fn adversary_sparql_fm12_shadow_vs_main() {}
