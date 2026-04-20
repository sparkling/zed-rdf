//! Tests for `Language::from_uri` — activated after pf-lsp-protocol merges.
//!
//! All tests are marked `#[ignore]` because `Language` is not yet exported
//! from the stub `rdf_lsp` crate. Once `pf-lsp-protocol` lands its changes to
//! `crates/rdf-lsp/src/lib.rs`, remove the `#[ignore]` attributes and the
//! `// TODO` comments to make these tests active.
//!
//! Expected mapping (from arch-memo §3):
//!
//! | Extension              | `Language` variant      |
//! |------------------------|-------------------------|
//! | `.nt`                  | `Language::NTriples`    |
//! | `.nq`                  | `Language::NQuads`      |
//! | `.ttl` / `.turtle`     | `Language::Turtle`      |
//! | `.trig`                | `Language::TriG`        |
//! | `.rdf` / `.xml`        | `Language::RdfXml`      |
//! | `.jsonld`              | `Language::JsonLd`      |
//! | `.trix`                | `Language::TriX`        |
//! | `.n3`                  | `Language::N3`          |
//! | `.sparql` / `.rq` / `.ru` | `Language::Sparql`   |
//! | `.shex`                | `Language::ShEx`        |
//! | `.dl`                  | `Language::Datalog`     |
//! | unknown                | `None`                  |

// Helper: build a `lsp_types::Url` from a filename string.
// The `lsp_types` crate is available as a transitive dep of `rdf_lsp`.
fn url_for(filename: &str) -> lsp_types::Url {
    lsp_types::Url::parse(&format!("file:///workspace/{filename}")).unwrap()
}

// ---------------------------------------------------------------------------
// NT
// ---------------------------------------------------------------------------

#[test]
fn nt_extension_detects_ntriples() {
    use rdf_lsp::Language;
    let uri = url_for("data.nt");
    assert_eq!(Language::from_uri(&uri), Some(Language::NTriples));
}

// ---------------------------------------------------------------------------
// NQ
// ---------------------------------------------------------------------------

#[test]
fn nq_extension_detects_nquads() {
    use rdf_lsp::Language;
    let uri = url_for("dataset.nq");
    assert_eq!(Language::from_uri(&uri), Some(Language::NQuads));
}

// ---------------------------------------------------------------------------
// Turtle
// ---------------------------------------------------------------------------

#[test]
fn ttl_extension_detects_turtle() {
    use rdf_lsp::Language;
    let uri = url_for("ontology.ttl");
    assert_eq!(Language::from_uri(&uri), Some(Language::Turtle));
}

#[test]
fn turtle_extension_detects_turtle() {
    use rdf_lsp::Language;
    let uri = url_for("ontology.turtle");
    assert_eq!(Language::from_uri(&uri), Some(Language::Turtle));
}

// ---------------------------------------------------------------------------
// TriG
// ---------------------------------------------------------------------------

#[test]
fn trig_extension_detects_trig() {
    use rdf_lsp::Language;
    let uri = url_for("graph.trig");
    assert_eq!(Language::from_uri(&uri), Some(Language::TriG));
}

// ---------------------------------------------------------------------------
// RDF/XML
// ---------------------------------------------------------------------------

#[test]
fn rdf_extension_detects_rdfxml() {
    use rdf_lsp::Language;
    let uri = url_for("schema.rdf");
    assert_eq!(Language::from_uri(&uri), Some(Language::RdfXml));
}

#[test]
fn xml_extension_detects_rdfxml() {
    use rdf_lsp::Language;
    let uri = url_for("schema.xml");
    assert_eq!(Language::from_uri(&uri), Some(Language::RdfXml));
}

// ---------------------------------------------------------------------------
// JSON-LD
// ---------------------------------------------------------------------------

#[test]
fn jsonld_extension_detects_jsonld() {
    use rdf_lsp::Language;
    let uri = url_for("context.jsonld");
    assert_eq!(Language::from_uri(&uri), Some(Language::JsonLd));
}

// ---------------------------------------------------------------------------
// TriX
// ---------------------------------------------------------------------------

#[test]
fn trix_extension_detects_trix() {
    use rdf_lsp::Language;
    let uri = url_for("graph.trix");
    assert_eq!(Language::from_uri(&uri), Some(Language::TriX));
}

// ---------------------------------------------------------------------------
// N3
// ---------------------------------------------------------------------------

#[test]
fn n3_extension_detects_n3() {
    use rdf_lsp::Language;
    let uri = url_for("rules.n3");
    assert_eq!(Language::from_uri(&uri), Some(Language::N3));
}

// ---------------------------------------------------------------------------
// SPARQL
// ---------------------------------------------------------------------------

#[test]
fn sparql_extension_detects_sparql() {
    use rdf_lsp::Language;
    let uri = url_for("query.sparql");
    assert_eq!(Language::from_uri(&uri), Some(Language::Sparql));
}

#[test]
fn rq_extension_detects_sparql() {
    use rdf_lsp::Language;
    let uri = url_for("query.rq");
    assert_eq!(Language::from_uri(&uri), Some(Language::Sparql));
}

#[test]
fn ru_extension_detects_sparql() {
    use rdf_lsp::Language;
    let uri = url_for("update.ru");
    assert_eq!(Language::from_uri(&uri), Some(Language::Sparql));
}

// ---------------------------------------------------------------------------
// ShEx
// ---------------------------------------------------------------------------

#[test]
fn shex_extension_detects_shex() {
    use rdf_lsp::Language;
    let uri = url_for("shapes.shex");
    assert_eq!(Language::from_uri(&uri), Some(Language::ShEx));
}

// ---------------------------------------------------------------------------
// Datalog
// ---------------------------------------------------------------------------

#[test]
fn dl_extension_detects_datalog() {
    use rdf_lsp::Language;
    let uri = url_for("rules.dl");
    assert_eq!(Language::from_uri(&uri), Some(Language::Datalog));
}

// ---------------------------------------------------------------------------
// Unknown extension
// ---------------------------------------------------------------------------

#[test]
fn unknown_extension_returns_none() {
    use rdf_lsp::Language;
    let uri = url_for("document.pdf");
    assert_eq!(Language::from_uri(&uri), None);
}

#[test]
fn no_extension_returns_none() {
    use rdf_lsp::Language;
    let uri = url_for("Makefile");
    assert_eq!(Language::from_uri(&uri), None);
}
