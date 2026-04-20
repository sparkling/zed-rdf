//! Phase B three-way shadow-diff tests: main vs shadow vs oracle.
//!
//! These tests diff three independent implementations against each other:
//! - main parser (`rdf_xml::RdfXmlParser` / `rdf_jsonld::JsonLdParser`)
//! - shadow parser (`rdf_xml_shadow::XmlShadowParser` / `rdf_jsonld_shadow::JsonLdShadowParser`)
//! - reference oracle (oxrdfxml / oxjsonld inline adapters)
//!
//! ## Lifecycle
//!
//! All tests in this file are `#[ignore]`-gated because both the main parsers
//! and the shadow parsers are Phase B stubs. Un-ignore conditions:
//!
//! - RDF/XML three-way tests: un-ignore once BOTH `pb-rdf-xml` (main) AND
//!   `pb-shadow-rdfxml` (shadow) agents land their implementations AND the
//!   `rdf-xml` / `rdf-xml-shadow` crates compile cleanly. At that point
//!   add them back as dev-deps in `crates/testing/rdf-diff/Cargo.toml` and
//!   update the imports here.
//!
//! - JSON-LD three-way tests: un-ignore once BOTH `pb-rdf-jsonld` (main)
//!   AND `pb-shadow-jsonld` (shadow) agents land their implementations.
//!
//! ## Crate availability
//!
//! - `rdf-jsonld`, `rdf-jsonld-shadow`: added as dev-deps; compile cleanly.
//! - `rdf-xml`, `rdf-xml-shadow`: NOT yet dev-deps; compilation errors in
//!   `parser.rs` block them. RDF/XML three-way tests are gated on
//!   `feature = "shadow-xml"` as a proxy flag until those errors are fixed.
//!
//! ## Feature gates
//!
//! - `shadow-xml` + `oracle-oxrdfxml`: RDF/XML three-way diff (blocked until
//!   rdf-xml and rdf-xml-shadow compile).
//! - `shadow-jsonld` + `oracle-oxjsonld`: JSON-LD three-way diff.
//!
//! ## ADR references
//!
//! - ADR-0019 §3 — shadow implementation independence requirement.
//! - ADR-0019 §2 — differential test harness responsibilities.
//! - ADR-0020 §1.4 — frozen `Parser` trait integration contract.
//! - ADR-0021 — Phase B scope.

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::doc_markdown,
    unused_imports,
)]

use std::collections::BTreeMap;

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, diff_many};
use rdf_diff::Parser as _;
use rdf_jsonld::JsonLdParser;
use rdf_jsonld_shadow::JsonLdShadowParser;
use rdf_xml::RdfXmlParser;
use rdf_xml_shadow::XmlShadowParser;

// ---------------------------------------------------------------------------
// Trivial test documents
// ---------------------------------------------------------------------------

/// Minimal RDF/XML document used by feature-gated three-way diff tests.
#[cfg(feature = "oracle-oxrdfxml")]
const TRIVIAL_RDFXML: &[u8] = b"\
<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n\
         xmlns:ex=\"http://ex/\">\n\
  <rdf:Description rdf:about=\"http://ex/s\">\n\
    <ex:p rdf:resource=\"http://ex/o\"/>\n\
  </rdf:Description>\n\
</rdf:RDF>\n";

/// Minimal JSON-LD document used by the three-way diff tests below.
/// Suppressed unless shadow-jsonld + oracle-oxjsonld are both enabled.
#[cfg(all(feature = "shadow-jsonld", feature = "oracle-oxjsonld"))]
const TRIVIAL_JSONLD: &[u8] = b"\
{\n\
  \"@id\": \"http://ex/s\",\n\
  \"http://ex/p\": [{\"@id\": \"http://ex/o\"}]\n\
}\n";

// ---------------------------------------------------------------------------
// Oracle helpers (feature-gated inline adapters)
// ---------------------------------------------------------------------------

#[cfg(feature = "oracle-oxrdfxml")]
fn oracle_parse_rdfxml(input: &[u8]) -> Result<Facts, Diagnostics> {
    use oxrdfxml::RdfXmlParser as OxParser;

    let mut parser = OxParser::new().for_slice(input);
    let mut raw: Vec<(Fact, FactProvenance)> = Vec::new();
    while let Some(step) = parser.next() {
        match step {
            Ok(triple) => {
                raw.push((
                    Fact {
                        subject: triple.subject.to_string(),
                        predicate: triple.predicate.as_str().to_string(),
                        object: triple.object.to_string(),
                        graph: None,
                    },
                    FactProvenance {
                        offset: None,
                        parser: "oxrdfxml-oracle".to_string(),
                    },
                ));
            }
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![format!("oxrdfxml-oracle: {e}")],
                    fatal: true,
                });
            }
        }
    }
    Ok(Facts::canonicalise(raw, BTreeMap::new()))
}

#[cfg(all(feature = "shadow-jsonld", feature = "oracle-oxjsonld"))]
fn oracle_parse_jsonld(input: &[u8]) -> Result<Facts, Diagnostics> {
    use oxjsonld::JsonLdParser as OxParser;
    use oxrdf::GraphName;

    let mut parser = OxParser::new().for_slice(input);
    let mut raw: Vec<(Fact, FactProvenance)> = Vec::new();
    while let Some(step) = parser.next() {
        match step {
            Ok(quad) => {
                let graph = match &quad.graph_name {
                    GraphName::DefaultGraph => None,
                    other => Some(other.to_string()),
                };
                raw.push((
                    Fact {
                        subject: quad.subject.to_string(),
                        predicate: quad.predicate.as_str().to_string(),
                        object: quad.object.to_string(),
                        graph,
                    },
                    FactProvenance {
                        offset: None,
                        parser: "oxjsonld-oracle".to_string(),
                    },
                ));
            }
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![format!("oxjsonld-oracle: {e}")],
                    fatal: true,
                });
            }
        }
    }
    Ok(Facts::canonicalise(raw, BTreeMap::new()))
}

// ---------------------------------------------------------------------------
// RDF/XML three-way diff
//
// Gated on `shadow-xml` because `rdf-xml` and `rdf-xml-shadow` are not
// yet dev-deps (compilation errors). When they are fixed:
//   1. Add both crates to dev-deps in Cargo.toml.
//   2. Add `use rdf_xml::RdfXmlParser; use rdf_xml_shadow::XmlShadowParser;`
//      to the top-level imports.
//   3. Replace the `shadow-xml` gate with `oracle-oxrdfxml`.
// ---------------------------------------------------------------------------

/// **PB-SD1 — RDF/XML three-way diff: main vs shadow vs oracle.**
#[cfg(feature = "oracle-oxrdfxml")]
#[test]
fn pb_rdfxml_three_way_main_shadow_oracle() {
    let main_outcome = RdfXmlParser::new()
        .parse(TRIVIAL_RDFXML)
        .expect("main RdfXmlParser must accept trivial RDF/XML");
    let shadow_outcome = XmlShadowParser::new()
        .parse(TRIVIAL_RDFXML)
        .expect("shadow XmlShadowParser must accept trivial RDF/XML");
    let oracle_facts = oracle_parse_rdfxml(TRIVIAL_RDFXML)
        .expect("oxrdfxml oracle must accept trivial RDF/XML");

    let report = diff_many([
        ("rdf-xml", &main_outcome.facts),
        ("rdf-xml-shadow", &shadow_outcome.facts),
        ("oxrdfxml-oracle", &oracle_facts),
    ])
    .expect("all three inputs must be in canonical form");

    assert!(
        report.is_clean(),
        "three-way RDF/XML diff found divergences:\n{:#?}",
        report.divergences,
    );
}

// ---------------------------------------------------------------------------
// JSON-LD three-way diff
// ---------------------------------------------------------------------------

/// **PB-SD2 — JSON-LD three-way diff: main vs shadow vs oracle.**
#[cfg(all(feature = "shadow-jsonld", feature = "oracle-oxjsonld"))]
#[test]
fn pb_jsonld_three_way_main_shadow_oracle() {
    let main_outcome = JsonLdParser::new()
        .parse(TRIVIAL_JSONLD)
        .expect("main JsonLdParser must accept trivial JSON-LD");
    let shadow_outcome = JsonLdShadowParser::new()
        .parse(TRIVIAL_JSONLD)
        .expect("shadow JsonLdShadowParser must accept trivial JSON-LD");
    let oracle_facts = oracle_parse_jsonld(TRIVIAL_JSONLD)
        .expect("oxjsonld oracle must accept trivial JSON-LD");

    let report = diff_many([
        ("rdf-jsonld", &main_outcome.facts),
        ("rdf-jsonld-shadow", &shadow_outcome.facts),
        ("oxjsonld-oracle", &oracle_facts),
    ])
    .expect("all three inputs must be in canonical form");

    assert!(
        report.is_clean(),
        "three-way JSON-LD diff found divergences:\n{:#?}",
        report.divergences,
    );
}

// ---------------------------------------------------------------------------
// Compile-shape check (always-on)
// ---------------------------------------------------------------------------

/// **PB-SD0 — Phase B shadow parser types implement `Parser`.**
#[test]
fn pb_shadow_parser_trait_objects_compile() {
    let _parsers: Vec<Box<dyn rdf_diff::Parser>> = vec![
        Box::new(JsonLdShadowParser::new()),
        Box::new(XmlShadowParser::new()),
    ];
}
