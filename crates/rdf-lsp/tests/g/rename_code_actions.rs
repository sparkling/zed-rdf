//! Phase G — rename and code-action integration tests.

use lsp_types::{Position, Url};
use rdf_lsp::Language;
use rdf_lsp::rename::{handle_rename, handle_code_actions};

fn ttl_uri() -> Url { Url::parse("file:///test.ttl").unwrap() }
fn sparql_uri() -> Url { Url::parse("file:///query.sparql").unwrap() }

// ---------------------------------------------------------------------------
// Rename — Turtle prefix
// ---------------------------------------------------------------------------

#[test]
fn rename_turtle_prefix_replaces_declaration_and_uses() {
    let text = "@prefix ex: <http://example.org/> .\nex:Foo a ex:Bar .\n";
    let pos = Position { line: 1, character: 0 }; // on 'ex'
    let result = handle_rename(text, Language::Turtle, ttl_uri(), pos, "example");
    assert!(result.is_some(), "expected a WorkspaceEdit");
    let changes = result.unwrap().changes.unwrap();
    let edits = changes.values().next().unwrap();
    // All edits must use the new prefix name
    assert!(edits.iter().all(|e| e.new_text.starts_with("example:")),
        "all edits should use 'example:' but got: {:?}", edits);
    // At least 3 occurrences: declaration + 2 uses
    assert!(edits.len() >= 3, "expected ≥3 edits, got {}", edits.len());
}

#[test]
fn rename_turtle_on_non_prefix_returns_none() {
    let text = "# no prefix\n<http://example.org/s> <http://example.org/p> \"o\" .\n";
    let pos = Position { line: 1, character: 1 }; // inside IRI, not a prefix
    let result = handle_rename(text, Language::Turtle, ttl_uri(), pos, "x");
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Rename — SPARQL variable
// ---------------------------------------------------------------------------

#[test]
fn rename_sparql_variable_replaces_all_occurrences() {
    let text = "SELECT ?x WHERE { ?x ?p ?o . }\n";
    let pos = Position { line: 0, character: 7 }; // on '?x'
    let result = handle_rename(text, Language::Sparql, sparql_uri(), pos, "subject");
    assert!(result.is_some());
    let changes = result.unwrap().changes.unwrap();
    let edits = changes.values().next().unwrap();
    assert!(edits.iter().all(|e| e.new_text.contains("subject")),
        "edits should contain new name: {:?}", edits);
    // Both ?x occurrences should be renamed
    assert_eq!(edits.len(), 2, "expected 2 occurrences of ?x");
}

#[test]
fn rename_rdfxml_returns_none() {
    let result = handle_rename("<rdf:RDF/>", Language::RdfXml, ttl_uri(), Position { line: 0, character: 0 }, "x");
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Code actions — sort prefixes
// ---------------------------------------------------------------------------

#[test]
fn code_action_sort_prefixes_offered_when_prefixes_present() {
    let text = "@prefix z: <http://z.org/> .\n@prefix a: <http://a.org/> .\nz:Foo a a:Bar .\n";
    let uri = ttl_uri();
    let actions = handle_code_actions(text, Language::Turtle, &uri, Position { line: 0, character: 0 });
    assert!(actions.iter().any(|a| a.title.contains("Sort")),
        "expected Sort action, got: {:?}", actions.iter().map(|a| &a.title).collect::<Vec<_>>());
}

#[test]
fn code_action_no_prefixes_no_sort() {
    let text = "<http://example.org/s> <http://example.org/p> \"o\" .\n";
    let uri = ttl_uri();
    let actions = handle_code_actions(text, Language::Turtle, &uri, Position { line: 0, character: 0 });
    assert!(!actions.iter().any(|a| a.title.contains("Sort")));
}

// ---------------------------------------------------------------------------
// Code actions — add missing prefix
// ---------------------------------------------------------------------------

#[test]
fn code_action_add_missing_rdf_prefix() {
    let text = "ex:Foo a rdf:Class .\n"; // rdf prefix used but not declared
    let uri = ttl_uri();
    let pos = Position { line: 0, character: 10 }; // on 'rdf'
    let actions = handle_code_actions(text, Language::Turtle, &uri, pos);
    assert!(actions.iter().any(|a| a.title.contains("missing prefix") && a.title.contains("rdf")),
        "expected add-missing-prefix for rdf, got: {:?}", actions.iter().map(|a| &a.title).collect::<Vec<_>>());
}

#[test]
fn code_action_no_add_missing_when_prefix_declared() {
    let text = "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\nex:Foo a rdf:Class .\n";
    let uri = ttl_uri();
    let pos = Position { line: 1, character: 10 }; // on 'rdf'
    let actions = handle_code_actions(text, Language::Turtle, &uri, pos);
    assert!(!actions.iter().any(|a| a.title.contains("missing prefix") && a.title.contains("rdf")));
}

// ---------------------------------------------------------------------------
// Code actions — extract prefix
// ---------------------------------------------------------------------------

#[test]
fn code_action_extract_prefix_for_known_namespace() {
    let text = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> .\n";
    let uri = ttl_uri();
    let pos = Position { line: 0, character: 0 }; // on '<'
    let actions = handle_code_actions(text, Language::Turtle, &uri, pos);
    assert!(actions.iter().any(|a| a.title.contains("Extract prefix")),
        "expected extract prefix action, got: {:?}", actions.iter().map(|a| &a.title).collect::<Vec<_>>());
}

#[test]
fn code_actions_empty_for_rdfxml() {
    let actions = handle_code_actions("<rdf:RDF/>", Language::RdfXml, &ttl_uri(), Position { line: 0, character: 0 });
    assert!(actions.is_empty());
}

// ---------------------------------------------------------------------------
// Incremental parse cache
// ---------------------------------------------------------------------------

#[test]
fn incremental_cache_skips_reparsing_unchanged_text() {
    use rdf_lsp::incremental::ParseCache;
    use lsp_types::Url;

    let mut cache = ParseCache::new();
    let uri = Url::parse("file:///test.ttl").unwrap();
    let mut parse_calls = 0u32;

    cache.update_full(uri.clone(), "a b c .".to_string(), |_| { parse_calls += 1; vec![] });
    assert_eq!(parse_calls, 1);

    cache.update_incremental(uri.clone(), "a b c .".to_string(), |_| { parse_calls += 1; vec![] });
    assert_eq!(parse_calls, 1, "unchanged text should not trigger re-parse");

    cache.update_incremental(uri.clone(), "x y z .".to_string(), |_| { parse_calls += 1; vec![] });
    assert_eq!(parse_calls, 2, "changed text should trigger re-parse");
}
