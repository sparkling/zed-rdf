//! Recursive-descent grammar for ShEx 2.x compact syntax (ShExC).
//!
//! Parses the input token stream produced by [`Lexer`] and builds an
//! in-memory [`Schema`] AST. The `encode` module then flattens the AST
//! into canonical facts.
//!
//! Grammar reference: https://shex.io/shex-semantics/#shexc
//!
//! Grammar decisions / deviations from the full ShExC spec:
//!
//! - Shape labels: IRI, prefixed name, or blank node.
//! - Triple expressions inside `{ … }` are separated by `;` (EachOf) or
//!   `,` (OneOf). Bare sequential constraints with no separator are treated
//!   as EachOf members.
//! - `EXTENDS @label { … }` is parsed but the extends target is just
//!   recorded on the `Shape`, not validated.
//! - `EXTRA <pred>*` is parsed and recorded but not encoded as a fact
//!   (out of scope for a structural fact emitter).
//! - `START = @<label>` and `ABSTRACT` are parsed and silently ignored.
//! - The `$(<iri>)` triple expression label syntax is parsed but the
//!   label itself is dropped (structural encoding does not need it).
//! - Value set items: IRIs, prefixed names, and string literals with
//!   optional datatype (`^^<iri>`) or language tag (`@lang`).

use std::collections::BTreeMap;

use crate::ast::{
    Cardinality, DatatypeRef, Label, NodeConstraint, Predicate, Schema, Shape, ShapeDecl,
    ShapeExpr, TripleConstraint, TripleExpr, ValueSetItem,
};
use crate::lexer::{LexError, Lexer, Spanned, Tok};

/// Parse error with byte offset and human-readable message.
#[derive(Debug, Clone)]
pub(crate) struct ParseError {
    pub(crate) message: String,
    pub(crate) offset: usize,
}

impl ParseError {
    pub(crate) fn new(offset: usize, msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            offset,
        }
    }
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self {
            message: e.message,
            offset: e.offset,
        }
    }
}

/// Recursive-descent ShExC parser.
pub(crate) struct Parser<'a> {
    lex: Lexer<'a>,
    /// Lookahead buffer (at most one token).
    peeked: Option<Spanned>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(src: &'a [u8]) -> Self {
        Parser {
            lex: Lexer::new(src),
            peeked: None,
        }
    }

    /// Parse the complete schema document.
    pub(crate) fn parse_schema(&mut self) -> Result<Schema, ParseError> {
        let mut schema = Schema::default();

        loop {
            let tok = self.peek_tok()?;
            match tok {
                None => break,
                // `PREFIX name: <iri>` or `@prefix name: <iri>`.
                Some(Tok::KwPrefix) => {
                    self.consume();
                    let (prefix, iri) = self.parse_prefix_directive()?;
                    schema.prefixes.push((prefix, iri));
                }
                // `BASE <iri>` or `@base <iri>`.
                Some(Tok::KwBase) => {
                    self.consume();
                    let iri = self.expect_iriref("BASE directive")?;
                    schema.base = Some(iri);
                }
                // `START = @<ref>` — parse and discard.
                Some(Tok::KwStart) => {
                    self.consume();
                    self.expect_tok(&Tok::Eq, "= after START")?;
                    self.parse_shape_ref()?;
                }
                // Shape declaration.
                Some(Tok::IriRef(_) | Tok::Pname { .. } | Tok::BNodeLabel(_)) => {
                    let decl = self.parse_shape_decl()?;
                    schema.shapes.push(decl);
                }
                Some(other) => {
                    return Err(ParseError::new(
                        self.lex.offset(),
                        format!("unexpected token at top level: {other:?}"),
                    ));
                }
            }
        }

        Ok(schema)
    }

    // -----------------------------------------------------------------------
    // Directive parsing
    // -----------------------------------------------------------------------

    fn parse_prefix_directive(&mut self) -> Result<(String, String), ParseError> {
        // Next should be `prefix:` (pname with empty local) or `prefix:`.
        let sp = self.next_tok()?.ok_or_else(|| {
            ParseError::new(self.lex.offset(), "expected prefix name after PREFIX")
        })?;
        let prefix = match sp.tok {
            Tok::Pname { prefix, local } if local.is_empty() => prefix,
            other => {
                return Err(ParseError::new(
                    sp.start,
                    format!("expected 'name:' after PREFIX, got {other:?}"),
                ));
            }
        };
        let iri = self.expect_iriref("PREFIX directive IRI")?;
        Ok((prefix, iri))
    }

    // -----------------------------------------------------------------------
    // Shape declaration
    // -----------------------------------------------------------------------

    fn parse_shape_decl(&mut self) -> Result<ShapeDecl, ParseError> {
        let sp = self.next_tok()?.ok_or_else(|| {
            ParseError::new(self.lex.offset(), "expected shape label")
        })?;
        let offset = sp.start;
        let label = tok_to_label(sp.tok, sp.start)?;
        let expr = self.parse_shape_expr()?;
        Ok(ShapeDecl {
            label,
            expr,
            offset,
        })
    }

    // -----------------------------------------------------------------------
    // Shape expressions
    // -----------------------------------------------------------------------

    fn parse_shape_expr(&mut self) -> Result<ShapeExpr, ParseError> {
        let lhs = self.parse_shape_expr_primary()?;
        self.parse_shape_expr_rhs(lhs)
    }

    fn parse_shape_expr_rhs(&mut self, lhs: ShapeExpr) -> Result<ShapeExpr, ParseError> {
        match self.peek_tok()? {
            Some(Tok::KwAnd) => {
                self.consume();
                let rhs = self.parse_shape_expr_primary()?;
                let combined = ShapeExpr::And(Box::new(lhs), Box::new(rhs));
                self.parse_shape_expr_rhs(combined)
            }
            Some(Tok::KwOr) => {
                self.consume();
                let rhs = self.parse_shape_expr_primary()?;
                let combined = ShapeExpr::Or(Box::new(lhs), Box::new(rhs));
                self.parse_shape_expr_rhs(combined)
            }
            _ => Ok(lhs),
        }
    }

    fn parse_shape_expr_primary(&mut self) -> Result<ShapeExpr, ParseError> {
        match self.peek_tok()? {
            Some(Tok::KwNot) => {
                self.consume();
                let inner = self.parse_shape_expr_primary()?;
                Ok(ShapeExpr::Not(Box::new(inner)))
            }
            Some(Tok::At) => {
                self.consume();
                let label = self.parse_label()?;
                Ok(ShapeExpr::Ref(label))
            }
            // Node constraint keywords.
            Some(Tok::KwIri) => {
                self.consume();
                Ok(ShapeExpr::NodeConstraint(NodeConstraint::Iri))
            }
            Some(Tok::KwLiteral) => {
                self.consume();
                Ok(ShapeExpr::NodeConstraint(NodeConstraint::Literal))
            }
            Some(Tok::KwNonLiteral) => {
                self.consume();
                Ok(ShapeExpr::NodeConstraint(NodeConstraint::NonLiteral))
            }
            Some(Tok::KwBNode) => {
                self.consume();
                Ok(ShapeExpr::NodeConstraint(NodeConstraint::BNode))
            }
            // Value set `[ … ]`.
            Some(Tok::LBracket) => {
                let items = self.parse_value_set()?;
                Ok(ShapeExpr::NodeConstraint(NodeConstraint::ValueSet(items)))
            }
            // Datatype as IRI or pname (but NOT when followed immediately by `{`
            // without a `.` — that case means the IRI is a shape label
            // re-declaration).  We always treat a bare IRI/pname here as a
            // datatype constraint since this is called after the label is already
            // consumed.
            Some(Tok::IriRef(_)) => {
                let sp = self.next_tok()?.unwrap();
                if let Tok::IriRef(iri) = sp.tok {
                    Ok(ShapeExpr::NodeConstraint(NodeConstraint::Datatype(
                        DatatypeRef::Iri(iri),
                    )))
                } else {
                    unreachable!()
                }
            }
            Some(Tok::Pname { .. }) => {
                let sp = self.next_tok()?.unwrap();
                if let Tok::Pname { prefix, local } = sp.tok {
                    Ok(ShapeExpr::NodeConstraint(NodeConstraint::Datatype(
                        DatatypeRef::Pname { prefix, local },
                    )))
                } else {
                    unreachable!()
                }
            }
            // `{…}` shape.
            Some(Tok::LBrace) => {
                let shape = self.parse_shape()?;
                Ok(ShapeExpr::Shape(shape))
            }
            // `CLOSED` shape.
            Some(Tok::KwClosed) => {
                let shape = self.parse_shape()?;
                Ok(ShapeExpr::Shape(shape))
            }
            // `EXTENDS` shape.
            Some(Tok::KwExtends) => {
                let shape = self.parse_shape()?;
                Ok(ShapeExpr::Shape(shape))
            }
            // `ABSTRACT` — parse as shape.
            Some(Tok::KwAbstract) => {
                self.consume();
                let shape = self.parse_shape()?;
                Ok(ShapeExpr::Shape(shape))
            }
            // `.` as wildcard (KwAny from grammar — also bare dot handled via lexer
            // as KwAny for completeness).
            Some(Tok::KwAny) => {
                self.consume();
                Ok(ShapeExpr::Any)
            }
            other => {
                let off = self.lex.offset();
                Err(ParseError::new(
                    off,
                    format!("expected shape expression, got {other:?}"),
                ))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Shape `{ tripleExpr* }`
    // -----------------------------------------------------------------------

    fn parse_shape(&mut self) -> Result<Shape, ParseError> {
        let mut closed = false;
        let mut extends: Vec<Label> = Vec::new();
        let mut extra_preds: Vec<Predicate> = Vec::new();

        // Consume optional CLOSED / EXTENDS / EXTRA modifiers before `{`.
        loop {
            match self.peek_tok()? {
                Some(Tok::KwClosed) => {
                    self.consume();
                    closed = true;
                }
                Some(Tok::KwExtends) => {
                    self.consume();
                    // `EXTENDS @<label>` — read possibly multiple refs.
                    let at_sp = self.next_tok()?.ok_or_else(|| {
                        ParseError::new(self.lex.offset(), "expected @<label> after EXTENDS")
                    })?;
                    if at_sp.tok != Tok::At {
                        return Err(ParseError::new(
                            at_sp.start,
                            format!("expected '@' after EXTENDS, got {:?}", at_sp.tok),
                        ));
                    }
                    let lbl = self.parse_label()?;
                    extends.push(lbl);
                }
                Some(Tok::KwExtra) => {
                    self.consume();
                    // Read predicates until LBrace.
                    while matches!(
                        self.peek_tok()?,
                        Some(Tok::IriRef(_) | Tok::Pname { .. } | Tok::KwA)
                    ) {
                        let p = self.parse_predicate()?;
                        extra_preds.push(p);
                    }
                }
                _ => break,
            }
        }
        // Drop extra_preds — not encoded.
        drop(extra_preds);

        self.expect_tok(&Tok::LBrace, "'{' to open shape body")?;
        let mut triple_exprs: Vec<TripleExpr> = Vec::new();

        loop {
            match self.peek_tok()? {
                Some(Tok::RBrace) => {
                    self.consume();
                    break;
                }
                None => {
                    return Err(ParseError::new(
                        self.lex.offset(),
                        "unterminated shape body: expected '}'",
                    ));
                }
                // Skip `$(<tripleExprLabel>)` — triple expression label.
                Some(Tok::Dollar) => {
                    self.consume();
                    self.expect_tok(&Tok::LParen, "'(' after '$'")?;
                    // The label inside — IRI or pname.
                    let sp = self.next_tok()?.ok_or_else(|| {
                        ParseError::new(self.lex.offset(), "expected IRI in triple expr label")
                    })?;
                    match sp.tok {
                        Tok::IriRef(_) | Tok::Pname { .. } => {}
                        other => {
                            return Err(ParseError::new(
                                sp.start,
                                format!("expected IRI in triple expr label, got {other:?}"),
                            ));
                        }
                    }
                    self.expect_tok(&Tok::RParen, "')' after triple expr label")?;
                }
                // `&<tripleExprRef>` — reference to an external triple expression.
                Some(Tok::Amp) => {
                    self.consume();
                    // Read the reference IRI or pname — discard it.
                    let sp = self.next_tok()?.ok_or_else(|| {
                        ParseError::new(self.lex.offset(), "expected IRI after '&'")
                    })?;
                    match sp.tok {
                        Tok::IriRef(_) | Tok::Pname { .. } => {}
                        other => {
                            return Err(ParseError::new(
                                sp.start,
                                format!("expected IRI after '&', got {other:?}"),
                            ));
                        }
                    }
                    // Cardinality may follow.
                    let _ = self.parse_cardinality()?;
                    // Separator.
                    self.skip_separators()?;
                }
                _ => {
                    let te = self.parse_triple_expr()?;
                    triple_exprs.push(te);
                    // After a triple expr, skip optional `;` separator.
                    self.skip_separators()?;
                }
            }
        }

        Ok(Shape {
            triple_exprs,
            closed,
            extends,
        })
    }

    /// Skip zero or more `;` separators between triple expressions.
    fn skip_separators(&mut self) -> Result<(), ParseError> {
        while self.peek_tok()? == Some(Tok::Semi) {
            self.consume();
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Triple expressions
    // -----------------------------------------------------------------------

    fn parse_triple_expr(&mut self) -> Result<TripleExpr, ParseError> {
        // A triple expression is a triple constraint, possibly followed by
        // `;`-separated constraints (EachOf) or `|`-separated (OneOf).
        // We parse one constraint and then check for separators.
        let first = self.parse_single_triple_constraint()?;
        // Check for `|` to form a OneOf.
        if self.peek_tok()? == Some(Tok::Pipe) {
            let mut members = vec![TripleExpr::Constraint(first)];
            while self.peek_tok()? == Some(Tok::Pipe) {
                self.consume();
                let tc = self.parse_single_triple_constraint()?;
                members.push(TripleExpr::Constraint(tc));
            }
            return Ok(TripleExpr::OneOf(members));
        }
        Ok(TripleExpr::Constraint(first))
    }

    fn parse_single_triple_constraint(&mut self) -> Result<TripleConstraint, ParseError> {
        let offset = self.lex.offset();

        // Optional INVERSE / `^`.
        let inverse = match self.peek_tok()? {
            Some(Tok::KwInverse) => {
                self.consume();
                true
            }
            Some(Tok::Caret) => {
                self.consume();
                true
            }
            _ => false,
        };

        let predicate = self.parse_predicate()?;

        // Value expression — optional.
        let value_expr = self.parse_inline_shape_expr()?;

        // Cardinality.
        let cardinality = self.parse_cardinality()?;

        Ok(TripleConstraint {
            inverse,
            predicate,
            value_expr: value_expr.map(Box::new),
            cardinality,
            offset,
        })
    }

    /// Parse the optional value expression for a triple constraint.
    /// Returns `None` if the next token is a separator, `}`, `;`, EOF, or
    /// a cardinality marker.
    fn parse_inline_shape_expr(&mut self) -> Result<Option<ShapeExpr>, ParseError> {
        match self.peek_tok()? {
            // Cardinality markers or delimiters end the value expr.
            // LBrace might be cardinality {n,m} or a nested shape — see below.
            Some(
                Tok::Star
                | Tok::Plus
                | Tok::Question
                | Tok::LBrace
                | Tok::Semi
                | Tok::RBrace
                | Tok::Pipe,
            )
            | None => {
                // LBrace could be `{n,m}` or `{ … }`. Disambiguate by peeking
                // what follows: if it's an integer, it's cardinality; otherwise shape.
                if self.peek_tok()? == Some(Tok::LBrace) {
                    if self.is_cardinality_brace()? {
                        return Ok(None);
                    }
                    // It's a nested shape.
                    let shape = self.parse_shape()?;
                    return Ok(Some(ShapeExpr::Shape(shape)));
                }
                Ok(None)
            }
            // `.` wildcard — matches any node.
            Some(Tok::KwAny) => {
                self.consume();
                Ok(Some(ShapeExpr::Any))
            }
            // Everything else: try to parse a full shape expression.
            _ => {
                // Guard against cardinality-only position: if we see `LBrace` followed
                // by an integer, it's cardinality, handled above.
                let expr = self.parse_shape_expr()?;
                Ok(Some(expr))
            }
        }
    }

    /// Look ahead to determine if the upcoming `{` is a cardinality `{n,m}`
    /// rather than a shape body.
    fn is_cardinality_brace(&mut self) -> Result<bool, ParseError> {
        // Save full state: both the lex position AND any peeked token.
        let saved_lex_pos = self.lex.offset();
        let saved_peeked = self.peeked.take();

        // Now consume the LBrace and peek what follows.
        // We need to also restore the peeked token correctly.
        // Since peeked was taken, next_tok will read from lex.
        // But lex is already past the `{` (peeked held the `{` token).
        // We need to know the lex position of the `{` token itself.
        //
        // Strategy: the `{` is in saved_peeked. After the `{`, lex is at
        // `saved_lex_pos`. Peek the next token from there.
        let result = if saved_peeked.as_ref().is_some_and(|s| s.tok == Tok::LBrace) {
            // Peeked already has `{`. lex is past the `{`.
            // Peek what comes after `{`.
            let next = self.lex.next()?;
            let is_cardinality = matches!(next.as_ref().map(|s| &s.tok), Some(Tok::Integer(_)));
            // Restore lex to just after `{` (saved_lex_pos), put back the peeked token.
            self.lex.seek(saved_lex_pos);
            is_cardinality
        } else {
            false
        };

        // Restore peeked.
        self.peeked = saved_peeked;
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Cardinality
    // -----------------------------------------------------------------------

    fn parse_cardinality(&mut self) -> Result<Cardinality, ParseError> {
        match self.peek_tok()? {
            Some(Tok::Star) => {
                self.consume();
                Ok(Cardinality::Star)
            }
            Some(Tok::Plus) => {
                self.consume();
                Ok(Cardinality::Plus)
            }
            Some(Tok::Question) => {
                self.consume();
                Ok(Cardinality::Optional)
            }
            Some(Tok::LBrace) => {
                if self.is_cardinality_brace()? {
                    self.consume(); // consume LBrace
                    let n = self.expect_integer("lower bound in cardinality")?;
                    match self.peek_tok()? {
                        Some(Tok::Comma) => {
                            self.consume();
                            match self.peek_tok()? {
                                Some(Tok::RBrace) => {
                                    // `{n,}` — unbounded.
                                    self.consume();
                                    Ok(Cardinality::AtLeast(n))
                                }
                                Some(Tok::Integer(_)) => {
                                    let m = self.expect_integer("upper bound in cardinality")?;
                                    self.expect_tok(&Tok::RBrace, "'}' to close cardinality")?;
                                    Ok(Cardinality::Range(n, m))
                                }
                                other => Err(ParseError::new(
                                    self.lex.offset(),
                                    format!("expected integer or '}}' after comma in cardinality, got {other:?}"),
                                )),
                            }
                        }
                        Some(Tok::RBrace) => {
                            self.consume();
                            Ok(Cardinality::Exact(n))
                        }
                        other => Err(ParseError::new(
                            self.lex.offset(),
                            format!("expected ',' or '}}' in cardinality, got {other:?}"),
                        )),
                    }
                } else {
                    Ok(Cardinality::One)
                }
            }
            _ => Ok(Cardinality::One),
        }
    }

    // -----------------------------------------------------------------------
    // Predicates
    // -----------------------------------------------------------------------

    fn parse_predicate(&mut self) -> Result<Predicate, ParseError> {
        let sp = self.next_tok()?.ok_or_else(|| {
            ParseError::new(self.lex.offset(), "expected predicate")
        })?;
        match sp.tok {
            Tok::KwA => Ok(Predicate::RdfType),
            Tok::IriRef(iri) => Ok(Predicate::Iri(iri)),
            Tok::Pname { prefix, local } => Ok(Predicate::Pname { prefix, local }),
            other => Err(ParseError::new(
                sp.start,
                format!("expected predicate (IRI or 'a'), got {other:?}"),
            )),
        }
    }

    // -----------------------------------------------------------------------
    // Value set `[ val* ]`
    // -----------------------------------------------------------------------

    fn parse_value_set(&mut self) -> Result<Vec<ValueSetItem>, ParseError> {
        self.expect_tok(&Tok::LBracket, "'[' to open value set")?;
        let mut items = Vec::new();
        loop {
            match self.peek_tok()? {
                Some(Tok::RBracket) => {
                    self.consume();
                    break;
                }
                None => {
                    return Err(ParseError::new(
                        self.lex.offset(),
                        "unterminated value set: expected ']'",
                    ));
                }
                Some(Tok::IriRef(_)) => {
                    let sp = self.next_tok()?.unwrap();
                    if let Tok::IriRef(iri) = sp.tok {
                        items.push(ValueSetItem::Iri(iri));
                    }
                }
                Some(Tok::Pname { .. }) => {
                    let sp = self.next_tok()?.unwrap();
                    if let Tok::Pname { prefix, local } = sp.tok {
                        items.push(ValueSetItem::Pname { prefix, local });
                    }
                }
                Some(Tok::StringLit(_)) => {
                    let sp = self.next_tok()?.unwrap();
                    if let Tok::StringLit(lex) = sp.tok {
                        // Optional datatype or language tag.
                        let formatted = match self.peek_tok()? {
                            Some(Tok::DataTypeMark) => {
                                self.consume();
                                let dt_sp = self.next_tok()?.ok_or_else(|| {
                                    ParseError::new(
                                        self.lex.offset(),
                                        "expected datatype IRI after '^^'",
                                    )
                                })?;
                                let dt_iri = match dt_sp.tok {
                                    Tok::IriRef(i) => i,
                                    Tok::Pname { prefix, local } => {
                                        format!("{prefix}:{local}")
                                    }
                                    other => {
                                        return Err(ParseError::new(
                                            dt_sp.start,
                                            format!("expected IRI after '^^', got {other:?}"),
                                        ));
                                    }
                                };
                                format!("\"{lex}\"^^<{dt_iri}>")
                            }
                            Some(Tok::LangTag(_)) => {
                                let sp2 = self.next_tok()?.unwrap();
                                if let Tok::LangTag(tag) = sp2.tok {
                                    format!("\"{lex}\"@{tag}")
                                } else {
                                    unreachable!()
                                }
                            }
                            _ => format!("\"{lex}\""),
                        };
                        items.push(ValueSetItem::Literal(formatted));
                    }
                }
                // `.` (KwAny) inside value set — not standard; handled defensively.
                Some(Tok::KwAny) if false => {
                    self.consume();
                }
                other => {
                    return Err(ParseError::new(
                        self.lex.offset(),
                        format!("unexpected token in value set: {other:?}"),
                    ));
                }
            }
        }
        Ok(items)
    }

    // -----------------------------------------------------------------------
    // Shape references and labels
    // -----------------------------------------------------------------------

    fn parse_shape_ref(&mut self) -> Result<Label, ParseError> {
        // `@<label>` — `@` already consumed by caller, or called after `=`.
        match self.peek_tok()? {
            Some(Tok::At) => {
                self.consume();
                self.parse_label()
            }
            _ => self.parse_label(),
        }
    }

    fn parse_label(&mut self) -> Result<Label, ParseError> {
        let sp = self.next_tok()?.ok_or_else(|| {
            ParseError::new(self.lex.offset(), "expected shape label (IRI or blank node)")
        })?;
        tok_to_label(sp.tok, sp.start)
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn expect_iriref(&mut self, context: &str) -> Result<String, ParseError> {
        let sp = self.next_tok()?.ok_or_else(|| {
            ParseError::new(self.lex.offset(), format!("expected IRI for {context}"))
        })?;
        match sp.tok {
            Tok::IriRef(iri) => Ok(iri),
            other => Err(ParseError::new(
                sp.start,
                format!("expected IRI for {context}, got {other:?}"),
            )),
        }
    }

    fn expect_integer(&mut self, context: &str) -> Result<u32, ParseError> {
        let sp = self.next_tok()?.ok_or_else(|| {
            ParseError::new(
                self.lex.offset(),
                format!("expected integer for {context}"),
            )
        })?;
        match sp.tok {
            Tok::Integer(n) => Ok(n),
            other => Err(ParseError::new(
                sp.start,
                format!("expected integer for {context}, got {other:?}"),
            )),
        }
    }

    fn expect_tok(&mut self, expected: &Tok, context: &str) -> Result<(), ParseError> {
        let sp = self.next_tok()?.ok_or_else(|| {
            ParseError::new(
                self.lex.offset(),
                format!("expected {expected:?} for {context}, got EOF"),
            )
        })?;
        if std::mem::discriminant(&sp.tok) != std::mem::discriminant(expected) {
            return Err(ParseError::new(
                sp.start,
                format!("expected {expected:?} for {context}, got {:?}", sp.tok),
            ));
        }
        Ok(())
    }

    /// Peek the next token kind without consuming it.
    fn peek_tok(&mut self) -> Result<Option<Tok>, ParseError> {
        if self.peeked.is_none() {
            self.peeked = self.lex.next()?;
        }
        Ok(self.peeked.as_ref().map(|s| s.tok.clone()))
    }

    /// Consume the peeked token.
    fn consume(&mut self) {
        self.peeked = None;
    }

    /// Consume and return the next token.
    fn next_tok(&mut self) -> Result<Option<Spanned>, ParseError> {
        if let Some(sp) = self.peeked.take() {
            return Ok(Some(sp));
        }
        Ok(self.lex.next()?)
    }
}

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

fn tok_to_label(tok: Tok, start: usize) -> Result<Label, ParseError> {
    match tok {
        Tok::IriRef(iri) => Ok(Label::Iri(iri)),
        Tok::Pname { prefix, local } => Ok(Label::Pname { prefix, local }),
        Tok::BNodeLabel(label) => Ok(Label::BNode(label)),
        other => Err(ParseError::new(
            start,
            format!("expected shape label (IRI, prefixed name, or blank node), got {other:?}"),
        )),
    }
}

/// Convenience: given the schema's prefix map, expand a `Label` to an IRI
/// string in angle-bracket form. Returns a `ParseError` if the prefix is
/// not declared.
pub(crate) fn expand_label(
    label: &Label,
    prefixes: &BTreeMap<String, String>,
    base: Option<&str>,
) -> Result<String, ParseError> {
    match label {
        Label::Iri(iri) => Ok(resolve_iri(iri, base)),
        Label::Pname { prefix, local } => {
            let ns = prefixes.get(prefix.as_str()).ok_or_else(|| {
                ParseError::new(
                    0,
                    format!("undefined prefix '{prefix}:' in shape label"),
                )
            })?;
            Ok(format!("<{ns}{local}>"))
        }
        Label::BNode(label) => Ok(format!("_:{label}")),
    }
}

fn resolve_iri(iri: &str, _base: Option<&str>) -> String {
    // Wrap in angle brackets for canonical form — base resolution is out of
    // scope for a structural fact emitter.
    format!("<{iri}>")
}
