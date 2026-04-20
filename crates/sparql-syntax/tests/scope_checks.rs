//! Unit tests for the three named scope-check error codes.
//!
//! Error codes under test:
//!
//! - `SPARQL-PROLOGUE-001` — `BASE`/`PREFIX` appear only in the Prologue
//!   (§4.1); mid-query use is a parse error (adversary FM5).
//! - `SPARQL-BIND-001` — `BIND` introduces a variable; that variable MUST NOT
//!   already appear in the surrounding group graph pattern up to the point of
//!   BIND (§18.2.1; adversary FM11b).
//! - `SPARQL-UPDATE-001` — `INSERT DATA` / `DELETE DATA` forbid variables
//!   (§3.1.1 / §3.1.2); `DELETE DATA` additionally forbids blank nodes
//!   (adversary FM8 complementary).
//!
//! Each error code has two tests:
//!   1. Bad input — confirms the code is emitted.
//!   2. Good input — confirms the code is NOT emitted (parse succeeds).

#![allow(clippy::missing_panics_doc)]

use rdf_diff::Parser;
use sparql_syntax::SparqlParser;

fn parse(src: &str) -> Result<rdf_diff::ParseOutcome, rdf_diff::Diagnostics> {
    SparqlParser::new().parse(src.as_bytes())
}

// ===========================================================================
// SPARQL-PROLOGUE-001
// ===========================================================================

/// Bad: BASE declaration inside a WHERE clause is a parse error.
///
/// Per §4.1, `Prologue ::= (BaseDecl | PrefixDecl)*`. BASE is syntactically
/// restricted to the prologue that precedes the query form keyword. Placing
/// BASE inside `{ ... }` is invalid SPARQL 1.1.
#[test]
fn prologue_001_base_mid_where_emits_error() {
    let src = "BASE <http://example/a/>\n\
               SELECT * WHERE {\n\
                 <foo> <http://example/p> ?o .\n\
                 BASE <http://example/b/>\n\
                 <bar> ?q ?r .\n\
               }";
    let result = parse(src);
    assert!(
        result.is_err(),
        "SPARQL-PROLOGUE-001: expected parse error for BASE inside WHERE, got Ok"
    );
    let diag = result.unwrap_err();
    assert!(
        diag.messages.iter().any(|m| m.contains("SPARQL-PROLOGUE-001")),
        "SPARQL-PROLOGUE-001: expected diagnostic code in messages, got: {:?}",
        diag.messages
    );
}

/// Good: BASE declaration in the prologue (before SELECT) is valid SPARQL 1.1.
#[test]
fn prologue_001_base_in_prologue_is_accepted() {
    // BASE is legal only before the query form keyword.
    let src = "BASE <http://example/>\nSELECT * WHERE { ?s ?p ?o }";
    let result = parse(src);
    assert!(
        result.is_ok(),
        "SPARQL-PROLOGUE-001: BASE in prologue must be accepted, got: {result:?}"
    );
    // No SPARQL-PROLOGUE-001 diagnostic should appear.
    if let Ok(outcome) = &result {
        assert!(
            !outcome
                .warnings
                .messages
                .iter()
                .any(|m| m.contains("SPARQL-PROLOGUE-001")),
            "SPARQL-PROLOGUE-001: unexpected diagnostic in warnings for legal BASE"
        );
    }
}

// ===========================================================================
// SPARQL-BIND-001
// ===========================================================================

/// Bad: BIND introduces a variable that already appears in the same group
/// graph pattern before the BIND.
///
/// Per §18.2.1, the BIND target must not appear in the enclosing group graph
/// pattern up to (and not including) the BIND expression. Using `?x` in a
/// triple pattern before `BIND(... AS ?x)` in the same group graph pattern
/// is a query error.
#[test]
fn bind_001_variable_already_in_scope_emits_error() {
    // ?x appears in the triple pattern before BIND introduces it.
    let src = "SELECT * WHERE { \
               ?s <http://ex/q> ?x . \
               BIND(STR(?o) AS ?x) \
               ?s <http://ex/p> ?o \
               }";
    let result = parse(src);
    assert!(
        result.is_err(),
        "SPARQL-BIND-001: expected parse error for BIND redefining ?x, got Ok"
    );
    let diag = result.unwrap_err();
    assert!(
        diag.messages.iter().any(|m| m.contains("SPARQL-BIND-001")),
        "SPARQL-BIND-001: expected diagnostic code in messages, got: {:?}",
        diag.messages
    );
}

/// Good: BIND introduces a new variable; the same variable is used only after
/// the BIND in the same group graph pattern.
///
/// Per §18.2.1, using `?x` in a triple pattern AFTER the BIND that introduces
/// it is legal.
#[test]
fn bind_001_fresh_variable_after_bind_is_accepted() {
    // ?x is introduced by BIND; the following triple pattern uses it legally.
    let src = "SELECT * WHERE { \
               ?s <http://ex/p> ?o . \
               BIND(CONCAT(STR(?o), \"-suffix\") AS ?x) \
               ?s <http://ex/q> ?x \
               }";
    let result = parse(src);
    assert!(
        result.is_ok(),
        "SPARQL-BIND-001: BIND followed by use of ?x must be accepted, got: {result:?}"
    );
}

// ===========================================================================
// SPARQL-UPDATE-001
// ===========================================================================

/// Bad: INSERT DATA contains a variable — variables are forbidden per §3.1.1.
///
/// The `InsertData` production in the SPARQL Update grammar requires ground
/// terms only (no variables). A variable such as `?x` in the triple data
/// must be rejected.
#[test]
fn update_001_insert_data_with_variable_emits_error() {
    let src = "INSERT DATA { <http://ex/s> <http://ex/p> ?x }";
    let result = parse(src);
    assert!(
        result.is_err(),
        "SPARQL-UPDATE-001: expected parse error for variable in INSERT DATA, got Ok"
    );
    let diag = result.unwrap_err();
    assert!(
        diag.messages.iter().any(|m| m.contains("SPARQL-UPDATE-001")),
        "SPARQL-UPDATE-001: expected diagnostic code in messages, got: {:?}",
        diag.messages
    );
}

/// Good: INSERT DATA with ground triples only (IRIs and literals) is valid
/// SPARQL Update per §3.1.1.
#[test]
fn update_001_insert_data_ground_triples_accepted() {
    let src = "INSERT DATA { <http://ex/s> <http://ex/p> <http://ex/o> }";
    let result = parse(src);
    assert!(
        result.is_ok(),
        "SPARQL-UPDATE-001: INSERT DATA with ground triples must be accepted, got: {result:?}"
    );
}

/// Bad: DELETE DATA contains a blank node — blank nodes are additionally
/// forbidden in DELETE DATA per §3.1.2.
///
/// While blank nodes are allowed in INSERT DATA (they are treated as fresh
/// blank nodes per-operation), DELETE DATA must use only IRIs and literals
/// to identify specific triples to delete.
#[test]
fn update_001_delete_data_with_bnode_emits_error() {
    let src = "DELETE DATA { _:b <http://ex/p> <http://ex/o> }";
    let result = parse(src);
    assert!(
        result.is_err(),
        "SPARQL-UPDATE-001: expected parse error for blank node in DELETE DATA, got Ok"
    );
    let diag = result.unwrap_err();
    assert!(
        diag.messages.iter().any(|m| m.contains("SPARQL-UPDATE-001")),
        "SPARQL-UPDATE-001: expected diagnostic code in messages for bnode in DELETE DATA, got: {:?}",
        diag.messages
    );
}

/// Good: DELETE DATA with ground triples (no blank nodes, no variables) is
/// valid SPARQL Update per §3.1.2.
#[test]
fn update_001_delete_data_ground_triples_accepted() {
    let src = "DELETE DATA { <http://ex/s> <http://ex/p> <http://ex/o> }";
    let result = parse(src);
    assert!(
        result.is_ok(),
        "SPARQL-UPDATE-001: DELETE DATA with ground triples must be accepted, got: {result:?}"
    );
}
