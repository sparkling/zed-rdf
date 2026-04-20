//! Recursive-descent grammar for N3 (Notation3).
//!
//! Covers the full Turtle 1.1 grammar plus the N3 extensions:
//! - `@keywords` directive — bare-word keyword mode.
//! - `{ … }` quoted formulas — emitted as named graphs with fresh bnode IDs.
//! - `=>` logical implication — emitted as a triple with
//!   predicate `<http://www.w3.org/2000/10/swap/log#implies>`.
//! - `is P of O` reverse property path — emits `(O, P, current_subject)`.
//! - `@forAll` / `@forSome` — parsed and skipped; a warning is appended.
//!
//! W3C N3 Team Submission: <https://www.w3.org/TeamSubmission/n3/>

use std::collections::BTreeMap;

use rdf_diff::{Fact, FactProvenance};

use crate::iri::{is_absolute, resolve};
use crate::lexer::{Lexer, NumKind, Spanned, Tok};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const LOG_IMPLIES: &str = "http://www.w3.org/2000/10/swap/log#implies";

/// Syntactic category of a parsed subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubjectKind {
    Iri,
    BNode,
    BlankNodePropertyList,
    Collection,
    Literal,
    Formula,
}

/// N3 grammar parser.
pub(crate) struct N3GrammarParser<'a> {
    lex: Lexer<'a>,
    parser_id: &'static str,
    base: Option<String>,
    prefixes: BTreeMap<String, String>,
    bnode_index: BTreeMap<String, String>,
    bnode_counter: usize,
    out: Vec<(Fact, FactProvenance)>,
    warnings: Vec<String>,
}

impl<'a> N3GrammarParser<'a> {
    pub(crate) fn new(src: &'a [u8], parser_id: &'static str) -> Self {
        Self {
            lex: Lexer::new(src),
            parser_id,
            base: None,
            prefixes: BTreeMap::new(),
            bnode_index: BTreeMap::new(),
            bnode_counter: 0,
            out: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub(crate) fn finish(
        self,
    ) -> (
        Vec<(Fact, FactProvenance)>,
        BTreeMap<String, String>,
        Vec<String>,
    ) {
        (self.out, self.prefixes, self.warnings)
    }

    /// Drive the top-level production `statement*`.
    pub(crate) fn parse_document(&mut self) -> Result<(), String> {
        loop {
            let Some(peek) = self.lex.peek()? else {
                return Ok(());
            };
            match &peek.tok {
                Tok::DirPrefix => self.directive_prefix(false)?,
                Tok::DirBase => self.directive_base(false)?,
                Tok::SparqlPrefix => self.directive_prefix(true)?,
                Tok::SparqlBase => self.directive_base(true)?,
                Tok::DirKeywords => self.directive_keywords()?,
                Tok::DirForAll => self.directive_forall_forsome("@forAll")?,
                Tok::DirForSome => self.directive_forall_forsome("@forSome")?,
                _ => self.triple_stmt(None)?,
            }
        }
    }

    // -- Directives -------------------------------------------------------

    fn directive_prefix(&mut self, sparql_style: bool) -> Result<(), String> {
        let kw = self.lex.next()?.ok_or_else(|| eof("prefix directive"))?;
        let prefix_tok = self
            .lex
            .next()?
            .ok_or_else(|| eof("prefix name after @prefix/PREFIX"))?;
        let prefix_name = match prefix_tok.tok {
            Tok::Pname { prefix, local } if local.is_empty() => prefix,
            _ => {
                return Err(syntax(
                    prefix_tok.start,
                    "expected 'prefix:' name after @prefix",
                ))
            }
        };
        let iri_tok = self
            .lex
            .next()?
            .ok_or_else(|| eof("IRI after @prefix/PREFIX"))?;
        let iri = match iri_tok.tok {
            Tok::IriRef(s) => self.resolve_iri(&s, iri_tok.start)?,
            _ => return Err(syntax(iri_tok.start, "expected <IRI> after @prefix")),
        };
        self.prefixes.insert(prefix_name, iri);
        if sparql_style {
            self.reject_dot(kw.start)?;
        } else {
            self.expect_dot(kw.start)?;
        }
        Ok(())
    }

    fn directive_base(&mut self, sparql_style: bool) -> Result<(), String> {
        let kw = self.lex.next()?.ok_or_else(|| eof("base directive"))?;
        let iri_tok = self
            .lex
            .next()?
            .ok_or_else(|| eof("IRI after @base/BASE"))?;
        let iri = match iri_tok.tok {
            Tok::IriRef(s) => self.resolve_iri(&s, iri_tok.start)?,
            _ => return Err(syntax(iri_tok.start, "expected <IRI> after @base")),
        };
        self.base = Some(iri);
        if sparql_style {
            self.reject_dot(kw.start)?;
        } else {
            self.expect_dot(kw.start)?;
        }
        Ok(())
    }

    /// `@keywords` directive — enables bare-keyword mode.
    ///
    /// Grammar: `@keywords ( bareWord (',' bareWord)* )? '.'`
    ///
    /// After this directive, bare words like `a`, `is`, `of`, `has` are
    /// recognized as N3 keywords without the `@` prefix. We simply set
    /// the `keywords_mode` flag on the lexer and skip the listed words.
    fn directive_keywords(&mut self) -> Result<(), String> {
        let kw_tok = self.lex.next()?.ok_or_else(|| eof("@keywords directive"))?;
        // Consume the list of keywords (comma-separated bare words).
        // They are informational in this mode (we already recognize all of
        // them); we just need to advance past them.
        loop {
            let peek = self.lex.peek()?.ok_or_else(|| eof("@keywords list"))?;
            if matches!(peek.tok, Tok::Dot) {
                break;
            }
            if matches!(peek.tok, Tok::Comma) {
                let _ = self.lex.next()?;
                continue;
            }
            // Should be a bare identifier / known keyword.
            let _ = self.lex.next()?;
        }
        self.lex.keywords_mode = true;
        self.expect_dot(kw_tok.start)?;
        Ok(())
    }

    /// `@forAll` / `@forSome` — parse and skip with a warning.
    ///
    /// Grammar: `(@forAll | @forSome) term (',' term)* '.'`
    fn directive_forall_forsome(&mut self, which: &'static str) -> Result<(), String> {
        let dir_tok = self.lex.next()?.ok_or_else(|| eof(which))?;
        self.warnings.push(format!(
            "N3-INFO-001: {which} at byte {} — quantifier skipped (beyond Phase B scope)",
            dir_tok.start,
        ));
        // Skip until the terminating `.`
        loop {
            let peek = self.lex.peek()?.ok_or_else(|| eof(&format!("'{which}' list")))?;
            if matches!(peek.tok, Tok::Dot) {
                break;
            }
            let _ = self.lex.next()?;
        }
        self.expect_dot(dir_tok.start)?;
        Ok(())
    }

    fn reject_dot(&mut self, anchor: usize) -> Result<(), String> {
        let save = self.lex.offset();
        let peek = self.lex.peek()?;
        if matches!(peek.as_ref().map(|s| &s.tok), Some(Tok::Dot)) {
            let _ = self.lex.next()?;
            return Err(format!(
                "TTL-DIR-001: SPARQL-style PREFIX/BASE directive must not end with '.' at byte {anchor}"
            ));
        }
        self.lex.seek(save);
        Ok(())
    }

    fn expect_dot(&mut self, anchor: usize) -> Result<(), String> {
        match self.lex.next()? {
            Some(Spanned { tok: Tok::Dot, .. }) => Ok(()),
            Some(s) => Err(format!(
                "TTL-DIR-001: directive not terminated with '.' at byte {}",
                s.start
            )),
            None => Err(format!(
                "TTL-DIR-001: directive not terminated with '.' (EOF) at byte {anchor}"
            )),
        }
    }

    // -- Triples -----------------------------------------------------------

    fn triple_stmt(&mut self, outer_graph: Option<&str>) -> Result<(), String> {
        let (subject, subject_kind) = self.parse_subject(outer_graph)?;
        let pol_optional = matches!(
            subject_kind,
            SubjectKind::BlankNodePropertyList | SubjectKind::Collection | SubjectKind::Formula
        );
        if pol_optional {
            let peek = self.lex.peek()?;
            let has_verb = peek.as_ref().map_or(false, |s| self.is_verb_start(&s.tok));
            if has_verb {
                self.parse_predicate_object_list(&subject, outer_graph)?;
            }
        } else {
            self.parse_predicate_object_list(&subject, outer_graph)?;
        }
        let next = self.lex.next()?.ok_or_else(|| eof("'.' after triple"))?;
        if !matches!(next.tok, Tok::Dot) {
            return Err(syntax(next.start, "expected '.' after triple statement"));
        }
        Ok(())
    }

    /// Variant used inside a formula `{ … }` block.
    /// Returns `true` if it consumed the closing `}`.
    fn triple_stmt_in_formula(&mut self, formula_graph: Option<&str>) -> Result<bool, String> {
        let (subject, subject_kind) = self.parse_subject(formula_graph)?;
        let pol_optional = matches!(
            subject_kind,
            SubjectKind::BlankNodePropertyList | SubjectKind::Collection | SubjectKind::Formula
        );
        if pol_optional {
            let peek = self.lex.peek()?;
            let has_verb = peek.as_ref().map_or(false, |s| self.is_verb_start(&s.tok));
            if has_verb {
                self.parse_predicate_object_list(&subject, formula_graph)?;
            }
        } else {
            self.parse_predicate_object_list(&subject, formula_graph)?;
        }
        let next = self
            .lex
            .next()?
            .ok_or_else(|| eof("'.' or '}' after triple in formula"))?;
        match next.tok {
            Tok::Dot => Ok(false),
            Tok::RBrace => Ok(true),
            _ => Err(syntax(
                next.start,
                "expected '.' or '}' after triple in formula block",
            )),
        }
    }

    fn is_verb_start(&self, tok: &Tok) -> bool {
        matches!(
            tok,
            Tok::KwA
                | Tok::IriRef(_)
                | Tok::Pname { .. }
                | Tok::KwIs
                | Tok::KwHas
                | Tok::Implies
                | Tok::BareIdent(_)
        )
    }

    fn parse_predicate_object_list(
        &mut self,
        subject: &str,
        graph: Option<&str>,
    ) -> Result<(), String> {
        loop {
            // Handle `is P of` reverse-property path.
            let peek = self
                .lex
                .peek()?
                .ok_or_else(|| eof("predicate in predicate-object list"))?;
            if matches!(peek.tok, Tok::KwIs) {
                self.parse_is_of(subject, graph)?;
            } else {
                let predicate = self.parse_verb()?;
                self.parse_object_list(subject, &predicate, graph)?;
            }
            // After an object list: `;`, `.`, `)`, `]`, `}`, or EOF.
            let peek = self
                .lex
                .peek()?
                .ok_or_else(|| eof("end of predicate-object list"))?;
            match peek.tok {
                Tok::Semicolon => {
                    let _ = self.lex.next()?;
                    loop {
                        let Some(after) = self.lex.peek()? else {
                            return Ok(());
                        };
                        if matches!(after.tok, Tok::Semicolon) {
                            let _ = self.lex.next()?;
                            continue;
                        }
                        if matches!(
                            after.tok,
                            Tok::Dot | Tok::RBracket | Tok::RParen | Tok::RBrace
                        ) {
                            return Ok(());
                        }
                        break;
                    }
                    // Fall through to parse another predicate.
                }
                _ => return Ok(()),
            }
        }
    }

    /// Parse `is P of` reverse-property-path shorthand.
    ///
    /// `subject is P of object` emits `(object, P, subject)`.
    /// Multiple objects are comma-separated.
    fn parse_is_of(&mut self, subject: &str, graph: Option<&str>) -> Result<(), String> {
        // Consume `is`.
        let is_tok = self.lex.next()?.ok_or_else(|| eof("'is' keyword"))?;
        debug_assert!(matches!(is_tok.tok, Tok::KwIs));
        // Parse the property (predicate) term.
        let predicate = self.parse_verb()?;
        // Expect `of`.
        let of_tok = self.lex.next()?.ok_or_else(|| eof("'of' after 'is P'"))?;
        let is_of = matches!(of_tok.tok, Tok::KwOf)
            || matches!(&of_tok.tok, Tok::BareIdent(s) if s == "of");
        if !is_of {
            return Err(syntax(of_tok.start, "expected 'of' in 'is P of' path"));
        }
        // Parse the object list; for each object, emit (object, P, subject).
        loop {
            let object = self.parse_object(graph)?;
            // Reverse: object is the subject, subject is the object.
            self.emit(&object, &predicate, subject, graph, is_tok.start);
            let peek = self
                .lex
                .peek()?
                .ok_or_else(|| eof("','/';'/'.'"))?;
            if matches!(peek.tok, Tok::Comma) {
                let _ = self.lex.next()?;
                continue;
            }
            return Ok(());
        }
    }

    fn parse_object_list(
        &mut self,
        subject: &str,
        predicate: &str,
        graph: Option<&str>,
    ) -> Result<(), String> {
        loop {
            let object = self.parse_object(graph)?;
            self.emit(subject, predicate, &object, graph, 0);
            let peek = self
                .lex
                .peek()?
                .ok_or_else(|| eof("','/';'/'.'"))?;
            if matches!(peek.tok, Tok::Comma) {
                let _ = self.lex.next()?;
                continue;
            }
            return Ok(());
        }
    }

    fn parse_verb(&mut self) -> Result<String, String> {
        let tok = self.lex.next()?.ok_or_else(|| eof("verb"))?;
        match tok.tok {
            Tok::KwA => Ok(format!("<{RDF_TYPE}>")),
            Tok::Implies => Ok(format!("<{LOG_IMPLIES}>")),
            Tok::IriRef(s) => {
                let iri = self.resolve_iri(&s, tok.start)?;
                Ok(format!("<{iri}>"))
            }
            Tok::Pname { prefix, local } => {
                let iri = self.expand_pname(&prefix, &local, tok.start)?;
                Ok(format!("<{iri}>"))
            }
            Tok::BareIdent(ref name) if name == "a" => Ok(format!("<{RDF_TYPE}>")),
            _ => Err(syntax(
                tok.start,
                "expected verb (IRI, pname, 'a', '=>', or 'is')",
            )),
        }
    }

    fn parse_subject(&mut self, graph: Option<&str>) -> Result<(String, SubjectKind), String> {
        let tok = self.lex.next()?.ok_or_else(|| eof("subject"))?;
        self.subject_or_object_from_tok(tok, graph, true)
    }

    fn parse_object(&mut self, graph: Option<&str>) -> Result<String, String> {
        let tok = self.lex.next()?.ok_or_else(|| eof("object"))?;
        let (term, _kind) = self.subject_or_object_from_tok(tok, graph, false)?;
        Ok(term)
    }

    fn subject_or_object_from_tok(
        &mut self,
        tok: Spanned,
        graph: Option<&str>,
        is_subject: bool,
    ) -> Result<(String, SubjectKind), String> {
        match tok.tok {
            Tok::IriRef(s) => {
                let iri = self.resolve_iri(&s, tok.start)?;
                Ok((format!("<{iri}>"), SubjectKind::Iri))
            }
            Tok::Pname { prefix, local } => {
                let iri = self.expand_pname(&prefix, &local, tok.start)?;
                Ok((format!("<{iri}>"), SubjectKind::Iri))
            }
            Tok::BNodeLabel(label) => Ok((self.bnode_for_label(&label), SubjectKind::BNode)),
            Tok::Variable(name) => {
                // N3 variables are treated as blank nodes (scoped to formula).
                Ok((self.bnode_for_label(&format!("?{name}")), SubjectKind::BNode))
            }
            Tok::LBracket => Ok((
                self.blank_node_property_list(graph)?,
                SubjectKind::BlankNodePropertyList,
            )),
            Tok::LParen => Ok((self.collection(graph)?, SubjectKind::Collection)),
            Tok::LBrace => {
                // N3 quoted formula `{ … }` — mint a fresh bnode formula ID.
                let formula_id = self.fresh_formula_bnode();
                self.parse_formula_body(&formula_id, tok.start)?;
                Ok((formula_id, SubjectKind::Formula))
            }
            Tok::BareIdent(ref name) if name == "a" && !is_subject => {
                // `a` as object is `rdf:type` (unusual but valid in N3).
                Ok((format!("<{RDF_TYPE}>"), SubjectKind::Iri))
            }
            Tok::StringLit(_) | Tok::NumberLit { .. } | Tok::KwTrue | Tok::KwFalse
                if !is_subject =>
            {
                Ok((self.literal_from_tok(tok)?, SubjectKind::Literal))
            }
            Tok::StringLit(_) | Tok::NumberLit { .. } | Tok::KwTrue | Tok::KwFalse => {
                Err(syntax(tok.start, "literal in subject position"))
            }
            _ => Err(syntax(tok.start, "expected subject/object term")),
        }
    }

    /// Parse a quoted formula body `{ triples* }`.
    /// All emitted triples get `graph = Some(formula_id)`.
    fn parse_formula_body(&mut self, formula_id: &str, start: usize) -> Result<(), String> {
        // We already consumed `{`.
        loop {
            let peek = self.lex.peek()?.ok_or_else(|| {
                format!(
                    "N3-SYNTAX-001: unterminated formula block '{{' at byte {start}"
                )
            })?;
            if matches!(peek.tok, Tok::RBrace) {
                let _ = self.lex.next()?;
                return Ok(());
            }
            // Inside a formula, directives (@prefix, @base) are allowed per
            // N3 spec and widely used.
            match &peek.tok {
                Tok::DirPrefix => self.directive_prefix(false)?,
                Tok::DirBase => self.directive_base(false)?,
                Tok::DirKeywords => self.directive_keywords()?,
                Tok::DirForAll => self.directive_forall_forsome("@forAll")?,
                Tok::DirForSome => self.directive_forall_forsome("@forSome")?,
                _ => {
                    let saw_rbrace = self.triple_stmt_in_formula(Some(formula_id))?;
                    if saw_rbrace {
                        return Ok(());
                    }
                }
            }
        }
    }

    fn literal_from_tok(&mut self, tok: Spanned) -> Result<String, String> {
        match tok.tok {
            Tok::StringLit(lex) => {
                let Some(peek) = self.lex.peek()? else {
                    return Ok(format!("\"{}\"", escape_lex(&lex)));
                };
                match &peek.tok {
                    Tok::LangTag(tag) => {
                        let tag = tag.clone();
                        let _ = self.lex.next()?;
                        Ok(format!("\"{}\"@{}", escape_lex(&lex), tag))
                    }
                    Tok::DataTypeMark => {
                        let _ = self.lex.next()?;
                        let dt = self.lex.next()?.ok_or_else(|| eof("datatype IRI"))?;
                        let iri = match dt.tok {
                            Tok::IriRef(s) => self.resolve_iri(&s, dt.start)?,
                            Tok::Pname { prefix, local } => {
                                self.expand_pname(&prefix, &local, dt.start)?
                            }
                            _ => return Err(syntax(dt.start, "expected datatype IRI after '^^'")),
                        };
                        if iri == XSD_STRING {
                            Ok(format!("\"{}\"", escape_lex(&lex)))
                        } else {
                            Ok(format!("\"{}\"^^<{}>", escape_lex(&lex), iri))
                        }
                    }
                    _ => Ok(format!("\"{}\"", escape_lex(&lex))),
                }
            }
            Tok::NumberLit { kind, lexeme } => {
                let dt = match kind {
                    NumKind::Integer => XSD_INTEGER,
                    NumKind::Decimal => XSD_DECIMAL,
                    NumKind::Double => XSD_DOUBLE,
                };
                Ok(format!("\"{lexeme}\"^^<{dt}>"))
            }
            Tok::KwTrue => Ok(format!("\"true\"^^<{XSD_BOOLEAN}>")),
            Tok::KwFalse => Ok(format!("\"false\"^^<{XSD_BOOLEAN}>")),
            _ => Err(syntax(tok.start, "expected literal")),
        }
    }

    fn blank_node_property_list(&mut self, graph: Option<&str>) -> Result<String, String> {
        let bnode = self.fresh_bnode();
        let peek = self.lex.peek()?.ok_or_else(|| eof("']'"))?;
        if matches!(peek.tok, Tok::RBracket) {
            let _ = self.lex.next()?;
            return Ok(bnode);
        }
        self.parse_predicate_object_list(&bnode, graph)?;
        let close = self.lex.next()?.ok_or_else(|| eof("']'"))?;
        if !matches!(close.tok, Tok::RBracket) {
            return Err(syntax(close.start, "expected ']'"));
        }
        Ok(bnode)
    }

    fn collection(&mut self, graph: Option<&str>) -> Result<String, String> {
        let peek = self.lex.peek()?.ok_or_else(|| eof("')'"))?;
        if matches!(peek.tok, Tok::RParen) {
            let _ = self.lex.next()?;
            return Ok(format!("<{RDF_NIL}>"));
        }
        let head = self.fresh_bnode();
        let mut current = head.clone();
        loop {
            let item = self.parse_object(graph)?;
            self.emit(&current, &format!("<{RDF_FIRST}>"), &item, graph, 0);
            let peek = self.lex.peek()?.ok_or_else(|| eof("')'"))?;
            if matches!(peek.tok, Tok::RParen) {
                let _ = self.lex.next()?;
                self.emit(
                    &current,
                    &format!("<{RDF_REST}>"),
                    &format!("<{RDF_NIL}>"),
                    graph,
                    0,
                );
                break;
            }
            let next_cell = self.fresh_bnode();
            self.emit(&current, &format!("<{RDF_REST}>"), &next_cell, graph, 0);
            current = next_cell;
        }
        Ok(head)
    }

    // -- IRI / prefix helpers --------------------------------------------

    fn resolve_iri(&self, raw: &str, offset: usize) -> Result<String, String> {
        if is_absolute(raw) {
            return Ok(raw.to_owned());
        }
        let Some(base) = &self.base else {
            return Err(format!(
                "TTL-BASE-001: relative IRI with no @base established at byte {offset}"
            ));
        };
        Ok(resolve(raw, base))
    }

    fn expand_pname(&self, prefix: &str, local: &str, offset: usize) -> Result<String, String> {
        let ns = self.prefixes.get(prefix).ok_or_else(|| {
            format!(
                "TTL-PFX-001: undeclared prefix '{prefix}:' at byte {offset}"
            )
        })?;
        Ok(format!("{ns}{local}"))
    }

    fn bnode_for_label(&mut self, label: &str) -> String {
        if let Some(existing) = self.bnode_index.get(label) {
            return existing.clone();
        }
        let fresh = format!("_:u{}", self.bnode_counter);
        self.bnode_counter += 1;
        self.bnode_index.insert(label.to_owned(), fresh.clone());
        fresh
    }

    fn fresh_bnode(&mut self) -> String {
        let b = format!("_:g{}", self.bnode_counter);
        self.bnode_counter += 1;
        b
    }

    fn fresh_formula_bnode(&mut self) -> String {
        let b = format!("_:formula_{}", self.bnode_counter);
        self.bnode_counter += 1;
        b
    }

    fn emit(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        graph: Option<&str>,
        offset: usize,
    ) {
        self.out.push((
            Fact {
                subject: subject.to_owned(),
                predicate: predicate.to_owned(),
                object: object.to_owned(),
                graph: graph.map(ToOwned::to_owned),
            },
            FactProvenance {
                offset: Some(offset),
                parser: self.parser_id.to_owned(),
            },
        ));
    }
}

fn escape_lex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out
}

fn eof(what: &str) -> String {
    format!("N3-EOF-001: unexpected EOF while parsing {what}")
}

fn syntax(offset: usize, msg: &str) -> String {
    format!("N3-SYNTAX-001: {msg} at byte {offset}")
}
