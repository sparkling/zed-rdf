//! Recursive-descent parser for Datalog syntax.
//!
//! Grammar (from ADR-0023):
//! ```text
//! program    ::= statement*
//! statement  ::= rule | fact | comment
//! fact       ::= atom '.'
//! rule       ::= atom ':-' body '.'
//! body       ::= literal (',' literal)*
//! literal    ::= atom | 'not' atom
//! atom       ::= relname '(' arglist? ')'
//! arglist    ::= arg (',' arg)*
//! arg        ::= variable | constant
//! variable   ::= [A-Z][a-zA-Z0-9_]*
//! constant   ::= [a-z][a-zA-Z0-9_]* | '"' [^"]* '"'
//! relname    ::= [a-z][a-zA-Z0-9_]*
//! comment    ::= '%' [^\n]* '\n'
//! ```
//! Comments are stripped by the lexer, so the parser never sees them.

use crate::ast::{Arg, Atom, Literal, Program, Rule, Statement};
use crate::lexer::{Token, TokenKind};

/// A parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Human-readable description.
    pub message: String,
    /// Byte offset where the error was detected.
    pub offset: usize,
}

/// Parse a flat token slice into a [`Program`], returning the first error
/// encountered.
///
/// # Errors
///
/// Returns a [`ParseError`] when the token stream does not conform to the
/// Datalog grammar. The error carries the byte offset of the offending token.
pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut ctx = Ctx::new(tokens);
    let prog = ctx.parse_program()?;
    Ok(prog)
}

// ---------------------------------------------------------------------------
// Internal parser context
// ---------------------------------------------------------------------------

struct Ctx<'t> {
    tokens: &'t [Token],
    pos: usize,
}

impl<'t> Ctx<'t> {
    const fn new(tokens: &'t [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Current token (never panics — returns the Eof sentinel if past end).
    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    /// Consume one token.
    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    /// Expect the current token to match `kind`; advance and return it, or
    /// return an error.
    fn expect_dot(&mut self) -> Result<(), ParseError> {
        let tok = self.peek();
        if tok.kind == TokenKind::Dot {
            self.advance();
            Ok(())
        } else {
            Err(ParseError {
                message: format!(
                    "expected '.' but found {:?} at offset {}",
                    tok.kind, tok.offset
                ),
                offset: tok.offset,
            })
        }
    }

    fn expect_lparen(&mut self) -> Result<(), ParseError> {
        let tok = self.peek();
        if tok.kind == TokenKind::LParen {
            self.advance();
            Ok(())
        } else {
            Err(ParseError {
                message: format!(
                    "expected '(' but found {:?} at offset {}",
                    tok.kind, tok.offset
                ),
                offset: tok.offset,
            })
        }
    }

    fn expect_rparen(&mut self) -> Result<(), ParseError> {
        let tok = self.peek();
        if tok.kind == TokenKind::RParen {
            self.advance();
            Ok(())
        } else {
            Err(ParseError {
                message: format!(
                    "expected ')' but found {:?} at offset {}",
                    tok.kind, tok.offset
                ),
                offset: tok.offset,
            })
        }
    }

    // -----------------------------------------------------------------------
    // Grammar rules
    // -----------------------------------------------------------------------

    /// `program ::= statement*`
    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        loop {
            if self.peek().kind == TokenKind::Eof {
                break;
            }
            let stmt = self.parse_statement()?;
            statements.push(stmt);
        }
        Ok(Program { statements })
    }

    /// `statement ::= rule | fact`
    ///
    /// We parse the head atom first, then look ahead for `:-` (rule) or `.`
    /// (fact).
    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let atom = self.parse_atom()?;
        match self.peek().kind {
            TokenKind::ColonDash => {
                self.advance(); // consume `:-`
                let body = self.parse_body()?;
                self.expect_dot()?;
                Ok(Statement::Rule(Rule { head: atom, body }))
            }
            TokenKind::Dot => {
                self.advance(); // consume `.`
                Ok(Statement::Fact(atom))
            }
            _ => {
                let tok = self.peek();
                Err(ParseError {
                    message: format!(
                        "expected '.' or ':-' after atom at offset {}",
                        tok.offset
                    ),
                    offset: tok.offset,
                })
            }
        }
    }

    /// `body ::= literal (',' literal)*`
    fn parse_body(&mut self) -> Result<Vec<Literal>, ParseError> {
        let mut lits = Vec::new();
        lits.push(self.parse_literal()?);
        while self.peek().kind == TokenKind::Comma {
            self.advance(); // consume `,`
            lits.push(self.parse_literal()?);
        }
        Ok(lits)
    }

    /// `literal ::= atom | 'not' atom`
    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        if self.peek().kind == TokenKind::Not {
            self.advance(); // consume `not`
            let atom = self.parse_atom()?;
            Ok(Literal::Negative(atom))
        } else {
            let atom = self.parse_atom()?;
            Ok(Literal::Positive(atom))
        }
    }

    /// `atom ::= relname '(' arglist? ')'`
    ///
    /// `relname` must be a lowercase identifier. An uppercase identifier is
    /// rejected with a fatal error (uppercase relation names are forbidden).
    fn parse_atom(&mut self) -> Result<Atom, ParseError> {
        let tok = self.peek();
        let relname = match &tok.kind {
            TokenKind::Lowercase(name) => {
                let name = name.clone();
                let _ = self.advance();
                name
            }
            TokenKind::Uppercase(_) => {
                return Err(ParseError {
                    message: format!(
                        "relation name must start with a lowercase letter at offset {}",
                        tok.offset
                    ),
                    offset: tok.offset,
                });
            }
            _ => {
                return Err(ParseError {
                    message: format!(
                        "expected relation name (lowercase identifier) at offset {}",
                        tok.offset
                    ),
                    offset: tok.offset,
                });
            }
        };

        self.expect_lparen()?;

        let args = if self.peek().kind == TokenKind::RParen {
            Vec::new()
        } else {
            self.parse_arglist()?
        };

        self.expect_rparen()?;

        Ok(Atom { relname, args })
    }

    /// `arglist ::= arg (',' arg)*`
    fn parse_arglist(&mut self) -> Result<Vec<Arg>, ParseError> {
        let mut args = Vec::new();
        args.push(self.parse_arg()?);
        while self.peek().kind == TokenKind::Comma {
            self.advance(); // consume `,`
            // Allow trailing comma before `)` to be a parse error via parse_arg
            args.push(self.parse_arg()?);
        }
        Ok(args)
    }

    /// `arg ::= variable | constant`
    ///
    /// `variable ::= [A-Z][a-zA-Z0-9_]*`
    /// `constant ::= [a-z][a-zA-Z0-9_]* | '"' [^"]* '"'`
    fn parse_arg(&mut self) -> Result<Arg, ParseError> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Uppercase(name) => {
                let name = name.clone();
                self.advance();
                Ok(Arg::Variable(name))
            }
            TokenKind::Lowercase(name) => {
                let name = name.clone();
                self.advance();
                Ok(Arg::Constant(name))
            }
            TokenKind::QuotedStr(s) => {
                let s = s.clone();
                self.advance();
                Ok(Arg::Constant(format!("\"{s}\"")))
            }
            _ => Err(ParseError {
                message: format!(
                    "expected argument (variable or constant) at offset {}",
                    tok.offset
                ),
                offset: tok.offset,
            }),
        }
    }
}
