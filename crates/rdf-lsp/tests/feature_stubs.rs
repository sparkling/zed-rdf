//! Integration tests for LSP feature handlers.

use rdf_lsp::Language;
use lsp_types::Position;

#[test]
fn hover_returns_none_for_unknown_iri() {
    use rdf_lsp::features::hover::handle_hover;
    let result = handle_hover("# empty Turtle document\n", Language::Turtle, Position { line: 0, character: 0 });
    assert!(result.is_none(), "expected None for unknown IRI, got {result:?}");
}

#[test]
fn completion_turtle_returns_prefix_keyword() {
    use rdf_lsp::features::completion::handle_completion;
    let list = handle_completion("", Language::Turtle, Position { line: 0, character: 0 });
    let labels: Vec<&str> = list.items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"@prefix"), "expected '@prefix' in completions, got: {labels:?}");
}

#[test]
fn document_symbols_turtle_empty_doc() {
    use rdf_lsp::features::document_symbols::handle_document_symbols;
    let symbols = handle_document_symbols("", Language::Turtle);
    assert!(symbols.is_empty(), "expected empty symbol list for empty document, got: {symbols:?}");
}

#[test]
fn formatting_rdfxml_returns_none() {
    use rdf_lsp::features::formatting::handle_formatting;
    let text = r#"<?xml version="1.0"?><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>"#;
    let edits = handle_formatting(text, Language::RdfXml);
    assert!(edits.is_none(), "expected None for RDF/XML formatting in Phase F, got: {edits:?}");
}

#[test]
fn goto_definition_unknown_iri_returns_none() {
    use rdf_lsp::features::goto_definition::handle_goto_definition;
    let text = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
    let location = handle_goto_definition(text, Language::NTriples, Position { line: 0, character: 0 });
    assert!(location.is_none(), "expected None for unresolvable IRI, got: {location:?}");
}
