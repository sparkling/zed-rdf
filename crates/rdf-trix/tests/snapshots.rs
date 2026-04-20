//! Snapshot tests for the TriX parser.
//!
//! Coverage:
//! - empty graph (no triples)
//! - default-graph triple (no `<uri>` graph name)
//! - named-graph triple (with `<uri>` graph name)
//! - blank nodes as subject and object
//! - language-tagged plain literal
//! - typed literal
//! - malformed XML (must reject)

use rdf_diff::Parser;
use rdf_trix::TriXParser;

fn parse(input: &[u8]) -> rdf_diff::ParseOutcome {
    TriXParser::new()
        .parse(input)
        .expect("parse should succeed")
}

fn parse_err(input: &[u8]) -> rdf_diff::Diagnostics {
    TriXParser::new()
        .parse(input)
        .expect_err("parse should fail")
}

// ---------------------------------------------------------------------------
// Helper: collect sorted canonical facts for assertion
// ---------------------------------------------------------------------------

fn sorted_facts(outcome: &rdf_diff::ParseOutcome) -> Vec<String> {
    outcome
        .facts
        .set
        .keys()
        .map(|f| {
            format!(
                "({} {} {} {:?})",
                f.subject, f.predicate, f.object, f.graph
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// A well-formed TriX document with one empty graph produces zero facts.
#[test]
fn empty_graph_produces_no_facts() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert!(
        outcome.facts.set.is_empty(),
        "expected zero facts, got: {:?}",
        sorted_facts(&outcome)
    );
}

/// A triple in a graph without a name URI goes into the default graph
/// (Fact::graph == None).
#[test]
fn default_graph_triple() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <uri>http://example.org/o</uri>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 1, "{:?}", sorted_facts(&outcome));

    let fact = outcome.facts.set.keys().next().unwrap();
    assert_eq!(fact.subject, "<http://example.org/s>");
    assert_eq!(fact.predicate, "<http://example.org/p>");
    assert_eq!(fact.object, "<http://example.org/o>");
    assert_eq!(fact.graph, None, "default graph triple must have None graph");
}

/// A triple inside a named graph carries the graph IRI.
#[test]
fn named_graph_triple() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <uri>http://example.org/graph1</uri>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <uri>http://example.org/o</uri>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 1, "{:?}", sorted_facts(&outcome));

    let fact = outcome.facts.set.keys().next().unwrap();
    assert_eq!(fact.subject, "<http://example.org/s>");
    assert_eq!(fact.predicate, "<http://example.org/p>");
    assert_eq!(fact.object, "<http://example.org/o>");
    assert_eq!(
        fact.graph,
        Some("<http://example.org/graph1>".to_owned()),
        "named graph triple must carry the graph IRI"
    );
}

/// Multiple named graphs each produce facts tagged with the correct graph.
#[test]
fn multiple_named_graphs() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <uri>http://example.org/g1</uri>
    <triple>
      <uri>http://example.org/a</uri>
      <uri>http://example.org/b</uri>
      <uri>http://example.org/c</uri>
    </triple>
  </graph>
  <graph>
    <uri>http://example.org/g2</uri>
    <triple>
      <uri>http://example.org/x</uri>
      <uri>http://example.org/y</uri>
      <uri>http://example.org/z</uri>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 2, "{:?}", sorted_facts(&outcome));

    let graphs: Vec<_> = outcome
        .facts
        .set
        .keys()
        .filter_map(|f| f.graph.as_deref())
        .collect();
    assert!(
        graphs.contains(&"<http://example.org/g1>"),
        "g1 missing from facts"
    );
    assert!(
        graphs.contains(&"<http://example.org/g2>"),
        "g2 missing from facts"
    );
}

/// Blank node subject and object are accepted and relabelled canonically.
#[test]
fn blank_nodes() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <bnode>n1</bnode>
      <uri>http://example.org/p</uri>
      <bnode>n2</bnode>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 1, "{:?}", sorted_facts(&outcome));

    let fact = outcome.facts.set.keys().next().unwrap();
    // After canonicalisation, blank nodes become _:c0, _:c1 etc.
    assert!(
        fact.subject.starts_with("_:"),
        "subject should be a blank node, got: {}",
        fact.subject
    );
    assert!(
        fact.object.starts_with("_:"),
        "object should be a blank node, got: {}",
        fact.object
    );
    // The two blank nodes must be distinct.
    assert_ne!(
        fact.subject, fact.object,
        "distinct blank nodes must remain distinct after canonicalisation"
    );
}

/// A `<plainLiteral xml:lang="en">` produces a language-tagged literal.
#[test]
fn language_tagged_literal() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <plainLiteral xml:lang="en">hello</plainLiteral>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 1, "{:?}", sorted_facts(&outcome));

    let fact = outcome.facts.set.keys().next().unwrap();
    // Language tag is case-folded per BCP-47: "en" → "en"
    assert_eq!(
        fact.object, "\"hello\"@en",
        "language-tagged literal mismatch"
    );
}

/// Language tags are BCP-47 case-folded (primary lowercase, region uppercase).
#[test]
fn language_tag_case_folding() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <plainLiteral xml:lang="EN-us">hello</plainLiteral>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    let fact = outcome.facts.set.keys().next().unwrap();
    assert_eq!(
        fact.object, "\"hello\"@en-US",
        "BCP-47 case-folding not applied: expected en-US"
    );
}

/// A `<typedLiteral datatype="...">` produces a typed literal.
#[test]
fn typed_literal() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <typedLiteral datatype="http://www.w3.org/2001/XMLSchema#integer">42</typedLiteral>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 1, "{:?}", sorted_facts(&outcome));

    let fact = outcome.facts.set.keys().next().unwrap();
    assert_eq!(
        fact.object,
        "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>",
        "typed literal mismatch"
    );
}

/// A plain literal (no language tag, no datatype) becomes a plain string literal.
#[test]
fn plain_literal_no_lang() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <plainLiteral>bare string</plainLiteral>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    let fact = outcome.facts.set.keys().next().unwrap();
    assert_eq!(fact.object, "\"bare string\"");
}

/// Literal values with special characters are escaped correctly.
#[test]
fn literal_escaping() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <plainLiteral>say "hello" \ world</plainLiteral>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    let fact = outcome.facts.set.keys().next().unwrap();
    // Backslash and double-quote must be escaped
    assert_eq!(fact.object, r#""say \"hello\" \\ world""#);
}

/// Missing XML declaration but otherwise valid TriX is still accepted.
#[test]
fn no_xml_declaration() {
    let input = br#"<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <uri>http://example.org/o</uri>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 1);
}

/// Multiple triples in a single graph.
#[test]
fn multiple_triples_in_one_graph() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <uri>http://example.org/o1</uri>
    </triple>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <uri>http://example.org/o2</uri>
    </triple>
  </graph>
</TriX>"#;

    let outcome = parse(input);
    assert_eq!(outcome.facts.set.len(), 2, "{:?}", sorted_facts(&outcome));
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

/// Truncated/malformed XML must be rejected with fatal=true.
#[test]
fn malformed_xml_truncated() {
    let input = b"<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\"><graph>";
    let diag = parse_err(input);
    assert!(diag.fatal, "expected fatal=true for truncated XML");
}

/// Invalid XML (unclosed attribute) must be rejected.
#[test]
fn malformed_xml_unclosed_attribute() {
    let input = b"<TriX xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\"><graph><triple><uri>foo</triple></graph></TriX>";
    // This has a <uri> not closed before </triple>
    let diag = parse_err(input);
    assert!(diag.fatal, "expected fatal=true for malformed XML");
}

/// Completely invalid bytes (not XML at all) must be rejected.
#[test]
fn malformed_xml_not_xml() {
    let input = b"this is not xml at all!!!";
    let diag = parse_err(input);
    assert!(diag.fatal, "expected fatal=true for non-XML input");
}

/// Wrong root element name must be rejected.
#[test]
fn wrong_root_element() {
    let input = b"<Root xmlns=\"http://www.w3.org/2004/03/trix/trix-1/\"></Root>";
    let diag = parse_err(input);
    assert!(diag.fatal, "expected fatal=true for wrong root element");
}

/// Missing TriX namespace must be rejected.
#[test]
fn missing_trix_namespace() {
    let input = b"<TriX></TriX>";
    let diag = parse_err(input);
    assert!(diag.fatal, "expected fatal=true for missing TriX namespace");
}

/// Empty input must be rejected.
#[test]
fn empty_input() {
    let diag = parse_err(b"");
    assert!(diag.fatal, "expected fatal=true for empty input");
}

/// A triple with only 2 terms must be rejected.
#[test]
fn triple_with_too_few_terms() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
    </triple>
  </graph>
</TriX>"#;

    let diag = parse_err(input);
    assert!(diag.fatal, "expected fatal=true for triple with 2 terms");
}

/// `typedLiteral` missing its datatype attribute must be rejected.
#[test]
fn typed_literal_missing_datatype() {
    let input = br#"<?xml version="1.0"?>
<TriX xmlns="http://www.w3.org/2004/03/trix/trix-1/">
  <graph>
    <triple>
      <uri>http://example.org/s</uri>
      <uri>http://example.org/p</uri>
      <typedLiteral>42</typedLiteral>
    </triple>
  </graph>
</TriX>"#;

    let diag = parse_err(input);
    assert!(diag.fatal, "expected fatal=true for typedLiteral missing datatype");
}

/// Parser id is correct.
#[test]
fn parser_id() {
    assert_eq!(TriXParser::new().id(), "rdf-trix");
}
