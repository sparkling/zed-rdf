//! Turtle 1.1 parser implementing [`rdf_diff::Parser`].
//!
//! Grammar reference: W3C Turtle 1.1 <https://www.w3.org/TR/turtle/>
//!
//! Produces canonical [`rdf_diff::Facts`] where:
//! - Every IRI is absolute (resolved via `@base` chain).
//! - Prefixed names are fully expanded.
//! - Blank-node labels are re-mapped to `_:b0`, `_:b1`, …
//! - Numeric literals carry XSD datatype IRIs.
//! - Language tags are lowercased.
//!
//! # Blank-node scoping note
//! Per W3C Turtle 1.1 §6: blank-node labels are local to the document.
//! `@prefix` re-declarations do NOT reset the blank-node label namespace.
//! This is a known spec ambiguity; coordinate with `v1-specpins` if
//! behaviours diverge across implementations.

use std::collections::{BTreeMap, HashMap};

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

use crate::{
    bnode::BNodeAllocator,
    diagnostics::DiagnosticsBuilder,
    iri::{expand_pname, resolve_iri},
    lexer::{Token, TokenKind, lex},
    literal::{
        RDF, boolean_literal, decimal_literal, double_literal, integer_literal, lang_literal,
        string_literal, typed_literal,
    },
    unescape::unescape_string,
};

/// `rdf:type` IRI.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `rdf:first` IRI.
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
/// `rdf:rest` IRI.
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";

/// Public parser entry-point.  Implements [`rdf_diff::Parser`] for Turtle 1.1.
#[derive(Debug, Default, Clone)]
pub struct TurtleParser;

impl Parser for TurtleParser {
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
        let result = state.parse_turtle(&tokens);
        match result {
            Ok(()) => {
                let facts = state.into_facts_and_build_diag();
                Ok(ParseOutcome {
                    facts: facts.0,
                    warnings: facts.1,
                })
            }
            Err(()) => Err(state.diag.build()),
        }
    }

    fn id(&self) -> &'static str {
        "rdf-turtle-shadow"
    }
}

/// Internal parse state shared between Turtle and `TriG` parsers.
pub(crate) struct ParseState {
    /// Prefix map: prefix → IRI prefix.
    pub(crate) prefixes: HashMap<String, String>,
    /// Current base IRI.
    pub(crate) base: Option<String>,
    /// Accumulated raw facts (subject, predicate, object, graph).
    pub(crate) raw_facts: Vec<(Fact, FactProvenance)>,
    /// Blank-node allocator — document-scoped.
    pub(crate) bnodes: BNodeAllocator,
    /// Diagnostic accumulator.
    pub(crate) diag: DiagnosticsBuilder,
    /// Current token index.
    pub(crate) pos: usize,
    /// Parser id used in provenance.
    pub(crate) parser_id: &'static str,
}

impl ParseState {
    pub(crate) fn new() -> Self {
        Self {
            prefixes: HashMap::new(),
            base: None,
            raw_facts: Vec::new(),
            bnodes: BNodeAllocator::new(),
            diag: DiagnosticsBuilder::new(),
            pos: 0,
            parser_id: "rdf-turtle-shadow",
        }
    }

    /// Consume state, returning `(Facts, Diagnostics)` without partial-move issues.
    pub(crate) fn into_facts_and_build_diag(self) -> (Facts, rdf_diff::Diagnostics) {
        let prefixes: BTreeMap<String, String> = self
            .prefixes
            .into_iter()
            .map(|(k, v)| (format!("{k}:"), v))
            .collect();
        let facts = Facts::canonicalise(self.raw_facts, prefixes);
        let diag = self.diag.build();
        (facts, diag)
    }

    /// Parse a complete Turtle document.
    pub(crate) fn parse_turtle(&mut self, tokens: &[Token]) -> Result<(), ()> {
        while self.pos < tokens.len() {
            self.parse_statement(tokens)?;
        }
        Ok(())
    }

    fn peek<'t>(&self, tokens: &'t [Token]) -> Option<&'t TokenKind> {
        tokens.get(self.pos).map(|t| &t.kind)
    }

    fn peek_offset(&self, tokens: &[Token]) -> usize {
        tokens.get(self.pos).map_or(0, |t| t.offset)
    }

    fn advance<'t>(&mut self, tokens: &'t [Token]) -> Option<&'t TokenKind> {
        let tok = tokens.get(self.pos).map(|t| &t.kind)?;
        self.pos += 1;
        Some(tok)
    }

    fn expect(&mut self, tokens: &[Token], expected: &TokenKind) -> Result<(), ()> {
        match self.peek(tokens) {
            Some(k) if k == expected => {
                self.pos += 1;
                Ok(())
            }
            other => {
                let off = self.peek_offset(tokens);
                self.diag.error(format!("expected {expected:?}, got {other:?} at byte {off}"));
                Err(())
            }
        }
    }

    pub(crate) fn expect_dot(&mut self, tokens: &[Token]) -> Result<(), ()> {
        self.expect(tokens, &TokenKind::Dot)
    }

    fn parse_statement(&mut self, tokens: &[Token]) -> Result<(), ()> {
        match self.peek(tokens) {
            Some(TokenKind::AtPrefix) => {
                self.pos += 1;
                self.parse_prefix_decl(tokens)?;
                self.expect_dot(tokens)?;
            }
            Some(TokenKind::AtBase) => {
                self.pos += 1;
                self.parse_base_decl(tokens)?;
                self.expect_dot(tokens)?;
            }
            Some(TokenKind::SparqlPrefix) => {
                self.pos += 1;
                self.parse_prefix_decl(tokens)?;
                // SPARQL-style: no trailing dot
            }
            Some(TokenKind::SparqlBase) => {
                self.pos += 1;
                self.parse_base_decl(tokens)?;
                // No trailing dot
            }
            _ => {
                self.parse_triples(tokens, None)?;
                self.expect_dot(tokens)?;
            }
        }
        Ok(())
    }

    pub(crate) fn parse_prefix_decl(&mut self, tokens: &[Token]) -> Result<(), ()> {
        // Expect: PNAME_NS IRI_REF
        let off = self.peek_offset(tokens);
        let prefix = match self.advance(tokens) {
            Some(TokenKind::PrefixedName { prefix, .. }) => prefix.clone(),
            other => {
                self.diag.error(format!("expected prefix name after @prefix at byte {off}, got {other:?}"));
                return Err(());
            }
        };
        let iri_raw = match self.advance(tokens) {
            Some(TokenKind::IriRef(raw)) => raw.clone(),
            other => {
                self.diag.error(format!("expected IRI after @prefix at byte {off}, got {other:?}"));
                return Err(());
            }
        };
        let iri = match resolve_iri(&iri_raw, self.base.as_deref()) {
            Ok(i) => i,
            Err(e) => {
                self.diag.error(format!("@prefix IRI error at byte {off}: {e}"));
                return Err(());
            }
        };
        self.prefixes.insert(prefix, iri);
        Ok(())
    }

    pub(crate) fn parse_base_decl(&mut self, tokens: &[Token]) -> Result<(), ()> {
        let off = self.peek_offset(tokens);
        let iri_raw = match self.advance(tokens) {
            Some(TokenKind::IriRef(raw)) => raw.clone(),
            other => {
                self.diag.error(format!("expected IRI after @base at byte {off}, got {other:?}"));
                return Err(());
            }
        };
        let iri = match resolve_iri(&iri_raw, self.base.as_deref()) {
            Ok(i) => i,
            Err(e) => {
                self.diag.error(format!("@base IRI error at byte {off}: {e}"));
                return Err(());
            }
        };
        self.base = Some(iri);
        Ok(())
    }

    /// Parse a triples statement: subject predicateObjectList
    fn parse_triples(&mut self, tokens: &[Token], graph: Option<&str>) -> Result<(), ()> {
        let off = self.peek_offset(tokens);
        // Subject may be a blank-node property list  `[ ... ]`
        let subject = if self.peek(tokens) == Some(&TokenKind::BracketOpen) {
            self.pos += 1;
            let bnode = self.bnodes.fresh();
            if self.peek(tokens) != Some(&TokenKind::BracketClose) {
                self.parse_predicate_object_list(tokens, &bnode, graph)?;
            }
            self.expect(tokens, &TokenKind::BracketClose)?;
            bnode
        } else {
            match self.parse_subject(tokens) {
                Ok(s) => s,
                Err(e) => {
                    self.diag.error(format!("invalid subject at byte {off}: {e}"));
                    return Err(());
                }
            }
        };

        // predicateObjectList is optional when subject was [ ... ] alone
        if !matches!(self.peek(tokens), Some(TokenKind::Dot) | None) {
            self.parse_predicate_object_list(tokens, &subject, graph)?;
        }
        Ok(())
    }

    fn parse_subject(&mut self, tokens: &[Token]) -> Result<String, String> {
        let off = self.peek_offset(tokens);
        match self.advance(tokens) {
            Some(TokenKind::IriRef(raw)) => {
                let raw = raw.clone();
                resolve_iri(&raw, self.base.as_deref()).map_err(|e| e.to_string())
            }
            Some(TokenKind::PrefixedName { prefix, local }) => {
                let (p, l) = (prefix.clone(), local.clone());
                expand_pname(&p, &l, &self.prefixes).map_err(|e| e.to_string())
            }
            Some(TokenKind::BNodeLabel(label)) => {
                let label = label.clone();
                Ok(self.bnodes.named(&label))
            }
            Some(TokenKind::BracketOpen) => {
                // Anonymous bnode as subject
                let bn = self.bnodes.fresh();
                if self.peek(tokens) != Some(&TokenKind::BracketClose) {
                    self.parse_predicate_object_list(tokens, &bn, None)
                        .map_err(|()| "predicate-object list error".to_owned())?;
                }
                self.expect(tokens, &TokenKind::BracketClose)
                    .map_err(|()| "expected ']'".to_owned())?;
                Ok(bn)
            }
            Some(TokenKind::ParenOpen) => {
                self.parse_collection(tokens, None).map_err(|()| "collection error".to_owned())
            }
            other => Err(format!("unexpected subject token {other:?} at byte {off}")),
        }
    }

    pub(crate) fn parse_predicate_object_list(
        &mut self,
        tokens: &[Token],
        subject: &str,
        graph: Option<&str>,
    ) -> Result<(), ()> {
        loop {
            let predicate = self.parse_verb(tokens)?;
            self.parse_object_list(tokens, subject, &predicate, graph)?;

            // ';' separates additional predicate-object pairs
            if self.peek(tokens) == Some(&TokenKind::Semicolon) {
                self.pos += 1;
                // Multiple semicolons are allowed
                while self.peek(tokens) == Some(&TokenKind::Semicolon) {
                    self.pos += 1;
                }
                // After semicolons, if we see a terminator, stop
                if matches!(
                    self.peek(tokens),
                    Some(TokenKind::Dot | TokenKind::BracketClose | TokenKind::BraceClose) | None
                ) {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    fn parse_verb(&mut self, tokens: &[Token]) -> Result<String, ()> {
        let off = self.peek_offset(tokens);
        match self.advance(tokens) {
            Some(TokenKind::AKeyword) => Ok(RDF_TYPE.to_owned()),
            Some(TokenKind::IriRef(raw)) => {
                let raw = raw.clone();
                resolve_iri(&raw, self.base.as_deref()).map_err(|e| {
                    self.diag.error(format!("predicate IRI error at byte {off}: {e}"));
                })
            }
            Some(TokenKind::PrefixedName { prefix, local }) => {
                let (p, l) = (prefix.clone(), local.clone());
                expand_pname(&p, &l, &self.prefixes).map_err(|e| {
                    self.diag.error(format!("predicate expand error at byte {off}: {e}"));
                })
            }
            other => {
                self.diag.error(format!("expected predicate at byte {off}, got {other:?}"));
                Err(())
            }
        }
    }

    fn parse_object_list(
        &mut self,
        tokens: &[Token],
        subject: &str,
        predicate: &str,
        graph: Option<&str>,
    ) -> Result<(), ()> {
        loop {
            let object = self.parse_object(tokens, graph)?;
            let offset = tokens.get(self.pos.saturating_sub(1)).map(|t| t.offset);
            self.emit_fact(subject, predicate, &object, graph, offset);
            if self.peek(tokens) == Some(&TokenKind::Comma) {
                self.pos += 1;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn parse_object(&mut self, tokens: &[Token], graph: Option<&str>) -> Result<String, ()> {
        let off = self.peek_offset(tokens);
        match self.peek(tokens) {
            Some(TokenKind::ParenOpen) => {
                self.pos += 1;
                self.parse_collection(tokens, graph)
            }
            Some(TokenKind::BracketOpen) => {
                self.pos += 1;
                let bn = self.bnodes.fresh();
                if self.peek(tokens) != Some(&TokenKind::BracketClose) {
                    self.parse_predicate_object_list(tokens, &bn, graph)?;
                }
                self.expect(tokens, &TokenKind::BracketClose)?;
                Ok(bn)
            }
            _ => self.parse_literal_or_iri(tokens, off),
        }
    }

    fn parse_literal_or_iri(&mut self, tokens: &[Token], off: usize) -> Result<String, ()> {
        match self.advance(tokens) {
            Some(TokenKind::IriRef(raw)) => {
                let raw = raw.clone();
                resolve_iri(&raw, self.base.as_deref()).map_err(|e| {
                    self.diag.error(format!("object IRI error at byte {off}: {e}"));
                })
            }
            Some(TokenKind::PrefixedName { prefix, local }) => {
                let (p, l) = (prefix.clone(), local.clone());
                expand_pname(&p, &l, &self.prefixes).map_err(|e| {
                    self.diag.error(format!("object expand error at byte {off}: {e}"));
                })
            }
            Some(TokenKind::BNodeLabel(label)) => {
                let label = label.clone();
                Ok(self.bnodes.named(&label))
            }
            Some(TokenKind::StringLiteral { raw, .. }) => {
                let raw = raw.clone();
                let lexical = unescape_string(&raw).map_err(|e| {
                    self.diag.error(format!("string unescape error at byte {off}: {e}"));
                })?;
                self.parse_literal_suffix(tokens, &lexical, off)
            }
            Some(TokenKind::IntegerLiteral(raw)) => {
                let raw = raw.clone();
                Ok(integer_literal(&raw))
            }
            Some(TokenKind::DecimalLiteral(raw)) => {
                let raw = raw.clone();
                Ok(decimal_literal(&raw))
            }
            Some(TokenKind::DoubleLiteral(raw)) => {
                let raw = raw.clone();
                Ok(double_literal(&raw))
            }
            Some(TokenKind::BooleanLiteral(b)) => {
                let b = *b;
                Ok(boolean_literal(b))
            }
            other => {
                self.diag.error(format!("unexpected object token {other:?} at byte {off}"));
                Err(())
            }
        }
    }

    fn parse_literal_suffix(
        &mut self,
        tokens: &[Token],
        lexical: &str,
        _off: usize,
    ) -> Result<String, ()> {
        match self.peek(tokens) {
            Some(TokenKind::DataTypeTag) => {
                self.pos += 1;
                let iri = self.parse_datatype_iri(tokens)?;
                Ok(typed_literal(lexical, &iri))
            }
            Some(TokenKind::LangTag(tag)) => {
                let tag = tag.clone();
                self.pos += 1;
                Ok(lang_literal(lexical, &tag))
            }
            _ => Ok(string_literal(lexical)),
        }
    }

    fn parse_datatype_iri(&mut self, tokens: &[Token]) -> Result<String, ()> {
        let off = self.peek_offset(tokens);
        match self.advance(tokens) {
            Some(TokenKind::IriRef(raw)) => {
                let raw = raw.clone();
                resolve_iri(&raw, self.base.as_deref()).map_err(|e| {
                    self.diag.error(format!("datatype IRI error at byte {off}: {e}"));
                })
            }
            Some(TokenKind::PrefixedName { prefix, local }) => {
                let (p, l) = (prefix.clone(), local.clone());
                expand_pname(&p, &l, &self.prefixes).map_err(|e| {
                    self.diag.error(format!("datatype expand error at byte {off}: {e}"));
                })
            }
            other => {
                self.diag.error(format!("expected datatype IRI at byte {off}, got {other:?}"));
                Err(())
            }
        }
    }

    /// Parse `( object* )` → rdf:first/rdf:rest chain.
    ///
    /// The opening `(` must already have been consumed.  Returns the head
    /// node IRI (or `rdf:nil` for an empty collection).
    pub(crate) fn parse_collection(&mut self, tokens: &[Token], graph: Option<&str>) -> Result<String, ()> {
        if self.peek(tokens) == Some(&TokenKind::ParenClose) {
            self.pos += 1;
            return Ok(format!("{RDF}nil"));
        }
        let head = self.bnodes.fresh();
        let mut current = head.clone();
        loop {
            let obj = self.parse_object(tokens, graph)?;
            let off = tokens.get(self.pos.saturating_sub(1)).map(|t| t.offset);
            self.emit_fact(&current, RDF_FIRST, &obj, graph, off);

            if self.peek(tokens) == Some(&TokenKind::ParenClose) {
                self.pos += 1;
                self.emit_fact(&current, RDF_REST, &format!("{RDF}nil"), graph, off);
                break;
            }
            let next = self.bnodes.fresh();
            self.emit_fact(&current, RDF_REST, &next, graph, off);
            current = next;
        }
        Ok(head)
    }

    pub(crate) fn emit_fact(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        graph: Option<&str>,
        offset: Option<usize>,
    ) {
        let fact = Fact {
            subject: subject.to_owned(),
            predicate: predicate.to_owned(),
            object: object.to_owned(),
            graph: graph.map(ToOwned::to_owned),
        };
        let prov = FactProvenance {
            offset,
            parser: self.parser_id.to_owned(),
        };
        self.raw_facts.push((fact, prov));
    }

    pub(crate) fn resolve_iri_tok(&mut self, raw: &str, off: usize) -> Result<String, ()> {
        resolve_iri(raw, self.base.as_deref()).map_err(|e| {
            self.diag.error(format!("IRI error at byte {off}: {e}"));
        })
    }

    pub(crate) fn expand_pname_tok(&mut self, prefix: &str, local: &str, off: usize) -> Result<String, ()> {
        expand_pname(prefix, local, &self.prefixes).map_err(|e| {
            self.diag.error(format!("prefix expand error at byte {off}: {e}"));
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse_ok(input: &str) -> Facts {
        let p = TurtleParser;
        match rdf_diff::Parser::parse(&p, input.as_bytes()) {
            Ok(outcome) => outcome.facts,
            Err(diag) => panic!("parse error: {:?}", diag.messages),
        }
    }

    fn fact_objects(facts: &Facts) -> Vec<String> {
        let mut v: Vec<String> = facts.set.keys().map(|f| f.object.clone()).collect();
        v.sort();
        v
    }

    fn fact_triples(facts: &Facts) -> Vec<(String, String, String)> {
        facts
            .set
            .keys()
            .map(|f| (f.subject.clone(), f.predicate.clone(), f.object.clone()))
            .collect()
    }

    #[test]
    fn simple_triple() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:foo ex:bar ex:baz .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        // After canonicalise_term, bare absolute IRIs are wrapped in `<>`.
        assert_eq!(f.subject, "<http://example.org/foo>");
        assert_eq!(f.predicate, "<http://example.org/bar>");
        assert_eq!(f.object, "<http://example.org/baz>");
    }

    #[test]
    fn base_resolution() {
        let ttl = r#"
@base <http://example.org/> .
<foo> <bar> <baz> .
"#;
        let facts = parse_ok(ttl);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://example.org/foo>");
        assert_eq!(f.object, "<http://example.org/baz>");
    }

    #[test]
    fn sparql_prefix_base() {
        let ttl = r#"
BASE <http://example.org/>
PREFIX ex: <http://example.org/>
ex:foo <bar> ex:baz .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 1);
    }

    #[test]
    fn string_literal_types() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p "hello" .
ex:s ex:p "bye"@en .
ex:s ex:p "42"^^<http://www.w3.org/2001/XMLSchema#integer> .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 3);
        let objs = fact_objects(&facts);
        assert!(objs.iter().any(|o| o.contains("xsd:string") || o.contains("XMLSchema#string")));
        assert!(objs.iter().any(|o| o.ends_with("@en")));
    }

    #[test]
    fn long_string_literal() {
        let ttl = "
@prefix ex: <http://example.org/> .
ex:s ex:p \"\"\"line1\nline2\"\"\" .
";
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("line1") && f.object.contains("line2"), "object was: {}", f.object);
    }

    #[test]
    fn numeric_literals() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:int 42 .
ex:s ex:dec 3.14 .
ex:s ex:dbl 1.0e10 .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 3);
        let objs = fact_objects(&facts);
        assert!(objs.iter().any(|o| o.contains("integer")));
        assert!(objs.iter().any(|o| o.contains("decimal")));
        assert!(objs.iter().any(|o| o.contains("double")));
    }

    #[test]
    fn boolean_literals() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p true .
ex:s ex:q false .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 2);
        let objs = fact_objects(&facts);
        assert!(objs.iter().any(|o| o.contains("\"true\"")));
        assert!(objs.iter().any(|o| o.contains("\"false\"")));
    }

    #[test]
    fn a_shorthand() {
        let ttl = r#"
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix ex: <http://example.org/> .
ex:s a ex:Class .
"#;
        let facts = parse_ok(ttl);
        let f = facts.set.keys().next().unwrap();
        // After canonicalise_term, bare absolute IRIs are wrapped in `<>`.
        assert_eq!(f.predicate, format!("<{RDF_TYPE}>"));
    }

    #[test]
    fn blank_node_property_list() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p [ ex:q ex:r ] .
"#;
        let facts = parse_ok(ttl);
        // 2 facts: ex:s ex:p _:b0 AND _:b0 ex:q ex:r
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn collection() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
ex:s ex:p ( ex:a ex:b ) .
"#;
        let facts = parse_ok(ttl);
        // (ex:a ex:b) → head bnode
        //   head rdf:first ex:a
        //   head rdf:rest bnode2
        //   bnode2 rdf:first ex:b
        //   bnode2 rdf:rest rdf:nil
        // + ex:s ex:p head
        // Total: 5 facts
        assert_eq!(facts.set.len(), 5);
    }

    #[test]
    fn empty_collection() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p () .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        // After canonicalise_term wraps the IRI in `<>`.
        assert_eq!(f.object, format!("<{RDF}nil>"));
    }

    #[test]
    fn bnode_not_reset_by_prefix_redecl() {
        // BNode labels are document-scoped — @prefix redeclaration does NOT
        // reset the BNode namespace. _:b should refer to the same bnode
        // across prefix redeclarations.
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p _:b .
@prefix ex: <http://other.org/> .
ex:s ex:q _:b .
"#;
        let facts = parse_ok(ttl);
        // Two facts; the bnode label _:b maps to the same canonical bnode
        assert_eq!(facts.set.len(), 2);
        let triples = fact_triples(&facts);
        // Both objects should be the same canonical bnode
        let obj0 = &triples[0].2;
        let obj1 = &triples[1].2;
        assert_eq!(obj0, obj1, "BNode label should be stable across @prefix redecls");
    }

    #[test]
    fn semicolon_predicate_list() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:a ex:x ; ex:b ex:y .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn comma_object_list() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p ex:a, ex:b, ex:c .
"#;
        let facts = parse_ok(ttl);
        assert_eq!(facts.set.len(), 3);
    }

    #[test]
    fn prefix_redefinition_changes_expansion() {
        let ttl = r#"
@prefix ex: <http://example.org/> .
ex:s ex:p ex:o .
@prefix ex: <http://other.org/> .
ex:s ex:p ex:o .
"#;
        let facts = parse_ok(ttl);
        // ex:s and ex:o expand differently after the redecl —
        // first triple: sub=http://example.org/s, obj=http://example.org/o
        // second triple: sub=http://other.org/s, obj=http://other.org/o
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn invalid_utf8_rejected() {
        let p = TurtleParser;
        let bad = b"\xFF\xFE";
        assert!(p.parse(bad).is_err());
    }
}
