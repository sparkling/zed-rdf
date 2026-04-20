//! Adversary-sparql fixture tests for `sparql-syntax` (main parser only).
//!
//! These tests consume the 13 adversary fixtures from
//! `crates/testing/rdf-diff/tests/adversary-sparql/` and validate the
//! main `SparqlParser`'s accept/reject behaviour against each one.
//!
//! Fixtures annotated "Accept" must parse successfully (`Ok`).
//! Fixtures annotated "Reject" must produce a fatal diagnostic (`Err`).
//!
//! Tests are un-gated (no `#[ignore]`) because the main parser is fully
//! implemented. If a test fails, add `// RETIREMENT: <plan>` above the
//! function and restore `#[ignore]`.

#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};

use rdf_diff::Parser;
use sparql_syntax::SparqlParser;

fn fixture_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/sparql-syntax at test time.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("..")
        .join("testing")
        .join("rdf-diff")
        .join("tests")
        .join("adversary-sparql")
}

fn read_fixture(name: &str) -> String {
    let path = fixture_root().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {name}: {e}"))
}

fn parse(src: &str) -> Result<rdf_diff::ParseOutcome, rdf_diff::Diagnostics> {
    SparqlParser::new().parse(src.as_bytes())
}

// ---------------------------------------------------------------------------
// FM1 — OPTIONAL + FILTER on unbound variable.
// Expected: Accept (parse ok; divergence is at evaluation semantics).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm1_optional_filter_unbound_accepted() {
    let src = read_fixture("fm1-optional-filter-unbound.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM1: main parser unexpectedly rejected OPTIONAL+FILTER(unbound): {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM2 — MINUS with disjoint variable sets.
// Expected: Accept.
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm2_minus_no_shared_variable_accepted() {
    let src = read_fixture("fm2-minus-no-shared-variable.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM2: main parser unexpectedly rejected MINUS with disjoint vars: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM3 — CONSTRUCT blank node per solution row.
// Expected: Accept.
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm3_construct_bnode_per_row_accepted() {
    let src = read_fixture("fm3-construct-bnode-per-row.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM3: main parser unexpectedly rejected CONSTRUCT with blank-node template: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM4 — HAVING references SELECT aggregate alias.
// Expected: Accept.
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm4_having_select_alias_accepted() {
    let src = read_fixture("fm4-having-select-alias.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM4: main parser unexpectedly rejected HAVING with SELECT alias: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM5 — BASE declaration inside WHERE clause.
// Expected: Reject (SPARQL-PROLOGUE-001 parse error).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm5_base_mid_query_rejected() {
    let src = read_fixture("fm5-base-mid-query.sparql");
    let result = parse(&src);
    assert!(
        result.is_err(),
        "FM5: main parser must reject BASE inside WHERE clause, but it accepted"
    );
    let diag = result.unwrap_err();
    assert!(
        diag.messages.iter().any(|m| m.contains("SPARQL-PROLOGUE-001")),
        "FM5: expected SPARQL-PROLOGUE-001 diagnostic, got: {:?}",
        diag.messages
    );
}

// ---------------------------------------------------------------------------
// FM6 — GRAPH ?g does not match default graph.
// Expected: Accept (parse ok; divergence is at evaluation).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm6_graph_variable_default_graph_accepted() {
    let src = read_fixture("fm6-graph-variable-default-graph.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM6: main parser unexpectedly rejected GRAPH ?g query: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM7 — FILTER NOT EXISTS vs OPTIONAL/FILTER(!BOUND) rewrite.
// Expected: Accept.
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm7_filter_not_exists_accepted() {
    let src = read_fixture("fm7-filter-not-exists-vs-optional.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM7: main parser unexpectedly rejected FILTER NOT EXISTS query: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM8 — INSERT DATA blank node scope per operation.
// Expected: Accept (as SPARQL Update per spec §3.1.1).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm8_insert_data_bnode_scope_accepted() {
    let src = read_fixture("fm8-insert-data-bnode-scope.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM8: main parser unexpectedly rejected INSERT DATA with GRAPH clause: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM9 — Inverse negated property path precedence.
// Expected: Accept (^!(p) parses as ^(!(p)) per §9.3).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm9_inverse_negated_property_path_accepted() {
    let src = read_fixture("fm9-inverse-negated-property-path.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM9: main parser unexpectedly rejected inverse negated property path: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM10 — Nested SERVICE (SERVICE inside SERVICE).
// Expected: Accept per spec grammar (no depth limit).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm10_service_nesting_accepted() {
    let src = read_fixture("fm10-service-nesting.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM10: main parser unexpectedly rejected nested SERVICE: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM11 — BIND legal scoping: variable introduced by BIND used after it.
// Expected: Accept.
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm11_bind_scoping_legal_accepted() {
    let src = read_fixture("fm11-bind-scoping.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM11: main parser unexpectedly rejected legal BIND scoping: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// FM11b — BIND violation: variable used before its BIND definition.
// Expected: Reject (SPARQL-BIND-001 query error per §18.2.1).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm11b_bind_scoping_violation_rejected() {
    let src = read_fixture("fm11b-bind-scoping-violation.sparql");
    let result = parse(&src);
    assert!(
        result.is_err(),
        "FM11b: main parser must reject BIND scoping violation, but it accepted"
    );
    let diag = result.unwrap_err();
    assert!(
        diag.messages.iter().any(|m| m.contains("SPARQL-BIND-001")),
        "FM11b: expected SPARQL-BIND-001 diagnostic, got: {:?}",
        diag.messages
    );
}

// ---------------------------------------------------------------------------
// FM12 — Subquery projection-list scope.
// Expected: Accept (outer ?internal is unbound but parse succeeds).
// ---------------------------------------------------------------------------
#[test]
fn adversary_fm12_subquery_projection_accepted() {
    let src = read_fixture("fm12-subquery-projection.sparql");
    let result = parse(&src);
    assert!(
        result.is_ok(),
        "FM12: main parser unexpectedly rejected subquery projection query: {result:?}"
    );
}
