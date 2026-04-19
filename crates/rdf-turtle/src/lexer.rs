//! Tokeniser for Turtle 1.1 / TriG.
//!
//! A small hand-rolled lexer with just enough state to distinguish
//! short-vs-long string literals (pin `TTL-LITESC-001`), numeric literal
//! shapes (integer / decimal / double), IRI references, prefixed names,
//! blank-node labels, directives, and single-byte punctuation.
//!
//! The lexer does **not** attempt IRI resolution or prefix expansion —
//! that is the grammar's job (`grammar.rs`). Strings have their `ECHAR`
//! / `UCHAR` escapes decoded here, so downstream consumers see a USV
//! `String`.

use crate::diag::{Diag, DiagnosticCode};

/// Numeric literal category (Turtle §2.5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NumKind {
    /// `xsd:integer` — no dot, no exponent.
    Integer,
    /// `xsd:decimal` — dot present, no exponent.
    Decimal,
    /// `xsd:double` — exponent present.
    Double,
}

/// A single lexical token. Offsets are byte indices into the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// `<iri>` body (decoded UCHARs applied).
    IriRef(String),
    /// `prefix:local` — both parts kept separate. `local` may be empty.
    Pname {
        /// Prefix name, possibly empty (`:local`).
        prefix: String,
        /// Local part, possibly empty (`ex:`).
        local: String,
    },
    /// `_:label`.
    BNodeLabel(String),
    /// `[]` / `[` / `]`.
    LBracket,
    /// `]`.
    RBracket,
    /// `(`.
    LParen,
    /// `)`.
    RParen,
    /// `{`.
    LBrace,
    /// `}`.
    RBrace,
    /// `,`.
    Comma,
    /// `;`.
    Semicolon,
    /// `.` (statement terminator).
    Dot,
    /// `a` keyword (rdf:type).
    KwA,
    /// `true` keyword.
    KwTrue,
    /// `false` keyword.
    KwFalse,
    /// `@prefix`.
    DirPrefix,
    /// `@base`.
    DirBase,
    /// SPARQL-style `PREFIX` (case-insensitive per §2.3).
    SparqlPrefix,
    /// SPARQL-style `BASE` (case-insensitive per §2.3).
    SparqlBase,
    /// TriG `GRAPH` keyword (§2.2).
    KwGraph,
    /// Language tag body (no leading `@`), validated shape only.
    LangTag(String),
    /// `^^` datatype marker.
    DataTypeMark,
    /// String literal — decoded USV contents.
    StringLit(String),
    /// Numeric literal — lexeme preserved verbatim.
    NumberLit {
        /// Numeric category.
        kind: NumKind,
        /// The raw lexeme as written (sign, digits, dot, exponent).
        lexeme: String,
    },
}

/// One token plus its byte-offset span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    /// The token payload.
    pub(crate) tok: Tok,
    /// Start byte offset (0-indexed).
    pub(crate) start: usize,
    /// End byte offset (exclusive).
    pub(crate) end: usize,
}

/// Hand-rolled Turtle / TriG lexer.
pub(crate) struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Build a fresh lexer over `src`.
    pub(crate) const fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    /// Current byte offset.
    pub(crate) const fn offset(&self) -> usize {
        self.pos
    }

    /// Set the current byte offset. Used by grammar lookahead to rewind.
    pub(crate) const fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Skip whitespace and `# …\n` comments.
    fn skip_trivia(&mut self) {
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if matches!(b, b' ' | b'\t' | b'\r' | b'\n') {
                self.pos += 1;
            } else if b == b'#' {
                while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Peek the next token without advancing. Convenience over `next`.
    pub(crate) fn peek(&mut self) -> Result<Option<Spanned>, Diag> {
        let save = self.pos;
        let out = self.next()?;
        self.pos = save;
        Ok(out)
    }

    /// Produce the next token, or `None` at EOF.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn next(&mut self) -> Result<Option<Spanned>, Diag> {
        self.skip_trivia();
        if self.pos >= self.src.len() {
            return Ok(None);
        }
        let start = self.pos;
        let b = self.src[self.pos];
        // Punctuation / single-byte tokens.
        let one = |this: &mut Self, tok: Tok| -> Result<Option<Spanned>, Diag> {
            this.pos += 1;
            Ok(Some(Spanned {
                tok,
                start: this.pos - 1,
                end: this.pos,
            }))
        };
        match b {
            b'(' => return one(self, Tok::LParen),
            b')' => return one(self, Tok::RParen),
            b'[' => return one(self, Tok::LBracket),
            b']' => return one(self, Tok::RBracket),
            b'{' => return one(self, Tok::LBrace),
            b'}' => return one(self, Tok::RBrace),
            b',' => return one(self, Tok::Comma),
            b';' => return one(self, Tok::Semicolon),
            _ => {}
        }
        // IRIREF.
        if b == b'<' {
            let (iri, end) = self.lex_iriref(start)?;
            return Ok(Some(Spanned {
                tok: Tok::IriRef(iri),
                start,
                end,
            }));
        }
        // String literal.
        if b == b'"' || b == b'\'' {
            let (s, end) = self.lex_string(start, b)?;
            return Ok(Some(Spanned {
                tok: Tok::StringLit(s),
                start,
                end,
            }));
        }
        // Language tag.
        if b == b'@' {
            return self.lex_at(start);
        }
        // Datatype marker `^^`.
        if b == b'^' {
            if self.src.get(self.pos + 1).copied() == Some(b'^') {
                self.pos += 2;
                return Ok(Some(Spanned {
                    tok: Tok::DataTypeMark,
                    start,
                    end: self.pos,
                }));
            }
            return Err(diag_syntax(start, "lone '^' — expected '^^'"));
        }
        // Numeric literal (sign-prefixed or plain digit/dot).
        if b == b'+' || b == b'-' || b == b'.' || b.is_ascii_digit() {
            if b == b'.' {
                // A bare '.' is a statement terminator unless followed by
                // a digit (which would make it a decimal literal).
                let next = self.src.get(self.pos + 1).copied();
                if !next.is_some_and(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                    return Ok(Some(Spanned {
                        tok: Tok::Dot,
                        start,
                        end: self.pos,
                    }));
                }
            }
            // Try numeric first; fall through to pname/keyword for `+`/`-`
            // only if no digit follows the sign.
            if let Some(tok) = self.lex_number(start)? {
                return Ok(Some(tok));
            }
        }
        // Blank node label `_:label`.
        if b == b'_' && self.src.get(self.pos + 1).copied() == Some(b':') {
            let (label, end) = self.lex_bnode(start)?;
            return Ok(Some(Spanned {
                tok: Tok::BNodeLabel(label),
                start,
                end,
            }));
        }
        // Prefixed name or keyword (`a`, `true`, `false`, `PREFIX`, `BASE`, `GRAPH`).
        if is_pn_chars_base(b) || b == b':' {
            let (prefix, local, end, had_colon) = self.lex_pname(start)?;
            // Bare keywords (`a`, `true`, `false`, SPARQL `PREFIX`/`BASE`/
            // `GRAPH`) must NOT carry a trailing colon. A trailing colon
            // means the identifier is a pname prefix (possibly with empty
            // local), NOT a keyword — otherwise `@prefix a: <…>` would be
            // lexed as `@prefix a <…>` because the colon is silently eaten.
            let tok = if had_colon {
                Tok::Pname { prefix, local }
            } else {
                match prefix.as_str() {
                    "a" => Tok::KwA,
                    "true" => Tok::KwTrue,
                    "false" => Tok::KwFalse,
                    p if p.eq_ignore_ascii_case("prefix") => Tok::SparqlPrefix,
                    p if p.eq_ignore_ascii_case("base") => Tok::SparqlBase,
                    p if p.eq_ignore_ascii_case("graph") => Tok::KwGraph,
                    _ => {
                        return Err(diag_syntax(
                            start,
                            "expected prefixed name (identifier must be followed by ':')",
                        ));
                    }
                }
            };
            return Ok(Some(Spanned { tok, start, end }));
        }
        Err(diag_syntax(
            start,
            &format!("unexpected byte 0x{b:02X} at top level"),
        ))
    }

    // -- sub-lexers -----------------------------------------------------

    fn lex_iriref(&mut self, start: usize) -> Result<(String, usize), Diag> {
        debug_assert_eq!(self.src[self.pos], b'<');
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == b'>' {
                self.pos += 1;
                crate::iri::validate_iri_body(&out, start + 1)?;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let decoded = self.lex_uchar(self.pos)?;
                out.push(decoded);
                continue;
            }
            // Forbidden characters inside an IRIREF (checked here *and*
            // by validate_iri_body after decoding).
            if matches!(b, 0x00..=0x20) {
                return Err(Diag {
                    code: DiagnosticCode::Syntax,
                    message: format!("control byte 0x{b:02X} inside <IRI>"),
                    offset: self.pos,
                    fatal: true,
                });
            }
            // Push one UTF-8 char.
            let ch_start = self.pos;
            let ch = next_utf8_char(self.src, &mut self.pos)
                .ok_or_else(|| diag_syntax(ch_start, "invalid UTF-8 in IRI"))?;
            out.push(ch);
        }
        Err(Diag {
            code: DiagnosticCode::Unterminated,
            message: "unterminated <IRI>".into(),
            offset: start,
            fatal: true,
        })
    }

    fn lex_string(&mut self, start: usize, quote: u8) -> Result<(String, usize), Diag> {
        // Detect triple-quote (long string).
        let triple = self.src.get(self.pos + 1).copied() == Some(quote)
            && self.src.get(self.pos + 2).copied() == Some(quote);
        if triple {
            self.pos += 3;
            return self.lex_long_string(start, quote);
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == quote {
                self.pos += 1;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let decoded = self.lex_echar_or_uchar(self.pos)?;
                out.push(decoded);
                continue;
            }
            // TTL-LITESC-001: forbidden raw characters in short strings.
            // Grammar's char class forbids LF/CR; the prose extends to
            // LS (U+2028) and NEL (U+0085).
            if matches!(b, b'\n' | b'\r') {
                return Err(Diag {
                    code: DiagnosticCode::LitEsc,
                    message: format!(
                        "unescaped {} (U+{:04X}) in short string",
                        if b == b'\n' { "LF" } else { "CR" },
                        u32::from(b),
                    ),
                    offset: self.pos,
                    fatal: true,
                });
            }
            let ch_start = self.pos;
            let ch = next_utf8_char(self.src, &mut self.pos)
                .ok_or_else(|| diag_syntax(ch_start, "invalid UTF-8 in string"))?;
            if matches!(ch, '\u{2028}' | '\u{0085}') {
                return Err(Diag {
                    code: DiagnosticCode::LitEsc,
                    message: format!(
                        "unescaped {} (U+{:04X}) in short string",
                        if ch == '\u{2028}' { "LS" } else { "NEL" },
                        u32::from(ch),
                    ),
                    offset: ch_start,
                    fatal: true,
                });
            }
            out.push(ch);
        }
        Err(Diag {
            code: DiagnosticCode::Unterminated,
            message: "unterminated short string".into(),
            offset: start,
            fatal: true,
        })
    }

    fn lex_long_string(&mut self, start: usize, quote: u8) -> Result<(String, usize), Diag> {
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            // Triple-quote close.
            if b == quote
                && self.src.get(self.pos + 1).copied() == Some(quote)
                && self.src.get(self.pos + 2).copied() == Some(quote)
            {
                self.pos += 3;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let decoded = self.lex_echar_or_uchar(self.pos)?;
                out.push(decoded);
                continue;
            }
            let ch_start = self.pos;
            let ch = next_utf8_char(self.src, &mut self.pos)
                .ok_or_else(|| diag_syntax(ch_start, "invalid UTF-8 in long string"))?;
            out.push(ch);
        }
        Err(Diag {
            code: DiagnosticCode::Unterminated,
            message: "unterminated \"\"\"…\"\"\" string".into(),
            offset: start,
            fatal: true,
        })
    }

    /// Decode one `ECHAR` or `UCHAR`. Position points at the `\\`.
    fn lex_echar_or_uchar(&mut self, at: usize) -> Result<char, Diag> {
        debug_assert_eq!(self.src[self.pos], b'\\');
        if self.pos + 1 >= self.src.len() {
            return Err(Diag {
                code: DiagnosticCode::LitEsc,
                message: "escape at end of input".into(),
                offset: at,
                fatal: true,
            });
        }
        let esc = self.src[self.pos + 1];
        match esc {
            b't' => {
                self.pos += 2;
                Ok('\t')
            }
            b'b' => {
                self.pos += 2;
                Ok('\u{0008}')
            }
            b'n' => {
                self.pos += 2;
                Ok('\n')
            }
            b'r' => {
                self.pos += 2;
                Ok('\r')
            }
            b'f' => {
                self.pos += 2;
                Ok('\u{000C}')
            }
            b'"' => {
                self.pos += 2;
                Ok('"')
            }
            b'\'' => {
                self.pos += 2;
                Ok('\'')
            }
            b'\\' => {
                self.pos += 2;
                Ok('\\')
            }
            b'u' | b'U' => self.lex_uchar(at),
            other => Err(Diag {
                code: DiagnosticCode::LitEsc,
                message: format!("unknown ECHAR escape '\\{}'", other as char),
                offset: at,
                fatal: true,
            }),
        }
    }

    /// Decode a `UCHAR` starting at the `\\`.
    fn lex_uchar(&mut self, at: usize) -> Result<char, Diag> {
        debug_assert_eq!(self.src[self.pos], b'\\');
        let marker = self.src.get(self.pos + 1).copied();
        let (width, step) = match marker {
            Some(b'u') => (4usize, 2usize),
            Some(b'U') => (8usize, 2usize),
            _ => {
                return Err(Diag {
                    code: DiagnosticCode::LitEsc,
                    message: "expected \\uXXXX or \\UXXXXXXXX".into(),
                    offset: at,
                    fatal: true,
                });
            }
        };
        let hex_start = self.pos + step;
        let hex_end = hex_start + width;
        if hex_end > self.src.len() {
            return Err(Diag {
                code: DiagnosticCode::LitEsc,
                message: "truncated UCHAR escape".into(),
                offset: at,
                fatal: true,
            });
        }
        let mut cp: u32 = 0;
        for &b in &self.src[hex_start..hex_end] {
            let d = match b {
                b'0'..=b'9' => u32::from(b - b'0'),
                b'a'..=b'f' => u32::from(b - b'a' + 10),
                b'A'..=b'F' => u32::from(b - b'A' + 10),
                _ => {
                    return Err(Diag {
                        code: DiagnosticCode::LitEsc,
                        message: "non-hex digit in UCHAR".into(),
                        offset: at,
                        fatal: true,
                    });
                }
            };
            cp = (cp << 4) | d;
        }
        self.pos = hex_end;
        // TTL-LITESC-001: surrogates are a fatal decode error.
        if (0xD800..=0xDFFF).contains(&cp) {
            return Err(Diag {
                code: DiagnosticCode::LitEsc,
                message: format!("surrogate UCHAR decode U+{cp:04X}"),
                offset: at,
                fatal: true,
            });
        }
        char::from_u32(cp).ok_or_else(|| Diag {
            code: DiagnosticCode::LitEsc,
            message: format!("invalid UCHAR code point U+{cp:04X}"),
            offset: at,
            fatal: true,
        })
    }

    fn lex_at(&mut self, start: usize) -> Result<Option<Spanned>, Diag> {
        debug_assert_eq!(self.src[self.pos], b'@');
        self.pos += 1;
        // Keyword directives.
        let rest = &self.src[self.pos..];
        if rest.starts_with(b"prefix") && !matches!(rest.get(6), Some(c) if is_pn_chars(*c)) {
            self.pos += 6;
            return Ok(Some(Spanned {
                tok: Tok::DirPrefix,
                start,
                end: self.pos,
            }));
        }
        if rest.starts_with(b"base") && !matches!(rest.get(4), Some(c) if is_pn_chars(*c)) {
            self.pos += 4;
            return Ok(Some(Spanned {
                tok: Tok::DirBase,
                start,
                end: self.pos,
            }));
        }
        // Language tag: [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*
        let tag_start = self.pos;
        if !self
            .src
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_alphabetic())
        {
            return Err(diag_syntax(start, "expected @prefix, @base, or language tag"));
        }
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        while self.pos < self.src.len() && self.src[self.pos] == b'-' {
            self.pos += 1;
            let subtag_start = self.pos;
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphanumeric() {
                self.pos += 1;
            }
            if self.pos == subtag_start {
                return Err(diag_syntax(self.pos, "empty subtag in language tag"));
            }
        }
        let tag = std::str::from_utf8(&self.src[tag_start..self.pos])
            .map_err(|_| diag_syntax(tag_start, "non-UTF-8 language tag"))?
            .to_owned();
        Ok(Some(Spanned {
            tok: Tok::LangTag(tag),
            start,
            end: self.pos,
        }))
    }

    fn lex_number(&mut self, start: usize) -> Result<Option<Spanned>, Diag> {
        let save = self.pos;
        let mut has_digit_before_dot = false;
        let mut has_dot = false;
        let mut has_exp = false;
        if matches!(self.src[self.pos], b'+' | b'-') {
            self.pos += 1;
        }
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
            self.pos += 1;
            has_digit_before_dot = true;
        }
        if self.pos < self.src.len() && self.src[self.pos] == b'.' {
            // Only a decimal literal if at least one digit follows.
            let after_dot = self.src.get(self.pos + 1).copied();
            if after_dot.is_some_and(|c| c.is_ascii_digit()) {
                has_dot = true;
                self.pos += 1;
                while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
            }
        }
        if self.pos < self.src.len() && matches!(self.src[self.pos], b'e' | b'E') {
            has_exp = true;
            self.pos += 1;
            if matches!(self.src.get(self.pos).copied(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == exp_start {
                return Err(diag_syntax(exp_start, "missing exponent digits"));
            }
        }
        if !has_digit_before_dot && !has_dot {
            // Not a number — caller must handle.
            self.pos = save;
            return Ok(None);
        }
        let lexeme = std::str::from_utf8(&self.src[save..self.pos])
            .expect("ASCII-only numeric tokens")
            .to_owned();
        let kind = if has_exp {
            NumKind::Double
        } else if has_dot {
            NumKind::Decimal
        } else {
            NumKind::Integer
        };
        Ok(Some(Spanned {
            tok: Tok::NumberLit { kind, lexeme },
            start,
            end: self.pos,
        }))
    }

    fn lex_bnode(&mut self, start: usize) -> Result<(String, usize), Diag> {
        debug_assert_eq!(self.src[self.pos], b'_');
        self.pos += 2; // skip "_:"
        let label_start = self.pos;
        // BLANK_NODE_LABEL: start = (PN_CHARS_U | [0-9]); body = (PN_CHARS | '.')* with
        // no trailing '.'.
        if self.pos >= self.src.len() {
            return Err(diag_syntax(start, "empty blank-node label"));
        }
        let first = self.src[self.pos];
        if !(is_pn_chars_u(first) || first.is_ascii_digit()) {
            return Err(diag_syntax(self.pos, "invalid blank-node label start"));
        }
        self.pos += 1;
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if is_pn_chars(b) || b == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        // Trim trailing dots — they are statement terminators, not label
        // content.
        while self.pos > label_start + 1 && self.src[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        let label = std::str::from_utf8(&self.src[label_start..self.pos])
            .map_err(|_| diag_syntax(label_start, "non-UTF-8 blank-node label"))?
            .to_owned();
        Ok((label, self.pos))
    }

    fn lex_pname(&mut self, start: usize) -> Result<(String, String, usize, bool), Diag> {
        // PN_PREFIX ::= PN_CHARS_BASE ((PN_CHARS | '.')* PN_CHARS)?
        let prefix_start = self.pos;
        if self.src[self.pos] != b':' {
            if !is_pn_chars_base(self.src[self.pos]) {
                return Err(diag_syntax(start, "expected PN_PREFIX start"));
            }
            self.pos += 1;
            while self.pos < self.src.len() {
                let b = self.src[self.pos];
                if is_pn_chars(b) || b == b'.' {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            // No trailing dot in PN_PREFIX.
            while self.pos > prefix_start + 1 && self.src[self.pos - 1] == b'.' {
                self.pos -= 1;
            }
        }
        let prefix_end = self.pos;
        // Must be followed by ':' for a pname. If there is no ':', this is
        // a bare keyword-shape (e.g. `a`, `true`). The caller uses the
        // `had_colon` flag to distinguish a bare keyword from an empty-
        // local pname — those cases are lexically identical up to the
        // colon but grammatically distinct (e.g. `a` vs `a:`).
        if self.pos >= self.src.len() || self.src[self.pos] != b':' {
            let name = std::str::from_utf8(&self.src[prefix_start..prefix_end])
                .map_err(|_| diag_syntax(prefix_start, "non-UTF-8 identifier"))?
                .to_owned();
            return Ok((name, String::new(), self.pos, false));
        }
        let prefix = std::str::from_utf8(&self.src[prefix_start..prefix_end])
            .map_err(|_| diag_syntax(prefix_start, "non-UTF-8 prefix"))?
            .to_owned();
        self.pos += 1; // consume ':'

        // PN_LOCAL body.
        let mut local = String::new();
        let local_start = self.pos;
        let mut first_char = true;
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == b'%' {
                if self.pos + 2 >= self.src.len()
                    || !is_hex(self.src[self.pos + 1])
                    || !is_hex(self.src[self.pos + 2])
                {
                    return Err(Diag {
                        code: DiagnosticCode::LocalEscape,
                        message: "malformed PERCENT escape in PN_LOCAL".into(),
                        offset: self.pos,
                        fatal: true,
                    });
                }
                local.push('%');
                local.push(self.src[self.pos + 1] as char);
                local.push(self.src[self.pos + 2] as char);
                self.pos += 3;
                first_char = false;
                continue;
            }
            if b == b'\\' {
                // PN_LOCAL_ESC ::= '\' [_~.!$&'()*+,;=/?#@%-]
                let next = self.src.get(self.pos + 1).copied();
                if next.is_some_and(is_pn_local_esc) {
                    local.push(next.unwrap() as char);
                    self.pos += 2;
                    first_char = false;
                    continue;
                }
                return Err(Diag {
                    code: DiagnosticCode::LocalEscape,
                    message: "invalid PN_LOCAL_ESC".into(),
                    offset: self.pos,
                    fatal: true,
                });
            }
            let ok_start = is_pn_chars_u(b) || b == b':' || b.is_ascii_digit();
            let ok_cont = is_pn_chars(b) || b == b':' || b == b'.';
            if (first_char && ok_start) || (!first_char && ok_cont) {
                // Consume one UTF-8 char.
                let ch_start = self.pos;
                let ch = next_utf8_char(self.src, &mut self.pos)
                    .ok_or_else(|| diag_syntax(ch_start, "invalid UTF-8 in local part"))?;
                local.push(ch);
                first_char = false;
                continue;
            }
            break;
        }
        // No trailing '.' in PN_LOCAL.
        while local.ends_with('.') {
            local.pop();
            self.pos -= 1;
        }
        // If we opened on ':' but produced no local chars, that is fine —
        // the prefix name `ex:` is a valid pname with empty local.
        let _ = local_start;
        Ok((prefix, local, self.pos, true))
    }
}

// -- character class helpers ------------------------------------------------

fn is_hex(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
}

fn is_pn_chars_base(b: u8) -> bool {
    b.is_ascii_alphabetic() || b >= 0x80
}

fn is_pn_chars_u(b: u8) -> bool {
    is_pn_chars_base(b) || b == b'_'
}

fn is_pn_chars(b: u8) -> bool {
    is_pn_chars_u(b) || b.is_ascii_digit() || b == b'-' || b == 0xB7
}

fn is_pn_local_esc(b: u8) -> bool {
    matches!(
        b,
        b'_' | b'~'
            | b'.'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'/'
            | b'?'
            | b'#'
            | b'@'
            | b'%'
            | b'-'
    )
}

/// Pull a single UTF-8 scalar out of `src` starting at `*pos`; advance
/// `*pos` past it. Returns `None` on invalid UTF-8.
fn next_utf8_char(src: &[u8], pos: &mut usize) -> Option<char> {
    let remaining = src.get(*pos..)?;
    let s = std::str::from_utf8(remaining).ok()?;
    let ch = s.chars().next()?;
    *pos += ch.len_utf8();
    Some(ch)
}

fn diag_syntax(offset: usize, msg: &str) -> Diag {
    Diag {
        code: DiagnosticCode::Syntax,
        message: msg.to_owned(),
        offset,
        fatal: true,
    }
}
