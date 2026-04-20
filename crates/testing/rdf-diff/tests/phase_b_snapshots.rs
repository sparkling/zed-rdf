//! Phase B snapshot tests: main-parser vs oracle for RDF/XML and JSON-LD.
//!
//! Each test parses a trivial document through the main parser and through
//! the oxrdfxml / oxjsonld reference oracle and asserts
//! `diff_many(&[main, oracle])` returns an empty [`DiffReport`].
//!
//! ## Lifecycle
//!
//! The main parsers are Phase B stubs that return `Err`. Tests that depend
//! on those stubs are `#[ignore]`-gated with a comment explaining the
//! un-ignore trigger.
//!
//! Un-ignore each test when the corresponding agent lands the real parser:
//! - `pb_rdfxml_*` tests: un-ignore once `pb-rdf-xml` agent fills in
//!   `crates/rdf-xml/src/lib.rs` AND resolves compilation errors in
//!   `crates/rdf-xml/src/parser.rs`, then add `rdf-xml` back as a
//!   dev-dep in `crates/testing/rdf-diff/Cargo.toml`.
//! - `pb_jsonld_*` tests: un-ignore once `pb-rdf-jsonld` agent fills in
//!   `crates/rdf-jsonld/src/lib.rs` so that `JsonLdParser::parse` returns
//!   `Ok`.
//!
//! ## Crate availability
//!
//! - `rdf-jsonld`: added as dev-dep; compiles cleanly; tests below are
//!   active (but `#[ignore]`-gated on the stub returning `Err`).
//! - `rdf-xml`: NOT yet a dev-dep; compilation errors in `parser.rs` block
//!   it. The RDF/XML snapshot tests below are compiled only when the
//!   `shadow-xml` feature is enabled (which it is not by default).
//!   Once `pb-rdf-xml` fixes the compilation, add `rdf-xml` back to
//!   dev-deps and remove the `#[cfg(feature = "shadow-xml")]` guards.
//!
//! ## ADR references
//!
//! - ADR-0019 §2 — differential test harness responsibilities.
//! - ADR-0019 §1 — oracle carve-out; ox* crates are dev-deps only.
//! - ADR-0020 §1.4 — frozen `Parser` trait integration contract.
//! - ADR-0021 — Phase B scope (RDF/XML, JSON-LD, TriX, N3).

#![allow(
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::doc_markdown,
    unused_imports,
)]

use std::collections::BTreeMap;

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, diff_many};
use rdf_diff::Parser as _;
use rdf_jsonld::JsonLdParser;
use rdf_xml::RdfXmlParser;

// ---------------------------------------------------------------------------
// Trivial test documents
// ---------------------------------------------------------------------------

/// Minimal RDF/XML document: one triple <ex/s> <ex/p> <ex/o>.
#[cfg(feature = "oracle-oxrdfxml")]
const TRIVIAL_RDFXML: &[u8] = b"\
<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n\
         xmlns:ex=\"http://ex/\">\n\
  <rdf:Description rdf:about=\"http://ex/s\">\n\
    <ex:p rdf:resource=\"http://ex/o\"/>\n\
  </rdf:Description>\n\
</rdf:RDF>\n";

/// Minimal JSON-LD document: one triple <ex/s> <ex/p> <ex/o>.
const TRIVIAL_JSONLD: &[u8] = b"\
{\n\
  \"@id\": \"http://ex/s\",\n\
  \"http://ex/p\": [{\"@id\": \"http://ex/o\"}]\n\
}\n";

// ---------------------------------------------------------------------------
// Oracle helpers
// ---------------------------------------------------------------------------

/// Minimal inline RDF/XML oracle using oxrdfxml directly.
/// Mirrors `crates/testing/rdf-diff-oracles/src/oxrdfxml_adapter.rs`.
/// Gated on `oracle-oxrdfxml` because in a `--no-default-features` build
/// the test body may be omitted; the dev-dep `oxrdfxml` is always compiled.
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

/// Minimal inline JSON-LD oracle using oxjsonld directly.
/// Mirrors `crates/testing/rdf-diff-oracles/src/oxjsonld_adapter.rs`.
#[cfg(feature = "oracle-oxjsonld")]
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
// RDF/XML snapshot tests
//
// The `rdf-xml` crate is not yet a dev-dep (compilation errors block it).
// These tests are gated on `feature = "shadow-xml"` as a proxy flag until
// `pb-rdf-xml` resolves the compilation issues and the crate is added back
// as a dev-dep.
//
// When rdf-xml compiles cleanly:
//   1. Add `rdf-xml = { path = "../../rdf-xml", version = "0.0.0" }` to
//      [dev-dependencies] in crates/testing/rdf-diff/Cargo.toml.
//   2. Replace `#[cfg(feature = "shadow-xml")]` guards below with
//      `#[cfg(feature = "oracle-oxrdfxml")]` (or unconditional for the
//      stub-guard tests).
//   3. Add `use rdf_xml::RdfXmlParser;` to the top-level imports above.
// ---------------------------------------------------------------------------

/// **PB-S1 — RDF/XML oracle self-diff is clean.**
///
/// Always-on sanity check (no dependency on the `rdf-xml` stub).
/// Verifies the inline oracle helper compiles and the diff pipeline is sound.
#[cfg(feature = "oracle-oxrdfxml")]
#[test]
fn pb_rdfxml_oracle_self_diff_clean() {
    let oracle_facts = oracle_parse_rdfxml(TRIVIAL_RDFXML)
        .expect("oxrdfxml oracle must accept trivial RDF/XML");

    let report =
        diff_many([("oxrdfxml-oracle-a", &oracle_facts), ("oxrdfxml-oracle-b", &oracle_facts)])
            .expect("canonical input");
    assert!(
        report.is_clean(),
        "oracle self-diff dirty: {:?}",
        report.divergences,
    );
}

/// **PB-S2 — RDF/XML main parser agrees with oxrdfxml oracle on trivial input.**
#[cfg(feature = "oracle-oxrdfxml")]
#[test]
fn pb_rdfxml_main_agrees_with_oracle_trivial() {
    let main_outcome = RdfXmlParser::new()
        .parse(TRIVIAL_RDFXML)
        .expect("main RdfXmlParser must accept trivial RDF/XML");
    let oracle_facts = oracle_parse_rdfxml(TRIVIAL_RDFXML)
        .expect("oxrdfxml oracle must accept trivial RDF/XML");

    let report = diff_many([
        ("rdf-xml", &main_outcome.facts),
        ("oxrdfxml-oracle", &oracle_facts),
    ])
    .expect("both inputs must be in canonical form");

    assert!(
        report.is_clean(),
        "main parser diverges from oxrdfxml oracle on trivial RDF/XML:\n{:#?}",
        report.divergences,
    );
}

// ---------------------------------------------------------------------------
// JSON-LD snapshot tests
// ---------------------------------------------------------------------------

/// **PB-S4 — JSON-LD oracle self-diff is clean.**
///
/// Always-on sanity check: the oxjsonld oracle agrees with itself.
#[cfg(feature = "oracle-oxjsonld")]
#[test]
fn pb_jsonld_oracle_self_diff_clean() {
    let oracle_facts =
        oracle_parse_jsonld(TRIVIAL_JSONLD).expect("oxjsonld oracle must accept trivial JSON-LD");

    let report =
        diff_many([("oxjsonld-oracle-a", &oracle_facts), ("oxjsonld-oracle-b", &oracle_facts)])
            .expect("canonical input");
    assert!(
        report.is_clean(),
        "oracle self-diff dirty: {:?}",
        report.divergences,
    );
}

/// **PB-S5 — JSON-LD main parser agrees with oxjsonld oracle on trivial input.**
#[cfg(feature = "oracle-oxjsonld")]
#[test]
fn pb_jsonld_main_agrees_with_oracle_trivial() {
    let main_outcome = JsonLdParser::new()
        .parse(TRIVIAL_JSONLD)
        .expect("main JsonLdParser must accept trivial JSON-LD");
    let oracle_facts =
        oracle_parse_jsonld(TRIVIAL_JSONLD).expect("oxjsonld oracle must accept trivial JSON-LD");

    let report = diff_many([
        ("rdf-jsonld", &main_outcome.facts),
        ("oxjsonld-oracle", &oracle_facts),
    ])
    .expect("both inputs must be in canonical form");

    assert!(
        report.is_clean(),
        "main parser diverges from oxjsonld oracle on trivial JSON-LD:\n{:#?}",
        report.divergences,
    );
}

// ---------------------------------------------------------------------------
// Compile-shape check (always-on, no broken-crate dependency)
// ---------------------------------------------------------------------------

/// **PB-S0 — Phase B parser types implement `Parser`.**
#[test]
fn pb_parser_trait_objects_compile() {
    let _parsers: Vec<Box<dyn rdf_diff::Parser>> = vec![
        Box::new(JsonLdParser::new()),
        Box::new(RdfXmlParser::new()),
    ];
}
