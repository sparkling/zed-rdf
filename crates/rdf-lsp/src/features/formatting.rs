//! Formatting handler — re-serialises RDF documents using the Phase-E writers.
//!
//! Supported languages (those with a Phase-E writer):
//! - N-Triples  → [`rdf_format::NTriplesWriter`]
//! - N-Quads    → [`rdf_format::NQuadsWriter`]
//! - Turtle     → [`rdf_format::TurtleWriter`]
//! - `TriG`       → [`rdf_format::TriGWriter`]
//!
//! All other languages return `None`; the dispatch layer must respond with
//! an empty success result in that case.

use lsp_types::{Position, Range, TextEdit};

use rdf_diff::Parser as ParserTrait;

use crate::Language;

/// Re-format `text` using the canonical writer for `lang`.
///
/// Returns a single `TextEdit` replacing the entire document with the
/// canonical serialisation on success.  Returns `None` when `lang` has no
/// Phase-F writer or when parsing fails (the document contains errors).
#[must_use]
pub fn handle_formatting(text: &str, lang: Language) -> Option<Vec<TextEdit>> {
    let formatted = match lang {
        Language::NTriples => format_ntriples(text)?,
        Language::NQuads => format_nquads(text)?,
        Language::Turtle => format_turtle(text)?,
        Language::TriG => format_trig(text)?,
        Language::RdfXml
        | Language::JsonLd
        | Language::TriX
        | Language::N3
        | Language::Sparql
        | Language::ShEx
        | Language::Datalog => return None,
    };

    // Count lines to build the replacement range covering the whole document.
    let line_count = u32::try_from(text.lines().count()).unwrap_or(u32::MAX);
    let last_col = text.lines().last().map_or(0, |l| {
        u32::try_from(l.len()).unwrap_or(u32::MAX)
    });
    let full_range = Range {
        start: Position { line: 0, character: 0 },
        end: Position {
            line: line_count,
            character: last_col,
        },
    };

    Some(vec![TextEdit {
        range: full_range,
        new_text: formatted,
    }])
}

// ---------------------------------------------------------------------------
// Per-format helpers
// ---------------------------------------------------------------------------

fn format_ntriples(text: &str) -> Option<String> {
    use rdf_format::NTriplesWriter;
    use rdf_ntriples::NTriplesParser;

    let outcome = NTriplesParser.parse(text.as_bytes()).ok()?;
    let mut buf = Vec::new();
    let mut writer = NTriplesWriter::new(&mut buf);
    for fact in outcome.facts.set.keys() {
        writer.write_fact(fact).ok()?;
    }
    writer.finish().ok()?;
    String::from_utf8(buf).ok()
}

fn format_nquads(text: &str) -> Option<String> {
    use rdf_format::NQuadsWriter;
    use rdf_ntriples::NQuadsParser;

    let outcome = NQuadsParser.parse(text.as_bytes()).ok()?;
    let mut buf = Vec::new();
    let mut writer = NQuadsWriter::new(&mut buf);
    for fact in outcome.facts.set.keys() {
        writer.write_fact(fact).ok()?;
    }
    writer.finish().ok()?;
    String::from_utf8(buf).ok()
}

fn format_turtle(text: &str) -> Option<String> {
    use rdf_format::TurtleWriter;
    use rdf_turtle::TurtleParser;

    let outcome = TurtleParser.parse(text.as_bytes()).ok()?;
    let mut buf = Vec::new();
    let mut writer = TurtleWriter::new(&mut buf);
    // Re-register any prefixes captured during parsing.
    for (prefix, iri) in &outcome.facts.prefixes {
        writer.with_prefix(prefix, iri);
    }
    for fact in outcome.facts.set.keys() {
        writer.write_fact(fact).ok()?;
    }
    writer.finish().ok()?;
    String::from_utf8(buf).ok()
}

fn format_trig(text: &str) -> Option<String> {
    use rdf_format::TriGWriter;
    use rdf_turtle::TriGParser;

    let outcome = TriGParser.parse(text.as_bytes()).ok()?;
    let mut buf = Vec::new();
    let mut writer = TriGWriter::new(&mut buf);
    for (prefix, iri) in &outcome.facts.prefixes {
        writer.with_prefix(prefix, iri);
    }
    for fact in outcome.facts.set.keys() {
        writer.write_fact(fact).ok()?;
    }
    writer.finish().ok()?;
    String::from_utf8(buf).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntriples_formatting_produces_edit() {
        let text =
            "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
        let edits = handle_formatting(text, Language::NTriples);
        assert!(edits.is_some(), "expected TextEdit for N-Triples");
        let edits = edits.unwrap();
        assert_eq!(edits.len(), 1);
        assert!(
            edits[0].new_text.contains("<http://example.org/s>"),
            "formatted output should contain the subject IRI"
        );
    }

    #[test]
    fn sparql_returns_none() {
        let result = handle_formatting("SELECT ?s WHERE { ?s ?p ?o }", Language::Sparql);
        assert!(result.is_none(), "SPARQL has no Phase-F formatter");
    }

    #[test]
    fn jsonld_returns_none() {
        let result = handle_formatting("{\"@context\": {}}", Language::JsonLd);
        assert!(result.is_none(), "JSON-LD has no Phase-F formatter");
    }
}
