//! Snapshot tests for `encode.rs` output.
//!
//! These tests verify that the AST-as-Facts encoding produced by
//! `encode_request` is stable and matches the expected canonical form.
//! Since `insta` is not a dependency of this crate, snapshots are
//! stored as inline `assert_eq!` constants in `tests/snapshots/`.
//!
//! Each test serialises the `Facts` set to a deterministic string form
//! (sorted predicate -> object pairs) and compares it against the
//! stored snapshot. The snapshot format is:
//!
//! ```text
//! <predicate> = <object>
//! ```
//!
//! one pair per line, sorted lexicographically by predicate then object.

#![allow(clippy::missing_panics_doc)]

use rdf_diff::Parser;
use sparql_syntax::SparqlParser;

fn parse_ok(src: &str) -> rdf_diff::Facts {
    SparqlParser::new()
        .parse(src.as_bytes())
        .unwrap_or_else(|e| panic!("expected parse ok, got: {e:?}"))
        .facts
}

/// Serialise a `Facts` set to a canonical multi-line string for snapshot
/// comparison. Each line is `<predicate> = <object>`, sorted by predicate
/// then object.
fn serialise(facts: &rdf_diff::Facts) -> String {
    let mut lines: Vec<String> = facts
        .set
        .keys()
        .map(|f| format!("{} = {}", f.predicate, f.object))
        .collect();
    lines.sort();
    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Snapshot 1 — SELECT query with projection and WHERE clause.
// ---------------------------------------------------------------------------
#[test]
fn snapshot_select_with_projection_and_where() {
    let facts = parse_ok("SELECT ?s ?o WHERE { ?s <http://ex/p> ?o }");
    let got = serialise(&facts);

    // Load expected from snapshot file.
    let expected = include_str!("snapshots/select_with_projection_and_where.txt").trim();
    assert_eq!(
        got, expected,
        "snapshot mismatch for SELECT with projection\ngot:\n{got}\nexpected:\n{expected}"
    );
}

// ---------------------------------------------------------------------------
// Snapshot 2 — ASK query.
// ---------------------------------------------------------------------------
#[test]
fn snapshot_ask_query() {
    let facts = parse_ok("ASK { ?s <http://ex/p> ?o }");
    let got = serialise(&facts);

    let expected = include_str!("snapshots/ask_query.txt").trim();
    assert_eq!(
        got, expected,
        "snapshot mismatch for ASK query\ngot:\n{got}\nexpected:\n{expected}"
    );
}

// ---------------------------------------------------------------------------
// Snapshot 3 — CONSTRUCT query.
// ---------------------------------------------------------------------------
#[test]
fn snapshot_construct_query() {
    let facts = parse_ok(
        "CONSTRUCT { ?s <http://ex/p> ?o } WHERE { ?s <http://ex/r> ?o }",
    );
    let got = serialise(&facts);

    let expected = include_str!("snapshots/construct_query.txt").trim();
    assert_eq!(
        got, expected,
        "snapshot mismatch for CONSTRUCT query\ngot:\n{got}\nexpected:\n{expected}"
    );
}

// ---------------------------------------------------------------------------
// Snapshot 4 — SPARQL UPDATE INSERT DATA.
// ---------------------------------------------------------------------------
#[test]
fn snapshot_insert_data() {
    let facts =
        parse_ok("INSERT DATA { <http://ex/s> <http://ex/p> <http://ex/o> }");
    let got = serialise(&facts);

    let expected = include_str!("snapshots/insert_data.txt").trim();
    assert_eq!(
        got, expected,
        "snapshot mismatch for INSERT DATA\ngot:\n{got}\nexpected:\n{expected}"
    );
}

// ---------------------------------------------------------------------------
// Snapshot 5 — SPARQL UPDATE DELETE WHERE.
// ---------------------------------------------------------------------------
#[test]
fn snapshot_delete_where() {
    let facts = parse_ok("DELETE WHERE { ?s <http://ex/p> ?o }");
    let got = serialise(&facts);

    let expected = include_str!("snapshots/delete_where.txt").trim();
    assert_eq!(
        got, expected,
        "snapshot mismatch for DELETE WHERE\ngot:\n{got}\nexpected:\n{expected}"
    );
}
