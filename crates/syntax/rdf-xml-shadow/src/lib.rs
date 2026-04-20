//! Independent shadow RDF/XML parser — Phase B implementation.
//!
//! Agent `pb-shadow-rdfxml` fills this in (model: claude-sonnet-4-6,
//! ADR-0019 §3 base-model disjointness). Gated behind the `shadow`
//! feature so main-parser builds never pull this crate's contents.
//!
//! The shadow must be written **without** reading `crates/rdf-xml/`.
//! Divergence between the two implementations is the signal.
//!
//! # Implementation notes
//!
//! Derived from the W3C RDF/XML Syntax Specification
//! <https://www.w3.org/TR/rdf-syntax-grammar/>.
//!
//! ## Supported features
//!
//! - `rdf:RDF` root element (optional — bare node elements are accepted)
//! - Node elements: `rdf:Description` and typed nodes
//! - `rdf:about`, `rdf:nodeID`, bare subject (generates blank node)
//! - Property elements with resource or literal objects
//! - `rdf:resource` attribute → IRI object
//! - `rdf:nodeID` attribute on property element → blank-node object
//! - `rdf:datatype` attribute → typed literal
//! - `rdf:parseType="Resource"` → blank-node subject for nested props
//! - `rdf:parseType="Literal"` → XML literal (serialized XML fragment)
//! - `rdf:parseType="Collection"` → `rdf:first`/`rdf:rest` chains
//! - `rdf:type` shorthand via `rdf:type` attribute on node elements
//! - `xml:lang` inheritance
//! - `xml:base` and `@base` IRI resolution
//! - Reification: `rdf:ID` on property elements
//! - `rdf:li` shorthand → `rdf:_n` container properties
//! - Abbreviated syntax: property attributes on node elements

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "shadow")]
mod parser;

#[cfg(feature = "shadow")]
use rdf_diff::{Diagnostics, ParseOutcome, Parser};

/// Independent shadow RDF/XML parser.
#[cfg(feature = "shadow")]
#[derive(Debug, Default, Clone, Copy)]
pub struct XmlShadowParser;

#[cfg(feature = "shadow")]
impl XmlShadowParser {
    /// Construct a fresh shadow RDF/XML parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "shadow")]
impl Parser for XmlShadowParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        parser::parse(input)
    }

    fn id(&self) -> &'static str {
        "rdf-xml-shadow"
    }
}

#[cfg(all(feature = "shadow", test))]
mod tests {
    use super::*;
    use rdf_diff::Parser;

    fn parse_str(input: &str) -> Result<rdf_diff::ParseOutcome, rdf_diff::Diagnostics> {
        XmlShadowParser::new().parse(input.as_bytes())
    }

    #[test]
    fn simple_description() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:p rdf:resource="http://example.org/o"/>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://example.org/s>");
        assert_eq!(fact.predicate, "<http://example.org/p>");
        assert_eq!(fact.object, "<http://example.org/o>");
        assert!(fact.graph.is_none());
    }

    #[test]
    fn literal_object() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:name>Alice</ex:name>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"Alice\"");
    }

    #[test]
    fn typed_literal() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:xsd="http://www.w3.org/2001/XMLSchema#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:age rdf:datatype="http://www.w3.org/2001/XMLSchema#integer">42</ex:age>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(
            fact.object,
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn lang_tagged_literal() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:label xml:lang="en">Hello</ex:label>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"Hello\"@en");
    }

    #[test]
    fn typed_node() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <ex:Person rdf:about="http://example.org/alice"/>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://example.org/alice>");
        assert_eq!(
            fact.predicate,
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
        );
        assert_eq!(fact.object, "<http://example.org/Person>");
    }

    #[test]
    fn blank_node_subject() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description>
    <ex:p>value</ex:p>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert!(
            fact.subject.starts_with("_:"),
            "expected blank node, got {}",
            fact.subject
        );
    }

    #[test]
    fn rdf_node_id_subject() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:nodeID="n1">
    <ex:p rdf:resource="http://example.org/o"/>
  </rdf:Description>
  <rdf:Description rdf:nodeID="n1">
    <ex:q rdf:resource="http://example.org/o2"/>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 2);
        // Both facts share the same blank-node subject
        let subjects: Vec<_> = outcome
            .facts
            .set
            .keys()
            .map(|f| f.subject.clone())
            .collect();
        assert_eq!(subjects[0], subjects[1]);
    }

    #[test]
    fn property_attribute_shorthand() {
        // Abbreviated syntax: property attributes on node elements
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s" ex:name="Bob"/>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://example.org/s>");
        assert_eq!(fact.predicate, "<http://example.org/name>");
        assert_eq!(fact.object, "\"Bob\"");
    }

    #[test]
    fn rdf_type_attribute() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s"
                   rdf:type="http://example.org/Cls"/>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(
            fact.predicate,
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
        );
        assert_eq!(fact.object, "<http://example.org/Cls>");
    }

    #[test]
    fn blank_node_resource_object() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:p rdf:nodeID="bn1"/>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert!(fact.object.starts_with("_:"));
    }

    #[test]
    fn parse_type_resource() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:p rdf:parseType="Resource">
      <ex:q rdf:resource="http://example.org/o"/>
    </ex:p>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        // Should produce 2 triples: s ex:p _:b, _:b ex:q o
        assert_eq!(outcome.facts.set.len(), 2);
    }

    #[test]
    fn parse_type_collection() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/">
  <rdf:Description rdf:about="http://example.org/s">
    <ex:items rdf:parseType="Collection">
      <rdf:Description rdf:about="http://example.org/a"/>
      <rdf:Description rdf:about="http://example.org/b"/>
    </ex:items>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        // Collection of 2: s ex:items _:n1, _:n1 rdf:first a, _:n1 rdf:rest _:n2,
        //                  _:n2 rdf:first b, _:n2 rdf:rest rdf:nil
        assert_eq!(outcome.facts.set.len(), 5);
    }

    #[test]
    fn empty_document() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>
"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 0);
    }

    #[test]
    fn xml_base_resolution() {
        let xml = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xml:base="http://example.org/">
  <rdf:Description rdf:about="s">
    <rdf:type rdf:resource="Thing"/>
  </rdf:Description>
</rdf:RDF>"#;
        let outcome = parse_str(xml).expect("should parse");
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(fact.subject, "<http://example.org/s>");
        assert_eq!(fact.object, "<http://example.org/Thing>");
    }
}
