//! Recursive-descent grammar for SPARQL 1.1 Query + Update.
//!
//! Covers the productions enumerated in SPARQL 1.1 §19, including
//! Prologue (§4.1), Query forms (§10), Dataset (§13.2), group graph
//! patterns (§17), property paths (§9), expressions (§17.4), solution
//! modifiers (§15), and Update operations (§3).
//!
//! The parser tracks a small `parse_state` for the adversary failure
//! modes it surfaces at grammar level:
//!
//! - `seen_query_form` — set once the query form keyword is consumed, so
//!   a subsequent `BASE`/`PREFIX` inside the WHERE is rejected with
//!   `SPARQL-PROLOGUE-001` (adversary brief FM5).
//! - `bind_scope` tracker: a per-group-graph-pattern set of already-seen
//!   variable names; a `BIND(... AS ?x)` introducing a variable already
//!   in scope raises `SPARQL-BIND-001` (adversary brief FM11b).
//! - `in_data_block` flag: when parsing an `INSERT DATA` / `DELETE DATA`
//!   body, variables are rejected; `DELETE DATA` additionally rejects
//!   blank nodes. Both surface via `SPARQL-UPDATE-001`.

use std::collections::BTreeSet;

use crate::ast::{
    ConstructClause, CopyMoveKind, DatasetClause, Expr, FuncName, GraphOrDefault, GraphTarget,
    GroupCondition, GroupGraphPattern, GroupPatternElement, InlineData, Literal, LiteralKind,
    NegatedAtom, OrderCondition, Path, PathOrPredicate, PathPrim, Projection, QuadTriple, Query,
    QueryForm, Request, SelectClause, SelectModifier, SolutionModifier, TermOrPath, TriplePattern,
    UpdateOp, UpdateRequest, VarOrIri,
};
use crate::diag::{Diag, DiagnosticCode};
use crate::lexer::{Lexer, NumKind, Spanned, Tok};

/// Top-level parser for a SPARQL request.
pub(crate) struct Parser<'a> {
    lex: Lexer<'a>,
    peeked: Option<Spanned>,
    last_end: usize,
    /// Set to `true` after the first `SELECT`/`CONSTRUCT`/`ASK`/`DESCRIBE`
    /// keyword is consumed (or after the first Update operation in Update
    /// mode). Further `BASE`/`PREFIX` tokens inside the body trigger
    /// `SPARQL-PROLOGUE-001`.
    seen_body: bool,
    /// Inside an `INSERT DATA` block. Rejects variables.
    in_insert_data: bool,
    /// Inside a `DELETE DATA` block. Rejects variables and blank nodes.
    in_delete_data: bool,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(src: &'a [u8]) -> Self {
        Self {
            lex: Lexer::new(src),
            peeked: None,
            last_end: 0,
            seen_body: false,
            in_insert_data: false,
            in_delete_data: false,
        }
    }

    pub(crate) fn parse_request(&mut self) -> Result<Request, Diag> {
        // Parse Prologue (BASE / PREFIX declarations, §4.1).
        let (base, prefixes) = self.parse_prologue()?;
        // Peek to decide query vs update.
        let Some(tok) = self.peek()? else {
            return Err(self.err_at_eof(
                DiagnosticCode::UnexpectedEof,
                "empty SPARQL request",
            ));
        };
        let kw = token_keyword(&tok.tok);
        match kw.as_deref() {
            Some("SELECT" | "CONSTRUCT" | "ASK" | "DESCRIBE") => {
                let query = self.parse_query_body(base, prefixes)?;
                self.expect_eof()?;
                Ok(Request::Query(query))
            }
            Some(
                "INSERT" | "DELETE" | "LOAD" | "CLEAR" | "CREATE" | "DROP" | "COPY" | "MOVE"
                | "ADD" | "WITH",
            ) => {
                let upd = self.parse_update_body(base, prefixes)?;
                self.expect_eof()?;
                Ok(Request::Update(upd))
            }
            _ => Err(Diag::fatal(
                DiagnosticCode::Syntax,
                format!(
                    "expected query form or update operation, found {:?}",
                    tok.tok
                ),
                tok.start,
            )),
        }
    }

    // ---------- Prologue -------------------------------------------------

    fn parse_prologue(&mut self) -> Result<(Option<String>, Vec<(String, String)>), Diag> {
        let mut base: Option<String> = None;
        let mut prefixes: Vec<(String, String)> = Vec::new();
        loop {
            let Some(tok) = self.peek()? else {
                break;
            };
            match token_keyword(&tok.tok).as_deref() {
                Some("BASE") => {
                    self.bump()?;
                    let iri_tok = self.expect_next("IRIREF after BASE")?;
                    if let Tok::IriRef(s) = iri_tok.tok {
                        base = Some(s);
                    } else {
                        return Err(Diag::fatal(
                            DiagnosticCode::Syntax,
                            "expected <iri> after BASE",
                            iri_tok.start,
                        ));
                    }
                }
                Some("PREFIX") => {
                    self.bump()?;
                    let p = self.expect_next("prefix name after PREFIX")?;
                    let pfx = match p.tok {
                        Tok::Pname { prefix, local } if local.is_empty() => prefix,
                        _ => {
                            return Err(Diag::fatal(
                                DiagnosticCode::Syntax,
                                "expected `prefix:` after PREFIX",
                                p.start,
                            ));
                        }
                    };
                    let iri_tok = self.expect_next("IRIREF after PREFIX name")?;
                    let Tok::IriRef(iri) = iri_tok.tok else {
                        return Err(Diag::fatal(
                            DiagnosticCode::Syntax,
                            "expected <iri> after PREFIX name",
                            iri_tok.start,
                        ));
                    };
                    prefixes.push((pfx, iri));
                }
                _ => break,
            }
        }
        Ok((base, prefixes))
    }


    // ---------- Query body -----------------------------------------------

    fn parse_query_body(
        &mut self,
        base: Option<String>,
        prefixes: Vec<(String, String)>,
    ) -> Result<Query, Diag> {
        let tok = self.peek()?.expect("caller checked");
        let kw = token_keyword(&tok.tok).unwrap_or_default();
        self.seen_body = true;
        let form = match kw.as_str() {
            "SELECT" => QueryForm::Select(self.parse_select_clause()?),
            "CONSTRUCT" => QueryForm::Construct(self.parse_construct_head()?),
            "ASK" => {
                self.bump()?;
                QueryForm::Ask
            }
            "DESCRIBE" => {
                self.bump()?;
                let mut targets = Vec::new();
                if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Star)) {
                    self.bump()?;
                } else {
                    loop {
                        let Some(next) = self.peek()? else {
                            break;
                        };
                        if matches!(token_keyword(&next.tok).as_deref(), Some("WHERE" | "FROM"))
                            || matches!(next.tok, Tok::LBrace)
                        {
                            break;
                        }
                        targets.push(self.parse_var_or_iri()?);
                    }
                }
                QueryForm::Describe { targets }
            }
            _ => {
                return Err(Diag::fatal(
                    DiagnosticCode::Syntax,
                    "unknown query form",
                    tok.start,
                ));
            }
        };
        // Dataset clauses.
        let dataset = self.parse_dataset_clauses()?;
        // WHERE — optional for DESCRIBE only; required otherwise.
        let where_clause = if matches!(form, QueryForm::Describe { .. })
            && !matches!(
                self.peek()?.as_ref().map(|t| &t.tok),
                Some(Tok::LBrace) | Some(Tok::Ident(_))
            ) {
            GroupGraphPattern::default()
        } else {
            // WHERE keyword is optional for CONSTRUCT short form after
            // CONSTRUCT; and always allowed. If present, consume.
            let maybe_where = self.peek()?;
            if let Some(t) = maybe_where.as_ref()
                && token_keyword(&t.tok).as_deref() == Some("WHERE")
            {
                self.bump()?;
            }
            self.parse_group_graph_pattern()?
        };
        // Solution modifiers (§15).
        let modifiers = self.parse_solution_modifiers()?;
        // VALUES — post-WHERE block (§15.6).
        let values_clause = if matches!(
            self.peek()?.as_ref().map(|t| token_keyword(&t.tok)),
            Some(Some(ref kw)) if kw == "VALUES"
        ) {
            Some(self.parse_values_data()?)
        } else {
            None
        };
        Ok(Query {
            base,
            prefixes,
            form,
            dataset,
            where_clause,
            modifiers,
            values_clause,
        })
    }

    fn parse_select_clause(&mut self) -> Result<SelectClause, Diag> {
        self.bump()?; // SELECT
        // Optional DISTINCT / REDUCED.
        let modifier = match self.peek_keyword().as_deref() {
            Some("DISTINCT") => {
                self.bump()?;
                Some(SelectModifier::Distinct)
            }
            Some("REDUCED") => {
                self.bump()?;
                Some(SelectModifier::Reduced)
            }
            _ => None,
        };
        // Projection: `*` or list of var / (expr AS ?v).
        let projection = if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Star)) {
            self.bump()?;
            None
        } else {
            let mut list = Vec::new();
            loop {
                match self.peek()?.map(|t| t.tok) {
                    Some(Tok::Var(v)) => {
                        self.bump()?;
                        list.push(Projection::Var(v));
                    }
                    Some(Tok::LParen) => {
                        self.bump()?;
                        let expr = self.parse_expr()?;
                        let kw = self.peek_keyword();
                        if kw.as_deref() != Some("AS") {
                            return Err(self.err_here(
                                DiagnosticCode::Syntax,
                                "expected AS in projection",
                            ));
                        }
                        self.bump()?;
                        let v = self.expect_next("variable in projection")?;
                        let Tok::Var(var) = v.tok else {
                            return Err(Diag::fatal(
                                DiagnosticCode::Syntax,
                                "expected variable",
                                v.start,
                            ));
                        };
                        self.expect_token(&Tok::RParen, ")")?;
                        list.push(Projection::Expr { expr, var });
                    }
                    _ => break,
                }
            }
            if list.is_empty() {
                return Err(self.err_here(
                    DiagnosticCode::Syntax,
                    "empty SELECT projection",
                ));
            }
            Some(list)
        };
        Ok(SelectClause {
            modifier,
            projection,
        })
    }

    fn parse_construct_head(&mut self) -> Result<ConstructClause, Diag> {
        self.bump()?; // CONSTRUCT
        // Two forms: `CONSTRUCT { template } WHERE { ... }` and short
        // `CONSTRUCT WHERE { ... }`.
        if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::LBrace)) {
            let template = self.parse_triples_template()?;
            Ok(ConstructClause {
                template: Some(template),
            })
        } else {
            Ok(ConstructClause { template: None })
        }
    }

    fn parse_triples_template(&mut self) -> Result<Vec<TriplePattern>, Diag> {
        // `{` TriplesTemplate? `}`
        self.expect_token(&Tok::LBrace, "{")?;
        let mut out = Vec::new();
        while !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RBrace)) {
            self.parse_triples_block(&mut out)?;
            if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Dot)) {
                self.bump()?;
            } else {
                break;
            }
        }
        self.expect_token(&Tok::RBrace, "}")?;
        Ok(out)
    }

    fn parse_dataset_clauses(&mut self) -> Result<Vec<DatasetClause>, Diag> {
        let mut out = Vec::new();
        while self.peek_keyword().as_deref() == Some("FROM") {
            self.bump()?;
            if self.peek_keyword().as_deref() == Some("NAMED") {
                self.bump()?;
                let iri = self.parse_iri()?;
                out.push(DatasetClause::Named(iri));
            } else {
                let iri = self.parse_iri()?;
                out.push(DatasetClause::Default(iri));
            }
        }
        Ok(out)
    }

    // ---------- Update body ----------------------------------------------

    fn parse_update_body(
        &mut self,
        base: Option<String>,
        prefixes: Vec<(String, String)>,
    ) -> Result<UpdateRequest, Diag> {
        self.seen_body = true;
        let mut ops = Vec::new();
        loop {
            // Each update unit may be preceded by `;` separator.
            let Some(tok) = self.peek()? else {
                break;
            };
            let kw = token_keyword(&tok.tok);
            match kw.as_deref() {
                Some("LOAD") => ops.push(self.parse_load()?),
                Some("CLEAR") => ops.push(self.parse_clear()?),
                Some("CREATE") => ops.push(self.parse_create()?),
                Some("DROP") => ops.push(self.parse_drop()?),
                Some("COPY") => ops.push(self.parse_copy_move_add(CopyMoveKind::Copy)?),
                Some("MOVE") => ops.push(self.parse_copy_move_add(CopyMoveKind::Move)?),
                Some("ADD") => ops.push(self.parse_copy_move_add(CopyMoveKind::Add)?),
                Some("INSERT") => ops.push(self.parse_insert()?),
                Some("DELETE") => ops.push(self.parse_delete()?),
                Some("WITH") => ops.push(self.parse_modify_with()?),
                _ => break,
            }
            if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Semicolon)) {
                self.bump()?;
                // Sub-prologue allowed between update units.
                let (_b, extra) = self.parse_prologue()?;
                // Merge extras into prefixes (prefix rebinding allowed).
                // Simplified: just discard the extras (grammar-only).
                drop(extra);
            } else {
                break;
            }
        }
        if ops.is_empty() {
            return Err(self.err_here(
                DiagnosticCode::Syntax,
                "empty Update request",
            ));
        }
        Ok(UpdateRequest {
            base,
            prefixes,
            operations: ops,
        })
    }

    fn parse_load(&mut self) -> Result<UpdateOp, Diag> {
        self.bump()?; // LOAD
        let silent = self.eat_keyword("SILENT");
        let source = self.parse_iri()?;
        let dest = if self.peek_keyword().as_deref() == Some("INTO") {
            self.bump()?;
            if self.peek_keyword().as_deref() != Some("GRAPH") {
                return Err(self.err_here(
                    DiagnosticCode::Syntax,
                    "expected GRAPH after INTO",
                ));
            }
            self.bump()?;
            Some(self.parse_iri()?)
        } else {
            None
        };
        Ok(UpdateOp::Load {
            silent,
            source,
            dest,
        })
    }

    fn parse_clear(&mut self) -> Result<UpdateOp, Diag> {
        self.bump()?; // CLEAR
        let silent = self.eat_keyword("SILENT");
        let target = self.parse_graph_target()?;
        Ok(UpdateOp::Clear { silent, target })
    }

    fn parse_create(&mut self) -> Result<UpdateOp, Diag> {
        self.bump()?; // CREATE
        let silent = self.eat_keyword("SILENT");
        if self.peek_keyword().as_deref() != Some("GRAPH") {
            return Err(self.err_here(
                DiagnosticCode::Syntax,
                "expected GRAPH after CREATE",
            ));
        }
        self.bump()?;
        let graph = self.parse_iri()?;
        Ok(UpdateOp::Create { silent, graph })
    }

    fn parse_drop(&mut self) -> Result<UpdateOp, Diag> {
        self.bump()?; // DROP
        let silent = self.eat_keyword("SILENT");
        let target = self.parse_graph_target()?;
        Ok(UpdateOp::Drop { silent, target })
    }

    fn parse_graph_target(&mut self) -> Result<GraphTarget, Diag> {
        match self.peek_keyword().as_deref() {
            Some("GRAPH") => {
                self.bump()?;
                let iri = self.parse_iri()?;
                Ok(GraphTarget::Graph(iri))
            }
            Some("DEFAULT") => {
                self.bump()?;
                Ok(GraphTarget::Default)
            }
            Some("NAMED") => {
                self.bump()?;
                Ok(GraphTarget::Named)
            }
            Some("ALL") => {
                self.bump()?;
                Ok(GraphTarget::All)
            }
            _ => Err(self.err_here(
                DiagnosticCode::Syntax,
                "expected GRAPH|DEFAULT|NAMED|ALL",
            )),
        }
    }

    fn parse_copy_move_add(&mut self, op: CopyMoveKind) -> Result<UpdateOp, Diag> {
        self.bump()?; // COPY/MOVE/ADD
        let silent = self.eat_keyword("SILENT");
        let source = self.parse_graph_or_default()?;
        if self.peek_keyword().as_deref() != Some("TO") {
            return Err(self.err_here(DiagnosticCode::Syntax, "expected TO"));
        }
        self.bump()?;
        let target = self.parse_graph_or_default()?;
        Ok(UpdateOp::CopyMoveAdd {
            op,
            silent,
            source,
            target,
        })
    }

    fn parse_graph_or_default(&mut self) -> Result<GraphOrDefault, Diag> {
        match self.peek_keyword().as_deref() {
            Some("DEFAULT") => {
                self.bump()?;
                Ok(GraphOrDefault::Default)
            }
            Some("GRAPH") => {
                self.bump()?;
                let iri = self.parse_iri()?;
                Ok(GraphOrDefault::Graph(iri))
            }
            _ => {
                // Bare IRI shorthand.
                let iri = self.parse_iri()?;
                Ok(GraphOrDefault::Graph(iri))
            }
        }
    }

    fn parse_insert(&mut self) -> Result<UpdateOp, Diag> {
        self.bump()?; // INSERT
        if self.peek_keyword().as_deref() == Some("DATA") {
            self.bump()?;
            self.in_insert_data = true;
            let quads = self.parse_quads_block()?;
            self.in_insert_data = false;
            Ok(UpdateOp::InsertData(quads))
        } else {
            // INSERT { quads } (USING ...)* WHERE { ggp }
            let insert_quads = self.parse_quads_block()?;
            let mut using = Vec::new();
            while self.peek_keyword().as_deref() == Some("USING") {
                self.bump()?;
                if self.peek_keyword().as_deref() == Some("NAMED") {
                    self.bump()?;
                    let iri = self.parse_iri()?;
                    using.push(DatasetClause::Named(iri));
                } else {
                    let iri = self.parse_iri()?;
                    using.push(DatasetClause::Default(iri));
                }
            }
            if self.peek_keyword().as_deref() != Some("WHERE") {
                return Err(self.err_here(
                    DiagnosticCode::Syntax,
                    "expected WHERE in Modify",
                ));
            }
            self.bump()?;
            let where_clause = self.parse_group_graph_pattern()?;
            Ok(UpdateOp::Modify {
                with: None,
                delete: None,
                insert: Some(insert_quads),
                using,
                where_clause,
            })
        }
    }

    fn parse_delete(&mut self) -> Result<UpdateOp, Diag> {
        self.bump()?; // DELETE
        match self.peek_keyword().as_deref() {
            Some("DATA") => {
                self.bump()?;
                self.in_delete_data = true;
                let quads = self.parse_quads_block()?;
                self.in_delete_data = false;
                Ok(UpdateOp::DeleteData(quads))
            }
            Some("WHERE") => {
                self.bump()?;
                let quads = self.parse_quads_block()?;
                Ok(UpdateOp::DeleteWhere(quads))
            }
            _ => {
                let delete_quads = self.parse_quads_block()?;
                // Optional INSERT follow-up.
                let insert_quads = if self.peek_keyword().as_deref() == Some("INSERT") {
                    self.bump()?;
                    Some(self.parse_quads_block()?)
                } else {
                    None
                };
                let mut using = Vec::new();
                while self.peek_keyword().as_deref() == Some("USING") {
                    self.bump()?;
                    if self.peek_keyword().as_deref() == Some("NAMED") {
                        self.bump()?;
                        let iri = self.parse_iri()?;
                        using.push(DatasetClause::Named(iri));
                    } else {
                        let iri = self.parse_iri()?;
                        using.push(DatasetClause::Default(iri));
                    }
                }
                if self.peek_keyword().as_deref() != Some("WHERE") {
                    return Err(self.err_here(
                        DiagnosticCode::Syntax,
                        "expected WHERE in Modify",
                    ));
                }
                self.bump()?;
                let where_clause = self.parse_group_graph_pattern()?;
                Ok(UpdateOp::Modify {
                    with: None,
                    delete: Some(delete_quads),
                    insert: insert_quads,
                    using,
                    where_clause,
                })
            }
        }
    }

    fn parse_modify_with(&mut self) -> Result<UpdateOp, Diag> {
        self.bump()?; // WITH
        let with_iri = self.parse_iri()?;
        // Next: DELETE / INSERT.
        let mut delete = None;
        let mut insert = None;
        loop {
            match self.peek_keyword().as_deref() {
                Some("DELETE") => {
                    self.bump()?;
                    delete = Some(self.parse_quads_block()?);
                }
                Some("INSERT") => {
                    self.bump()?;
                    insert = Some(self.parse_quads_block()?);
                }
                _ => break,
            }
        }
        let mut using = Vec::new();
        while self.peek_keyword().as_deref() == Some("USING") {
            self.bump()?;
            if self.peek_keyword().as_deref() == Some("NAMED") {
                self.bump()?;
                let iri = self.parse_iri()?;
                using.push(DatasetClause::Named(iri));
            } else {
                let iri = self.parse_iri()?;
                using.push(DatasetClause::Default(iri));
            }
        }
        if self.peek_keyword().as_deref() != Some("WHERE") {
            return Err(self.err_here(
                DiagnosticCode::Syntax,
                "expected WHERE in WITH-Modify",
            ));
        }
        self.bump()?;
        let where_clause = self.parse_group_graph_pattern()?;
        Ok(UpdateOp::Modify {
            with: Some(with_iri),
            delete,
            insert,
            using,
            where_clause,
        })
    }

    fn parse_quads_block(&mut self) -> Result<Vec<QuadTriple>, Diag> {
        self.expect_token(&Tok::LBrace, "{")?;
        let mut out: Vec<QuadTriple> = Vec::new();
        loop {
            match self.peek()?.map(|t| t.tok) {
                Some(Tok::RBrace) => {
                    self.bump()?;
                    break;
                }
                Some(Tok::Ident(ref kw)) if eq_kw(kw, "GRAPH") => {
                    self.bump()?;
                    let name = self.parse_var_or_iri()?;
                    self.expect_token(&Tok::LBrace, "{")?;
                    let mut triples = Vec::new();
                    while !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RBrace)) {
                        self.parse_triples_block(&mut triples)?;
                        if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Dot)) {
                            self.bump()?;
                        } else {
                            break;
                        }
                    }
                    self.expect_token(&Tok::RBrace, "}")?;
                    out.push(QuadTriple {
                        graph: Some(name),
                        triples,
                    });
                }
                _ => {
                    let mut triples = Vec::new();
                    self.parse_triples_block(&mut triples)?;
                    if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Dot)) {
                        self.bump()?;
                    }
                    if !triples.is_empty() {
                        out.push(QuadTriple {
                            graph: None,
                            triples,
                        });
                    }
                }
            }
        }
        Ok(out)
    }

    // ---------- Group graph pattern -------------------------------------

    fn parse_group_graph_pattern(&mut self) -> Result<GroupGraphPattern, Diag> {
        self.expect_token(&Tok::LBrace, "{")?;
        // Sub-SELECT short-cut.
        if self.peek_keyword().as_deref() == Some("SELECT") {
            let sub = self.parse_query_body(None, Vec::new())?;
            self.expect_token(&Tok::RBrace, "}")?;
            let mut ggp = GroupGraphPattern::default();
            ggp.elements
                .push(GroupPatternElement::SubQuery(Box::new(sub)));
            return Ok(ggp);
        }
        let mut ggp = GroupGraphPattern::default();
        let mut scope: BTreeSet<String> = BTreeSet::new();
        loop {
            let Some(tok) = self.peek()? else {
                return Err(self.err_at_eof(
                    DiagnosticCode::UnexpectedEof,
                    "unterminated group graph pattern",
                ));
            };
            match tok.tok {
                Tok::RBrace => {
                    self.bump()?;
                    return Ok(ggp);
                }
                Tok::Dot => {
                    self.bump()?;
                    continue;
                }
                Tok::LBrace => {
                    // Could be nested group or start of UNION alternatives.
                    let first = self.parse_group_graph_pattern()?;
                    // UNION chain.
                    let mut alts = vec![first];
                    while self.peek_keyword().as_deref() == Some("UNION") {
                        self.bump()?;
                        alts.push(self.parse_group_graph_pattern()?);
                    }
                    if alts.len() > 1 {
                        ggp.elements.push(GroupPatternElement::Union(alts));
                    } else {
                        ggp.elements
                            .push(GroupPatternElement::Group(alts.pop().unwrap()));
                    }
                }
                Tok::Ident(ref s) => {
                    let kw_upper = s.to_ascii_uppercase();
                    match kw_upper.as_str() {
                        "BASE" | "PREFIX" => {
                            // Mid-body → FM5 / SPARQL-PROLOGUE-001.
                            return Err(Diag::fatal(
                                DiagnosticCode::Prologue,
                                format!(
                                    "{kw_upper} declaration is not allowed inside a group graph pattern (SPARQL 1.1 §4.1: Prologue ::= (BaseDecl | PrefixDecl)*)"
                                ),
                                tok.start,
                            ));
                        }
                        "OPTIONAL" => {
                            self.bump()?;
                            let inner = self.parse_group_graph_pattern()?;
                            ggp.elements.push(GroupPatternElement::Optional(inner));
                        }
                        "MINUS" => {
                            self.bump()?;
                            let inner = self.parse_group_graph_pattern()?;
                            ggp.elements.push(GroupPatternElement::Minus(inner));
                        }
                        "FILTER" => {
                            self.bump()?;
                            let expr = self.parse_filter_expr()?;
                            ggp.elements.push(GroupPatternElement::Filter(expr));
                        }
                        "BIND" => {
                            self.bump()?;
                            self.expect_token(&Tok::LParen, "(")?;
                            let expr = self.parse_expr()?;
                            if self.peek_keyword().as_deref() != Some("AS") {
                                return Err(self.err_here(
                                    DiagnosticCode::Syntax,
                                    "expected AS in BIND",
                                ));
                            }
                            self.bump()?;
                            let v = self.expect_next("variable in BIND")?;
                            let Tok::Var(var) = v.tok else {
                                return Err(Diag::fatal(
                                    DiagnosticCode::Syntax,
                                    "expected ?var after AS",
                                    v.start,
                                ));
                            };
                            self.expect_token(&Tok::RParen, ")")?;
                            if scope.contains(&var) {
                                return Err(Diag::fatal(
                                    DiagnosticCode::BindScope,
                                    format!(
                                        "BIND(... AS ?{var}) introduces variable already in scope (SPARQL 1.1 §18.2.1)"
                                    ),
                                    tok.start,
                                ));
                            }
                            scope.insert(var.clone());
                            ggp.elements.push(GroupPatternElement::Bind { expr, var });
                        }
                        "VALUES" => {
                            let data = self.parse_values_data()?;
                            for v in &data.vars {
                                scope.insert(v.clone());
                            }
                            ggp.elements.push(GroupPatternElement::Values(data));
                        }
                        "SERVICE" => {
                            self.bump()?;
                            let silent = self.eat_keyword("SILENT");
                            let endpoint = self.parse_var_or_iri()?;
                            let pattern = self.parse_group_graph_pattern()?;
                            ggp.elements.push(GroupPatternElement::Service {
                                silent,
                                endpoint,
                                pattern,
                            });
                        }
                        "GRAPH" => {
                            self.bump()?;
                            let name = self.parse_var_or_iri()?;
                            let pattern = self.parse_group_graph_pattern()?;
                            ggp.elements
                                .push(GroupPatternElement::Graph { name, pattern });
                        }
                        _ => {
                            // Treat as start of a triples block (could be
                            // an identifier start of a pname?).
                            let mut triples = Vec::new();
                            self.parse_triples_block(&mut triples)?;
                            collect_triples_scope(&triples, &mut scope);
                            ggp.elements.push(GroupPatternElement::Triples(triples));
                        }
                    }
                }
                _ => {
                    // Triples block.
                    let mut triples = Vec::new();
                    self.parse_triples_block(&mut triples)?;
                    collect_triples_scope(&triples, &mut scope);
                    ggp.elements.push(GroupPatternElement::Triples(triples));
                }
            }
        }
    }

    fn parse_filter_expr(&mut self) -> Result<Expr, Diag> {
        // FILTER expr — either a bracketed expression `(expr)` or a
        // BuiltInCall / function call; we just parse a full expression,
        // requiring parentheses around the expression form.
        if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::LParen)) {
            self.bump()?;
            let e = self.parse_expr()?;
            self.expect_token(&Tok::RParen, ")")?;
            return Ok(e);
        }
        self.parse_primary_expr()
    }

    fn parse_triples_block(&mut self, out: &mut Vec<TriplePattern>) -> Result<(), Diag> {
        // One or more TriplesSameSubjectPath, `.` separated.
        self.parse_triples_same_subject(out)?;
        while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Dot)) {
            self.bump()?; // `.`
            // Peek next token — if it isn't a triples subject, we're done.
            let Some(nxt) = self.peek()? else {
                return Ok(());
            };
            match nxt.tok {
                Tok::RBrace | Tok::LBrace => return Ok(()),
                Tok::Ident(ref s) => {
                    if matches!(
                        s.to_ascii_uppercase().as_str(),
                        "OPTIONAL"
                            | "MINUS"
                            | "FILTER"
                            | "BIND"
                            | "VALUES"
                            | "SERVICE"
                            | "GRAPH"
                            | "UNION"
                            | "BASE"
                            | "PREFIX"
                    ) {
                        return Ok(());
                    }
                    // identifier in subject position: `a` or pname prefix-like.
                    // Let the subject parser handle it.
                    self.parse_triples_same_subject(out)?;
                }
                Tok::Var(_)
                | Tok::IriRef(_)
                | Tok::Pname { .. }
                | Tok::LBracket
                | Tok::LParen
                | Tok::BNodeLabel(_)
                | Tok::AnonBNode
                | Tok::Nil => {
                    self.parse_triples_same_subject(out)?;
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    }

    fn parse_triples_same_subject(
        &mut self,
        out: &mut Vec<TriplePattern>,
    ) -> Result<(), Diag> {
        let subject = self.parse_triple_term(true)?;
        self.parse_property_list(subject, out)?;
        Ok(())
    }

    fn parse_property_list(
        &mut self,
        subject: TermOrPath,
        out: &mut Vec<TriplePattern>,
    ) -> Result<(), Diag> {
        loop {
            let predicate = self.parse_verb()?;
            // ObjectList: object (, object)*
            loop {
                let object = self.parse_triple_term(false)?;
                out.push(TriplePattern {
                    subject: subject.clone(),
                    predicate: predicate.clone(),
                    object,
                });
                if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Comma)) {
                    self.bump()?;
                } else {
                    break;
                }
            }
            if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Semicolon)) {
                self.bump()?;
                // After `;` we allow another predicate, but also allow
                // empty (`;` tail).
                if matches!(
                    self.peek()?.as_ref().map(|t| &t.tok),
                    Some(Tok::Dot) | Some(Tok::RBrace) | Some(Tok::RBracket) | None
                ) {
                    return Ok(());
                }
                continue;
            }
            return Ok(());
        }
    }

    fn parse_verb(&mut self) -> Result<PathOrPredicate, Diag> {
        // Variable predicate short-circuit (not a path).
        if let Some(Tok::Var(v)) = self.peek()?.map(|t| t.tok) {
            self.bump()?;
            return Ok(PathOrPredicate::Predicate(TermOrPath::Var(v)));
        }
        // `a` | Path.
        let path = self.parse_path()?;
        // Unwrap degenerate forms into cleaner variants.
        match path {
            Path::Prim(ref p) => match p.as_ref() {
                PathPrim::A => Ok(PathOrPredicate::A),
                PathPrim::Iri(iri) => Ok(PathOrPredicate::Predicate(TermOrPath::Iri(iri.clone()))),
                PathPrim::Prefixed { prefix, local } => {
                    Ok(PathOrPredicate::Predicate(TermOrPath::PrefixedName {
                        prefix: prefix.clone(),
                        local: local.clone(),
                    }))
                }
            },
            _ => Ok(PathOrPredicate::Path(path)),
        }
    }

    fn parse_path(&mut self) -> Result<Path, Diag> {
        // PathAlternative — `/` sequence separated by `|`.
        let mut alt = vec![self.parse_path_sequence()?];
        while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Pipe)) {
            self.bump()?;
            alt.push(self.parse_path_sequence()?);
        }
        if alt.len() == 1 {
            Ok(alt.pop().unwrap())
        } else {
            Ok(Path::Alt(alt))
        }
    }

    fn parse_path_sequence(&mut self) -> Result<Path, Diag> {
        let mut seq = vec![self.parse_path_elt_or_inverse()?];
        while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Slash)) {
            self.bump()?;
            seq.push(self.parse_path_elt_or_inverse()?);
        }
        if seq.len() == 1 {
            Ok(seq.pop().unwrap())
        } else {
            Ok(Path::Seq(seq))
        }
    }

    fn parse_path_elt_or_inverse(&mut self) -> Result<Path, Diag> {
        // `^` PathElt | PathElt
        // Per SPARQL-PATH-001 (FM9): `^` wraps the entire PathElt
        // *including* any negated property set that follows.
        if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Caret)) {
            self.bump()?;
            let inner = self.parse_path_elt()?;
            return Ok(Path::Inverse(Box::new(inner)));
        }
        self.parse_path_elt()
    }

    fn parse_path_elt(&mut self) -> Result<Path, Diag> {
        // PathPrimary + optional PathMod ( `?` | `*` | `+` ).
        let primary = self.parse_path_primary()?;
        let out = match self.peek()?.map(|t| t.tok) {
            Some(Tok::Question) => {
                self.bump()?;
                Path::Opt(Box::new(primary))
            }
            Some(Tok::Star) => {
                self.bump()?;
                Path::ZeroOrMore(Box::new(primary))
            }
            Some(Tok::Plus) => {
                self.bump()?;
                Path::OneOrMore(Box::new(primary))
            }
            _ => primary,
        };
        Ok(out)
    }

    fn parse_path_primary(&mut self) -> Result<Path, Diag> {
        let tok = self
            .peek()?
            .ok_or_else(|| self.err_at_eof(DiagnosticCode::UnexpectedEof, "path expected"))?;
        match tok.tok {
            Tok::IriRef(s) => {
                self.bump()?;
                Ok(Path::Prim(Box::new(PathPrim::Iri(s))))
            }
            Tok::Pname { prefix, local } => {
                self.bump()?;
                Ok(Path::Prim(Box::new(PathPrim::Prefixed { prefix, local })))
            }
            Tok::Ident(ref s) if s == "a" => {
                self.bump()?;
                Ok(Path::Prim(Box::new(PathPrim::A)))
            }
            Tok::Bang => {
                self.bump()?;
                // PathNegatedPropertySet: either `!elt` or `!( ... )`.
                if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::LParen)) {
                    self.bump()?;
                    let mut atoms = Vec::new();
                    if !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RParen)) {
                        atoms.push(self.parse_negated_atom()?);
                        while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Pipe)) {
                            self.bump()?;
                            atoms.push(self.parse_negated_atom()?);
                        }
                    }
                    self.expect_token(&Tok::RParen, ")")?;
                    Ok(Path::Negated(atoms))
                } else {
                    let atom = self.parse_negated_atom()?;
                    Ok(Path::Negated(vec![atom]))
                }
            }
            Tok::LParen => {
                self.bump()?;
                let p = self.parse_path()?;
                self.expect_token(&Tok::RParen, ")")?;
                Ok(p)
            }
            _ => Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "expected path element",
                tok.start,
            )),
        }
    }

    fn parse_negated_atom(&mut self) -> Result<NegatedAtom, Diag> {
        let inv = matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Caret));
        if inv {
            self.bump()?;
        }
        let tok = self
            .peek()?
            .ok_or_else(|| self.err_at_eof(DiagnosticCode::UnexpectedEof, "expected atom"))?;
        let atom = match tok.tok {
            Tok::IriRef(s) => {
                self.bump()?;
                PathPrim::Iri(s)
            }
            Tok::Pname { prefix, local } => {
                self.bump()?;
                PathPrim::Prefixed { prefix, local }
            }
            Tok::Ident(ref s) if s == "a" => {
                self.bump()?;
                PathPrim::A
            }
            _ => {
                return Err(Diag::fatal(
                    DiagnosticCode::Syntax,
                    "expected IRI / a in negated property set",
                    tok.start,
                ));
            }
        };
        Ok(if inv {
            NegatedAtom::Inv(atom)
        } else {
            NegatedAtom::Fwd(atom)
        })
    }

    fn parse_triple_term(&mut self, is_subject: bool) -> Result<TermOrPath, Diag> {
        let tok = self
            .peek()?
            .ok_or_else(|| self.err_at_eof(DiagnosticCode::UnexpectedEof, "term expected"))?;
        let t = tok.tok.clone();
        let start = tok.start;
        match t {
            Tok::IriRef(s) => {
                self.bump()?;
                Ok(TermOrPath::Iri(s))
            }
            Tok::Pname { prefix, local } => {
                self.bump()?;
                Ok(TermOrPath::PrefixedName { prefix, local })
            }
            Tok::Var(v) => {
                self.bump()?;
                if self.in_insert_data || self.in_delete_data {
                    return Err(Diag::fatal(
                        DiagnosticCode::UpdateDataForm,
                        "variables are forbidden inside INSERT DATA / DELETE DATA (§3.1.1 / §3.1.2)",
                        start,
                    ));
                }
                Ok(TermOrPath::Var(v))
            }
            Tok::BNodeLabel(l) => {
                self.bump()?;
                if self.in_delete_data {
                    return Err(Diag::fatal(
                        DiagnosticCode::UpdateDataForm,
                        "blank nodes are forbidden inside DELETE DATA (§3.1.2)",
                        start,
                    ));
                }
                Ok(TermOrPath::BNodeLabel(l))
            }
            Tok::AnonBNode => {
                self.bump()?;
                if self.in_delete_data {
                    return Err(Diag::fatal(
                        DiagnosticCode::UpdateDataForm,
                        "blank nodes are forbidden inside DELETE DATA (§3.1.2)",
                        start,
                    ));
                }
                Ok(TermOrPath::BNodeAnon)
            }
            Tok::Nil => {
                self.bump()?;
                Ok(TermOrPath::Nil)
            }
            Tok::LBracket => {
                self.bump()?;
                // Anonymous blank node with property list.
                let mut props = Vec::new();
                while !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RBracket)) {
                    let pred = self.parse_verb()?;
                    let mut objs = Vec::new();
                    objs.push(self.parse_triple_term(false)?);
                    while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Comma)) {
                        self.bump()?;
                        objs.push(self.parse_triple_term(false)?);
                    }
                    props.push((pred, objs));
                    if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Semicolon)) {
                        self.bump()?;
                    } else {
                        break;
                    }
                }
                self.expect_token(&Tok::RBracket, "]")?;
                Ok(TermOrPath::BNodePropertyList(props))
            }
            Tok::LParen => {
                self.bump()?;
                // Collection.
                let mut items = Vec::new();
                while !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RParen)) {
                    items.push(self.parse_triple_term(false)?);
                }
                self.expect_token(&Tok::RParen, ")")?;
                Ok(TermOrPath::Collection(items))
            }
            Tok::StringLit(s) => {
                self.bump()?;
                // Optional language tag or datatype.
                match self.peek()?.map(|t| t.tok) {
                    Some(Tok::LangTag(lt)) => {
                        self.bump()?;
                        Ok(TermOrPath::Literal(Literal {
                            lexical: s,
                            kind: LiteralKind::Lang(lt),
                        }))
                    }
                    Some(Tok::DataTypeMark) => {
                        self.bump()?;
                        let t = self.expect_next("datatype after ^^")?;
                        match t.tok {
                            Tok::IriRef(iri) => Ok(TermOrPath::Literal(Literal {
                                lexical: s,
                                kind: LiteralKind::Typed(iri),
                            })),
                            Tok::Pname { prefix, local } => {
                                Ok(TermOrPath::Literal(Literal {
                                    lexical: s,
                                    kind: LiteralKind::TypedPrefixed { prefix, local },
                                }))
                            }
                            _ => Err(Diag::fatal(
                                DiagnosticCode::Syntax,
                                "expected datatype IRI after ^^",
                                t.start,
                            )),
                        }
                    }
                    _ => Ok(TermOrPath::Literal(Literal {
                        lexical: s,
                        kind: LiteralKind::Simple,
                    })),
                }
            }
            Tok::NumberLit { lexeme, .. } => {
                self.bump()?;
                Ok(TermOrPath::NumericLit(lexeme))
            }
            Tok::Ident(ref s) if s == "true" || eq_kw(s, "true") => {
                self.bump()?;
                Ok(TermOrPath::BoolLit(true))
            }
            Tok::Ident(ref s) if s == "false" || eq_kw(s, "false") => {
                self.bump()?;
                Ok(TermOrPath::BoolLit(false))
            }
            Tok::Minus | Tok::Plus => {
                // Signed numeric literal.
                let sign = if matches!(t, Tok::Minus) { "-" } else { "+" };
                self.bump()?;
                let nxt = self.expect_next("number after sign")?;
                if let Tok::NumberLit { lexeme, .. } = nxt.tok {
                    Ok(TermOrPath::NumericLit(format!("{sign}{lexeme}")))
                } else {
                    Err(Diag::fatal(
                        DiagnosticCode::Syntax,
                        "expected number after sign",
                        nxt.start,
                    ))
                }
            }
            _ => {
                let _ = is_subject;
                Err(Diag::fatal(
                    DiagnosticCode::Syntax,
                    format!("unexpected token in term position: {t:?}"),
                    start,
                ))
            }
        }
    }

    // ---------- VALUES ---------------------------------------------------

    fn parse_values_data(&mut self) -> Result<InlineData, Diag> {
        // `VALUES` already at the front; consume the keyword.
        let kw = self.peek_keyword();
        if kw.as_deref() == Some("VALUES") {
            self.bump()?;
        }
        // Either `?var { ... }` (single variable) or `(vars) { rows }`.
        let mut vars = Vec::new();
        let mut rows = Vec::new();
        match self.peek()?.map(|t| t.tok) {
            Some(Tok::Var(v)) => {
                self.bump()?;
                vars.push(v);
                self.expect_token(&Tok::LBrace, "{")?;
                while !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RBrace)) {
                    let value = self.parse_data_value()?;
                    rows.push(vec![value]);
                }
                self.expect_token(&Tok::RBrace, "}")?;
            }
            Some(Tok::LParen) => {
                self.bump()?;
                while let Some(Tok::Var(v)) = self.peek()?.map(|t| t.tok) {
                    self.bump()?;
                    vars.push(v);
                }
                self.expect_token(&Tok::RParen, ")")?;
                self.expect_token(&Tok::LBrace, "{")?;
                while !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RBrace)) {
                    self.expect_token(&Tok::LParen, "(")?;
                    let mut row = Vec::new();
                    while !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RParen)) {
                        row.push(self.parse_data_value()?);
                    }
                    self.expect_token(&Tok::RParen, ")")?;
                    if row.len() != vars.len() {
                        return Err(self.err_here(
                            DiagnosticCode::Syntax,
                            "VALUES row arity mismatch",
                        ));
                    }
                    rows.push(row);
                }
                self.expect_token(&Tok::RBrace, "}")?;
            }
            _ => {
                return Err(self.err_here(
                    DiagnosticCode::Syntax,
                    "expected VALUES variable or ( vars )",
                ));
            }
        }
        Ok(InlineData { vars, rows })
    }

    fn parse_data_value(&mut self) -> Result<Option<TermOrPath>, Diag> {
        let tok = self
            .peek()?
            .ok_or_else(|| self.err_at_eof(DiagnosticCode::UnexpectedEof, "data value expected"))?;
        // UNDEF keyword → None.
        if let Tok::Ident(ref s) = tok.tok
            && eq_kw(s, "UNDEF")
        {
            self.bump()?;
            return Ok(None);
        }
        Ok(Some(self.parse_triple_term(false)?))
    }

    // ---------- Solution modifiers --------------------------------------

    fn parse_solution_modifiers(&mut self) -> Result<SolutionModifier, Diag> {
        let mut m = SolutionModifier::default();
        // GROUP BY
        if self.peek_keyword().as_deref() == Some("GROUP") {
            self.bump()?;
            if self.peek_keyword().as_deref() != Some("BY") {
                return Err(self.err_here(
                    DiagnosticCode::Syntax,
                    "expected BY after GROUP",
                ));
            }
            self.bump()?;
            loop {
                match self.peek()?.map(|t| t.tok) {
                    Some(Tok::Var(v)) => {
                        self.bump()?;
                        m.group_by.push(GroupCondition::Var(v));
                    }
                    Some(Tok::LParen) => {
                        self.bump()?;
                        let expr = self.parse_expr()?;
                        if self.peek_keyword().as_deref() == Some("AS") {
                            self.bump()?;
                            let v = self.expect_next("var after AS")?;
                            let Tok::Var(var) = v.tok else {
                                return Err(Diag::fatal(
                                    DiagnosticCode::Syntax,
                                    "expected variable",
                                    v.start,
                                ));
                            };
                            self.expect_token(&Tok::RParen, ")")?;
                            m.group_by.push(GroupCondition::ExprAs { expr, var });
                        } else {
                            self.expect_token(&Tok::RParen, ")")?;
                            m.group_by.push(GroupCondition::Expr(expr));
                        }
                    }
                    _ => {
                        if m.group_by.is_empty() {
                            return Err(self.err_here(
                                DiagnosticCode::Syntax,
                                "empty GROUP BY",
                            ));
                        }
                        break;
                    }
                }
            }
        }
        // HAVING
        while self.peek_keyword().as_deref() == Some("HAVING") {
            self.bump()?;
            self.expect_token(&Tok::LParen, "(")?;
            let expr = self.parse_expr()?;
            self.expect_token(&Tok::RParen, ")")?;
            m.having.push(expr);
        }
        // ORDER BY
        if self.peek_keyword().as_deref() == Some("ORDER") {
            self.bump()?;
            if self.peek_keyword().as_deref() != Some("BY") {
                return Err(self.err_here(
                    DiagnosticCode::Syntax,
                    "expected BY after ORDER",
                ));
            }
            self.bump()?;
            loop {
                let kw = self.peek_keyword();
                let desc = match kw.as_deref() {
                    Some("ASC") => {
                        self.bump()?;
                        false
                    }
                    Some("DESC") => {
                        self.bump()?;
                        true
                    }
                    _ => false,
                };
                let expr = if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::LParen)) {
                    self.bump()?;
                    let e = self.parse_expr()?;
                    self.expect_token(&Tok::RParen, ")")?;
                    e
                } else if let Some(tok) = self.peek()? {
                    if matches!(tok.tok, Tok::Var(_)) || matches!(tok.tok, Tok::Ident(_)) {
                        self.parse_primary_expr()?
                    } else {
                        break;
                    }
                } else {
                    break;
                };
                m.order_by.push(OrderCondition {
                    expr,
                    descending: desc,
                });
                // Continue while we see more order terms.
                let la = self.peek_keyword();
                if matches!(la.as_deref(), Some("ASC") | Some("DESC")) {
                    continue;
                }
                if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::LParen)) {
                    continue;
                }
                if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Var(_))) {
                    continue;
                }
                break;
            }
        }
        // LIMIT / OFFSET (either order).
        loop {
            match self.peek_keyword().as_deref() {
                Some("LIMIT") => {
                    self.bump()?;
                    let n = self.parse_u64()?;
                    m.limit = Some(n);
                }
                Some("OFFSET") => {
                    self.bump()?;
                    let n = self.parse_u64()?;
                    m.offset = Some(n);
                }
                _ => break,
            }
        }
        Ok(m)
    }

    fn parse_u64(&mut self) -> Result<u64, Diag> {
        let tok = self.expect_next("integer")?;
        if let Tok::NumberLit {
            kind: NumKind::Integer,
            lexeme,
        } = tok.tok
        {
            lexeme
                .parse::<u64>()
                .map_err(|_| Diag::fatal(DiagnosticCode::Syntax, "integer out of range", tok.start))
        } else {
            Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "expected integer",
                tok.start,
            ))
        }
    }

    // ---------- Expressions ---------------------------------------------

    fn parse_expr(&mut self) -> Result<Expr, Diag> {
        // ConditionalOrExpression (||).
        let mut left = self.parse_and_expr()?;
        while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::PipePipe)) {
            self.bump()?;
            let right = self.parse_and_expr()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, Diag> {
        let mut left = self.parse_rel_expr()?;
        while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::AmpAmp)) {
            self.bump()?;
            let right = self.parse_rel_expr()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_rel_expr(&mut self) -> Result<Expr, Diag> {
        let mut left = self.parse_add_expr()?;
        loop {
            match self.peek()?.map(|t| t.tok) {
                Some(Tok::Eq) => {
                    self.bump()?;
                    let right = self.parse_add_expr()?;
                    left = Expr::Eq(Box::new(left), Box::new(right));
                }
                Some(Tok::NotEq) => {
                    self.bump()?;
                    let right = self.parse_add_expr()?;
                    left = Expr::NotEq(Box::new(left), Box::new(right));
                }
                Some(Tok::Lt) => {
                    self.bump()?;
                    let right = self.parse_add_expr()?;
                    left = Expr::Lt(Box::new(left), Box::new(right));
                }
                Some(Tok::LtEq) => {
                    self.bump()?;
                    let right = self.parse_add_expr()?;
                    left = Expr::LtEq(Box::new(left), Box::new(right));
                }
                Some(Tok::Gt) => {
                    self.bump()?;
                    let right = self.parse_add_expr()?;
                    left = Expr::Gt(Box::new(left), Box::new(right));
                }
                Some(Tok::GtEq) => {
                    self.bump()?;
                    let right = self.parse_add_expr()?;
                    left = Expr::GtEq(Box::new(left), Box::new(right));
                }
                Some(Tok::Ident(ref s)) if eq_kw(s, "IN") => {
                    self.bump()?;
                    let args = self.parse_expr_list()?;
                    left = Expr::In(Box::new(left), args);
                }
                Some(Tok::Ident(ref s)) if eq_kw(s, "NOT") => {
                    // `NOT IN`
                    let save = self.lex.offset();
                    self.bump()?;
                    if self.peek_keyword().as_deref() == Some("IN") {
                        self.bump()?;
                        let args = self.parse_expr_list()?;
                        left = Expr::NotIn(Box::new(left), args);
                    } else {
                        // rollback not possible cheaply — require IN to follow NOT here.
                        let _ = save;
                        return Err(self.err_here(
                            DiagnosticCode::Syntax,
                            "expected IN after NOT",
                        ));
                    }
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_add_expr(&mut self) -> Result<Expr, Diag> {
        let mut left = self.parse_mul_expr()?;
        loop {
            match self.peek()?.map(|t| t.tok) {
                Some(Tok::Plus) => {
                    self.bump()?;
                    let right = self.parse_mul_expr()?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Some(Tok::Minus) => {
                    self.bump()?;
                    let right = self.parse_mul_expr()?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_mul_expr(&mut self) -> Result<Expr, Diag> {
        let mut left = self.parse_unary_expr()?;
        loop {
            match self.peek()?.map(|t| t.tok) {
                Some(Tok::Star) => {
                    self.bump()?;
                    let right = self.parse_unary_expr()?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
                Some(Tok::Slash) => {
                    self.bump()?;
                    let right = self.parse_unary_expr()?;
                    left = Expr::Div(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, Diag> {
        match self.peek()?.map(|t| t.tok) {
            Some(Tok::Bang) => {
                self.bump()?;
                let e = self.parse_unary_expr()?;
                Ok(Expr::UnaryNot(Box::new(e)))
            }
            Some(Tok::Plus) => {
                self.bump()?;
                let e = self.parse_unary_expr()?;
                Ok(Expr::UnaryPos(Box::new(e)))
            }
            Some(Tok::Minus) => {
                self.bump()?;
                let e = self.parse_unary_expr()?;
                Ok(Expr::UnaryNeg(Box::new(e)))
            }
            _ => self.parse_primary_expr(),
        }
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, Diag> {
        let tok = self
            .peek()?
            .ok_or_else(|| self.err_at_eof(DiagnosticCode::UnexpectedEof, "expression expected"))?;
        let start = tok.start;
        match tok.tok {
            Tok::LParen => {
                self.bump()?;
                let e = self.parse_expr()?;
                self.expect_token(&Tok::RParen, ")")?;
                Ok(e)
            }
            Tok::Var(v) => {
                self.bump()?;
                Ok(Expr::Var(v))
            }
            Tok::IriRef(iri) => {
                self.bump()?;
                // Possibly function call.
                if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::LParen) | Some(Tok::Nil))
                {
                    let args = self.parse_arg_list()?;
                    Ok(Expr::Func {
                        name: FuncName::Iri(iri),
                        args,
                        distinct: false,
                    })
                } else {
                    Ok(Expr::Iri(iri))
                }
            }
            Tok::Pname { prefix, local } => {
                self.bump()?;
                if matches!(
                    self.peek()?.as_ref().map(|t| &t.tok),
                    Some(Tok::LParen) | Some(Tok::Nil)
                ) {
                    let args = self.parse_arg_list()?;
                    Ok(Expr::Func {
                        name: FuncName::Prefixed { prefix, local },
                        args,
                        distinct: false,
                    })
                } else {
                    Ok(Expr::Prefixed { prefix, local })
                }
            }
            Tok::StringLit(s) => {
                self.bump()?;
                let kind = match self.peek()?.map(|t| t.tok) {
                    Some(Tok::LangTag(lt)) => {
                        self.bump()?;
                        LiteralKind::Lang(lt)
                    }
                    Some(Tok::DataTypeMark) => {
                        self.bump()?;
                        let t = self.expect_next("datatype IRI")?;
                        match t.tok {
                            Tok::IriRef(iri) => LiteralKind::Typed(iri),
                            Tok::Pname { prefix, local } => {
                                LiteralKind::TypedPrefixed { prefix, local }
                            }
                            _ => {
                                return Err(Diag::fatal(
                                    DiagnosticCode::Syntax,
                                    "expected datatype IRI",
                                    t.start,
                                ));
                            }
                        }
                    }
                    _ => LiteralKind::Simple,
                };
                Ok(Expr::Literal(Literal { lexical: s, kind }))
            }
            Tok::NumberLit { lexeme, .. } => {
                self.bump()?;
                Ok(Expr::NumericLit(lexeme))
            }
            Tok::Ident(ref s) => {
                let kw = s.to_ascii_uppercase();
                if kw == "TRUE" || kw == "FALSE" {
                    self.bump()?;
                    return Ok(Expr::BoolLit(kw == "TRUE"));
                }
                if kw == "EXISTS" {
                    self.bump()?;
                    let p = self.parse_group_graph_pattern()?;
                    return Ok(Expr::Exists(p));
                }
                if kw == "NOT" {
                    self.bump()?;
                    if self.peek_keyword().as_deref() == Some("EXISTS") {
                        self.bump()?;
                        let p = self.parse_group_graph_pattern()?;
                        return Ok(Expr::NotExists(p));
                    }
                    return Err(self.err_here(
                        DiagnosticCode::Syntax,
                        "expected EXISTS after NOT",
                    ));
                }
                // Built-in / aggregate function.
                self.bump()?;
                // COUNT(DISTINCT *) or COUNT(DISTINCT expr)
                if eq_kw(&kw, "COUNT") {
                    self.expect_token(&Tok::LParen, "(")?;
                    let distinct = self.eat_keyword("DISTINCT");
                    if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Star)) {
                        self.bump()?;
                        self.expect_token(&Tok::RParen, ")")?;
                        return Ok(Expr::CountStar { distinct });
                    }
                    let arg = self.parse_expr()?;
                    self.expect_token(&Tok::RParen, ")")?;
                    return Ok(Expr::Func {
                        name: FuncName::Builtin("COUNT".to_owned()),
                        args: vec![arg],
                        distinct,
                    });
                }
                // Other aggregates that accept DISTINCT.
                if matches!(
                    kw.as_str(),
                    "SUM" | "MIN" | "MAX" | "AVG" | "SAMPLE" | "GROUP_CONCAT"
                ) {
                    self.expect_token(&Tok::LParen, "(")?;
                    let distinct = self.eat_keyword("DISTINCT");
                    let arg = self.parse_expr()?;
                    // Handle optional GROUP_CONCAT SEPARATOR.
                    if kw == "GROUP_CONCAT" && matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Semicolon))
                    {
                        self.bump()?;
                        if self.peek_keyword().as_deref() != Some("SEPARATOR") {
                            return Err(self.err_here(
                                DiagnosticCode::Syntax,
                                "expected SEPARATOR",
                            ));
                        }
                        self.bump()?;
                        self.expect_token(&Tok::Eq, "=")?;
                        let sep = self.expect_next("string after SEPARATOR")?;
                        let _ = sep; // we accept any literal
                    }
                    self.expect_token(&Tok::RParen, ")")?;
                    return Ok(Expr::Func {
                        name: FuncName::Builtin(kw),
                        args: vec![arg],
                        distinct,
                    });
                }
                // Generic built-in: identifier followed by `(args)` or NIL.
                if matches!(
                    self.peek()?.as_ref().map(|t| &t.tok),
                    Some(Tok::LParen) | Some(Tok::Nil)
                ) {
                    let args = self.parse_arg_list()?;
                    return Ok(Expr::Func {
                        name: FuncName::Builtin(kw),
                        args,
                        distinct: false,
                    });
                }
                // Bare identifier (keyword) that isn't a function — treat
                // as syntax error.
                Err(Diag::fatal(
                    DiagnosticCode::Syntax,
                    format!("unexpected identifier {s:?} in expression"),
                    start,
                ))
            }
            Tok::Nil => {
                self.bump()?;
                // RDF nil — but in expression context only legal as part
                // of a function call already consumed. Report as error here.
                Err(Diag::fatal(
                    DiagnosticCode::Syntax,
                    "() not valid in expression position",
                    start,
                ))
            }
            _ => Err(Diag::fatal(
                DiagnosticCode::Syntax,
                format!("unexpected token in expression: {:?}", tok.tok),
                start,
            )),
        }
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, Diag> {
        // `( DISTINCT? arg (, arg)* )` or NIL.
        if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Nil)) {
            self.bump()?;
            return Ok(Vec::new());
        }
        self.expect_token(&Tok::LParen, "(")?;
        let _ = self.eat_keyword("DISTINCT");
        let mut args = Vec::new();
        if !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RParen)) {
            args.push(self.parse_expr()?);
            while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Comma)) {
                self.bump()?;
                args.push(self.parse_expr()?);
            }
        }
        self.expect_token(&Tok::RParen, ")")?;
        Ok(args)
    }

    fn parse_expr_list(&mut self) -> Result<Vec<Expr>, Diag> {
        // `( expr (, expr)* )` — same as arg list but without DISTINCT.
        if matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Nil)) {
            self.bump()?;
            return Ok(Vec::new());
        }
        self.expect_token(&Tok::LParen, "(")?;
        let mut args = Vec::new();
        if !matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::RParen)) {
            args.push(self.parse_expr()?);
            while matches!(self.peek()?.as_ref().map(|t| &t.tok), Some(Tok::Comma)) {
                self.bump()?;
                args.push(self.parse_expr()?);
            }
        }
        self.expect_token(&Tok::RParen, ")")?;
        Ok(args)
    }

    // ---------- Low-level helpers ---------------------------------------

    fn parse_iri(&mut self) -> Result<String, Diag> {
        let tok = self.expect_next("IRI")?;
        match tok.tok {
            Tok::IriRef(s) => Ok(s),
            _ => Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "expected <IRI>",
                tok.start,
            )),
        }
    }

    fn parse_var_or_iri(&mut self) -> Result<VarOrIri, Diag> {
        let tok = self.expect_next("var or IRI")?;
        match tok.tok {
            Tok::Var(v) => Ok(VarOrIri::Var(v)),
            Tok::IriRef(s) => Ok(VarOrIri::Iri(s)),
            Tok::Pname { prefix, local } => Ok(VarOrIri::Prefixed { prefix, local }),
            _ => Err(Diag::fatal(
                DiagnosticCode::Syntax,
                "expected ?var or IRI",
                tok.start,
            )),
        }
    }

    fn peek(&mut self) -> Result<Option<Spanned>, Diag> {
        if self.peeked.is_none() {
            self.peeked = self.lex.next_tok()?;
        }
        Ok(self.peeked.clone())
    }

    fn peek_keyword(&mut self) -> Option<String> {
        let tok = self.peek().ok().flatten()?;
        token_keyword(&tok.tok)
    }

    fn bump(&mut self) -> Result<Spanned, Diag> {
        let tok = if let Some(t) = self.peeked.take() {
            t
        } else {
            self.lex
                .next_tok()?
                .ok_or_else(|| self.err_at_eof(DiagnosticCode::UnexpectedEof, "unexpected EOF"))?
        };
        self.last_end = tok.end;
        Ok(tok)
    }

    fn expect_next(&mut self, what: &str) -> Result<Spanned, Diag> {
        self.bump().map_err(|mut d| {
            d.message = format!("expected {what}: {}", d.message);
            d
        })
    }

    fn expect_token(&mut self, want: &Tok, label: &str) -> Result<(), Diag> {
        let tok = self.bump()?;
        if std::mem::discriminant(&tok.tok) != std::mem::discriminant(want) {
            return Err(Diag::fatal(
                DiagnosticCode::Syntax,
                format!("expected `{label}`, got {:?}", tok.tok),
                tok.start,
            ));
        }
        Ok(())
    }

    fn eat_keyword(&mut self, kw: &str) -> bool {
        if self.peek_keyword().as_deref().map(str::to_ascii_uppercase).as_deref() == Some(kw) {
            let _ = self.bump();
            true
        } else {
            false
        }
    }

    fn expect_eof(&mut self) -> Result<(), Diag> {
        if let Some(tok) = self.peek()? {
            return Err(Diag::fatal(
                DiagnosticCode::Syntax,
                format!("trailing tokens after request: {:?}", tok.tok),
                tok.start,
            ));
        }
        Ok(())
    }

    fn err_here(&mut self, code: DiagnosticCode, msg: impl Into<String>) -> Diag {
        let off = self.peek().ok().flatten().map_or(self.last_end, |t| t.start);
        Diag::fatal(code, msg, off)
    }

    fn err_at_eof(&self, code: DiagnosticCode, msg: impl Into<String>) -> Diag {
        Diag::fatal(code, msg, self.last_end)
    }
}

fn token_keyword(t: &Tok) -> Option<String> {
    match t {
        Tok::Ident(s) => Some(s.to_ascii_uppercase()),
        _ => None,
    }
}

fn eq_kw(s: &str, kw: &str) -> bool {
    s.eq_ignore_ascii_case(kw)
}

fn collect_triples_scope(triples: &[TriplePattern], scope: &mut BTreeSet<String>) {
    for t in triples {
        collect_term_scope(&t.subject, scope);
        if let PathOrPredicate::Predicate(p) = &t.predicate {
            collect_term_scope(p, scope);
        }
        collect_term_scope(&t.object, scope);
    }
}

fn collect_term_scope(t: &TermOrPath, scope: &mut BTreeSet<String>) {
    match t {
        TermOrPath::Var(v) => {
            scope.insert(v.clone());
        }
        TermOrPath::Collection(items) => {
            for i in items {
                collect_term_scope(i, scope);
            }
        }
        TermOrPath::BNodePropertyList(props) => {
            for (_, objs) in props {
                for o in objs {
                    collect_term_scope(o, scope);
                }
            }
        }
        _ => {}
    }
}
