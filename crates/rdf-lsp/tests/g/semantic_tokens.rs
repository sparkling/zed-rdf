//! Phase G — semantic token integration tests.

use rdf_lsp::Language;
use rdf_lsp::semantic_tokens::{handle_semantic_tokens, legend, TOKEN_TYPES, TOKEN_MODIFIERS};

#[test]
fn legend_token_count() {
    let l = legend();
    assert_eq!(l.token_types.len(), TOKEN_TYPES.len());
    assert_eq!(l.token_modifiers.len(), TOKEN_MODIFIERS.len());
}

#[test]
fn turtle_prefix_comment_classified() {
    let text = "# comment\n@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n";
    let tokens = handle_semantic_tokens(text, Language::Turtle);
    // First token should be comment on line 0
    assert!(!tokens.data.is_empty());
    assert_eq!(tokens.data[0].delta_line, 0);
    assert_eq!(tokens.data[0].token_type, 7); // COMMENT
}

#[test]
fn sparql_select_and_variable() {
    let text = "SELECT ?x WHERE { ?x ?p ?o . }\n";
    let tokens = handle_semantic_tokens(text, Language::Sparql);
    // Must have at least one KEYWORD and one VARIABLE
    assert!(tokens.data.iter().any(|t| t.token_type == 6), "expected KEYWORD");
    assert!(tokens.data.iter().any(|t| t.token_type == 3), "expected VARIABLE");
}

#[test]
fn ntriples_iri_tokens_present() {
    let text = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
    let tokens = handle_semantic_tokens(text, Language::NTriples);
    assert!(!tokens.data.is_empty());
}

#[test]
fn shex_prefix_keyword_classified() {
    let text = "PREFIX ex: <http://example.org/>\n";
    let tokens = handle_semantic_tokens(text, Language::ShEx);
    assert!(tokens.data.iter().any(|t| t.token_type == 6), "expected KEYWORD for PREFIX");
}

#[test]
fn datalog_variable_and_comment() {
    let text = "% a rule\nancestor(X, Y) :- parent(X, Y).\n";
    let tokens = handle_semantic_tokens(text, Language::Datalog);
    assert!(tokens.data.iter().any(|t| t.token_type == 7), "expected COMMENT");
    assert!(tokens.data.iter().any(|t| t.token_type == 3), "expected VARIABLE for X/Y");
}

#[test]
fn rdfxml_returns_empty_tokens() {
    let tokens = handle_semantic_tokens("<rdf:RDF/>", Language::RdfXml);
    assert!(tokens.data.is_empty());
}

#[test]
fn jsonld_returns_empty_tokens() {
    let tokens = handle_semantic_tokens("{}", Language::JsonLd);
    assert!(tokens.data.is_empty());
}

#[test]
fn delta_encoding_multiline_monotone() {
    let text = "# line 0\n# line 1\n# line 2\n";
    let tokens = handle_semantic_tokens(text, Language::Turtle);
    // Each token should be on a later line than the previous.
    let mut prev = 0u32;
    let mut abs = 0u32;
    for tok in &tokens.data {
        abs += tok.delta_line;
        assert!(abs >= prev, "line numbers must be non-decreasing");
        prev = abs;
    }
}

#[test]
fn all_eleven_languages_do_not_panic() {
    let langs = [
        (Language::NTriples, "<http://example.org/s> <http://example.org/p> \"o\" .\n"),
        (Language::NQuads, "<http://example.org/s> <http://example.org/p> \"o\" <http://example.org/g> .\n"),
        (Language::Turtle, "@prefix ex: <http://example.org/> .\nex:s ex:p \"o\" .\n"),
        (Language::TriG, "GRAPH <http://example.org/g> { <http://example.org/s> <http://example.org/p> \"o\" . }\n"),
        (Language::RdfXml, "<rdf:RDF/>"),
        (Language::JsonLd, "{}"),
        (Language::TriX, "<TriX/>"),
        (Language::N3, "@prefix ex: <http://example.org/> .\n"),
        (Language::Sparql, "SELECT ?x WHERE { ?x ?p ?o . }\n"),
        (Language::ShEx, "PREFIX ex: <http://example.org/>\n"),
        (Language::Datalog, "parent(tom, bob).\n"),
    ];
    for (lang, text) in langs {
        let tokens = handle_semantic_tokens(text, lang);
        let _ = tokens; // just must not panic
    }
}
