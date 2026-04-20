//! Completion handler — returns language-appropriate keyword and vocabulary
//! term completions.

use lsp_types::{CompletionItem, CompletionItemKind, CompletionList, Position};

use crate::Language;

/// Return a `CompletionList` for the given language and cursor position.
///
/// Phase F scope: static keyword lists + top RDF-vocab terms for Turtle/TriG.
/// The `_pos` parameter is accepted for API compatibility; context-sensitive
/// narrowing is deferred to Phase G.
#[must_use]
pub fn handle_completion(text: &str, lang: Language, pos: Position) -> CompletionList {
    let _ = (text, pos); // position-aware narrowing deferred to Phase G
    let items = match lang {
        Language::Turtle | Language::TriG | Language::N3 => turtle_items(),
        Language::Sparql => sparql_items(),
        Language::ShEx => shex_items(),
        Language::NTriples
        | Language::NQuads
        | Language::RdfXml
        | Language::JsonLd
        | Language::TriX
        | Language::Datalog => vec![],
    };

    CompletionList {
        is_incomplete: false,
        items,
    }
}

// ---------------------------------------------------------------------------
// Per-language item lists
// ---------------------------------------------------------------------------

fn keyword(label: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(CompletionItemKind::KEYWORD),
        ..Default::default()
    }
}

fn term_item(label: &str, iri: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(CompletionItemKind::VALUE),
        detail: Some(detail.to_owned()),
        documentation: Some(lsp_types::Documentation::String(iri.to_owned())),
        insert_text: Some(label.to_owned()),
        ..Default::default()
    }
}

/// Turtle/TriG/N3 completions: keywords + top 10 rdf vocab terms.
fn turtle_items() -> Vec<CompletionItem> {
    use rdf_vocab::rdf;

    let mut items: Vec<CompletionItem> = vec![
        keyword("@prefix"),
        keyword("@base"),
        keyword("a"),
        keyword("true"),
        keyword("false"),
    ];

    // Top 10 rdf: terms (most commonly used in Turtle documents).
    let rdf_terms: [(&str, &str); 10] = [
        ("rdf:type", rdf::TYPE),
        ("rdf:Property", rdf::PROPERTY),
        ("rdf:List", rdf::LIST),
        ("rdf:nil", rdf::NIL),
        ("rdf:first", rdf::FIRST),
        ("rdf:rest", rdf::REST),
        ("rdf:value", rdf::VALUE),
        ("rdf:Bag", rdf::BAG),
        ("rdf:Seq", rdf::SEQ),
        ("rdf:Alt", rdf::ALT),
    ];

    for (label, iri) in &rdf_terms {
        items.push(term_item(label, iri, "rdf:"));
    }

    items
}

/// SPARQL keyword completions.
fn sparql_items() -> Vec<CompletionItem> {
    const KEYWORDS: &[&str] = &[
        "SELECT",
        "WHERE",
        "FILTER",
        "OPTIONAL",
        "UNION",
        "PREFIX",
        "BASE",
        "ASK",
        "CONSTRUCT",
        "DESCRIBE",
        "INSERT",
        "DELETE",
        "LIMIT",
        "OFFSET",
        "ORDER BY",
        "GROUP BY",
    ];
    KEYWORDS.iter().copied().map(keyword).collect()
}

/// `ShEx` keyword completions.
fn shex_items() -> Vec<CompletionItem> {
    const KEYWORDS: &[&str] = &[
        "IRI",
        "LITERAL",
        "BNODE",
        "NONLITERAL",
        "AND",
        "OR",
        "NOT",
    ];
    KEYWORDS.iter().copied().map(keyword).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turtle_completions_include_prefix_keyword() {
        let list = handle_completion("", Language::Turtle, Position { line: 0, character: 0 });
        assert!(!list.is_incomplete);
        let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"@prefix"), "expected @prefix in Turtle completions");
        assert!(labels.contains(&"a"), "expected 'a' keyword");
        assert!(labels.contains(&"rdf:type"), "expected rdf:type");
    }

    #[test]
    fn sparql_completions_include_select() {
        let list = handle_completion("", Language::Sparql, Position { line: 0, character: 0 });
        let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"SELECT"));
        assert!(labels.contains(&"WHERE"));
        assert!(labels.contains(&"FILTER"));
    }

    #[test]
    fn ntriples_completions_are_empty() {
        let list = handle_completion("", Language::NTriples, Position { line: 0, character: 0 });
        assert!(list.items.is_empty(), "N-Triples has no completions");
    }

    #[test]
    fn shex_completions_include_iri_keyword() {
        let list = handle_completion("", Language::ShEx, Position { line: 0, character: 0 });
        let labels: Vec<_> = list.items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"IRI"));
        assert!(labels.contains(&"AND"));
    }
}
