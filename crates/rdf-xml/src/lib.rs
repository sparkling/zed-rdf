//! Main RDF/XML parser — Phase B implementation.
//!
//! Agent `pb-rdf-xml` fills this in. The public surface is the
//! [`RdfXmlParser`] type implementing [`rdf_diff::Parser`].
//!
//! # Pinned spec reading
//! RDF/XML Syntax Specification (W3C Recommendation 2004-02-10):
//! <https://www.w3.org/TR/rdf-syntax-grammar/>
//!
//! # Non-goals (Phase B scope)
//! - Named graphs / `TriG` semantics — RDF/XML is triples-only.
//! - `rdf:parseType="Literal"` XML literal canonicalisation (Phase C).
//! - Streaming / push-parser APIs.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions,
)]

mod parser;

use rdf_diff::{Diagnostics, ParseOutcome, Parser};

/// Assumed document base used when no `xml:base` is present and the
/// document is parsed without an explicit base URI. The W3C rdfxml test
/// suite uses this as the assumed base for all test inputs.
pub const DEFAULT_BASE: &str =
    "https://w3c.github.io/rdf-tests/rdf/rdf11/rdf-xml/";

/// Main RDF/XML parser.
///
/// Stateless — construct with [`RdfXmlParser::new`] and reuse across inputs.
#[derive(Debug, Default, Clone, Copy)]
pub struct RdfXmlParser;

impl RdfXmlParser {
    /// Construct a fresh RDF/XML parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Parse with an explicit document base URI. The base URI is used to
    /// resolve `rdf:about`, `rdf:ID`, `rdf:resource`, and `xml:base`
    /// relative references.
    pub fn parse_with_base(
        &self,
        input: &[u8],
        base: &str,
    ) -> Result<ParseOutcome, Diagnostics> {
        parser::parse(input, base)
    }
}

impl Parser for RdfXmlParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        parser::parse(input, DEFAULT_BASE)
    }

    fn id(&self) -> &'static str {
        "rdf-xml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Result<rdf_diff::Facts, Diagnostics> {
        RdfXmlParser::new().parse(input.as_bytes()).map(|o| o.facts)
    }

    fn parse_base(input: &str, base: &str) -> Result<rdf_diff::Facts, Diagnostics> {
        RdfXmlParser::new()
            .parse_with_base(input.as_bytes(), base)
            .map(|o| o.facts)
    }

    #[test]
    fn empty_rdf_rdf_is_ok() {
        let facts = parse(r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>"#)
            .expect("accept");
        assert_eq!(facts.set.len(), 0);
    }

    #[test]
    fn simple_triple_rdf_about() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/foo">
    <eg:bar>hello</eg:bar>
  </rdf:Description>
</rdf:RDF>"#)
            .expect("accept");
        assert_eq!(facts.set.len(), 1);
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://example.org/foo>");
        assert_eq!(fact.predicate, "<http://example.org/bar>");
        assert_eq!(fact.object, "\"hello\"");
        assert_eq!(fact.graph, None);
    }

    #[test]
    fn typed_literal() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/foo">
    <eg:bar rdf:datatype="http://www.w3.org/2001/XMLSchema#integer">42</eg:bar>
  </rdf:Description>
</rdf:RDF>"#)
            .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>");
    }

    #[test]
    fn lang_tagged_literal() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/node">
    <eg:property xml:lang="fr">chat</eg:property>
  </rdf:Description>
</rdf:RDF>"#)
            .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"chat\"@fr");
    }

    #[test]
    fn blank_node_subject() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description>
    <eg:property>value</eg:property>
  </rdf:Description>
</rdf:RDF>"#)
            .expect("accept");
        assert_eq!(facts.set.len(), 1);
        let fact = facts.set.keys().next().unwrap();
        assert!(fact.subject.starts_with("_:"));
    }

    #[test]
    fn rdf_resource_property() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <eg:p rdf:resource="http://example.org/o"/>
  </rdf:Description>
</rdf:RDF>"#)
            .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "<http://example.org/o>");
    }

    #[test]
    fn typed_node_element() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:foaf="http://xmlns.com/foaf/0.1/">
  <foaf:Person rdf:about="http://example.org/alice">
    <foaf:name>Alice</foaf:name>
  </foaf:Person>
</rdf:RDF>"#)
            .expect("accept");
        let has_type = facts.set.keys().any(|f| {
            f.predicate == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
                && f.object == "<http://xmlns.com/foaf/0.1/Person>"
        });
        assert!(has_type, "expected rdf:type triple");
    }

    #[test]
    fn forbidden_rdf_rdf_node_elt_rejected() {
        let err = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:RDF/>
</rdf:RDF>"#)
            .expect_err("should reject rdf:RDF as node element");
        assert!(err.fatal);
    }

    #[test]
    fn forbidden_rdf_id_node_elt_rejected() {
        let err = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:ID/>
</rdf:RDF>"#)
            .expect_err("should reject rdf:ID as node element");
        assert!(err.fatal);
    }

    #[test]
    fn xml_base_applies_to_rdf_about() {
        let facts = parse_base(
            r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                     xmlns:eg="http://example.org/"
                     xml:base="http://example.org/dir/file">
 <rdf:Description rdf:ID="frag" eg:value="v" />
</rdf:RDF>"#,
            "http://example.org/dir/file",
        )
        .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://example.org/dir/file#frag>");
    }

    #[test]
    fn rdf_li_auto_numbering() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Bag>
    <rdf:li>1</rdf:li>
    <rdf:li>2</rdf:li>
  </rdf:Bag>
</rdf:RDF>"#)
            .expect("accept");
        let has_1 = facts.set.keys().any(|f| {
            f.predicate == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#_1>"
        });
        let has_2 = facts.set.keys().any(|f| {
            f.predicate == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#_2>"
        });
        assert!(has_1 && has_2);
    }

    #[test]
    fn parse_type_resource() {
        let facts = parse(r#"
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:eg="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <eg:p rdf:parseType="Resource">
      <eg:q>value</eg:q>
    </eg:p>
  </rdf:Description>
</rdf:RDF>"#)
            .expect("accept");
        // Two triples: s eg:p _:bn, _:bn eg:q "value"
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn parser_id_is_stable() {
        assert_eq!(RdfXmlParser::new().id(), "rdf-xml");
    }
}
