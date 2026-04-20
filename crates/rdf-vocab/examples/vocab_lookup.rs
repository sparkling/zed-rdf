//! Example: look up RDF vocabulary terms.
//!
//! Run with:
//!   cargo run --example vocab_lookup

use rdf_vocab::{rdf, rdfs, owl, xsd};

fn main() {
    println!("=== Core RDF terms ===");
    println!("rdf:type = {}", rdf::TYPE);
    println!("rdf:Property = {}", rdf::PROPERTY);
    println!("rdfs:Class = {}", rdfs::CLASS);
    println!("rdfs:label = {}", rdfs::LABEL);
    println!("owl:Class = {}", owl::CLASS);
    println!("xsd:string = {}", xsd::STRING);
    println!("xsd:integer = {}", xsd::INTEGER);
    println!();
    println!("=== SKOS ===");
    use rdf_vocab::skos;
    println!("skos:Concept = {}", skos::CONCEPT);
    println!("skos:prefLabel = {}", skos::PREF_LABEL);
    println!();
    println!("=== Schema.org ===");
    use rdf_vocab::schema;
    println!("schema:Person = {}", schema::PERSON);
    println!("schema:name = {}", schema::NAME);
}
