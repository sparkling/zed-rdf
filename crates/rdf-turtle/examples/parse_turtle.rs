//! Example: parse a Turtle document and print diagnostic output.
//!
//! Run with:
//!   cargo run --example parse_turtle

use rdf_diff::Parser as _;
use rdf_turtle::TurtleParser;

fn main() {
    let input = r#"
@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex:Alice a ex:Person ;
    ex:name "Alice" ;
    ex:age 30 .

ex:Bob a ex:Person ;
    ex:name "Bob" ;
    ex:knows ex:Alice .
"#;

    match TurtleParser.parse(input.as_bytes()) {
        Ok(outcome) => {
            println!("Parsed {} facts:", outcome.facts.set.len());
            for (fact, _) in &outcome.facts.set {
                println!("  {} {} {}", fact.subject, fact.predicate, fact.object);
            }
            if !outcome.warnings.messages.is_empty() {
                println!("Warnings:");
                for w in &outcome.warnings.messages {
                    println!("  {w}");
                }
            }
        }
        Err(diag) => {
            eprintln!("Parse error (fatal={}):", diag.fatal);
            for msg in &diag.messages {
                eprintln!("  {msg}");
            }
            std::process::exit(1);
        }
    }
}
