//! Snapshot tests for `rdf-n3`.
//!
//! Covers:
//! - Plain Turtle (N3 must accept all valid Turtle).
//! - `@keywords` shorthand.
//! - Simple quoted formula `{ … }`.
//! - `=>` (log:implies) triple.
//! - `is P of O` reverse property path.
//! - Negative cases (syntax errors).
//! - `@forAll` / `@forSome` warning emission.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use rdf_diff::{Diagnostics, Fact, Parser as _};
use rdf_n3::N3Parser;

fn parse(src: &str) -> rdf_diff::ParseOutcome {
    N3Parser::new()
        .parse(src.as_bytes())
        .unwrap_or_else(|d: Diagnostics| panic!("rejected: {:?}", d))
}

fn reject(src: &str) -> Diagnostics {
    match N3Parser::new().parse(src.as_bytes()) {
        Ok(o) => panic!("expected rejection for:\n{src}\ngot: {:?}", o.facts.set.keys().collect::<Vec<_>>()),
        Err(d) => d,
    }
}

fn facts(out: &rdf_diff::ParseOutcome) -> Vec<Fact> {
    out.facts.set.keys().cloned().collect()
}

// ==========================================================================
// 1. Plain Turtle — N3 is a superset, so all valid Turtle must parse.
// ==========================================================================

#[test]
fn turtle_plain_triple() {
    let out = parse("<http://a/s> <http://a/p> <http://a/o> .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].subject, "<http://a/s>");
    assert_eq!(f[0].predicate, "<http://a/p>");
    assert_eq!(f[0].object, "<http://a/o>");
}

#[test]
fn turtle_prefix_directive() {
    let out = parse("@prefix ex: <http://example/> . ex:s ex:p ex:o .");
    let f = facts(&out);
    assert_eq!(f[0].subject, "<http://example/s>");
    assert_eq!(f[0].predicate, "<http://example/p>");
    assert_eq!(f[0].object, "<http://example/o>");
}

#[test]
fn turtle_sparql_prefix_and_base() {
    let out = parse("BASE <http://example/>\nPREFIX ex: <http://example/>\nex:s <p> <o> .");
    let f = facts(&out);
    assert_eq!(f[0].predicate, "<http://example/p>");
}

#[test]
fn turtle_a_keyword_is_rdf_type() {
    let out = parse("<http://a/s> a <http://a/C> .");
    let f = facts(&out);
    assert_eq!(
        f[0].predicate,
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
    );
}

#[test]
fn turtle_comma_and_semicolon() {
    let out = parse(
        "<http://a/s> <http://a/p> <http://a/o1>, <http://a/o2> ; <http://a/q> <http://a/o3> .",
    );
    assert_eq!(facts(&out).len(), 3);
}

#[test]
fn turtle_blank_node_property_list() {
    let out = parse("[ <http://a/p> <http://a/o> ] <http://a/q> <http://a/o2> .");
    let f = facts(&out);
    assert_eq!(f.len(), 2);
    assert!(f.iter().any(|t| t.subject.starts_with("_:")));
}

#[test]
fn turtle_empty_collection_is_rdf_nil() {
    let out = parse("<http://a/s> <http://a/p> () .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(
        f[0].object,
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"
    );
}

#[test]
fn turtle_literal_lang_tag() {
    let out = parse(r#"<http://a/s> <http://a/p> "Hello"@en ."#);
    let f = facts(&out);
    assert_eq!(f[0].object, "\"Hello\"@en");
}

#[test]
fn turtle_integer_literal() {
    let out = parse("<http://a/s> <http://a/p> 42 .");
    let f = facts(&out);
    assert!(f[0].object.contains("integer"));
}

#[test]
fn turtle_boolean_true() {
    let out = parse("<http://a/s> <http://a/p> true .");
    let f = facts(&out);
    assert!(f[0].object.contains("boolean"));
    assert!(f[0].object.contains("true"));
}

#[test]
fn turtle_base_relative_iri() {
    let out = parse("@base <http://example/> . <s> <p> <o> .");
    let f = facts(&out);
    assert_eq!(f[0].subject, "<http://example/s>");
}

#[test]
fn turtle_xsd_string_collapses() {
    let out = parse(
        "<http://a/s> <http://a/p> \"hi\"^^<http://www.w3.org/2001/XMLSchema#string> .",
    );
    let f = facts(&out);
    assert_eq!(f[0].object, "\"hi\"");
}

#[test]
fn turtle_uchar_escape() {
    let out = parse(r#"<http://a/s> <http://a/p> "\u00E9" ."#);
    let f = facts(&out);
    assert!(f[0].object.contains('é'));
}

#[test]
fn turtle_long_string() {
    let out = parse("<http://a/s> <http://a/p> \"\"\"line1\nline2\"\"\" .");
    let f = facts(&out);
    assert!(f[0].object.contains('\n'));
}

#[test]
fn turtle_short_string_rejects_raw_newline() {
    let d = reject("<http://a/s> <http://a/p> \"line1\nline2\" .");
    assert!(d.fatal);
    // The error message should reference the literal escape pin.
    assert!(
        d.messages.iter().any(|m| m.contains("TTL-LITESC-001")),
        "expected TTL-LITESC-001 in {:?}",
        d.messages
    );
}

// ==========================================================================
// 2. @keywords directive
// ==========================================================================

#[test]
fn keywords_directive_enables_bare_a() {
    // After @keywords, bare `a` is rdf:type.
    let out = parse(
        "@keywords a .\n\
         <http://a/s> a <http://a/C> .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(
        f[0].predicate,
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
    );
}

#[test]
fn keywords_directive_multiple_words() {
    // @keywords with multiple words separated by commas.
    let out = parse(
        "@keywords a, is, of, has .\n\
         <http://a/s> a <http://a/C> .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 1);
}

#[test]
fn keywords_empty_list_accepted() {
    // @keywords with no words is valid.
    let out = parse(
        "@keywords .\n\
         <http://a/s> <http://a/p> <http://a/o> .",
    );
    assert_eq!(facts(&out).len(), 1);
}

// ==========================================================================
// 3. Quoted formulas { … }
// ==========================================================================

#[test]
fn formula_as_subject_emits_triples_in_graph() {
    // `{ <s> <p> <o> . }` as a subject in a triple.
    // The triples inside the formula are emitted with graph = Some(formula_bnode).
    let out = parse(
        "{ <http://a/s> <http://a/p> <http://a/o> . } <http://a/q> <http://a/r> .",
    );
    let f = facts(&out);
    // Should have at least 2 facts: the formula-scoped inner triple and
    // the outer triple.
    assert!(f.len() >= 2, "expected >= 2 facts, got {}", f.len());
    // Verify there is at least one fact inside a formula graph.
    assert!(
        f.iter().any(|t| t.graph.is_some()),
        "expected at least one formula-scoped fact"
    );
    // And at least one fact in the default graph.
    assert!(
        f.iter().any(|t| t.graph.is_none()),
        "expected at least one default-graph fact"
    );
}

#[test]
fn formula_as_object() {
    // `<s> <p> { <a> <b> <c> . } .`
    let out = parse(
        "<http://a/s> <http://a/p> { <http://a/a> <http://a/b> <http://a/c> . } .",
    );
    let f = facts(&out);
    assert!(f.len() >= 2);
    // The outer triple has the formula bnode as its object (canonicalised to _:cN).
    let outer = f.iter().find(|t| t.graph.is_none()).expect("outer triple");
    assert!(
        outer.object.starts_with("_:"),
        "object should be a blank node (formula), got {}",
        outer.object
    );
}

#[test]
fn empty_formula() {
    // Empty formula `{}` is valid; the object is a blank node (canonicalised to _:cN).
    let out = parse("<http://a/s> <http://a/p> {} .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    // After canonicalisation, the formula blank node is relabelled to _:cN.
    assert!(
        f[0].object.starts_with("_:"),
        "object should be a blank node, got {}",
        f[0].object
    );
}

#[test]
fn nested_formula() {
    // Nested formula `{ { <a> <b> <c> . } <d> <e> . }`.
    let out = parse(
        "{ { <http://a/a> <http://a/b> <http://a/c> . } <http://a/d> <http://a/e> . } <http://a/q> <http://a/r> .",
    );
    let f = facts(&out);
    // Should have the innermost triple (two levels of graph), the outer
    // formula triple, and the outermost connecting triple.
    assert!(f.len() >= 3, "expected >= 3 facts, got {}", f.len());
}

// ==========================================================================
// 4. `=>` (log:implies)
// ==========================================================================

#[test]
fn implies_operator_as_predicate() {
    // `<s> => <o> .` — emits triple with predicate log:implies.
    let out = parse("<http://a/s> => <http://a/o> .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(
        f[0].predicate,
        "<http://www.w3.org/2000/10/swap/log#implies>"
    );
    assert_eq!(f[0].subject, "<http://a/s>");
    assert_eq!(f[0].object, "<http://a/o>");
}

#[test]
fn implies_with_formulas() {
    // Classic N3 rule: `{ <s> <p> <o> . } => { <s> <q> <r> . } .`
    let out = parse(
        "{ <http://a/s> <http://a/p> <http://a/o> . } \
         => \
         { <http://a/s> <http://a/q> <http://a/r> . } .",
    );
    let f = facts(&out);
    // The outer triple uses log:implies as predicate.
    let outer = f
        .iter()
        .find(|t| {
            t.graph.is_none()
                && t.predicate == "<http://www.w3.org/2000/10/swap/log#implies>"
        })
        .expect("log:implies triple in default graph");
    // Both subject and object are blank nodes (formula IDs, canonicalised to _:cN).
    assert!(
        outer.subject.starts_with("_:"),
        "subject should be a blank node (formula), got {}",
        outer.subject
    );
    assert!(
        outer.object.starts_with("_:"),
        "object should be a blank node (formula), got {}",
        outer.object
    );
}

// ==========================================================================
// 5. `is P of O` reverse property path
// ==========================================================================

#[test]
fn is_of_emits_reversed_triple() {
    // `<y> is <p> of <x> .` should emit `(<x>, <p>, <y>)`.
    let out = parse("<http://a/y> is <http://a/p> of <http://a/x> .");
    let f = facts(&out);
    assert_eq!(f.len(), 1, "expected 1 fact, got {:?}", f);
    // With `is P of`, the emitted triple reverses: object = <y>, predicate = <p>, subject = <x>
    assert_eq!(f[0].predicate, "<http://a/p>");
    // `<y>` was the original subject, `<x>` was the `of` argument.
    // is P of means: the subject IS the P of the object → (object, P, subject).
    assert_eq!(f[0].subject, "<http://a/x>");
    assert_eq!(f[0].object, "<http://a/y>");
}

#[test]
fn is_of_with_prefix() {
    let out = parse(
        "@prefix ex: <http://example/> .\n\
         ex:y is ex:parent of ex:x .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].subject, "<http://example/x>");
    assert_eq!(f[0].predicate, "<http://example/parent>");
    assert_eq!(f[0].object, "<http://example/y>");
}

#[test]
fn is_of_multiple_objects() {
    // `<y> is <p> of <x1>, <x2> .` — emits two triples.
    let out = parse(
        "<http://a/y> is <http://a/p> of <http://a/x1>, <http://a/x2> .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 2, "expected 2 facts, got {:?}", f);
}

// ==========================================================================
// 6. @forAll / @forSome — parse + warn, don't error
// ==========================================================================

#[test]
fn forall_skipped_with_warning() {
    let out = parse(
        "@forAll <http://a/x> .\n\
         <http://a/s> <http://a/p> <http://a/o> .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    // Warning should mention @forAll.
    assert!(
        out.warnings.messages.iter().any(|m| m.contains("@forAll")),
        "expected @forAll warning, got {:?}",
        out.warnings
    );
}

#[test]
fn forsome_skipped_with_warning() {
    let out = parse(
        "@forSome <http://a/x> .\n\
         <http://a/s> <http://a/p> <http://a/o> .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert!(
        out.warnings.messages.iter().any(|m| m.contains("@forSome")),
        "expected @forSome warning, got {:?}",
        out.warnings
    );
}

// ==========================================================================
// 7. Negative cases
// ==========================================================================

#[test]
fn reject_undeclared_prefix() {
    let d = reject("undeclared:s <http://a/p> <http://a/o> .");
    assert!(d.fatal);
    assert!(
        d.messages.iter().any(|m| m.contains("TTL-PFX-001") || m.contains("undeclared")),
        "expected prefix error, got {:?}",
        d.messages
    );
}

#[test]
fn reject_unterminated_iri() {
    let d = reject("<http://a/s <http://a/p> <http://a/o> .");
    assert!(d.fatal);
}

#[test]
fn reject_literal_as_subject() {
    let d = reject("\"hello\" <http://a/p> <http://a/o> .");
    assert!(d.fatal);
    assert!(
        d.messages.iter().any(|m| m.contains("subject")),
        "expected subject error, got {:?}",
        d.messages
    );
}

#[test]
fn reject_missing_dot_after_triple() {
    let d = reject("<http://a/s> <http://a/p> <http://a/o>");
    assert!(d.fatal);
}

#[test]
fn reject_sparql_prefix_with_dot() {
    let d = reject("PREFIX ex: <http://a/> .\nex:s ex:p ex:o .");
    assert!(d.fatal);
    assert!(
        d.messages.iter().any(|m| m.contains("TTL-DIR-001") || m.contains("SPARQL-style")),
        "expected directive error, got {:?}",
        d.messages
    );
}

#[test]
fn reject_unterminated_formula() {
    let d = reject("{ <http://a/s> <http://a/p> <http://a/o> .");
    assert!(d.fatal);
}

// ==========================================================================
// 8. N3 variables treated as blank nodes
// ==========================================================================

#[test]
fn variable_treated_as_blank_node() {
    // N3 variables `?x` are scoped blank nodes.
    let out = parse("<http://a/s> <http://a/p> ?x .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert!(
        f[0].object.starts_with("_:"),
        "variable should be blank node, got {}",
        f[0].object
    );
}

// ==========================================================================
// 9. Edge cases for full Turtle compliance
// ==========================================================================

#[test]
fn trailing_semicolons_accepted() {
    let out = parse("<http://a/s> <http://a/p> <http://a/o> ; .");
    assert_eq!(facts(&out).len(), 1);
}

#[test]
fn multiple_statements() {
    let out = parse(
        "<http://a/s1> <http://a/p> <http://a/o> .\n\
         <http://a/s2> <http://a/p> <http://a/o> .\n\
         <http://a/s3> <http://a/p> <http://a/o> .\n",
    );
    assert_eq!(facts(&out).len(), 3);
}

#[test]
fn bnode_label_document_scope() {
    // Blank-node labels are document-scoped per Turtle TTL-BNPFX-001.
    let out = parse(
        "@prefix ex: <http://a/> . _:b ex:p ex:o1 .\n\
         @prefix ex: <http://b/> . _:b ex:p ex:o2 .\n",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 2);
    let subjects: std::collections::BTreeSet<_> = f.iter().map(|t| t.subject.clone()).collect();
    assert_eq!(subjects.len(), 1, "_:b must be ONE bnode across redefs");
}
