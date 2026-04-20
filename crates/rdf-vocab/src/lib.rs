//! RDF vocabulary term definitions (Phase E, ADR-0017 §4).
//!
//! Each supported vocabulary is a module that exposes:
//!
//! - `pub const NS: &str` — the namespace IRI (with trailing `#` or `/`)
//! - One `pub const` per term, named in `SCREAMING_SNAKE_CASE`, containing the
//!   full IRI string.  Each constant carries `///` doc-comments with `Label:`
//!   and `Description:` fields for Phase F hover-doc rendering.
//!
//! # Vocabularies
//!
//! | Module | Namespace |
//! |--------|-----------|
//! | [`xsd`] | `http://www.w3.org/2001/XMLSchema#` |
//! | [`rdf`] | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` |
//! | [`rdfs`] | `http://www.w3.org/2000/01/rdf-schema#` |
//! | [`owl`] | `http://www.w3.org/2002/07/owl#` |
//! | [`skos`] | `http://www.w3.org/2004/02/skos/core#` |
//! | [`sh`] | `http://www.w3.org/ns/shacl#` |
//! | [`dcterms`] | `http://purl.org/dc/terms/` |
//! | [`dcat`] | `http://www.w3.org/ns/dcat#` |
//! | [`foaf`] | `http://xmlns.com/foaf/0.1/` |
//! | [`schema`] | `https://schema.org/` |
//! | [`prov`] | `http://www.w3.org/ns/prov#` |

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod xsd;
pub mod rdf;
pub mod rdfs;
pub mod owl;
pub mod skos;
pub mod sh;
pub mod dcterms;
pub mod dcat;
pub mod foaf;
pub mod schema;
pub mod prov;
