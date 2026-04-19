//! Recursive-descent SPARQL 1.1 parser.
//!
//! Consumes the token stream produced by [`crate::lexer`] and builds the
//! [`crate::ast`] tree. The grammar follows W3C SPARQL 1.1 Query Language
//! recommendation §19 and SPARQL 1.1 Update §4.
//!
//! This is an independent second implementation; it deliberately uses a
//! different approach from any "main" parser: a simple hand-written
//! recursive descent with a `Cursor` over a token slice.

use crate::{
    ast::*,
    lexer::{Token, TokenKind},
    ParseError,
};

/// Parse a SPARQL document (query or update) from a token slice.
///
/// Returns `(Document, warnings)`. Warnings is a `Vec<String>` of
/// non-fatal messages (e.g. deprecated syntax detected).
///
/// # Errors
///
/// Returns [`ParseError`] on any unrecoverable syntax error.
pub fn parse_document(
    tokens: &[Token],
    _src: &str,
) -> Result<(Document, Vec<String>), ParseError> {
    let mut cursor = Cursor::new(tokens);
    let prologue = parse_prologue(&mut cursor)?;

    // Determine if this is a query or update by peeking at the first
    // significant token after the prologue.
    let body = match cursor.peek_kind() {
        TokenKind::Select
        | TokenKind::Construct
        | TokenKind::Ask
        | TokenKind::Describe => {
            let query = parse_query(&mut cursor, &prologue)?;
            DocumentBody::Query(Box::new(query))
        }
        // Update keywords
        TokenKind::Insert
        | TokenKind::Delete
        | TokenKind::Load
        | TokenKind::Clear
        | TokenKind::Create
        | TokenKind::Drop
        | TokenKind::Copy
        | TokenKind::Move
        | TokenKind::Add
        | TokenKind::With => {
            let ops = parse_update_ops(&mut cursor)?;
            DocumentBody::Update(ops)
        }
        TokenKind::Eof => {
            // Empty document — treat as empty update
            DocumentBody::Update(vec![])
        }
        _ => {
            let tok = cursor.peek();
            return Err(ParseError::syntax(
                &tok.text,
                "expected SELECT, CONSTRUCT, ASK, DESCRIBE, or update keyword",
            ));
        }
    };

    cursor.expect_eof()?;

    Ok((Document { prologue, body }, vec![]))
}

// ── Prologue ────────────────────────────────────────────────────────────────

fn parse_prologue(cursor: &mut Cursor<'_>) -> Result<Prologue, ParseError> {
    let mut prologue = Prologue::default();
    loop {
        match cursor.peek_kind() {
            TokenKind::Base => {
                cursor.advance();
                let iri_tok = cursor.expect_kind(TokenKind::IriRef, "IRI after BASE")?;
                prologue.base = Some(strip_angle_brackets(&iri_tok.text));
            }
            TokenKind::Prefix => {
                cursor.advance();
                // Expect a prefixed-name token like "ex:" OR just ":"
                let prefix_tok = cursor.peek().clone();
                // The prefix name is either PrefixedName (e.g. "ex:") with empty local,
                // or a plain identifier followed by ':'
                let prefix_label = match &prefix_tok.kind {
                    TokenKind::PrefixedName => {
                        // e.g. "ex:" — strip the trailing colon and local part
                        cursor.advance();
                        let text = &prefix_tok.text;
                        if let Some(colon_pos) = text.find(':') {
                            text[..colon_pos].to_owned()
                        } else {
                            text.clone()
                        }
                    }
                    TokenKind::Eof => {
                        return Err(ParseError::syntax("EOF", "expected prefix name after PREFIX"));
                    }
                    _ => {
                        // Might be ":" (default prefix)
                        cursor.advance();
                        prefix_tok.text.trim_end_matches(':').to_owned()
                    }
                };
                let iri_tok = cursor.expect_kind(TokenKind::IriRef, "IRI after PREFIX")?;
                prologue.prefixes.push(PrefixDecl {
                    prefix: prefix_label,
                    iri: strip_angle_brackets(&iri_tok.text),
                });
            }
            _ => break,
        }
    }
    Ok(prologue)
}

// ── Query ───────────────────────────────────────────────────────────────────

fn parse_query(cursor: &mut Cursor<'_>, _prologue: &Prologue) -> Result<Query, ParseError> {
    let form = match cursor.peek_kind() {
        TokenKind::Select => parse_select_clause(cursor)?,
        TokenKind::Construct => parse_construct_clause(cursor)?,
        TokenKind::Ask => {
            cursor.advance();
            QueryForm::Ask
        }
        TokenKind::Describe => parse_describe_clause(cursor)?,
        _ => {
            let tok = cursor.peek();
            return Err(ParseError::syntax(
                &tok.text,
                "expected SELECT, CONSTRUCT, ASK, or DESCRIBE",
            ));
        }
    };

    let dataset = parse_dataset_clauses(cursor)?;

    // WHERE clause (optional keyword for ASK/DESCRIBE)
    let where_clause = if cursor.peek_kind() == TokenKind::Where {
        cursor.advance();
        parse_group_graph_pattern(cursor)?
    } else if cursor.peek_kind() == TokenKind::LBrace {
        parse_group_graph_pattern(cursor)?
    } else {
        // No WHERE — allowed for ASK and DESCRIBE
        GroupGraphPattern::Group(vec![])
    };

    let modifiers = parse_solution_modifiers(cursor)?;

    let values = if cursor.peek_kind() == TokenKind::Values {
        Some(parse_values_clause(cursor)?)
    } else {
        None
    };

    Ok(Query {
        dataset,
        form,
        where_clause,
        modifiers,
        values,
    })
}

fn parse_select_clause(cursor: &mut Cursor<'_>) -> Result<QueryForm, ParseError> {
    cursor.expect_kind(TokenKind::Select, "SELECT")?;

    let modifier = match cursor.peek_kind() {
        TokenKind::Distinct => {
            cursor.advance();
            Some(SelectModifier::Distinct)
        }
        TokenKind::Reduced => {
            cursor.advance();
            Some(SelectModifier::Reduced)
        }
        _ => None,
    };

    let projection = if cursor.peek_kind() == TokenKind::Star {
        cursor.advance();
        SelectProjection::Star
    } else {
        let mut vars = Vec::new();
        loop {
            match cursor.peek_kind() {
                TokenKind::Variable => {
                    let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
                    vars.push(SelectVar::Var(v));
                }
                TokenKind::LParen => {
                    cursor.advance();
                    let expr = parse_expr(cursor)?;
                    cursor.expect_kind(TokenKind::As, "AS")?;
                    let alias_tok = cursor.expect_kind(TokenKind::Variable, "?alias")?;
                    let alias = alias_tok.text.trim_start_matches(['?', '$']).to_owned();
                    cursor.expect_kind(TokenKind::RParen, ")")?;
                    vars.push(SelectVar::Alias { expr, alias });
                }
                _ => break,
            }
        }
        if vars.is_empty() {
            return Err(ParseError::syntax(
                &cursor.peek().text,
                "expected variable or * in SELECT",
            ));
        }
        SelectProjection::Vars(vars)
    };

    Ok(QueryForm::Select(SelectClause {
        modifier,
        projection,
    }))
}

fn parse_construct_clause(cursor: &mut Cursor<'_>) -> Result<QueryForm, ParseError> {
    cursor.expect_kind(TokenKind::Construct, "CONSTRUCT")?;

    if cursor.peek_kind() == TokenKind::LBrace {
        cursor.advance();
        let template = parse_triple_patterns_in_block(cursor)?;
        cursor.expect_kind(TokenKind::RBrace, "}")?;
        Ok(QueryForm::Construct(ConstructTemplate::Template(template)))
    } else if cursor.peek_kind() == TokenKind::Where {
        Ok(QueryForm::Construct(ConstructTemplate::Where))
    } else {
        // CONSTRUCT WHERE shorthand
        Ok(QueryForm::Construct(ConstructTemplate::Where))
    }
}

fn parse_describe_clause(cursor: &mut Cursor<'_>) -> Result<QueryForm, ParseError> {
    cursor.expect_kind(TokenKind::Describe, "DESCRIBE")?;

    let mut items = Vec::new();
    if cursor.peek_kind() == TokenKind::Star {
        cursor.advance();
        // DESCRIBE * — no items
    } else {
        loop {
            match cursor.peek_kind() {
                TokenKind::Variable => {
                    let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
                    items.push(VarOrIri::Var(v));
                }
                TokenKind::IriRef => {
                    let t = cursor.advance().text.clone();
                    items.push(VarOrIri::Iri(Iri::Absolute(t)));
                }
                TokenKind::PrefixedName => {
                    let t = cursor.advance().text.clone();
                    items.push(VarOrIri::Iri(Iri::Prefixed(t)));
                }
                _ => break,
            }
        }
    }

    Ok(QueryForm::Describe(items))
}

fn parse_dataset_clauses(cursor: &mut Cursor<'_>) -> Result<Vec<DatasetClause>, ParseError> {
    let mut clauses = Vec::new();
    loop {
        if cursor.peek_kind() != TokenKind::From {
            break;
        }
        cursor.advance();
        let named = if cursor.peek_kind() == TokenKind::Named {
            cursor.advance();
            true
        } else {
            false
        };
        let iri = parse_iri(cursor)?;
        clauses.push(DatasetClause { iri, named });
    }
    Ok(clauses)
}

fn parse_solution_modifiers(cursor: &mut Cursor<'_>) -> Result<SolutionModifiers, ParseError> {
    let mut mods = SolutionModifiers::default();

    // GROUP BY
    if cursor.peek_kind() == TokenKind::Group {
        cursor.advance();
        cursor.expect_kind(TokenKind::By, "BY after GROUP")?;
        loop {
            match cursor.peek_kind() {
                TokenKind::Variable => {
                    let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
                    mods.group_by.push(GroupCondition::Var(v));
                }
                TokenKind::LParen => {
                    cursor.advance();
                    let expr = parse_expr(cursor)?;
                    let alias = if cursor.peek_kind() == TokenKind::As {
                        cursor.advance();
                        let a = cursor.expect_kind(TokenKind::Variable, "?alias")?;
                        Some(a.text.trim_start_matches(['?', '$']).to_owned())
                    } else {
                        None
                    };
                    cursor.expect_kind(TokenKind::RParen, ")")?;
                    mods.group_by.push(GroupCondition::Expr { expr, alias });
                }
                _ => break,
            }
        }
    }

    // HAVING
    if cursor.peek_kind() == TokenKind::Having {
        cursor.advance();
        loop {
            // Parse one or more constraint expressions
            match cursor.peek_kind() {
                TokenKind::LParen | TokenKind::Bang | TokenKind::Variable
                | TokenKind::IriRef | TokenKind::PrefixedName | TokenKind::StringLiteral
                | TokenKind::IntegerLiteral | TokenKind::DecimalLiteral
                | TokenKind::DoubleLiteral | TokenKind::True | TokenKind::False => {
                    let e = parse_expr(cursor)?;
                    mods.having.push(e);
                }
                _ => break,
            }
        }
    }

    // ORDER BY
    if cursor.peek_kind() == TokenKind::Order {
        cursor.advance();
        cursor.expect_kind(TokenKind::By, "BY after ORDER")?;
        loop {
            match cursor.peek_kind() {
                TokenKind::Asc => {
                    cursor.advance();
                    cursor.expect_kind(TokenKind::LParen, "(")?;
                    let expr = parse_expr(cursor)?;
                    cursor.expect_kind(TokenKind::RParen, ")")?;
                    mods.order_by.push(OrderCondition {
                        direction: OrderDirection::Asc,
                        expr,
                    });
                }
                TokenKind::Desc => {
                    cursor.advance();
                    cursor.expect_kind(TokenKind::LParen, "(")?;
                    let expr = parse_expr(cursor)?;
                    cursor.expect_kind(TokenKind::RParen, ")")?;
                    mods.order_by.push(OrderCondition {
                        direction: OrderDirection::Desc,
                        expr,
                    });
                }
                TokenKind::Variable => {
                    let v = cursor.advance().text.clone();
                    mods.order_by.push(OrderCondition {
                        direction: OrderDirection::Asc,
                        expr: Expr::Var(v.trim_start_matches(['?', '$']).to_owned()),
                    });
                }
                TokenKind::LParen => {
                    let expr = parse_expr(cursor)?;
                    mods.order_by.push(OrderCondition {
                        direction: OrderDirection::Asc,
                        expr,
                    });
                }
                _ => break,
            }
        }
    }

    // LIMIT
    if cursor.peek_kind() == TokenKind::Limit {
        cursor.advance();
        let n = cursor.expect_kind(TokenKind::IntegerLiteral, "integer after LIMIT")?;
        mods.limit = n.text.parse::<u64>().ok();
    }

    // OFFSET
    if cursor.peek_kind() == TokenKind::Offset {
        cursor.advance();
        let n = cursor.expect_kind(TokenKind::IntegerLiteral, "integer after OFFSET")?;
        mods.offset = n.text.parse::<u64>().ok();
    }

    Ok(mods)
}

fn parse_values_clause(cursor: &mut Cursor<'_>) -> Result<ValuesClause, ParseError> {
    cursor.expect_kind(TokenKind::Values, "VALUES")?;
    parse_inline_data_body(cursor)
}

fn parse_inline_data_body(cursor: &mut Cursor<'_>) -> Result<ValuesClause, ParseError> {
    // Two forms: ValuesClauseShort (single var) or ValuesClauseUnit (multi-var)
    if cursor.peek_kind() == TokenKind::Variable {
        // Single var shorthand: VALUES ?x { ... }
        let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
        cursor.expect_kind(TokenKind::LBrace, "{")?;
        let mut rows = Vec::new();
        loop {
            match cursor.peek_kind() {
                TokenKind::RBrace => break,
                TokenKind::Undef => {
                    cursor.advance();
                    rows.push(vec![None]);
                }
                _ => {
                    let term = parse_rdf_term(cursor)?;
                    rows.push(vec![Some(term)]);
                }
            }
        }
        cursor.expect_kind(TokenKind::RBrace, "}")?;
        Ok(ValuesClause {
            vars: vec![v],
            rows,
        })
    } else {
        // Multi-var: VALUES (?x ?y) { (...) (...) }
        cursor.expect_kind(TokenKind::LParen, "( after VALUES")?;
        let mut vars = Vec::new();
        while cursor.peek_kind() == TokenKind::Variable {
            let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
            vars.push(v);
        }
        cursor.expect_kind(TokenKind::RParen, ")")?;
        cursor.expect_kind(TokenKind::LBrace, "{")?;
        let mut rows = Vec::new();
        loop {
            match cursor.peek_kind() {
                TokenKind::RBrace => break,
                TokenKind::LParen => {
                    cursor.advance();
                    let mut row = Vec::new();
                    loop {
                        match cursor.peek_kind() {
                            TokenKind::RParen => break,
                            TokenKind::Undef => {
                                cursor.advance();
                                row.push(None);
                            }
                            _ => {
                                let term = parse_rdf_term(cursor)?;
                                row.push(Some(term));
                            }
                        }
                    }
                    cursor.expect_kind(TokenKind::RParen, ")")?;
                    rows.push(row);
                }
                _ => break,
            }
        }
        cursor.expect_kind(TokenKind::RBrace, "}")?;
        Ok(ValuesClause { vars, rows })
    }
}

// ── Group Graph Pattern ──────────────────────────────────────────────────────

fn parse_group_graph_pattern(cursor: &mut Cursor<'_>) -> Result<GroupGraphPattern, ParseError> {
    cursor.expect_kind(TokenKind::LBrace, "{")?;

    // Check for subquery: SELECT inside
    if cursor.peek_kind() == TokenKind::Select {
        let prologue = Prologue::default();
        let query = parse_query(cursor, &prologue)?;
        cursor.expect_kind(TokenKind::RBrace, "}")?;
        return Ok(GroupGraphPattern::SubQuery(Box::new(query)));
    }

    let elements = parse_graph_pattern_elements(cursor)?;
    cursor.expect_kind(TokenKind::RBrace, "}")?;
    Ok(GroupGraphPattern::Group(elements))
}

#[allow(clippy::too_many_lines)]
fn parse_graph_pattern_elements(
    cursor: &mut Cursor<'_>,
) -> Result<Vec<GraphPatternElement>, ParseError> {
    let mut elements: Vec<GraphPatternElement> = Vec::new();

    loop {
        match cursor.peek_kind() {
            TokenKind::RBrace | TokenKind::Eof => break,

            TokenKind::Optional => {
                cursor.advance();
                let pat = parse_group_graph_pattern(cursor)?;
                elements.push(GraphPatternElement::Optional(pat));
            }

            TokenKind::MinusKw => {
                cursor.advance();
                let pat = parse_group_graph_pattern(cursor)?;
                elements.push(GraphPatternElement::Minus(pat));
            }

            TokenKind::Filter => {
                cursor.advance();
                let expr = if cursor.peek_kind() == TokenKind::LParen {
                    // Could be a constraint or a function call with parens
                    cursor.advance();
                    let e = parse_expr(cursor)?;
                    cursor.expect_kind(TokenKind::RParen, ")")?;
                    e
                } else {
                    parse_expr(cursor)?
                };
                elements.push(GraphPatternElement::Filter(expr));
            }

            TokenKind::Bind => {
                cursor.advance();
                cursor.expect_kind(TokenKind::LParen, "( after BIND")?;
                let expr = parse_expr(cursor)?;
                cursor.expect_kind(TokenKind::As, "AS")?;
                let var_tok = cursor.expect_kind(TokenKind::Variable, "?var")?;
                let var = var_tok.text.trim_start_matches(['?', '$']).to_owned();
                cursor.expect_kind(TokenKind::RParen, ")")?;
                elements.push(GraphPatternElement::Bind { expr, var });
            }

            TokenKind::Graph => {
                cursor.advance();
                let name = parse_var_or_iri(cursor)?;
                let pattern = parse_group_graph_pattern(cursor)?;
                elements.push(GraphPatternElement::Graph { name, pattern });
            }

            TokenKind::Service => {
                cursor.advance();
                let silent = if cursor.peek_kind() == TokenKind::Silent {
                    cursor.advance();
                    true
                } else {
                    false
                };
                let endpoint = parse_var_or_iri(cursor)?;
                let pattern = parse_group_graph_pattern(cursor)?;
                elements.push(GraphPatternElement::Service {
                    endpoint,
                    silent,
                    pattern,
                });
            }

            TokenKind::Values => {
                cursor.advance();
                let data = parse_inline_data_body(cursor)?;
                elements.push(GraphPatternElement::InlineData(data));
            }

            TokenKind::LBrace => {
                // Could be a nested group or start of a UNION chain
                let pat = parse_group_graph_pattern(cursor)?;
                if cursor.peek_kind() == TokenKind::Union {
                    // UNION chain
                    let mut left = pat;
                    while cursor.peek_kind() == TokenKind::Union {
                        cursor.advance();
                        let right = parse_group_graph_pattern(cursor)?;
                        left = GroupGraphPattern::Group(vec![
                            GraphPatternElement::Union(left, right)
                        ]);
                    }
                    match left {
                        GroupGraphPattern::Group(mut inner) => {
                            elements.append(&mut inner);
                        }
                        other => {
                            // Wrap as a union element
                            elements.push(GraphPatternElement::Optional(other));
                        }
                    }
                } else {
                    // Inline the inner group
                    match pat {
                        GroupGraphPattern::Group(mut inner) => elements.append(&mut inner),
                        sub => elements.push(GraphPatternElement::Optional(sub)),
                    }
                }
            }

            _ => {
                // Triple pattern
                match parse_triples_block(cursor) {
                    Ok(mut triples) => {
                        for t in triples.drain(..) {
                            elements.push(GraphPatternElement::Triple(t));
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }

    Ok(elements)
}

fn parse_triples_block(cursor: &mut Cursor<'_>) -> Result<Vec<TriplePattern>, ParseError> {
    let mut triples = Vec::new();
    loop {
        match cursor.peek_kind() {
            TokenKind::RBrace
            | TokenKind::Optional
            | TokenKind::Filter
            | TokenKind::Bind
            | TokenKind::Graph
            | TokenKind::Service
            | TokenKind::Values
            | TokenKind::Union
            | TokenKind::MinusKw
            | TokenKind::LBrace
            | TokenKind::Eof => break,
            _ => {}
        }
        let subject = parse_term(cursor)?;
        let pred_objs = parse_pred_obj_list(cursor)?;
        for (predicate, object) in pred_objs {
            triples.push(TriplePattern {
                subject: subject.clone(),
                predicate,
                object,
            });
        }
        // optional trailing dot
        if cursor.peek_kind() == TokenKind::Dot {
            cursor.advance();
        } else {
            break;
        }
    }
    Ok(triples)
}

fn parse_pred_obj_list(cursor: &mut Cursor<'_>) -> Result<Vec<(Predicate, Term)>, ParseError> {
    let mut pairs = Vec::new();
    loop {
        let pred = parse_predicate(cursor)?;
        let objects = parse_object_list(cursor)?;
        for obj in objects {
            pairs.push((pred.clone(), obj));
        }
        if cursor.peek_kind() == TokenKind::Semi {
            cursor.advance();
            // ';' may be followed by another pred-obj pair or end
            match cursor.peek_kind() {
                TokenKind::Dot | TokenKind::RBrace | TokenKind::Eof | TokenKind::RBracket => break,
                _ => continue,
            }
        } else {
            break;
        }
    }
    Ok(pairs)
}

fn parse_object_list(cursor: &mut Cursor<'_>) -> Result<Vec<Term>, ParseError> {
    let mut objects = Vec::new();
    objects.push(parse_term(cursor)?);
    while cursor.peek_kind() == TokenKind::Comma {
        cursor.advance();
        objects.push(parse_term(cursor)?);
    }
    Ok(objects)
}

fn parse_predicate(cursor: &mut Cursor<'_>) -> Result<Predicate, ParseError> {
    // Check if we have a property path
    match cursor.peek_kind() {
        TokenKind::A | TokenKind::IriRef | TokenKind::PrefixedName | TokenKind::Variable => {
            // Could still be a path if followed by path operators
            let base_term = match cursor.peek_kind() {
                TokenKind::A => {
                    cursor.advance();
                    PathExpr::Iri(Iri::A)
                }
                TokenKind::IriRef => {
                    let t = cursor.advance().text.clone();
                    PathExpr::Iri(Iri::Absolute(t))
                }
                TokenKind::PrefixedName => {
                    let t = cursor.advance().text.clone();
                    PathExpr::Iri(Iri::Prefixed(t))
                }
                _ => {
                    // variable in predicate — not a path
                    let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
                    return Ok(Predicate::Term(Term::Var(v)));
                }
            };
            // Check for path modifiers
            match cursor.peek_kind() {
                TokenKind::Slash
                | TokenKind::Pipe
                | TokenKind::Question
                | TokenKind::Star
                | TokenKind::Plus => {
                    let path = parse_path_continue(cursor, base_term)?;
                    Ok(Predicate::Path(path))
                }
                _ => {
                    // Convert back to Term
                    let term = match base_term {
                        PathExpr::Iri(iri) => Term::Iri(iri),
                        _ => unreachable!(),
                    };
                    Ok(Predicate::Term(term))
                }
            }
        }
        TokenKind::Caret => {
            cursor.advance();
            let inner = parse_path_primary(cursor)?;
            let path = PathExpr::Inverse(Box::new(inner));
            let path = parse_path_continue(cursor, path)?;
            Ok(Predicate::Path(path))
        }
        TokenKind::Bang => {
            cursor.advance();
            let inner = parse_path_primary(cursor)?;
            let path = PathExpr::Negated(Box::new(inner));
            let path = parse_path_continue(cursor, path)?;
            Ok(Predicate::Path(path))
        }
        TokenKind::LParen => {
            let path = parse_path_expr(cursor)?;
            Ok(Predicate::Path(path))
        }
        _ => {
            let tok = cursor.peek();
            Err(ParseError::syntax(
                &tok.text,
                "expected predicate (IRI, variable, or property path)",
            ))
        }
    }
}

fn parse_path_primary(cursor: &mut Cursor<'_>) -> Result<PathExpr, ParseError> {
    match cursor.peek_kind() {
        TokenKind::A => {
            cursor.advance();
            Ok(PathExpr::Iri(Iri::A))
        }
        TokenKind::IriRef => {
            let t = cursor.advance().text.clone();
            Ok(PathExpr::Iri(Iri::Absolute(t)))
        }
        TokenKind::PrefixedName => {
            let t = cursor.advance().text.clone();
            Ok(PathExpr::Iri(Iri::Prefixed(t)))
        }
        TokenKind::LParen => parse_path_expr(cursor),
        _ => {
            let tok = cursor.peek();
            Err(ParseError::syntax(
                &tok.text,
                "expected IRI or ( in property path",
            ))
        }
    }
}

fn parse_path_expr(cursor: &mut Cursor<'_>) -> Result<PathExpr, ParseError> {
    cursor.expect_kind(TokenKind::LParen, "(")?;
    let inner = parse_path_alternative(cursor)?;
    cursor.expect_kind(TokenKind::RParen, ")")?;
    Ok(PathExpr::Group(Box::new(inner)))
}

fn parse_path_alternative(cursor: &mut Cursor<'_>) -> Result<PathExpr, ParseError> {
    let mut left = parse_path_sequence(cursor)?;
    while cursor.peek_kind() == TokenKind::Pipe {
        cursor.advance();
        let right = parse_path_sequence(cursor)?;
        left = PathExpr::Alternative(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_path_sequence(cursor: &mut Cursor<'_>) -> Result<PathExpr, ParseError> {
    let mut left = parse_path_elt_or_inverse(cursor)?;
    while cursor.peek_kind() == TokenKind::Slash {
        cursor.advance();
        let right = parse_path_elt_or_inverse(cursor)?;
        left = PathExpr::Sequence(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_path_elt_or_inverse(cursor: &mut Cursor<'_>) -> Result<PathExpr, ParseError> {
    if cursor.peek_kind() == TokenKind::Caret {
        cursor.advance();
        let inner = parse_path_primary(cursor)?;
        let inner = apply_path_mod(cursor, inner)?;
        Ok(PathExpr::Inverse(Box::new(inner)))
    } else {
        let primary = parse_path_primary(cursor)?;
        apply_path_mod(cursor, primary)
    }
}

fn apply_path_mod(cursor: &mut Cursor<'_>, path: PathExpr) -> Result<PathExpr, ParseError> {
    match cursor.peek_kind() {
        TokenKind::Question => {
            cursor.advance();
            Ok(PathExpr::ZeroOrOne(Box::new(path)))
        }
        TokenKind::Star => {
            cursor.advance();
            Ok(PathExpr::ZeroOrMore(Box::new(path)))
        }
        TokenKind::Plus => {
            cursor.advance();
            Ok(PathExpr::OneOrMore(Box::new(path)))
        }
        _ => Ok(path),
    }
}

fn parse_path_continue(cursor: &mut Cursor<'_>, base: PathExpr) -> Result<PathExpr, ParseError> {
    let base = apply_path_mod(cursor, base)?;
    match cursor.peek_kind() {
        TokenKind::Slash => {
            cursor.advance();
            let right = parse_path_elt_or_inverse(cursor)?;
            let seq = PathExpr::Sequence(Box::new(base), Box::new(right));
            parse_path_continue(cursor, seq)
        }
        TokenKind::Pipe => {
            cursor.advance();
            let right = parse_path_sequence(cursor)?;
            Ok(PathExpr::Alternative(Box::new(base), Box::new(right)))
        }
        _ => Ok(base),
    }
}

fn parse_term(cursor: &mut Cursor<'_>) -> Result<Term, ParseError> {
    match cursor.peek_kind() {
        TokenKind::Variable => {
            let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
            Ok(Term::Var(v))
        }
        TokenKind::IriRef => {
            let t = cursor.advance().text.clone();
            Ok(Term::Iri(Iri::Absolute(t)))
        }
        TokenKind::PrefixedName => {
            let t = cursor.advance().text.clone();
            Ok(Term::Iri(Iri::Prefixed(t)))
        }
        TokenKind::A => {
            cursor.advance();
            Ok(Term::Iri(Iri::A))
        }
        TokenKind::BlankNodeLabel => {
            let t = cursor.advance().text.clone();
            Ok(Term::BlankNode(t))
        }
        TokenKind::Anon => {
            cursor.advance();
            Ok(Term::AnonBlankNode)
        }
        TokenKind::StringLiteral => {
            let lex = cursor.advance().text.clone();
            let annotation = parse_literal_annotation(cursor)?;
            Ok(Term::Literal(Literal { lexical: lex, annotation }))
        }
        TokenKind::IntegerLiteral => {
            let t = cursor.advance().text.clone();
            Ok(Term::Literal(Literal {
                lexical: t.clone(),
                annotation: LiteralAnnotation::Datatype(Iri::Prefixed(
                    "xsd:integer".to_owned(),
                )),
            }))
        }
        TokenKind::DecimalLiteral => {
            let t = cursor.advance().text.clone();
            Ok(Term::Literal(Literal {
                lexical: t.clone(),
                annotation: LiteralAnnotation::Datatype(Iri::Prefixed(
                    "xsd:decimal".to_owned(),
                )),
            }))
        }
        TokenKind::DoubleLiteral => {
            let t = cursor.advance().text.clone();
            Ok(Term::Literal(Literal {
                lexical: t.clone(),
                annotation: LiteralAnnotation::Datatype(Iri::Prefixed(
                    "xsd:double".to_owned(),
                )),
            }))
        }
        TokenKind::True => {
            cursor.advance();
            Ok(Term::Literal(Literal {
                lexical: "true".to_owned(),
                annotation: LiteralAnnotation::Datatype(Iri::Prefixed(
                    "xsd:boolean".to_owned(),
                )),
            }))
        }
        TokenKind::False => {
            cursor.advance();
            Ok(Term::Literal(Literal {
                lexical: "false".to_owned(),
                annotation: LiteralAnnotation::Datatype(Iri::Prefixed(
                    "xsd:boolean".to_owned(),
                )),
            }))
        }
        TokenKind::LBracket => {
            // Blank-node property list
            cursor.advance();
            let pairs = parse_pred_obj_list(cursor)?;
            cursor.expect_kind(TokenKind::RBracket, "]")?;
            Ok(Term::BNodePropList(pairs))
        }
        TokenKind::LParen => {
            // Collection
            cursor.advance();
            let mut items = Vec::new();
            while cursor.peek_kind() != TokenKind::RParen {
                items.push(parse_term(cursor)?);
            }
            cursor.expect_kind(TokenKind::RParen, ")")?;
            Ok(Term::Collection(items))
        }
        _ => {
            let tok = cursor.peek();
            Err(ParseError::syntax(&tok.text, "expected RDF term"))
        }
    }
}

fn parse_rdf_term(cursor: &mut Cursor<'_>) -> Result<RdfTerm, ParseError> {
    parse_term(cursor)
}

fn parse_literal_annotation(cursor: &mut Cursor<'_>) -> Result<LiteralAnnotation, ParseError> {
    match cursor.peek_kind() {
        TokenKind::LangTag => {
            let tag = cursor.advance().text.trim_start_matches('@').to_owned();
            Ok(LiteralAnnotation::Lang(tag))
        }
        TokenKind::DatatypeSep => {
            cursor.advance();
            let iri = parse_iri(cursor)?;
            Ok(LiteralAnnotation::Datatype(iri))
        }
        _ => Ok(LiteralAnnotation::Plain),
    }
}

fn parse_iri(cursor: &mut Cursor<'_>) -> Result<Iri, ParseError> {
    match cursor.peek_kind() {
        TokenKind::IriRef => {
            let t = cursor.advance().text.clone();
            Ok(Iri::Absolute(t))
        }
        TokenKind::PrefixedName => {
            let t = cursor.advance().text.clone();
            Ok(Iri::Prefixed(t))
        }
        TokenKind::A => {
            cursor.advance();
            Ok(Iri::A)
        }
        _ => {
            let tok = cursor.peek();
            Err(ParseError::syntax(&tok.text, "expected IRI"))
        }
    }
}

fn parse_var_or_iri(cursor: &mut Cursor<'_>) -> Result<VarOrIri, ParseError> {
    match cursor.peek_kind() {
        TokenKind::Variable => {
            let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
            Ok(VarOrIri::Var(v))
        }
        _ => {
            let iri = parse_iri(cursor)?;
            Ok(VarOrIri::Iri(iri))
        }
    }
}

// ── Triple patterns in block ─────────────────────────────────────────────────

fn parse_triple_patterns_in_block(cursor: &mut Cursor<'_>) -> Result<Vec<TriplePattern>, ParseError> {
    let mut triples = Vec::new();
    while cursor.peek_kind() != TokenKind::RBrace && cursor.peek_kind() != TokenKind::Eof {
        let subj = parse_term(cursor)?;
        let pred_objs = parse_pred_obj_list(cursor)?;
        for (pred, obj) in pred_objs {
            triples.push(TriplePattern {
                subject: subj.clone(),
                predicate: pred,
                object: obj,
            });
        }
        if cursor.peek_kind() == TokenKind::Dot {
            cursor.advance();
        }
    }
    Ok(triples)
}

// ── Expressions ─────────────────────────────────────────────────────────────

fn parse_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    parse_or_expr(cursor)
}

fn parse_or_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    let mut left = parse_and_expr(cursor)?;
    while cursor.peek_kind() == TokenKind::Or {
        cursor.advance();
        let right = parse_and_expr(cursor)?;
        left = Expr::BinOp {
            op: BinOp::Or,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }
    Ok(left)
}

fn parse_and_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    let mut left = parse_relational_expr(cursor)?;
    while cursor.peek_kind() == TokenKind::And {
        cursor.advance();
        let right = parse_relational_expr(cursor)?;
        left = Expr::BinOp {
            op: BinOp::And,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }
    Ok(left)
}

fn parse_relational_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    let left = parse_additive_expr(cursor)?;
    let op = match cursor.peek_kind() {
        TokenKind::Eq => BinOp::Eq,
        TokenKind::NotEq => BinOp::Ne,
        TokenKind::Lt => BinOp::Lt,
        TokenKind::Gt => BinOp::Gt,
        TokenKind::Le => BinOp::Le,
        TokenKind::Ge => BinOp::Ge,
        TokenKind::In => {
            cursor.advance();
            cursor.expect_kind(TokenKind::LParen, "(")?;
            let mut list = Vec::new();
            while cursor.peek_kind() != TokenKind::RParen {
                list.push(parse_expr(cursor)?);
                if cursor.peek_kind() == TokenKind::Comma {
                    cursor.advance();
                }
            }
            cursor.expect_kind(TokenKind::RParen, ")")?;
            return Ok(Expr::In {
                lhs: Box::new(left),
                rhs: list,
            });
        }
        TokenKind::Not => {
            // NOT IN
            cursor.advance();
            cursor.expect_kind(TokenKind::In, "IN")?;
            cursor.expect_kind(TokenKind::LParen, "(")?;
            let mut list = Vec::new();
            while cursor.peek_kind() != TokenKind::RParen {
                list.push(parse_expr(cursor)?);
                if cursor.peek_kind() == TokenKind::Comma {
                    cursor.advance();
                }
            }
            cursor.expect_kind(TokenKind::RParen, ")")?;
            return Ok(Expr::NotIn {
                lhs: Box::new(left),
                rhs: list,
            });
        }
        _ => return Ok(left),
    };
    cursor.advance();
    let right = parse_additive_expr(cursor)?;
    Ok(Expr::BinOp {
        op,
        lhs: Box::new(left),
        rhs: Box::new(right),
    })
}

fn parse_additive_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    let mut left = parse_multiplicative_expr(cursor)?;
    loop {
        let op = match cursor.peek_kind() {
            TokenKind::Plus => BinOp::Add,
            TokenKind::Minus => BinOp::Sub,
            _ => break,
        };
        cursor.advance();
        let right = parse_multiplicative_expr(cursor)?;
        left = Expr::BinOp {
            op,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }
    Ok(left)
}

fn parse_multiplicative_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    let mut left = parse_unary_expr(cursor)?;
    loop {
        let op = match cursor.peek_kind() {
            TokenKind::Star => BinOp::Mul,
            TokenKind::Slash => BinOp::Div,
            _ => break,
        };
        cursor.advance();
        let right = parse_unary_expr(cursor)?;
        left = Expr::BinOp {
            op,
            lhs: Box::new(left),
            rhs: Box::new(right),
        };
    }
    Ok(left)
}

fn parse_unary_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    match cursor.peek_kind() {
        TokenKind::Bang => {
            cursor.advance();
            let e = parse_primary_expr(cursor)?;
            Ok(Expr::Not(Box::new(e)))
        }
        TokenKind::Plus => {
            cursor.advance();
            parse_primary_expr(cursor)
        }
        TokenKind::Minus => {
            cursor.advance();
            let e = parse_primary_expr(cursor)?;
            Ok(Expr::Neg(Box::new(e)))
        }
        _ => parse_primary_expr(cursor),
    }
}

#[allow(clippy::too_many_lines)]
fn parse_primary_expr(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    match cursor.peek_kind() {
        TokenKind::LParen => {
            cursor.advance();
            let e = parse_expr(cursor)?;
            cursor.expect_kind(TokenKind::RParen, ")")?;
            Ok(e)
        }
        TokenKind::Variable => {
            let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
            Ok(Expr::Var(v))
        }
        TokenKind::True => {
            cursor.advance();
            Ok(Expr::Bool(true))
        }
        TokenKind::False => {
            cursor.advance();
            Ok(Expr::Bool(false))
        }
        TokenKind::IntegerLiteral => {
            let t = cursor.advance().text.clone();
            let n = t.parse::<i64>().unwrap_or(0);
            Ok(Expr::Integer(n))
        }
        TokenKind::DecimalLiteral => {
            let t = cursor.advance().text.clone();
            Ok(Expr::Decimal(t))
        }
        TokenKind::DoubleLiteral => {
            let t = cursor.advance().text.clone();
            Ok(Expr::Double(t))
        }
        TokenKind::StringLiteral => {
            let lex = cursor.advance().text.clone();
            let annotation = parse_literal_annotation(cursor)?;
            Ok(Expr::Literal(Literal { lexical: lex, annotation }))
        }
        TokenKind::IriRef => {
            let t = cursor.advance().text.clone();
            // Could be followed by args (like a function call)
            // For now, treat as IRI constant
            Ok(Expr::Iri(Iri::Absolute(t)))
        }
        TokenKind::PrefixedName => {
            let t = cursor.advance().text.clone();
            Ok(Expr::Iri(Iri::Prefixed(t)))
        }
        // Built-in functions
        kind if is_builtin_func(kind.clone()) => {
            parse_builtin_call(cursor)
        }
        // Aggregate functions
        TokenKind::Count | TokenKind::Sum | TokenKind::Min | TokenKind::Max
        | TokenKind::Avg | TokenKind::Sample | TokenKind::GroupConcat => {
            parse_aggregate(cursor)
        }
        TokenKind::Exists => {
            cursor.advance();
            let pat = parse_group_graph_pattern(cursor)?;
            Ok(Expr::Exists(Box::new(pat)))
        }
        TokenKind::Not => {
            cursor.advance();
            cursor.expect_kind(TokenKind::Exists, "EXISTS after NOT")?;
            let pat = parse_group_graph_pattern(cursor)?;
            Ok(Expr::NotExists(Box::new(pat)))
        }
        _ => {
            let tok = cursor.peek();
            Err(ParseError::syntax(&tok.text, "expected expression"))
        }
    }
}

fn is_builtin_func(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Str
            | TokenKind::Lang
            | TokenKind::LangMatches
            | TokenKind::Datatype
            | TokenKind::Bound
            | TokenKind::IriFunc
            | TokenKind::UriFunc
            | TokenKind::BnodeFunc
            | TokenKind::Rand
            | TokenKind::Abs
            | TokenKind::Ceil
            | TokenKind::Floor
            | TokenKind::Round
            | TokenKind::Concat
            | TokenKind::Strlen
            | TokenKind::Substr
            | TokenKind::Ucase
            | TokenKind::Lcase
            | TokenKind::EncodeForUri
            | TokenKind::Contains
            | TokenKind::Strstarts
            | TokenKind::Strends
            | TokenKind::Strbefore
            | TokenKind::Strafter
            | TokenKind::Year
            | TokenKind::Month
            | TokenKind::Day
            | TokenKind::Hours
            | TokenKind::Minutes
            | TokenKind::Seconds
            | TokenKind::Timezone
            | TokenKind::Tz
            | TokenKind::Now
            | TokenKind::Uuid
            | TokenKind::Struuid
            | TokenKind::Md5
            | TokenKind::Sha1
            | TokenKind::Sha256
            | TokenKind::Sha384
            | TokenKind::Sha512
            | TokenKind::Coalesce
            | TokenKind::If
            | TokenKind::Strlang
            | TokenKind::Strdt
            | TokenKind::Sameterm
            | TokenKind::IsIri
            | TokenKind::IsUri
            | TokenKind::IsBlank
            | TokenKind::IsLiteral
            | TokenKind::IsNumeric
            | TokenKind::Regex
            | TokenKind::Replace
    )
}

fn parse_builtin_call(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    let name = cursor.advance().text.to_ascii_uppercase();
    cursor.expect_kind(TokenKind::LParen, "(")?;
    let mut args = Vec::new();
    // BOUND takes a single variable
    if name == "BOUND" {
        if cursor.peek_kind() == TokenKind::Variable {
            let v = cursor.advance().text.trim_start_matches(['?', '$']).to_owned();
            args.push(Expr::Var(v));
        }
    } else {
        while cursor.peek_kind() != TokenKind::RParen && cursor.peek_kind() != TokenKind::Eof {
            args.push(parse_expr(cursor)?);
            if cursor.peek_kind() == TokenKind::Comma {
                cursor.advance();
            } else {
                break;
            }
        }
    }
    cursor.expect_kind(TokenKind::RParen, ")")?;
    Ok(Expr::BuiltIn { name, args })
}

fn parse_aggregate(cursor: &mut Cursor<'_>) -> Result<Expr, ParseError> {
    let name = cursor.advance().text.to_ascii_uppercase();
    cursor.expect_kind(TokenKind::LParen, "(")?;

    let distinct = if cursor.peek_kind() == TokenKind::Distinct {
        cursor.advance();
        true
    } else {
        false
    };

    let star = cursor.peek_kind() == TokenKind::Star;
    let args = if star {
        cursor.advance();
        vec![]
    } else {
        let mut a = Vec::new();
        while cursor.peek_kind() != TokenKind::RParen
            && cursor.peek_kind() != TokenKind::Eof
            && cursor.peek_kind() != TokenKind::Separator
        {
            a.push(parse_expr(cursor)?);
            if cursor.peek_kind() == TokenKind::Comma {
                cursor.advance();
            } else {
                break;
            }
        }
        a
    };

    let separator = if cursor.peek_kind() == TokenKind::Separator {
        cursor.advance();
        cursor.expect_kind(TokenKind::Eq, "=")?;
        let s = cursor.expect_kind(TokenKind::StringLiteral, "string separator")?;
        Some(s.text.clone())
    } else {
        None
    };

    cursor.expect_kind(TokenKind::RParen, ")")?;
    Ok(Expr::Aggregate {
        name,
        distinct,
        args,
        star,
        separator,
    })
}

// ── Update Operations ────────────────────────────────────────────────────────

fn parse_update_ops(cursor: &mut Cursor<'_>) -> Result<Vec<UpdateOp>, ParseError> {
    let mut ops = Vec::new();
    loop {
        match cursor.peek_kind() {
            TokenKind::Eof => break,
            TokenKind::Semi => {
                cursor.advance();
                // semicolons separate update operations; also handle prologue between them
                // (BASE/PREFIX can appear between operations)
                loop {
                    match cursor.peek_kind() {
                        TokenKind::Base | TokenKind::Prefix => {
                            // consume but we can't update the prologue here easily;
                            // just skip for now
                            cursor.advance();
                            cursor.advance(); // skip the IRI/name too
                        }
                        _ => break,
                    }
                }
                if cursor.peek_kind() == TokenKind::Eof {
                    break;
                }
            }
            _ => {
                let op = parse_update_op(cursor)?;
                ops.push(op);
            }
        }
    }
    Ok(ops)
}

#[allow(clippy::too_many_lines)]
fn parse_update_op(cursor: &mut Cursor<'_>) -> Result<UpdateOp, ParseError> {
    match cursor.peek_kind() {
        TokenKind::Load => {
            cursor.advance();
            let silent = consume_silent(cursor);
            let src_tok = cursor.expect_kind(TokenKind::IriRef, "IRI after LOAD")?;
            let source = strip_angle_brackets(&src_tok.text);
            let into_graph = if cursor.peek_kind() == TokenKind::Into {
                cursor.advance();
                cursor.expect_kind(TokenKind::Graph, "GRAPH after INTO")?;
                let g = cursor.expect_kind(TokenKind::IriRef, "graph IRI")?;
                Some(strip_angle_brackets(&g.text))
            } else {
                None
            };
            Ok(UpdateOp::Load { silent, source, into_graph })
        }

        TokenKind::Clear => {
            cursor.advance();
            let silent = consume_silent(cursor);
            let graph_ref = parse_graph_ref(cursor)?;
            Ok(UpdateOp::Clear { silent, graph_ref })
        }

        TokenKind::Drop => {
            cursor.advance();
            let silent = consume_silent(cursor);
            let graph_ref = parse_graph_ref(cursor)?;
            Ok(UpdateOp::Drop { silent, graph_ref })
        }

        TokenKind::Create => {
            cursor.advance();
            let silent = consume_silent(cursor);
            cursor.expect_kind(TokenKind::Graph, "GRAPH after CREATE")?;
            let g = cursor.expect_kind(TokenKind::IriRef, "graph IRI")?;
            Ok(UpdateOp::Create {
                silent,
                iri: strip_angle_brackets(&g.text),
            })
        }

        TokenKind::Add => {
            cursor.advance();
            let silent = consume_silent(cursor);
            let from = parse_graph_or_default(cursor)?;
            cursor.expect_kind(TokenKind::Into, "TO")?;  // spec says TO
            let to = parse_graph_or_default(cursor)?;
            Ok(UpdateOp::Add { silent, from, to })
        }

        TokenKind::Move => {
            cursor.advance();
            let silent = consume_silent(cursor);
            let from = parse_graph_or_default(cursor)?;
            cursor.expect_kind(TokenKind::Into, "TO")?;
            let to = parse_graph_or_default(cursor)?;
            Ok(UpdateOp::Move { silent, from, to })
        }

        TokenKind::Copy => {
            cursor.advance();
            let silent = consume_silent(cursor);
            let from = parse_graph_or_default(cursor)?;
            cursor.expect_kind(TokenKind::Into, "TO")?;
            let to = parse_graph_or_default(cursor)?;
            Ok(UpdateOp::Copy { silent, from, to })
        }

        TokenKind::Insert => {
            cursor.advance();
            cursor.expect_kind(TokenKind::Data, "DATA after INSERT")?;
            cursor.expect_kind(TokenKind::LBrace, "{")?;
            let triples = parse_triple_patterns_in_block(cursor)?;
            cursor.expect_kind(TokenKind::RBrace, "}")?;
            Ok(UpdateOp::InsertData(triples))
        }

        TokenKind::Delete => {
            cursor.advance();
            if cursor.peek_kind() == TokenKind::Data {
                cursor.advance();
                cursor.expect_kind(TokenKind::LBrace, "{")?;
                let triples = parse_triple_patterns_in_block(cursor)?;
                cursor.expect_kind(TokenKind::RBrace, "}")?;
                Ok(UpdateOp::DeleteData(triples))
            } else if cursor.peek_kind() == TokenKind::Where {
                cursor.advance();
                let pat = parse_group_graph_pattern(cursor)?;
                Ok(UpdateOp::DeleteWhere(pat))
            } else {
                // DELETE { template } INSERT { template } WHERE { ... }
                cursor.expect_kind(TokenKind::LBrace, "{")?;
                let delete = parse_triple_patterns_in_block(cursor)?;
                cursor.expect_kind(TokenKind::RBrace, "}")?;
                let insert = if cursor.peek_kind() == TokenKind::Insert {
                    cursor.advance();
                    cursor.expect_kind(TokenKind::LBrace, "{")?;
                    let t = parse_triple_patterns_in_block(cursor)?;
                    cursor.expect_kind(TokenKind::RBrace, "}")?;
                    t
                } else {
                    vec![]
                };
                let using = parse_using_clauses(cursor)?;
                cursor.expect_kind(TokenKind::Where, "WHERE")?;
                let where_pattern = parse_group_graph_pattern(cursor)?;
                Ok(UpdateOp::Modify {
                    with: None,
                    delete,
                    insert,
                    using,
                    where_pattern,
                })
            }
        }

        TokenKind::With => {
            cursor.advance();
            let with_iri_tok = cursor.expect_kind(TokenKind::IriRef, "graph IRI after WITH")?;
            let with_iri = strip_angle_brackets(&with_iri_tok.text);
            // Must be followed by DELETE or INSERT
            let (delete, insert) = parse_modify_templates(cursor)?;
            let using = parse_using_clauses(cursor)?;
            cursor.expect_kind(TokenKind::Where, "WHERE")?;
            let where_pattern = parse_group_graph_pattern(cursor)?;
            Ok(UpdateOp::Modify {
                with: Some(with_iri),
                delete,
                insert,
                using,
                where_pattern,
            })
        }

        _ => {
            let tok = cursor.peek();
            Err(ParseError::syntax(
                &tok.text,
                "expected update operation (INSERT, DELETE, LOAD, etc.)",
            ))
        }
    }
}

fn parse_modify_templates(
    cursor: &mut Cursor<'_>,
) -> Result<(Vec<TriplePattern>, Vec<TriplePattern>), ParseError> {
    let mut delete = Vec::new();
    let mut insert = Vec::new();
    if cursor.peek_kind() == TokenKind::Delete {
        cursor.advance();
        cursor.expect_kind(TokenKind::LBrace, "{")?;
        delete = parse_triple_patterns_in_block(cursor)?;
        cursor.expect_kind(TokenKind::RBrace, "}")?;
        if cursor.peek_kind() == TokenKind::Insert {
            cursor.advance();
            cursor.expect_kind(TokenKind::LBrace, "{")?;
            insert = parse_triple_patterns_in_block(cursor)?;
            cursor.expect_kind(TokenKind::RBrace, "}")?;
        }
    } else if cursor.peek_kind() == TokenKind::Insert {
        cursor.advance();
        cursor.expect_kind(TokenKind::LBrace, "{")?;
        insert = parse_triple_patterns_in_block(cursor)?;
        cursor.expect_kind(TokenKind::RBrace, "}")?;
    }
    Ok((delete, insert))
}

fn parse_using_clauses(cursor: &mut Cursor<'_>) -> Result<Vec<DatasetClause>, ParseError> {
    let mut clauses = Vec::new();
    loop {
        if cursor.peek_kind() != TokenKind::Using {
            break;
        }
        cursor.advance();
        let named = if cursor.peek_kind() == TokenKind::Named {
            cursor.advance();
            true
        } else {
            false
        };
        let iri = parse_iri(cursor)?;
        clauses.push(DatasetClause { iri, named });
    }
    Ok(clauses)
}

fn parse_graph_ref(cursor: &mut Cursor<'_>) -> Result<GraphRef, ParseError> {
    match cursor.peek_kind() {
        TokenKind::Graph => {
            cursor.advance();
            let g = cursor.expect_kind(TokenKind::IriRef, "graph IRI")?;
            Ok(GraphRef::Named(strip_angle_brackets(&g.text)))
        }
        TokenKind::Default => {
            cursor.advance();
            Ok(GraphRef::Default)
        }
        TokenKind::Named => {
            cursor.advance();
            Ok(GraphRef::Named2)
        }
        TokenKind::All => {
            cursor.advance();
            Ok(GraphRef::All)
        }
        _ => {
            let tok = cursor.peek();
            Err(ParseError::syntax(
                &tok.text,
                "expected GRAPH <iri>, DEFAULT, NAMED, or ALL",
            ))
        }
    }
}

fn parse_graph_or_default(cursor: &mut Cursor<'_>) -> Result<GraphOrDefault, ParseError> {
    if cursor.peek_kind() == TokenKind::Default {
        cursor.advance();
        Ok(GraphOrDefault::Default)
    } else {
        // Optional GRAPH keyword
        if cursor.peek_kind() == TokenKind::Graph {
            cursor.advance();
        }
        let g = cursor.expect_kind(TokenKind::IriRef, "graph IRI")?;
        Ok(GraphOrDefault::Named(strip_angle_brackets(&g.text)))
    }
}

fn consume_silent(cursor: &mut Cursor<'_>) -> bool {
    if cursor.peek_kind() == TokenKind::Silent {
        cursor.advance();
        true
    } else {
        false
    }
}

// ── Utility ─────────────────────────────────────────────────────────────────

fn strip_angle_brackets(s: &str) -> String {
    s.trim_start_matches('<').trim_end_matches('>').to_owned()
}

// ── Cursor ──────────────────────────────────────────────────────────────────

struct Cursor<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            static EOF: std::sync::OnceLock<Token> = std::sync::OnceLock::new();
            EOF.get_or_init(|| Token {
                kind: TokenKind::Eof,
                text: String::new(),
                offset: 0,
            })
        })
    }

    fn peek_kind(&self) -> TokenKind {
        self.peek().kind.clone()
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect_kind(&mut self, kind: TokenKind, desc: &str) -> Result<&Token, ParseError> {
        if std::mem::discriminant(&self.peek_kind()) == std::mem::discriminant(&kind) {
            Ok(self.advance())
        } else {
            let near = self.peek().text.clone();
            Err(ParseError::syntax(
                &near,
                &format!("expected {desc}, got {:?}", near),
            ))
        }
    }

    fn expect_eof(&self) -> Result<(), ParseError> {
        if matches!(self.peek_kind(), TokenKind::Eof) {
            Ok(())
        } else {
            Err(ParseError::syntax(
                &self.peek().text,
                "unexpected trailing tokens",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenise;

    fn parse(src: &str) -> Document {
        let tokens = tokenise(src).expect("lex");
        let (doc, _warns) = parse_document(&tokens, src).expect("parse");
        doc
    }

    #[test]
    fn parse_select_star() {
        let doc = parse("SELECT * WHERE { }");
        if let DocumentBody::Query(q) = doc.body {
            assert!(matches!(
                q.form,
                QueryForm::Select(SelectClause {
                    projection: SelectProjection::Star,
                    ..
                })
            ));
        } else {
            panic!("expected query");
        }
    }

    #[test]
    fn parse_ask() {
        let doc = parse("ASK { }");
        if let DocumentBody::Query(q) = doc.body {
            assert!(matches!(q.form, QueryForm::Ask));
        } else {
            panic!("expected query");
        }
    }

    #[test]
    fn parse_insert_data() {
        let doc = parse("INSERT DATA { <http://a> <http://b> <http://c> . }");
        assert!(matches!(
            doc.body,
            DocumentBody::Update(_)
        ));
    }

    #[test]
    fn parse_select_with_filter() {
        let doc = parse(
            "SELECT ?x WHERE { ?x <http://ex.org/p> ?v . FILTER(?v > 10) }",
        );
        if let DocumentBody::Query(q) = doc.body {
            if let GroupGraphPattern::Group(elems) = q.where_clause {
                assert!(elems.iter().any(|e| matches!(e, GraphPatternElement::Filter(_))));
            } else {
                panic!("expected group");
            }
        } else {
            panic!("expected query");
        }
    }

    #[test]
    fn parse_service() {
        let doc = parse(
            "SELECT * WHERE { SERVICE <http://sparql.org/ep> { ?x ?y ?z } }",
        );
        if let DocumentBody::Query(q) = doc.body {
            if let GroupGraphPattern::Group(elems) = q.where_clause {
                assert!(elems.iter().any(|e| matches!(e, GraphPatternElement::Service { .. })));
            } else {
                panic!("expected group");
            }
        } else {
            panic!("expected query");
        }
    }

    #[test]
    fn parse_bind() {
        let doc = parse("SELECT ?x ?y WHERE { ?x <http://p> ?v . BIND(?v + 1 AS ?y) }");
        if let DocumentBody::Query(q) = doc.body {
            if let GroupGraphPattern::Group(elems) = q.where_clause {
                assert!(elems.iter().any(|e| matches!(e, GraphPatternElement::Bind { .. })));
            } else {
                panic!("expected group");
            }
        } else {
            panic!("expected query");
        }
    }

    #[test]
    fn parse_optional() {
        let doc = parse("SELECT * WHERE { ?s ?p ?o . OPTIONAL { ?s <http://label> ?l } }");
        if let DocumentBody::Query(q) = doc.body {
            if let GroupGraphPattern::Group(elems) = q.where_clause {
                assert!(elems.iter().any(|e| matches!(e, GraphPatternElement::Optional(_))));
            } else {
                panic!("expected group");
            }
        } else {
            panic!("expected query");
        }
    }

    #[test]
    fn parse_clear_all() {
        let doc = parse("CLEAR ALL");
        assert!(matches!(
            doc.body,
            DocumentBody::Update(ref ops) if matches!(
                ops.as_slice(),
                [UpdateOp::Clear { graph_ref: GraphRef::All, .. }]
            )
        ));
    }

    #[test]
    fn parse_prefix_and_select() {
        let doc = parse(
            "PREFIX ex: <http://example.org/> SELECT ?x WHERE { ?x a ex:Foo }",
        );
        assert!(!doc.prologue.prefixes.is_empty());
    }
}
