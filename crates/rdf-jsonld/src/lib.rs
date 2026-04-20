//! Main JSON-LD syntax parser — Phase B implementation.
//!
//! Agent `pb-rdf-jsonld` fills this in. The public surface is the
//! [`JsonLdParser`] type implementing [`rdf_diff::Parser`].
//!
//! # Pinned spec reading
//! JSON-LD 1.1 (W3C Recommendation 2020-07-16):
//! <https://www.w3.org/TR/json-ld11/>
//! JSON-LD 1.1 Processing Algorithms and API:
//! <https://www.w3.org/TR/json-ld11-api/>
//!
//! # Scope (Phase B)
//! Syntax parsing and `@context` well-formedness only. No expand, compact,
//! or normalize semantics — those are Phase E. See ADR-0021 §Consequences.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions,
    clippy::option_if_let_else,
)]

mod context;
mod error;
mod to_rdf;

use rdf_diff::{Diagnostics, ParseOutcome, Parser};

/// Main JSON-LD syntax parser.
///
/// Stateless — construct with [`JsonLdParser::new`] and reuse across inputs.
/// An optional document base IRI can be provided via [`JsonLdParser::with_base`].
#[derive(Debug, Clone)]
pub struct JsonLdParser {
    /// The document base IRI, used to resolve relative IRI references.
    base: Option<String>,
}

impl Default for JsonLdParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonLdParser {
    /// Construct a fresh JSON-LD parser with no document base.
    #[must_use]
    pub const fn new() -> Self {
        Self { base: None }
    }

    /// Construct a parser with the given document base IRI.
    #[must_use]
    pub fn with_base(base: impl Into<String>) -> Self {
        Self {
            base: Some(base.into()),
        }
    }
}

impl Parser for JsonLdParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        // Stage 1: UTF-8 decode.
        let text = std::str::from_utf8(input).map_err(|e| Diagnostics {
            messages: vec![format!(
                "JSONLD-UTF8-001: invalid UTF-8 at byte {}",
                e.valid_up_to()
            )],
            fatal: true,
        })?;

        // Strip optional leading UTF-8 BOM.
        let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);

        // Stage 2: JSON parse.
        let doc: serde_json::Value =
            serde_json::from_str(text).map_err(|e| Diagnostics {
                messages: vec![format!("JSONLD-JSON-001: malformed JSON: {e}")],
                fatal: true,
            })?;

        // Stage 3: toRdf conversion.
        to_rdf::convert(&doc, self.base.as_deref(), self.id())
    }

    fn id(&self) -> &'static str {
        "rdf-jsonld"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Result<rdf_diff::Facts, Diagnostics> {
        JsonLdParser::new().parse(input.as_bytes()).map(|o| o.facts)
    }

    fn parse_with_base(input: &str, base: &str) -> Result<rdf_diff::Facts, Diagnostics> {
        JsonLdParser::with_base(base)
            .parse(input.as_bytes())
            .map(|o| o.facts)
    }

    // Helper to get the single fact from a result.
    fn single_fact(facts: &rdf_diff::Facts) -> &rdf_diff::Fact {
        assert_eq!(facts.set.len(), 1, "expected exactly one fact");
        facts.set.keys().next().unwrap()
    }

    #[test]
    fn plain_literal_with_full_uris() {
        // Test 0001
        let facts = parse(r#"{"@id":"http://greggkellogg.net/foaf#me","http://xmlns.com/foaf/0.1/name":"Gregg Kellogg"}"#)
            .expect("accept");
        let f = single_fact(&facts);
        assert_eq!(f.subject, "<http://greggkellogg.net/foaf#me>");
        assert_eq!(f.predicate, "<http://xmlns.com/foaf/0.1/name>");
        assert_eq!(f.object, "\"Gregg Kellogg\"");
    }

    #[test]
    fn curie_from_context() {
        // Test 0002: prefix expansion
        let input = r#"{
            "@context": {"foaf": "http://xmlns.com/foaf/0.1/"},
            "@id": "http://greggkellogg.net/foaf#me",
            "foaf:name": "Gregg Kellogg"
        }"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert_eq!(f.predicate, "<http://xmlns.com/foaf/0.1/name>");
    }

    #[test]
    fn default_subject_is_bnode() {
        // Test 0003: no @id → blank node subject
        let input = r#"{
            "@context": {"foaf": "http://xmlns.com/foaf/0.1/"},
            "@type": "foaf:Person"
        }"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert!(f.subject.starts_with("_:"));
        assert_eq!(f.predicate, "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>");
        assert_eq!(f.object, "<http://xmlns.com/foaf/0.1/Person>");
    }

    #[test]
    fn lang_tagged_literal() {
        // Test 0004 — rdf_diff::Facts::canonicalise case-folds BCP-47 tags,
        // so "en-us" → "en-US".
        let input = r#"{"http://www.w3.org/2000/01/rdf-schema#label":{"@value":"A plain literal with a lang tag.","@language":"en-us"}}"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        // canonicalise: en-us → en-US (region tag upcased)
        assert_eq!(f.object, "\"A plain literal with a lang tag.\"@en-US");
    }

    #[test]
    fn nested_node_gets_bnode() {
        // Test 0005: nested node without @id → bnode
        let input = r#"{
            "@id": "http://greggkellogg.net/foaf#me",
            "http://xmlns.com/foaf/0.1/knows": {
                "http://xmlns.com/foaf/0.1/name": {"@value": "Herman Iván", "@language": "hu"}
            }
        }"#;
        let facts = parse(input).expect("accept");
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn typed_literal_date() {
        // Test 0006
        let input = r#"{
            "@id":  "http://greggkellogg.net/foaf#me",
            "http://purl.org/dc/terms/created": {
                "@value": "1957-02-27",
                "@type": "http://www.w3.org/2001/XMLSchema#date"
            }
        }"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert_eq!(
            f.object,
            "\"1957-02-27\"^^<http://www.w3.org/2001/XMLSchema#date>"
        );
    }

    #[test]
    fn type_is_expanded_iri() {
        // Test 0007
        let input = r#"{"@id":"http://greggkellogg.net/foaf#me","@type":"http://xmlns.com/foaf/0.1/Person"}"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert_eq!(f.object, "<http://xmlns.com/foaf/0.1/Person>");
    }

    #[test]
    fn integer_literal() {
        // Test 0023
        let input = r#"{"@context":{"chem":"http://example/chem#"},"chem:protons":12}"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert_eq!(
            f.object,
            "\"12\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn boolean_literal() {
        // Test 0024
        let input =
            r#"{"@context":{"sensor":"http://example/sensor#"},"sensor:active":true}"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert_eq!(
            f.object,
            "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"
        );
    }

    #[test]
    fn array_of_values_emits_multiple_facts() {
        // Test 0012: array → multiple triples
        let input = r#"{
            "@context": {"foaf": "http://xmlns.com/foaf/0.1/"},
            "@id": "http://greggkellogg.net/foaf#me",
            "foaf:knows": ["Manu Sporny", "Dave Longley"]
        }"#;
        let facts = parse(input).expect("accept");
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn empty_list() {
        // Test 0013
        let input = r#"{
            "@context": {"foaf": "http://xmlns.com/foaf/0.1/"},
            "@id": "http://greggkellogg.net/foaf#me",
            "foaf:knows": {"@list": []}
        }"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert_eq!(
            f.object,
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"
        );
    }

    #[test]
    fn single_item_list() {
        // Test 0014
        let input = r#"{
            "@context": {"foaf": "http://xmlns.com/foaf/0.1/"},
            "@id": "http://greggkellogg.net/foaf#me",
            "foaf:knows": {"@list": ["Manu Sporny"]}
        }"#;
        let facts = parse(input).expect("accept");
        // Three triples: knows→bnode, bnode:first→"Manu Sporny", bnode:rest→nil
        assert_eq!(facts.set.len(), 3);
    }

    #[test]
    fn empty_id_resolves_to_base() {
        // Test 0016: @id "" → document base
        let input = r#"{"@id":"","@type":"http://www.w3.org/2000/01/rdf-schema#Resource"}"#;
        let facts = parse_with_base(
            input,
            "https://w3c.github.io/json-ld-api/tests/toRdf/0016-in.jsonld",
        )
        .expect("accept");
        let f = single_fact(&facts);
        assert_eq!(
            f.subject,
            "<https://w3c.github.io/json-ld-api/tests/toRdf/0016-in.jsonld>"
        );
    }

    #[test]
    fn type_coercion_id() {
        // Test 0019: @type: @id coercion
        let input = r#"{
            "@context": {
                "foaf": "http://xmlns.com/foaf/0.1/",
                "knows": {"@id": "http://xmlns.com/foaf/0.1/knows", "@type": "@id"}
            },
            "@id": "http://greggkellogg.net/foaf#me",
            "knows": "http://manu.sporny.org/#me"
        }"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert_eq!(f.object, "<http://manu.sporny.org/#me>");
    }

    #[test]
    fn named_graph_with_at_graph() {
        // Test 0028: top-level @graph node with named graph IRI
        let input = r#"{
            "@id": "http://example.org/sig1",
            "@type": "http://www.w3.org/1999/02/22-rdf-syntax-ns#Graph",
            "@graph": {
                "@id": "http://example.org/fact1",
                "http://purl.org/dc/terms/title": "Hello World!"
            }
        }"#;
        let facts = parse(input).expect("accept");
        // Should have: sig1 rdf:type rdf:Graph (default graph)
        // AND fact1 dct:title "Hello World!" (named graph: sig1)
        assert_eq!(facts.set.len(), 2);
        let in_named: Vec<_> = facts
            .set
            .keys()
            .filter(|f| f.graph.is_some())
            .collect();
        assert_eq!(in_named.len(), 1);
        assert_eq!(
            in_named[0].graph.as_deref(),
            Some("<http://example.org/sig1>")
        );
    }

    #[test]
    fn two_item_list() {
        // Test 0015
        let input = r#"{
            "@context": {"foaf": "http://xmlns.com/foaf/0.1/"},
            "@id": "http://greggkellogg.net/foaf#me",
            "foaf:knows": {"@list": ["Manu Sporny", "Dave Longley"]}
        }"#;
        let facts = parse(input).expect("accept");
        // 5 triples: knows→b0, b0:first→"Manu", b0:rest→b1, b1:first→"Dave", b1:rest→nil
        assert_eq!(facts.set.len(), 5);
    }

    #[test]
    fn container_list_from_array() {
        // Test 0025: @container: @list turns array into rdf:List
        let input = r#"{
            "@context": {
                "knows": {"@id": "http://xmlns.com/foaf/0.1/knows", "@container": "@list"}
            },
            "@id": "http://greggkellogg.net/foaf#me",
            "knows": ["Manu Sporny"]
        }"#;
        let facts = parse(input).expect("accept");
        // 3 triples: knows→b0, b0:first→"Manu", b0:rest→nil
        assert_eq!(facts.set.len(), 3);
    }

    #[test]
    fn multiple_types() {
        // Test 0026: @type array
        let input = r#"{
            "@context": {"rdfs": "http://www.w3.org/2000/01/rdf-schema#"},
            "@type": ["rdfs:Resource", "rdfs:Class"]
        }"#;
        let facts = parse(input).expect("accept");
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn double_literal() {
        // Test 0022: JSON float → xsd:double
        let input = r#"{"@context":{"measure":"http://example/measure#"},"measure:cups":5.3}"#;
        let facts = parse(input).expect("accept");
        let f = single_fact(&facts);
        assert!(
            f.object.contains("xsd:double") || f.object.ends_with(">"),
            "should be xsd:double typed literal, got: {}",
            f.object
        );
        assert!(
            f.object.starts_with('"'),
            "should be a literal: {}",
            f.object
        );
    }

    #[test]
    fn malformed_json_rejected() {
        let err = JsonLdParser::new()
            .parse(b"{not valid json}")
            .expect_err("reject");
        assert!(err.fatal);
        assert!(err.messages[0].starts_with("JSONLD-JSON-001"));
    }

    #[test]
    fn empty_input_rejected() {
        let err = JsonLdParser::new().parse(b"").expect_err("reject");
        assert!(err.fatal);
    }

    #[test]
    fn parser_id_is_stable() {
        assert_eq!(JsonLdParser::new().id(), "rdf-jsonld");
    }
}
