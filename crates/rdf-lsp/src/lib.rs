//! RDF Language Server Protocol implementation (Phase F, ADR-0025).
//!
//! Supports: NT, NQ, Turtle, TriG, RDF/XML, JSON-LD, TriX, N3,
//! SPARQL, ShEx, Datalog.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod dispatch;
pub mod features;

/// Start the LSP server, reading from stdin and writing to stdout.
pub fn run_server() {
    eprintln!("rdf-lsp: not yet implemented");
}
