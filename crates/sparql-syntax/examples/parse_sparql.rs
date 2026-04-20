//! Example: parse a SPARQL query and check for syntax errors.
//!
//! Run with:
//!   cargo run --example parse_sparql

use rdf_diff::Parser as _;
use sparql_syntax::SparqlParser;

fn main() {
    let queries = [
        ("Valid SELECT", "SELECT ?s ?p ?o WHERE { ?s ?p ?o . } LIMIT 10"),
        ("Valid ASK", "ASK { <http://example.org/foo> ?p ?o . }"),
        ("Invalid (missing WHERE)", "SELECT ?x LIMIT 5"),
    ];

    for (label, query) in queries {
        print!("{label}: ");
        match SparqlParser.parse(query.as_bytes()) {
            Ok(_) => println!("OK"),
            Err(diag) => println!("ERROR — {}", diag.messages.join("; ")),
        }
    }
}
