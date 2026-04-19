//! `TriG` 1.1 parser implementing [`rdf_diff::Parser`].
//!
//! Grammar reference: W3C `TriG` <https://www.w3.org/TR/trig/>
//!
//! `TriG` extends Turtle with named graph blocks:
//!
//! ```text
//! trigDoc  ::= (directive | block)*
//! block    ::= triplesOrGraph | wrappedGraph | triples2 | directive
//! triplesOrGraph ::= labelOrSubject (wrappedGraph | predicateObjectList '.')
//! wrappedGraph   ::= '{' triplesBlock? '}'
//! triplesBlock   ::= triples ('.' triples?)*
//! ```
//!
//! Key semantic notes:
//! - The default graph is used for triples outside any `{ }` block.
//! - Blank-node labels are document-scoped (shared across default and named graphs).
//! - `@prefix` and `@base` declarations are document-scoped.
//! - Nested blank-node property lists inside named graphs carry the graph name.

use rdf_diff::{Diagnostics, ParseOutcome, Parser};

use crate::{
    lexer::{Token, TokenKind, lex},
    turtle::ParseState,
};

/// Public parser entry-point. Implements [`rdf_diff::Parser`] for `TriG` 1.1.
#[derive(Debug, Default, Clone)]
pub struct TriGParser;

impl Parser for TriGParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let text = match std::str::from_utf8(input) {
            Ok(s) => s,
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![format!("input is not valid UTF-8: {e}")],
                    fatal: true,
                });
            }
        };
        let tokens = match lex(text) {
            Ok(t) => t,
            Err(e) => {
                return Err(Diagnostics {
                    messages: vec![e.to_string()],
                    fatal: true,
                });
            }
        };
        let mut state = ParseState::new();
        state.parser_id = "rdf-trig-shadow";
        let result = parse_trig_doc(&mut state, &tokens);
        match result {
            Ok(()) => {
                let (facts, diag) = state.into_facts_and_build_diag();
                Ok(ParseOutcome {
                    facts,
                    warnings: diag,
                })
            }
            Err(()) => Err(state.diag.build()),
        }
    }

    fn id(&self) -> &'static str {
        "rdf-trig-shadow"
    }
}

/// Parse a `TriG` document.
fn parse_trig_doc(state: &mut ParseState, tokens: &[Token]) -> Result<(), ()> {
    while state.pos < tokens.len() {
        parse_trig_statement(state, tokens)?;
    }
    Ok(())
}

/// Parse a single statement in a `TriG` document.
fn parse_trig_statement(state: &mut ParseState, tokens: &[Token]) -> Result<(), ()> {
    match state_peek(state, tokens) {
        Some(TokenKind::AtPrefix) => {
            state.pos += 1;
            state.parse_prefix_decl(tokens)?;
            state.expect_dot(tokens)?;
        }
        Some(TokenKind::AtBase) => {
            state.pos += 1;
            state.parse_base_decl(tokens)?;
            state.expect_dot(tokens)?;
        }
        Some(TokenKind::SparqlPrefix) => {
            state.pos += 1;
            state.parse_prefix_decl(tokens)?;
        }
        Some(TokenKind::SparqlBase) => {
            state.pos += 1;
            state.parse_base_decl(tokens)?;
        }
        Some(TokenKind::BraceOpen) => {
            // Default graph block: { triples }
            state.pos += 1;
            parse_triples_block(state, tokens, None)?;
            expect_brace_close(state, tokens)?;
        }
        _ => {
            // Could be: named graph block, or default-graph triple
            parse_block(state, tokens)?;
        }
    }
    Ok(())
}

fn state_peek<'t>(state: &ParseState, tokens: &'t [Token]) -> Option<&'t TokenKind> {
    tokens.get(state.pos).map(|t| &t.kind)
}

fn expect_brace_close(state: &mut ParseState, tokens: &[Token]) -> Result<(), ()> {
    match state_peek(state, tokens) {
        Some(TokenKind::BraceClose) => {
            state.pos += 1;
            Ok(())
        }
        other => {
            let off = tokens.get(state.pos).map_or(0, |t| t.offset);
            state.diag.error(format!("expected '}}' at byte {off}, got {other:?}"));
            Err(())
        }
    }
}

/// Parse one `TriG` `block` — either a graph block or a bare triple statement.
fn parse_block(state: &mut ParseState, tokens: &[Token]) -> Result<(), ()> {
    // The next token could be:
    // 1. An IRI / prefixed name that is:
    //    a. A graph name followed by `{` → named graph block
    //    b. A graph name followed by predicateObjectList → triple in default graph
    // 2. A bare triple (with bnode subject etc.)
    //
    // We need one token of lookahead past the subject to distinguish (1a) from (1b).

    // Speculatively determine if the next IRI/name token is followed by `{`
    let graph_ahead = is_graph_block(state, tokens);

    if graph_ahead {
        parse_named_graph_block(state, tokens)
    } else {
        // It's a regular triple statement in default graph
        parse_trig_triples_statement(state, tokens)
    }
}

/// Returns true if the upcoming sequence looks like `graphName '{'`.
fn is_graph_block(state: &ParseState, tokens: &[Token]) -> bool {
    // We look at token[pos] and token[pos+1].
    // If token[pos] is an IRI/prefixed-name/bnode, and token[pos+1] is `{`,
    // then this is a named-graph block.
    let subject_len = token_term_len(state, tokens);
    if subject_len == 0 {
        return false;
    }
    matches!(tokens.get(state.pos + subject_len).map(|t| &t.kind), Some(TokenKind::BraceOpen))
}

/// Returns the number of tokens that form a subject/graph-name term at `state.pos`.
fn token_term_len(state: &ParseState, tokens: &[Token]) -> usize {
    match tokens.get(state.pos).map(|t| &t.kind) {
        Some(TokenKind::IriRef(_) | TokenKind::PrefixedName { .. } | TokenKind::BNodeLabel(_)) => 1,
        _ => 0,
    }
}

/// Parse `graphName '{' triples? '}'`.
fn parse_named_graph_block(state: &mut ParseState, tokens: &[Token]) -> Result<(), ()> {
    let off = tokens.get(state.pos).map_or(0, |t| t.offset);
    let graph_name = match tokens.get(state.pos).map(|t| &t.kind) {
        Some(TokenKind::IriRef(raw)) => {
            let raw = raw.clone();
            state.pos += 1;
            state.resolve_iri_tok(&raw, off)?
        }
        Some(TokenKind::PrefixedName { prefix, local }) => {
            let (p, l) = (prefix.clone(), local.clone());
            state.pos += 1;
            state.expand_pname_tok(&p, &l, off)?
        }
        Some(TokenKind::BNodeLabel(label)) => {
            let label = label.clone();
            state.pos += 1;
            state.bnodes.named(&label)
        }
        other => {
            state.diag.error(format!("expected graph name at byte {off}, got {other:?}"));
            return Err(());
        }
    };

    // Consume `{`
    match state_peek(state, tokens) {
        Some(TokenKind::BraceOpen) => state.pos += 1,
        other => {
            let off = tokens.get(state.pos).map_or(0, |t| t.offset);
            state.diag.error(format!("expected '{{' after graph name at byte {off}, got {other:?}"));
            return Err(());
        }
    }

    parse_triples_block(state, tokens, Some(&graph_name))?;
    expect_brace_close(state, tokens)?;
    Ok(())
}

/// Parse `triples ('.' triples?)*` inside a graph block.
fn parse_triples_block(state: &mut ParseState, tokens: &[Token], graph: Option<&str>) -> Result<(), ()> {
    while !matches!(state_peek(state, tokens), Some(TokenKind::BraceClose) | None) {
        parse_trig_inner_triple(state, tokens, graph)?;
        if state_peek(state, tokens) == Some(&TokenKind::Dot) {
            state.pos += 1;
        } else {
            break;
        }
    }
    Ok(())
}

/// Parse a triple inside a graph block (subject predicateObjectList).
fn parse_trig_inner_triple(
    state: &mut ParseState,
    tokens: &[Token],
    graph: Option<&str>,
) -> Result<(), ()> {
    let off = tokens.get(state.pos).map_or(0, |t| t.offset);
    // Subject
    let subject = if state_peek(state, tokens) == Some(&TokenKind::BracketOpen) {
        state.pos += 1;
        let bn = state.bnodes.fresh();
        if state_peek(state, tokens) != Some(&TokenKind::BracketClose) {
            state.parse_predicate_object_list(tokens, &bn, graph)?;
        }
        match state_peek(state, tokens) {
            Some(TokenKind::BracketClose) => state.pos += 1,
            other => {
                state.diag.error(format!("expected ']' at byte {off}, got {other:?}"));
                return Err(());
            }
        }
        bn
    } else {
        match tokens.get(state.pos).map(|t| &t.kind) {
            Some(TokenKind::IriRef(raw)) => {
                let raw = raw.clone();
                state.pos += 1;
                state.resolve_iri_tok(&raw, off)?
            }
            Some(TokenKind::PrefixedName { prefix, local }) => {
                let (p, l) = (prefix.clone(), local.clone());
                state.pos += 1;
                state.expand_pname_tok(&p, &l, off)?
            }
            Some(TokenKind::BNodeLabel(label)) => {
                let label = label.clone();
                state.pos += 1;
                state.bnodes.named(&label)
            }
            Some(TokenKind::ParenOpen) => {
                state.pos += 1;
                state.parse_collection(tokens, graph)?
            }
            other => {
                state.diag.error(format!("expected subject at byte {off}, got {other:?}"));
                return Err(());
            }
        }
    };

    // predicateObjectList
    if !matches!(
        state_peek(state, tokens),
        Some(TokenKind::Dot | TokenKind::BraceClose) | None
    ) {
        state.parse_predicate_object_list(tokens, &subject, graph)?;
    }
    Ok(())
}

/// Parse a top-level `TriG` statement that turned out to be a bare triple.
fn parse_trig_triples_statement(state: &mut ParseState, tokens: &[Token]) -> Result<(), ()> {
    parse_trig_inner_triple(state, tokens, None)?;
    state.expect_dot(tokens)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse_ok(input: &str) -> rdf_diff::Facts {
        let p = TriGParser;
        match rdf_diff::Parser::parse(&p, input.as_bytes()) {
            Ok(outcome) => outcome.facts,
            Err(diag) => panic!("parse error: {:?}", diag.messages),
        }
    }

    #[test]
    fn default_graph_triple() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p ex:o .
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert!(f.graph.is_none());
    }

    #[test]
    fn named_graph_block() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:g {
    ex:s ex:p ex:o .
}
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        // After canonicalise_term, graph IRI is wrapped in `<>`.
        assert_eq!(f.graph.as_deref(), Some("<http://example.org/g>"));
    }

    #[test]
    fn multiple_named_graphs() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:g1 { ex:s ex:p ex:o1 . }
ex:g2 { ex:s ex:p ex:o2 . }
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 2);
        let graphs: std::collections::BTreeSet<_> =
            facts.set.keys().filter_map(|f| f.graph.as_deref()).collect();
        // After canonicalise_term, graph IRIs are wrapped in `<>`.
        assert!(graphs.contains("<http://example.org/g1>"));
        assert!(graphs.contains("<http://example.org/g2>"));
    }

    #[test]
    fn default_and_named_graph() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p ex:o .
ex:g { ex:s ex:q ex:r . }
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn default_graph_block() {
        let trig = r#"
@prefix ex: <http://example.org/> .
{ ex:s ex:p ex:o . }
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert!(f.graph.is_none());
    }

    #[test]
    fn bnode_scoped_across_graphs() {
        // BNode labels are document-scoped in TriG, shared between graphs.
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:g1 { ex:s ex:p _:x . }
ex:g2 { ex:s ex:q _:x . }
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 2);
        let objs: Vec<_> = facts.set.keys().map(|f| f.object.clone()).collect();
        // Both should share the same canonical bnode
        assert_eq!(objs[0], objs[1], "BNode _:x should map to same canonical node across graphs");
    }

    #[test]
    fn prefix_in_named_graph() {
        let trig = r#"
@prefix ex: <http://example.org/> .
<http://example.org/graph> {
    ex:subject a ex:Type .
}
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        // After canonicalise_term, bare absolute IRIs are wrapped in `<>`.
        assert_eq!(f.predicate, "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>");
    }

    #[test]
    fn empty_graph_block() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:g { }
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 0);
    }

    #[test]
    fn collection_in_named_graph() {
        let trig = r#"
@prefix ex: <http://example.org/> .
ex:g { ex:s ex:p (ex:a ex:b) . }
"#;
        let facts = parse_ok(trig);
        // 5 facts: 1 for ex:s ex:p head, plus 4 for the list (2 elements × 2 arcs)
        assert_eq!(facts.set.len(), 5);
        // All facts should be in the named graph (canonicalized with `<>`)
        for f in facts.set.keys() {
            assert_eq!(f.graph.as_deref(), Some("<http://example.org/g>"));
        }
    }

    #[test]
    fn sparql_style_directives_trig() {
        let trig = r#"
BASE <http://example.org/>
PREFIX ex: <http://example.org/>
ex:g { <foo> ex:bar <baz> . }
"#;
        let facts = parse_ok(trig);
        assert_eq!(facts.set.len(), 1);
    }
}
