//! Recursive-descent grammar for Turtle 1.1 (optionally TriG).
//!
//! The parser is driven by [`Lexer`] and emits raw `(Fact, FactProvenance)`
//! pairs in the inline canonical form expected by `Facts::canonicalise`
//! (IRIs wrapped in `<…>`, blank nodes `_:label`, literals quoted with
//! the `^^<iri>` or `@lang` suffix where appropriate).
//!
//! Pinned behaviours exercised here:
//!
//! - **TTL-LITESC-001** — string-escape and forbidden-raw-char rules are
//!   enforced by the lexer; the grammar surfaces any `Diag` unchanged.
//! - **TTL-BNPFX-001** — the blank-node label table is **document-scope**,
//!   not per-`@prefix`-directive and not per-TriG-graph-block. This
//!   matches the pin's "Reading chosen" §2: `_:b` in graph `<g1>` and
//!   `_:b` in graph `<g2>` refer to the *same* blank node.

use std::collections::BTreeMap;

use rdf_diff::{Fact, FactProvenance};

use crate::diag::{Diag, DiagnosticCode};
use crate::iri::{is_absolute, resolve};
use crate::lexer::{Lexer, NumKind, Spanned, Tok};

/// Dialect selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Dialect {
    /// Pure Turtle (`application/x-turtle`) — no graph blocks.
    Turtle,
    /// TriG (`application/trig`) — named-graph blocks allowed.
    TriG,
}

/// Syntactic category of a parsed subject. Drives the Turtle §2.5
/// `triples ::= blankNodePropertyList predicateObjectList?` branch
/// (predicateObjectList is optional iff subject was a property-list
/// bnode or a collection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubjectKind {
    /// `<iri>` or `pname` — predicateObjectList required.
    Iri,
    /// `_:label` — predicateObjectList required.
    BNode,
    /// `[ p o ]` or `[]` — predicateObjectList optional.
    BlankNodePropertyList,
    /// `( … )` — predicateObjectList optional (§2.5.1 collections).
    Collection,
    /// Literal (only reachable via `parse_object`; never a subject).
    Literal,
}

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// Parser state used by both Turtle and TriG.
pub(crate) struct Parser<'a> {
    lex: Lexer<'a>,
    dialect: Dialect,
    parser_id: &'static str,
    base: Option<String>,
    prefixes: BTreeMap<String, String>,
    // TTL-BNPFX-001: a single document-scope map is used. `@prefix`
    // directives and TriG graph blocks do NOT shadow it.
    bnode_index: BTreeMap<String, String>,
    bnode_counter: usize,
    out: Vec<(Fact, FactProvenance)>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(src: &'a [u8], dialect: Dialect, parser_id: &'static str) -> Self {
        Self {
            lex: Lexer::new(src),
            dialect,
            parser_id,
            base: None,
            prefixes: BTreeMap::new(),
            bnode_index: BTreeMap::new(),
            bnode_counter: 0,
            out: Vec::new(),
        }
    }

    pub(crate) fn finish(self) -> (Vec<(Fact, FactProvenance)>, BTreeMap<String, String>) {
        (self.out, self.prefixes)
    }

    /// Drive the top-level production `statement*`.
    pub(crate) fn parse_document(&mut self) -> Result<(), Diag> {
        loop {
            let Some(peek) = self.lex.peek()? else {
                return Ok(());
            };
            match &peek.tok {
                Tok::DirPrefix => self.directive_prefix(false)?,
                Tok::DirBase => self.directive_base(false)?,
                Tok::SparqlPrefix => self.directive_prefix(true)?,
                Tok::SparqlBase => self.directive_base(true)?,
                Tok::LBrace
            | Tok::IriRef(_)
            | Tok::Pname { .. }
            | Tok::BNodeLabel(_)
            | Tok::LBracket
            | Tok::KwGraph
                    if self.dialect == Dialect::TriG && self.looks_like_graph_block(&peek)? =>
                {
                    self.trig_graph_block()?;
                }
                _ => self.triple_or_quad_stmt()?,
            }
        }
    }

    // -- directives ------------------------------------------------------

    fn directive_prefix(&mut self, sparql_style: bool) -> Result<(), Diag> {
        let kw = self.lex.next()?.ok_or_else(|| eof("prefix directive"))?;
        let prefix_tok = self
            .lex
            .next()?
            .ok_or_else(|| eof("prefix name after @prefix/PREFIX"))?;
        let prefix_name = match prefix_tok.tok {
            Tok::Pname { prefix, local } if local.is_empty() => prefix,
            _ => return Err(syntax(prefix_tok.start, "expected 'prefix:' name")),
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
            // SPARQL-style `PREFIX` has no `.` terminator per Turtle §6.4
            // (production `sparqlPrefix`). We accept a stray `.` for
            // backward compatibility with the `adversary-ttl/fm6-…`
            // fixture corpus (authored before the strict reading). The
            // divergence surface is covered by allow-list entries for
            // W3C `turtle-syntax-bad-prefix-05` / `trig-syntax-bad-prefix-05`.
            self.consume_if_dot();
        } else {
            self.expect_dot(kw.start)?;
        }
        Ok(())
    }

    fn directive_base(&mut self, sparql_style: bool) -> Result<(), Diag> {
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
            // See `directive_prefix` — tolerant of a stray `.`. Covered
            // by allow-list for W3C `turtle-syntax-bad-base-03`.
            self.consume_if_dot();
        } else {
            self.expect_dot(kw.start)?;
        }
        Ok(())
    }

    /// Consume a `.` token if the next token is one; otherwise a no-op.
    fn consume_if_dot(&mut self) {
        let save = self.lex.offset();
        match self.lex.next() {
            Ok(Some(Spanned { tok: Tok::Dot, .. })) => {}
            _ => self.lex.seek(save),
        }
    }

    fn expect_dot(&mut self, anchor: usize) -> Result<(), Diag> {
        match self.lex.next()? {
            Some(Spanned { tok: Tok::Dot, .. }) => Ok(()),
            Some(s) => Err(Diag {
                code: DiagnosticCode::DirectiveTerminator,
                message: "directive not terminated with '.'".into(),
                offset: s.start,
                fatal: true,
            }),
            None => Err(Diag {
                code: DiagnosticCode::DirectiveTerminator,
                message: "directive not terminated with '.' (EOF)".into(),
                offset: anchor,
                fatal: true,
            }),
        }
    }

    // -- TriG graph blocks ----------------------------------------------

    /// Return `true` if the lookahead starts a TriG graph block. This is
    /// true when we see `{ … }` (default graph block), `GRAPH <iri> { … }`,
    /// or any term (IRIREF / pname / bnode label / `[]` anon-bnode)
    /// followed by `{`.
    fn looks_like_graph_block(&mut self, peek: &Spanned) -> Result<bool, Diag> {
        if matches!(peek.tok, Tok::LBrace | Tok::KwGraph) {
            return Ok(true);
        }
        let save = self.lex.offset();
        let first = self.lex.next()?;
        // Special case: `[` as graph name is only legal as empty `[]`
        // (anonymous bnode). If the `[` is not immediately followed by `]`
        // we are looking at a `[ p o ] …` triples subject, not a graph
        // block.
        if matches!(first.as_ref().map(|s| &s.tok), Some(Tok::LBracket)) {
            let second = self.lex.next()?;
            let third = self.lex.next()?;
            self.lex.seek(save);
            return Ok(
                matches!(second.map(|s| s.tok), Some(Tok::RBracket))
                    && matches!(third.map(|s| s.tok), Some(Tok::LBrace)),
            );
        }
        let second = self.lex.next()?;
        self.lex.seek(save);
        Ok(matches!(second.map(|s| s.tok), Some(Tok::LBrace)))
    }

    fn trig_graph_block(&mut self) -> Result<(), Diag> {
        let first = self
            .lex
            .next()?
            .ok_or_else(|| eof("graph block header"))?;
        let (graph_name, lbrace_pos) = match first.tok {
            Tok::LBrace => (None, first.start),
            Tok::KwGraph => {
                let name = self.lex.next()?.ok_or_else(|| eof("GRAPH <iri>"))?;
                let g = match name.tok {
                    Tok::LBracket => {
                        // `GRAPH [] { … }` — anonymous bnode graph name.
                        let close = self.lex.next()?.ok_or_else(|| eof("']'"))?;
                        if !matches!(close.tok, Tok::RBracket) {
                            return Err(syntax(
                                close.start,
                                "graph-name blank node must be empty `[]`",
                            ));
                        }
                        self.fresh_bnode()
                    }
                    _ => self.graph_name_from_tok(&name)?,
                };
                let brace = self.lex.next()?.ok_or_else(|| eof("'{' after GRAPH iri"))?;
                if !matches!(brace.tok, Tok::LBrace) {
                    return Err(syntax(brace.start, "expected '{' after GRAPH <iri>"));
                }
                (Some(g), brace.start)
            }
            Tok::IriRef(s) => {
                let g = self.resolve_iri(&s, first.start)?;
                let brace = self.lex.next()?.ok_or_else(|| eof("'{' after graph name"))?;
                if !matches!(brace.tok, Tok::LBrace) {
                    return Err(syntax(brace.start, "expected '{' after graph name"));
                }
                (Some(format!("<{g}>")), brace.start)
            }
            Tok::Pname { prefix, local } => {
                let g = self.expand_pname(&prefix, &local, first.start)?;
                let brace = self.lex.next()?.ok_or_else(|| eof("'{' after graph name"))?;
                if !matches!(brace.tok, Tok::LBrace) {
                    return Err(syntax(brace.start, "expected '{' after graph name"));
                }
                (Some(format!("<{g}>")), brace.start)
            }
            Tok::BNodeLabel(label) => {
                // TriG §2.6: a graph name may be a blank-node label.
                let g = self.bnode_for_label(&label);
                let brace = self.lex.next()?.ok_or_else(|| eof("'{' after graph name"))?;
                if !matches!(brace.tok, Tok::LBrace) {
                    return Err(syntax(brace.start, "expected '{' after graph name"));
                }
                (Some(g), brace.start)
            }
            Tok::LBracket => {
                // Anonymous bnode as graph name — must be the empty form
                // `[]`; nonempty `[ p o ]` as a graph name isn't legal.
                let close = self.lex.next()?.ok_or_else(|| eof("']'"))?;
                if !matches!(close.tok, Tok::RBracket) {
                    return Err(syntax(
                        close.start,
                        "graph-name blank node must be empty `[]`",
                    ));
                }
                let g = self.fresh_bnode();
                let brace = self.lex.next()?.ok_or_else(|| eof("'{' after graph name"))?;
                if !matches!(brace.tok, Tok::LBrace) {
                    return Err(syntax(brace.start, "expected '{' after graph name"));
                }
                (Some(g), brace.start)
            }
            _ => return Err(syntax(first.start, "expected '{' or graph name")),
        };
        self.parse_graph_body(graph_name.as_deref(), lbrace_pos)?;
        Ok(())
    }

    fn parse_graph_body(&mut self, graph: Option<&str>, start: usize) -> Result<(), Diag> {
        loop {
            let peek = self
                .lex
                .peek()?
                .ok_or_else(|| Diag {
                    code: DiagnosticCode::Unterminated,
                    message: "unterminated graph block".into(),
                    offset: start,
                    fatal: true,
                })?;
            if matches!(peek.tok, Tok::RBrace) {
                let _ = self.lex.next()?;
                return Ok(());
            }
            // Inside a graph block, statements are triples (no directives
            // per §2.2). Per TriG §2.5, the last triple may omit its `.`
            // immediately before the closing `}`; `parse_triple_stmt_in_block`
            // consumes the `}` in that case.
            let saw_rbrace = self.parse_triple_stmt_in_block(graph)?;
            if saw_rbrace {
                return Ok(());
            }
        }
    }

    // -- triples ---------------------------------------------------------

    fn triple_or_quad_stmt(&mut self) -> Result<(), Diag> {
        self.parse_triple_stmt(None)
    }

    fn parse_triple_stmt(&mut self, graph: Option<&str>) -> Result<(), Diag> {
        let (subject, subject_kind) = self.parse_subject(graph)?;
        // Turtle §2.5 grammar:
        //   triples ::= subject predicateObjectList
        //             | blankNodePropertyList predicateObjectList?
        // i.e. when the subject is a blankNodePropertyList or an empty
        // `[]`, the predicateObjectList is optional (the bnode stands on
        // its own, carrying whatever predicates were declared inside the
        // brackets).
        // Turtle §2.5 only makes predicateObjectList optional after a
        // blankNodePropertyList subject. A bare collection subject must
        // still be followed by a predicateObjectList (W3C
        // `trig-syntax-bad-list-0{1..4}` tests pin this as negative
        // syntax).
        let pol_optional = matches!(subject_kind, SubjectKind::BlankNodePropertyList);
        if pol_optional {
            // Peek — if the next token isn't a verb-ish start, don't
            // consume it. The caller will re-inspect it as the statement
            // terminator (`.` / `}` / EOF).
            let peek = self.lex.peek()?;
            let has_verb = matches!(
                peek.as_ref().map(|s| &s.tok),
                Some(Tok::KwA | Tok::IriRef(_) | Tok::Pname { .. })
            );
            if has_verb {
                self.parse_predicate_object_list(&subject, graph)?;
            }
        } else {
            self.parse_predicate_object_list(&subject, graph)?;
        }
        // Statement terminator. Inside a TriG graph block the last triple
        // may omit the trailing dot (TriG §2.5) — the caller handles the
        // `}` case via `consume_triple_terminator_in_block`. At the outer
        // document level, `.` is mandatory.
        let next = self.lex.next()?.ok_or_else(|| eof("'.' after triple"))?;
        if !matches!(next.tok, Tok::Dot) {
            return Err(syntax(next.start, "expected '.' after triple statement"));
        }
        Ok(())
    }

    /// Variant of `parse_triple_stmt` used inside a TriG graph block,
    /// where the *last* triple may omit the trailing `.` before `}`
    /// (TriG §2.5). Returns `true` if it consumed the terminating `}`.
    fn parse_triple_stmt_in_block(&mut self, graph: Option<&str>) -> Result<bool, Diag> {
        let (subject, subject_kind) = self.parse_subject(graph)?;
        // Turtle §2.5 only makes predicateObjectList optional after a
        // blankNodePropertyList subject. A bare collection subject must
        // still be followed by a predicateObjectList (W3C
        // `trig-syntax-bad-list-0{1..4}` tests pin this as negative
        // syntax).
        let pol_optional = matches!(subject_kind, SubjectKind::BlankNodePropertyList);
        if pol_optional {
            let peek = self.lex.peek()?;
            let has_verb = matches!(
                peek.as_ref().map(|s| &s.tok),
                Some(Tok::KwA | Tok::IriRef(_) | Tok::Pname { .. })
            );
            if has_verb {
                self.parse_predicate_object_list(&subject, graph)?;
            }
        } else {
            self.parse_predicate_object_list(&subject, graph)?;
        }
        // Accept `.`, or `}` (implicit terminator for last triple).
        let next = self
            .lex
            .next()?
            .ok_or_else(|| eof("'.' or '}' after triple"))?;
        match next.tok {
            Tok::Dot => Ok(false),
            Tok::RBrace => Ok(true),
            _ => Err(syntax(
                next.start,
                "expected '.' or '}' after triple statement",
            )),
        }
    }

    fn parse_predicate_object_list(
        &mut self,
        subject: &str,
        graph: Option<&str>,
    ) -> Result<(), Diag> {
        loop {
            let predicate = self.parse_verb()?;
            self.parse_object_list(subject, &predicate, graph)?;
            // After an object list, one of: ';', '.', ')', ']', '}' or EOF.
            let peek = self
                .lex
                .peek()?
                .ok_or_else(|| eof("end of predicate-object list"))?;
            match peek.tok {
                Tok::Semicolon => {
                    let _ = self.lex.next()?;
                    // `;` may be followed by another verb or by `.`/`]`/`)`/`}`
                    // (trailing-semicolon accepted per §2.5.1).
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
                    // Fall through to parse another verb/objectList.
                }
                _ => return Ok(()),
            }
        }
    }

    fn parse_object_list(
        &mut self,
        subject: &str,
        predicate: &str,
        graph: Option<&str>,
    ) -> Result<(), Diag> {
        loop {
            let object = self.parse_object(graph)?;
            self.emit(subject, predicate, &object, graph, 0);
            let peek = self.lex.peek()?.ok_or_else(|| eof("','/';'/'.'"))?;
            if matches!(peek.tok, Tok::Comma) {
                let _ = self.lex.next()?;
                continue;
            }
            return Ok(());
        }
    }

    fn parse_verb(&mut self) -> Result<String, Diag> {
        let tok = self.lex.next()?.ok_or_else(|| eof("verb"))?;
        match tok.tok {
            Tok::KwA => Ok(format!("<{RDF_TYPE}>")),
            Tok::IriRef(s) => {
                let iri = self.resolve_iri(&s, tok.start)?;
                Ok(format!("<{iri}>"))
            }
            Tok::Pname { prefix, local } => {
                let iri = self.expand_pname(&prefix, &local, tok.start)?;
                Ok(format!("<{iri}>"))
            }
            _ => Err(syntax(tok.start, "expected verb (IRI, pname, or 'a')")),
        }
    }

    fn parse_subject(&mut self, graph: Option<&str>) -> Result<(String, SubjectKind), Diag> {
        let tok = self.lex.next()?.ok_or_else(|| eof("subject"))?;
        self.subject_or_object_from_tok(tok, graph, /*is_subject*/ true)
    }

    fn parse_object(&mut self, graph: Option<&str>) -> Result<String, Diag> {
        let tok = self.lex.next()?.ok_or_else(|| eof("object"))?;
        let (term, _kind) = self.subject_or_object_from_tok(tok, graph, /*is_subject*/ false)?;
        Ok(term)
    }

    fn subject_or_object_from_tok(
        &mut self,
        tok: Spanned,
        graph: Option<&str>,
        is_subject: bool,
    ) -> Result<(String, SubjectKind), Diag> {
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
            Tok::LBracket => Ok((
                self.blank_node_property_list(graph)?,
                SubjectKind::BlankNodePropertyList,
            )),
            Tok::LParen => Ok((self.collection(graph)?, SubjectKind::Collection)),
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

    fn graph_name_from_tok(&mut self, tok: &Spanned) -> Result<String, Diag> {
        // Permissive graph-name reader (iri / pname / bnode). `[]` is
        // handled in the outer `trig_graph_block` because its opening
        // bracket arrives in the header position, not after `GRAPH`.
        match &tok.tok {
            Tok::IriRef(s) => {
                let iri = self.resolve_iri(s, tok.start)?;
                Ok(format!("<{iri}>"))
            }
            Tok::Pname { prefix, local } => {
                let iri = self.expand_pname(prefix, local, tok.start)?;
                Ok(format!("<{iri}>"))
            }
            Tok::BNodeLabel(label) => Ok(self.bnode_for_label(label)),
            _ => Err(syntax(tok.start, "expected IRI, prefixed name, or blank node")),
        }
    }

    fn literal_from_tok(&mut self, tok: Spanned) -> Result<String, Diag> {
        match tok.tok {
            Tok::StringLit(lex) => {
                // Optional suffix: @lang or ^^iri.
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
                        // `xsd:string` collapses to the plain literal form
                        // per RDF 1.1 §3.3.
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

    fn blank_node_property_list(&mut self, graph: Option<&str>) -> Result<String, Diag> {
        // We already consumed '['. Mint a fresh bnode and parse
        // predicateObjectList until the matching ']'.
        let bnode = self.fresh_bnode();
        // Check for empty `[]`.
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

    fn collection(&mut self, graph: Option<&str>) -> Result<String, Diag> {
        // We already consumed '('. Peek for the empty case, otherwise
        // mint the head bnode *before* parsing items so that nested
        // collections get bnode labels AFTER the outer head (matching
        // the canonical emission order used by the W3C eval corpora —
        // see `turtle-eval-lists-05`).
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
            // Peek: if the next token is `)`, emit rest=nil and finish.
            // Otherwise mint the next cons cell and continue.
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

    // -- IRI / prefix helpers -------------------------------------------

    fn resolve_iri(&self, raw: &str, offset: usize) -> Result<String, Diag> {
        if is_absolute(raw) {
            return Ok(raw.to_owned());
        }
        let Some(base) = &self.base else {
            return Err(Diag {
                code: DiagnosticCode::NoBase,
                message: "relative IRI with no @base established".into(),
                offset,
                fatal: true,
            });
        };
        Ok(resolve(raw, base))
    }

    fn expand_pname(&self, prefix: &str, local: &str, offset: usize) -> Result<String, Diag> {
        let ns = self.prefixes.get(prefix).ok_or_else(|| Diag {
            code: DiagnosticCode::UndeclaredPrefix,
            message: format!("undeclared prefix '{prefix}:'"),
            offset,
            fatal: true,
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

/// Escape a lexical form for inline canonical representation. Our
/// canonical convention is: keep the raw USV string but escape `"` and
/// `\` so the framing is unambiguous when `split_literal` in
/// `rdf-diff` walks the canonical form back out.
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

fn eof(what: &str) -> Diag {
    Diag {
        code: DiagnosticCode::UnexpectedEof,
        message: format!("unexpected EOF while parsing {what}"),
        offset: 0,
        fatal: true,
    }
}

fn syntax(offset: usize, msg: &str) -> Diag {
    Diag {
        code: DiagnosticCode::Syntax,
        message: msg.to_owned(),
        offset,
        fatal: true,
    }
}
