//! Integration tests for the `rdf-turtle-shadow` crate.
//!
//! Tests cover Turtle 1.1 and `TriG` edge cases from the W3C
//! recommendations, with particular focus on the areas flagged in
//! ADR-0020: `@prefix`/`@base` resolution, long literals, numeric
//! typing, BNode scoping, and collection syntaxes.
//!
//! # Canonical form note
//!
//! `rdf_diff::Facts::canonicalise` wraps bare absolute IRIs in `<…>`
//! (e.g., `http://example.org/s` becomes `<http://example.org/s>`).
//! Literals retain their `"…"^^<…>` or `"…"@lang` form.
//! Blank-node labels are relabelled to `_:b0`, `_:b1`, …

#[cfg(feature = "shadow")]
mod turtle {
    use rdf_diff::Parser as _;
    use rdf_turtle_shadow::turtle::TurtleParser;

    fn parse(input: &str) -> rdf_diff::Facts {
        TurtleParser.parse(input.as_bytes()).expect("parse failed").facts
    }

    // ── @prefix / @base resolution ───────────────────────────────────────

    #[test]
    fn prefix_empty_string() {
        // The default (empty) prefix
        let ttl = r#"
@prefix : <http://default.example/> .
:s :p :o .
"#;
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://default.example/s>");
        assert_eq!(f.predicate, "<http://default.example/p>");
        assert_eq!(f.object, "<http://default.example/o>");
    }

    #[test]
    fn base_then_relative_iri() {
        let ttl = r#"
@base <http://example.org/base/> .
<subject> <predicate> <object> .
"#;
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://example.org/base/subject>");
        assert_eq!(f.object, "<http://example.org/base/object>");
    }

    #[test]
    fn base_changed_mid_document() {
        let ttl = r#"
@base <http://example.org/a/> .
<s1> <p> <o1> .
@base <http://example.org/b/> .
<s2> <p> <o2> .
"#;
        let facts = parse(ttl);
        assert_eq!(facts.set.len(), 2);
        let mut subjects: Vec<_> = facts.set.keys().map(|f| f.subject.clone()).collect();
        subjects.sort();
        assert_eq!(subjects[0], "<http://example.org/a/s1>");
        assert_eq!(subjects[1], "<http://example.org/b/s2>");
    }

    #[test]
    fn prefix_redefinition_expands_correctly() {
        let ttl = r#"
@prefix ex: <http://first.example/> .
ex:foo <http://example.org/p> "before" .
@prefix ex: <http://second.example/> .
ex:foo <http://example.org/p> "after" .
"#;
        let facts = parse(ttl);
        assert_eq!(facts.set.len(), 2);
        let subjects: std::collections::BTreeSet<_> =
            facts.set.keys().map(|f| f.subject.clone()).collect();
        assert!(subjects.contains("<http://first.example/foo>"));
        assert!(subjects.contains("<http://second.example/foo>"));
    }

    #[test]
    fn base_resolution_absolute_overrides() {
        // An absolute IRI reference should not be resolved against base
        let ttl = r#"
@base <http://example.org/> .
<http://other.example/s> <http://example.org/p> <http://other.example/o> .
"#;
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://other.example/s>");
        assert_eq!(f.object, "<http://other.example/o>");
    }

    #[test]
    fn base_with_path_up() {
        let ttl = r#"
@base <http://example.org/a/b/c> .
<../d> <http://example.org/p> <http://example.org/o> .
"#;
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://example.org/a/d>");
    }

    // ── Long literal forms ────────────────────────────────────────────────

    #[test]
    fn triple_double_quote_string() {
        let ttl = "
@prefix ex: <http://example.org/> .
ex:s ex:p \"\"\"multi\nline\nliteral\"\"\" .
";
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("multi"), "got: {}", f.object);
        assert!(f.object.contains("line"), "got: {}", f.object);
    }

    #[test]
    fn triple_single_quote_string() {
        let ttl = "
@prefix ex: <http://example.org/> .
ex:s ex:p '''multi\nline''' .
";
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("multi"), "got: {}", f.object);
    }

    #[test]
    fn long_literal_with_embedded_quotes() {
        // Long string may contain single and double quote chars
        let ttl = r#"@prefix ex: <http://example.org/> .
ex:s ex:p """She said "hello" and 'goodbye'""" ."#;
        let facts = parse(ttl);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("hello"), "got: {}", f.object);
    }

    #[test]
    fn long_literal_with_escape_sequences() {
        let ttl = r#"@prefix ex: <http://example.org/> .
ex:s ex:p """\u0041\u0042\u0043""" ."#;
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("ABC"), "got: {}", f.object);
    }

    // ── Numeric literal typing ────────────────────────────────────────────

    #[test]
    fn positive_integer() {
        let ttl = "@prefix ex: <http://example.org/> . ex:s ex:p +42 .";
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(
            f.object.contains("integer"),
            "expected xsd:integer, got: {}",
            f.object
        );
    }

    #[test]
    fn negative_integer() {
        let ttl = "@prefix ex: <http://example.org/> . ex:s ex:p -1 .";
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("integer"), "got: {}", f.object);
    }

    #[test]
    fn decimal_literal() {
        let ttl = "@prefix ex: <http://example.org/> . ex:s ex:p 0.5 .";
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("decimal"), "got: {}", f.object);
    }

    #[test]
    fn double_scientific() {
        let ttl = "@prefix ex: <http://example.org/> . ex:s ex:p 1.5E+3 .";
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("double"), "got: {}", f.object);
    }

    #[test]
    fn double_no_fraction() {
        let ttl = "@prefix ex: <http://example.org/> . ex:s ex:p 1E10 .";
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("double"), "got: {}", f.object);
    }

    // ── BNode scoping ─────────────────────────────────────────────────────

    #[test]
    fn bnode_label_stable_after_prefix_redecl() {
        // ADR-0020 known ambiguity: BNode labels MUST be stable across
        // `@prefix` re-declarations within a document.
        let ttl = r#"
@prefix ex: <http://example.org/> .
_:x ex:name "first" .
@prefix ex: <http://other.org/> .
_:x ex:name "second" .
"#;
        let facts = parse(ttl);
        // Both triples have the same subject bnode
        let subjects: Vec<_> = facts.set.keys().map(|f| f.subject.clone()).collect();
        assert_eq!(subjects.len(), 2);
        assert_eq!(subjects[0], subjects[1], "BNode _:x must be stable across prefix redecl");
    }

    #[test]
    fn distinct_bnode_labels() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
_:a ex:p "1" .
_:b ex:p "2" .
"#;
        let facts = parse(ttl);
        let subjects: std::collections::BTreeSet<_> =
            facts.set.keys().map(|f| f.subject.clone()).collect();
        assert_eq!(subjects.len(), 2, "distinct labels must map to distinct bnodes");
    }

    #[test]
    fn fresh_bnode_from_brackets() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p [] .
ex:s ex:q [] .
"#;
        let facts = parse(ttl);
        // Each `[]` generates a fresh, distinct bnode
        assert_eq!(facts.set.len(), 2);
        let objs: Vec<_> = facts.set.keys().map(|f| f.object.clone()).collect();
        assert_ne!(objs[0], objs[1], "each [] should yield a distinct bnode");
    }

    // ── Collection syntax ─────────────────────────────────────────────────

    #[test]
    fn nested_collection() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p (ex:a (ex:b) ex:c) .
"#;
        let facts = parse(ttl);
        // At minimum: outer list has 3 elements (3×2=6 arcs), inner (1 element, 2 arcs)
        // plus 1 for ex:s ex:p head
        assert!(facts.set.len() > 6, "expected many facts for nested list, got {}", facts.set.len());
    }

    #[test]
    fn single_element_collection() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:s ex:p (ex:only) .
"#;
        let facts = parse(ttl);
        // head rdf:first ex:only
        // head rdf:rest rdf:nil
        // ex:s ex:p head
        assert_eq!(facts.set.len(), 3);
    }

    #[test]
    fn collection_with_literals() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p ("hello" 42 true) .
"#;
        let facts = parse(ttl);
        // 3 elements × 2 arcs + 1 for ex:s ex:p head = 7
        assert_eq!(facts.set.len(), 7);
    }

    // ── Misc edge cases ───────────────────────────────────────────────────

    #[test]
    fn multiple_objects_comma_separated() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p ex:a, ex:b, ex:c .
"#;
        let facts = parse(ttl);
        assert_eq!(facts.set.len(), 3);
    }

    #[test]
    fn multiple_predicates_semicolon() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:a ex:x ;
     ex:b ex:y ;
     ex:c ex:z .
"#;
        let facts = parse(ttl);
        assert_eq!(facts.set.len(), 3);
    }

    #[test]
    fn trailing_semicolon_ok() {
        // Turtle 1.1 allows trailing semicolons per grammar
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p ex:o ; .
"#;
        let facts = parse(ttl);
        assert_eq!(facts.set.len(), 1);
    }

    #[test]
    fn unicode_iri() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
<http://example.org/\u00E9> ex:p ex:o .
"#;
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://example.org/é>");
    }

    #[test]
    fn lang_tag_canonicalised() {
        // Per BCP 47 canonical form (via rdf_diff::canonicalise_term):
        // language subtag lowercased, region subtag uppercased.
        let ttl = r#"@prefix ex: <http://example.org/> . ex:s ex:p "Hello"@EN-US ."#;
        let facts = parse(ttl);
        let f = facts.set.keys().next().unwrap();
        // bcp47_case_fold("EN-US") = "en-US"
        assert!(f.object.ends_with("@en-US"), "expected bcp47 canonical lang tag, got: {}", f.object);
    }

    #[test]
    fn empty_document_ok() {
        let facts = parse("");
        assert_eq!(facts.set.len(), 0);
    }

    #[test]
    fn only_comments() {
        let facts = parse("# just a comment\n# another");
        assert_eq!(facts.set.len(), 0);
    }

    #[test]
    fn parser_id() {
        assert_eq!(TurtleParser.id(), "rdf-turtle-shadow");
    }
}

#[cfg(feature = "shadow")]
mod trig_tests {
    use rdf_diff::Parser as _;
    use rdf_turtle_shadow::trig::TriGParser;

    fn parse(input: &str) -> rdf_diff::Facts {
        TriGParser.parse(input.as_bytes()).expect("parse failed").facts
    }

    #[test]
    fn parser_id() {
        assert_eq!(TriGParser.id(), "rdf-trig-shadow");
    }

    #[test]
    fn bnode_scope_across_named_graphs() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:g1 { _:x ex:p "a" . }
ex:g2 { _:x ex:q "b" . }
"#;
        let facts = parse(trig);
        let objs_by_graph: Vec<_> = facts.set.keys().collect();
        assert_eq!(objs_by_graph.len(), 2);
        // Both facts should have the same subject bnode (document-scoped)
        let s0 = &objs_by_graph[0].subject;
        let s1 = &objs_by_graph[1].subject;
        assert_eq!(s0, s1, "BNodes must be document-scoped in TriG");
    }

    #[test]
    fn iri_named_graph() {
        let trig = r#"
@prefix ex: <http://example.org/> .
<http://named.example/g> { ex:s ex:p ex:o . }
"#;
        let facts = parse(trig);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.graph.as_deref(), Some("<http://named.example/g>"));
    }

    #[test]
    fn multiple_triples_in_graph() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:g {
    ex:s1 ex:p ex:o1 .
    ex:s2 ex:p ex:o2 .
    ex:s3 ex:p ex:o3 .
}
"#;
        let facts = parse(trig);
        assert_eq!(facts.set.len(), 3);
    }
}
