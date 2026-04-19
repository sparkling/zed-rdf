//! Smoke tests for `rdf-turtle`.
//!
//! Exercises the main Turtle + TriG grammar productions against known-
//! good byte inputs and verifies the emitted canonical fact set has the
//! expected shape. These tests are intentionally independent of the
//! shadow parser and the oracle adapters — they run on a bare `cargo
//! test -p rdf-turtle` with no `--features` flags.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use rdf_diff::{Diagnostics, Fact, Parser as _};
use rdf_turtle::{TriGParser, TurtleParser};

fn parse_ttl(src: &str) -> rdf_diff::ParseOutcome {
    TurtleParser::new()
        .parse(src.as_bytes())
        .unwrap_or_else(|d: Diagnostics| panic!("rejected: {d:?}"))
}

fn parse_trig(src: &str) -> rdf_diff::ParseOutcome {
    TriGParser::new()
        .parse(src.as_bytes())
        .unwrap_or_else(|d: Diagnostics| panic!("trig rejected: {d:?}"))
}

fn reject_ttl(src: &str) -> Diagnostics {
    match TurtleParser::new().parse(src.as_bytes()) {
        Ok(_) => panic!("expected rejection for: {src}"),
        Err(d) => d,
    }
}

fn facts(out: &rdf_diff::ParseOutcome) -> Vec<Fact> {
    out.facts.set.keys().cloned().collect()
}

// ---------------------------------------------------------------------
// Basic productions
// ---------------------------------------------------------------------

#[test]
fn iri_iri_iri_triple() {
    let out = parse_ttl("<http://a/s> <http://a/p> <http://a/o> .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].subject, "<http://a/s>");
    assert_eq!(f[0].predicate, "<http://a/p>");
    assert_eq!(f[0].object, "<http://a/o>");
}

#[test]
fn prefix_directive_expands() {
    let out = parse_ttl("@prefix ex: <http://example/> . ex:s ex:p ex:o .");
    let f = facts(&out);
    assert_eq!(f[0].subject, "<http://example/s>");
    assert_eq!(f[0].predicate, "<http://example/p>");
    assert_eq!(f[0].object, "<http://example/o>");
}

#[test]
fn sparql_style_prefix_and_base() {
    let out = parse_ttl(
        "BASE <http://example/>\nPREFIX ex: <http://example/>\nex:s <p> <o> .",
    );
    let f = facts(&out);
    assert_eq!(f[0].predicate, "<http://example/p>");
}

#[test]
fn a_keyword_in_predicate_position() {
    let out = parse_ttl("<http://a/s> a <http://a/C> .");
    let f = facts(&out);
    assert_eq!(
        f[0].predicate,
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
    );
}

#[test]
fn comma_and_semicolon_object_lists() {
    let out = parse_ttl(
        "<http://a/s> <http://a/p> <http://a/o1>, <http://a/o2> ; <http://a/q> <http://a/o3> .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 3);
}

// ---------------------------------------------------------------------
// TTL-LITESC-001 — literal escapes and forbidden raw characters
// ---------------------------------------------------------------------

#[test]
fn short_string_escapes_decoded() {
    let out = parse_ttl(r#"<http://a/s> <http://a/p> "a\tb\nc\"d\\e" ."#);
    let f = facts(&out);
    // Object is canonical `"lex"` form; the decoded string should contain
    // the escaped characters. `"` and `\` are re-escaped by our inline
    // canonical framing — but `\t` / `\n` are raw in the stored lex form.
    assert!(f[0].object.contains('\t'));
    assert!(f[0].object.contains('\n'));
}

#[test]
fn uchar_escapes_decoded() {
    let out = parse_ttl(r#"<http://a/s> <http://a/p> "\u00E9\U0001F600" ."#);
    let f = facts(&out);
    assert!(f[0].object.contains('é'));
    assert!(f[0].object.contains('😀'));
}

#[test]
fn long_string_accepts_raw_newlines() {
    let out = parse_ttl("<http://a/s> <http://a/p> \"\"\"line1\nline2\"\"\" .");
    let f = facts(&out);
    assert!(f[0].object.contains('\n'));
}

#[test]
fn short_string_rejects_raw_newline() {
    let diag = reject_ttl("<http://a/s> <http://a/p> \"line1\nline2\" .");
    assert!(diag.fatal);
    assert!(
        diag.messages.iter().any(|m| m.starts_with("TTL-LITESC-001")),
        "expected TTL-LITESC-001, got {:?}",
        diag.messages,
    );
}

#[test]
fn surrogate_uchar_rejected() {
    let diag = reject_ttl(r#"<http://a/s> <http://a/p> "\uD800" ."#);
    assert!(diag.messages.iter().any(|m| m.starts_with("TTL-LITESC-001")));
}

#[test]
fn unknown_echar_rejected() {
    let diag = reject_ttl(r#"<http://a/s> <http://a/p> "\q" ."#);
    assert!(diag.messages.iter().any(|m| m.starts_with("TTL-LITESC-001")));
}

// ---------------------------------------------------------------------
// Numeric literal typing
// ---------------------------------------------------------------------

#[test]
fn numeric_literal_types() {
    let out = parse_ttl(
        "@prefix ex: <http://example/> .\n\
         ex:s ex:p1 1 .\n\
         ex:s ex:p2 1.0 .\n\
         ex:s ex:p3 1.0e0 .\n\
         ex:s ex:p4 +1 .\n\
         ex:s ex:p5 -0 .\n",
    );
    let objects: Vec<_> = facts(&out).into_iter().map(|f| f.object).collect();
    let joined = objects.join(";");
    assert!(joined.contains("\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
    assert!(joined.contains("\"1.0\"^^<http://www.w3.org/2001/XMLSchema#decimal>"));
    assert!(joined.contains("\"1.0e0\"^^<http://www.w3.org/2001/XMLSchema#double>"));
    assert!(joined.contains("\"+1\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
    assert!(joined.contains("\"-0\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
}

#[test]
fn boolean_literal_types() {
    let out = parse_ttl(
        "<http://a/s> <http://a/p> true . <http://a/s> <http://a/p> false .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 2);
    assert!(
        f.iter()
            .any(|t| t.object == "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"),
    );
}

// ---------------------------------------------------------------------
// Language tags + datatypes
// ---------------------------------------------------------------------

#[test]
fn lang_tag_applied_and_folded() {
    let out = parse_ttl(r#"<http://a/s> <http://a/p> "Hello"@EN-us ."#);
    let f = facts(&out);
    // BCP-47 case fold happens in rdf-diff's canonicalise_term.
    assert_eq!(f[0].object, "\"Hello\"@en-US");
}

#[test]
fn datatype_marker_applied() {
    let out = parse_ttl(
        "<http://a/s> <http://a/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .",
    );
    let f = facts(&out);
    assert_eq!(
        f[0].object,
        "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>",
    );
}

#[test]
fn xsd_string_collapses_to_plain_literal() {
    // RDF 1.1 §3.3: "^^xsd:string is the same as the plain form"
    let out = parse_ttl(
        "<http://a/s> <http://a/p> \"hi\"^^<http://www.w3.org/2001/XMLSchema#string> .",
    );
    let f = facts(&out);
    assert_eq!(f[0].object, "\"hi\"");
}

// ---------------------------------------------------------------------
// Collections
// ---------------------------------------------------------------------

#[test]
fn empty_collection_is_rdf_nil() {
    let out = parse_ttl("<http://a/s> <http://a/p> () .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(
        f[0].object,
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>",
    );
}

#[test]
fn single_element_collection() {
    let out = parse_ttl("<http://a/s> <http://a/p> (<http://a/o>) .");
    let f = facts(&out);
    // Expect 3 facts: s p _:b ; _:b first <o> ; _:b rest rdf:nil.
    assert_eq!(f.len(), 3);
}

#[test]
fn nested_collections_not_flattened() {
    let out = parse_ttl(
        "<http://a/s> <http://a/p> ((<http://a/x>)) .",
    );
    let f = facts(&out);
    // Outer head rdf:first → inner head (a bnode); inner head rdf:first → <x>.
    // That means two distinct bnodes + `<a/x>` as a rest-chain terminal value.
    // We assert on count: outer has first+rest (2), inner has first+rest (2),
    // plus the subject link = 5 facts.
    assert_eq!(f.len(), 5);
}

// ---------------------------------------------------------------------
// Blank nodes
// ---------------------------------------------------------------------

#[test]
fn bnode_property_list_emits_anchor_triple() {
    let out = parse_ttl(
        "[ <http://a/p> <http://a/o> ] <http://a/q> <http://a/o2> .",
    );
    let f = facts(&out);
    assert_eq!(f.len(), 2);
    assert!(f.iter().any(|t| t.subject.starts_with("_:")));
}

#[test]
fn empty_bnode_property_list() {
    let out = parse_ttl("<http://a/s> <http://a/p> [] .");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert!(f[0].object.starts_with("_:"));
}

#[test]
fn bnode_label_document_scope_across_prefix_redef() {
    // TTL-BNPFX-001: @prefix redefinition must NOT rescope _:b.
    let out = parse_ttl(
        "@prefix ex: <http://a/> . _:b ex:p ex:o1 .\n\
         @prefix ex: <http://b/> . _:b ex:p ex:o2 .\n",
    );
    let f = facts(&out);
    // Both triples share the same subject after canonicalisation.
    assert_eq!(f.len(), 2);
    let subjects: std::collections::BTreeSet<_> =
        f.iter().map(|t| t.subject.clone()).collect();
    assert_eq!(subjects.len(), 1, "_:b must be ONE bnode across redefs");
}

// ---------------------------------------------------------------------
// @base chaining
// ---------------------------------------------------------------------

#[test]
fn chained_base_directives() {
    let out = parse_ttl(
        "@base <http://a/> . <r1> <p> <o> .\n\
         @base <http://b/> . <r2> <p> <o> .\n\
         @base <http://c/> . <r3> <p> <o> .\n",
    );
    let subjects: Vec<_> =
        facts(&out).into_iter().map(|t| t.subject).collect();
    assert!(subjects.contains(&"<http://a/r1>".to_owned()));
    assert!(subjects.contains(&"<http://b/r2>".to_owned()));
    assert!(subjects.contains(&"<http://c/r3>".to_owned()));
}

#[test]
fn base_and_sparql_base_replacement() {
    let out = parse_ttl("@base <http://a/> .\nBASE <http://b/>\n<rel> <p> <o> .");
    let f = facts(&out);
    assert_eq!(f[0].subject, "<http://b/rel>");
    assert_eq!(f[0].predicate, "<http://b/p>");
}

// ---------------------------------------------------------------------
// Trailing semicolon
// ---------------------------------------------------------------------

#[test]
fn trailing_semicolon_accepted() {
    let out = parse_ttl(
        "<http://a/s> <http://a/p> <http://a/o> ; .",
    );
    assert_eq!(facts(&out).len(), 1);
}

#[test]
fn repeated_trailing_semicolons_accepted() {
    let out = parse_ttl(
        "<http://a/s> <http://a/p> <http://a/o> ; ; ; .",
    );
    assert_eq!(facts(&out).len(), 1);
}

// ---------------------------------------------------------------------
// PN_LOCAL edge cases
// ---------------------------------------------------------------------

#[test]
fn pn_local_leading_digit_accepted() {
    let out = parse_ttl("@prefix ex: <http://a/> . ex:123 ex:p ex:o .");
    let f = facts(&out);
    assert_eq!(f[0].subject, "<http://a/123>");
}

#[test]
fn pn_local_percent_passed_through() {
    let out = parse_ttl("@prefix ex: <http://a/> . ex:caf%C3%A9 ex:p ex:o .");
    let f = facts(&out);
    // MUST NOT be decoded to caf\u00e9 — pin IRI-PCT-001.
    assert_eq!(f[0].subject, "<http://a/caf%C3%A9>");
}

// ---------------------------------------------------------------------
// TriG: named graph blocks
// ---------------------------------------------------------------------

#[test]
fn trig_default_graph_block() {
    let out = parse_trig("{ <http://a/s> <http://a/p> <http://a/o> . }");
    let f = facts(&out);
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].graph, None);
}

#[test]
fn trig_named_graph_block() {
    let out = parse_trig(
        "<http://a/g1> { <http://a/s> <http://a/p> <http://a/o> . }",
    );
    let f = facts(&out);
    assert_eq!(f[0].graph.as_deref(), Some("<http://a/g1>"));
}

#[test]
fn trig_graph_keyword_form() {
    let out = parse_trig(
        "GRAPH <http://a/g1> { <http://a/s> <http://a/p> <http://a/o> . }",
    );
    assert_eq!(facts(&out)[0].graph.as_deref(), Some("<http://a/g1>"));
}

#[test]
fn trig_bnode_document_scope_across_graphs() {
    // Per pin TTL-BNPFX-001: _:b in one graph block and _:b in another
    // are the SAME bnode. This is the document-scope reading we pinned.
    let out = parse_trig(
        "{ _:b <http://a/p> <http://a/o1> . }\n\
         <http://a/g1> { _:b <http://a/p> <http://a/o2> . }",
    );
    let f = facts(&out);
    // After canonicalisation the two facts share the same subject label.
    let subjects: std::collections::BTreeSet<_> =
        f.iter().map(|t| t.subject.clone()).collect();
    assert_eq!(subjects.len(), 1, "TriG _:b is document-scope per TTL-BNPFX-001");
}
