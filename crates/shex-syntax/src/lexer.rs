//! Tokeniser for ShEx 2.x compact syntax (ShExC).
//!
//! A hand-rolled lexer that produces one token at a time. It handles:
//! - IRIREF `<…>`
//! - Prefixed names `prefix:local`
//! - Blank-node labels `_:label`
//! - String literals `"…"` / `'…'` / `"""…"""` / `'''…'''`
//! - Keywords: `PREFIX`, `BASE`, `CLOSED`, `EXTENDS`, `AND`, `OR`,
//!   `NOT`, `IRI`, `LITERAL`, `NONLITERAL`, `BNODE`, `INVERSE`, `ANY`
//! - Punctuation: `{`, `}`, `(`, `)`, `[`, `]`, `;`, `,`, `@`, `$`
//! - Cardinality: `*`, `+`, `?`, `{n}`, `{n,m}`, `{n,}`
//! - Comments `#…\n`

/// A single lexical token produced by the ShEx lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tok {
    /// `<iri>` body (UCHAR-decoded).
    IriRef(String),
    /// `prefix:local` — both parts kept separate. `local` may be empty.
    Pname {
        /// Prefix part (before `:`), possibly empty.
        prefix: String,
        /// Local part (after `:`), possibly empty.
        local: String,
    },
    /// `_:label`.
    BNodeLabel(String),
    /// `PREFIX` keyword (case-insensitive).
    KwPrefix,
    /// `BASE` keyword (case-insensitive).
    KwBase,
    /// `CLOSED` keyword.
    KwClosed,
    /// `EXTENDS` keyword.
    KwExtends,
    /// `AND` keyword.
    KwAnd,
    /// `OR` keyword.
    KwOr,
    /// `NOT` keyword.
    KwNot,
    /// `IRI` node kind keyword.
    KwIri,
    /// `LITERAL` node kind keyword.
    KwLiteral,
    /// `NONLITERAL` node kind keyword.
    KwNonLiteral,
    /// `BNODE` node kind keyword.
    KwBNode,
    /// `INVERSE` keyword.
    KwInverse,
    /// `ANY` or `.` as wildcard.
    KwAny,
    /// `a` keyword (rdf:type).
    KwA,
    /// `EXTRA` keyword.
    KwExtra,
    /// `START` keyword.
    KwStart,
    /// `ABSTRACT` keyword.
    KwAbstract,
    /// String literal body (ECHAR/UCHAR decoded).
    StringLit(String),
    /// `{` (left brace — shape body open OR cardinality open).
    LBrace,
    /// `}` (right brace).
    RBrace,
    /// `(` (left paren).
    LParen,
    /// `)` (right paren).
    RParen,
    /// `[` (left bracket — value set).
    LBracket,
    /// `]` (right bracket).
    RBracket,
    /// `;` (semicolon — triple constraint separator).
    Semi,
    /// `,` (comma — one-of separator).
    Comma,
    /// `|` (pipe — one-of in some positions).
    Pipe,
    /// `@` prefix for shape references and language tags.
    At,
    /// `$` (variable reference prefix, e.g. `$<iri>`).
    Dollar,
    /// `&` (reference in AND-type group expressions).
    Amp,
    /// `*` cardinality.
    Star,
    /// `+` cardinality.
    Plus,
    /// `?` cardinality.
    Question,
    /// `^` (inverse predicate marker, alternative to INVERSE keyword).
    Caret,
    /// `=` (shape label assignment).
    Eq,
    /// `^^` datatype marker (for literals in value sets).
    DataTypeMark,
    /// Unsigned integer (used in cardinality `{n}` or `{n,m}`).
    Integer(u32),
    /// Language tag (after `@`, for value set literals).
    LangTag(String),
}

/// One token together with its source byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Spanned {
    /// Token payload.
    pub(crate) tok: Tok,
    /// Start byte offset (0-indexed, inclusive).
    pub(crate) start: usize,
    /// End byte offset (0-indexed, exclusive).
    pub(crate) end: usize,
}

/// A parse error produced by the lexer.
#[derive(Debug, Clone)]
pub(crate) struct LexError {
    /// Human-readable message.
    pub(crate) message: String,
    /// Byte offset.
    pub(crate) offset: usize,
}

impl LexError {
    fn new(offset: usize, msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
            offset,
        }
    }
}

/// Hand-rolled ShExC lexer.
pub(crate) struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Build a fresh lexer over `src`.
    pub(crate) const fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    /// Current byte position.
    pub(crate) const fn offset(&self) -> usize {
        self.pos
    }

    /// Save/restore position (used for lookahead).
    pub(crate) const fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Skip whitespace and `#…\n` comments.
    fn skip_trivia(&mut self) {
        loop {
            // Skip whitespace.
            while self.pos < self.src.len()
                && matches!(self.src[self.pos], b' ' | b'\t' | b'\r' | b'\n')
            {
                self.pos += 1;
            }
            // Skip line comment.
            if self.pos < self.src.len() && self.src[self.pos] == b'#' {
                while self.pos < self.src.len() && self.src[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Produce the next token, or `None` at EOF.
    pub(crate) fn next(&mut self) -> Result<Option<Spanned>, LexError> {
        self.skip_trivia();
        if self.pos >= self.src.len() {
            return Ok(None);
        }
        let start = self.pos;
        let b = self.src[self.pos];

        // Single-byte tokens.
        macro_rules! one {
            ($tok:expr) => {{
                self.pos += 1;
                return Ok(Some(Spanned {
                    tok: $tok,
                    start,
                    end: self.pos,
                }));
            }};
        }

        match b {
            b'(' => one!(Tok::LParen),
            b')' => one!(Tok::RParen),
            b'[' => one!(Tok::LBracket),
            b']' => one!(Tok::RBracket),
            b';' => one!(Tok::Semi),
            b',' => one!(Tok::Comma),
            b'|' => one!(Tok::Pipe),
            b'$' => one!(Tok::Dollar),
            b'&' => one!(Tok::Amp),
            b'*' => one!(Tok::Star),
            b'+' => one!(Tok::Plus),
            b'?' => one!(Tok::Question),
            b'=' => one!(Tok::Eq),
            // `.` — wildcard (KwAny) in ShExC.
            b'.' => one!(Tok::KwAny),
            _ => {}
        }

        // `{` — could be LBrace or start of cardinality `{n,m}`, both emit LBrace here.
        // Cardinality parsing is handled in the grammar after seeing LBrace.
        if b == b'{' {
            one!(Tok::LBrace);
        }
        if b == b'}' {
            one!(Tok::RBrace);
        }

        // `^^` or `^` (inverse predicate).
        if b == b'^' {
            if self.src.get(self.pos + 1).copied() == Some(b'^') {
                self.pos += 2;
                return Ok(Some(Spanned {
                    tok: Tok::DataTypeMark,
                    start,
                    end: self.pos,
                }));
            }
            one!(Tok::Caret);
        }

        // IRIREF `<…>`.
        if b == b'<' {
            let (iri, end) = self.lex_iriref(start)?;
            return Ok(Some(Spanned {
                tok: Tok::IriRef(iri),
                start,
                end,
            }));
        }

        // String literal `"…"` or `'…'`.
        if b == b'"' || b == b'\'' {
            let (s, end) = self.lex_string(start, b)?;
            return Ok(Some(Spanned {
                tok: Tok::StringLit(s),
                start,
                end,
            }));
        }

        // `@` — shape references, language tags, `@prefix`, `@base` (Turtle style).
        if b == b'@' {
            return self.lex_at(start);
        }

        // Unsigned integers (used inside cardinality braces).
        if b.is_ascii_digit() {
            return Ok(Some(self.lex_integer(start)));
        }

        // Blank-node label `_:…`.
        if b == b'_' && self.src.get(self.pos + 1).copied() == Some(b':') {
            let (label, end) = self.lex_bnode(start)?;
            return Ok(Some(Spanned {
                tok: Tok::BNodeLabel(label),
                start,
                end,
            }));
        }

        // Prefixed name or keyword.
        if is_pn_chars_base(b) || b == b':' {
            return self.lex_pname_or_keyword(start);
        }

        Err(LexError::new(
            start,
            format!("unexpected byte 0x{b:02X} in ShEx input"),
        ))
    }

    // -----------------------------------------------------------------------
    // Sub-lexers
    // -----------------------------------------------------------------------

    fn lex_iriref(&mut self, start: usize) -> Result<(String, usize), LexError> {
        debug_assert_eq!(self.src[self.pos], b'<');
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == b'>' {
                self.pos += 1;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let ch = self.lex_uchar(self.pos)?;
                out.push(ch);
                continue;
            }
            if matches!(b, 0x00..=0x20) {
                return Err(LexError::new(
                    self.pos,
                    format!("control byte 0x{b:02X} inside <IRI>"),
                ));
            }
            let ch = self
                .next_utf8_char()
                .ok_or_else(|| LexError::new(self.pos, "invalid UTF-8 inside <IRI>"))?;
            out.push(ch);
        }
        Err(LexError::new(start, "unterminated <IRI>"))
    }

    fn lex_string(&mut self, start: usize, quote: u8) -> Result<(String, usize), LexError> {
        // Detect triple-quoted strings.
        let triple = self.src.get(self.pos + 1).copied() == Some(quote)
            && self.src.get(self.pos + 2).copied() == Some(quote);
        if triple {
            self.pos += 3;
            return self.lex_long_string(start, quote);
        }
        self.pos += 1; // skip opening quote
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == quote {
                self.pos += 1;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let ch = self.lex_echar_or_uchar(self.pos)?;
                out.push(ch);
                continue;
            }
            if matches!(b, b'\n' | b'\r') {
                return Err(LexError::new(
                    self.pos,
                    "unescaped newline in short string literal",
                ));
            }
            let ch = self
                .next_utf8_char()
                .ok_or_else(|| LexError::new(self.pos, "invalid UTF-8 in string"))?;
            out.push(ch);
        }
        Err(LexError::new(start, "unterminated string literal"))
    }

    fn lex_long_string(&mut self, start: usize, quote: u8) -> Result<(String, usize), LexError> {
        let mut out = String::new();
        while self.pos < self.src.len() {
            let b = self.src[self.pos];
            if b == quote
                && self.src.get(self.pos + 1).copied() == Some(quote)
                && self.src.get(self.pos + 2).copied() == Some(quote)
            {
                self.pos += 3;
                return Ok((out, self.pos));
            }
            if b == b'\\' {
                let ch = self.lex_echar_or_uchar(self.pos)?;
                out.push(ch);
                continue;
            }
            let ch = self
                .next_utf8_char()
                .ok_or_else(|| LexError::new(self.pos, "invalid UTF-8 in long string"))?;
            out.push(ch);
        }
        Err(LexError::new(start, "unterminated triple-quoted string"))
    }

    fn lex_echar_or_uchar(&mut self, at: usize) -> Result<char, LexError> {
        debug_assert_eq!(self.src[self.pos], b'\\');
        if self.pos + 1 >= self.src.len() {
            return Err(LexError::new(at, "escape at end of input"));
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
            other => Err(LexError::new(
                at,
                format!("unknown escape '\\{}'", other as char),
            )),
        }
    }

    fn lex_uchar(&mut self, at: usize) -> Result<char, LexError> {
        debug_assert_eq!(self.src[self.pos], b'\\');
        let marker = self.src.get(self.pos + 1).copied();
        let (width, step) = match marker {
            Some(b'u') => (4usize, 2usize),
            Some(b'U') => (8usize, 2usize),
            _ => return Err(LexError::new(at, "expected \\uXXXX or \\UXXXXXXXX")),
        };
        let hex_start = self.pos + step;
        let hex_end = hex_start + width;
        if hex_end > self.src.len() {
            return Err(LexError::new(at, "truncated UCHAR escape"));
        }
        let mut cp: u32 = 0;
        for &b in &self.src[hex_start..hex_end] {
            let d = match b {
                b'0'..=b'9' => u32::from(b - b'0'),
                b'a'..=b'f' => u32::from(b - b'a' + 10),
                b'A'..=b'F' => u32::from(b - b'A' + 10),
                _ => return Err(LexError::new(at, "non-hex digit in UCHAR")),
            };
            cp = (cp << 4) | d;
        }
        self.pos = hex_end;
        if (0xD800..=0xDFFF).contains(&cp) {
            return Err(LexError::new(at, format!("surrogate U+{cp:04X} in UCHAR")));
        }
        char::from_u32(cp).ok_or_else(|| LexError::new(at, format!("invalid code point U+{cp:04X}")))
    }

    fn lex_at(&mut self, start: usize) -> Result<Option<Spanned>, LexError> {
        debug_assert_eq!(self.src[self.pos], b'@');
        self.pos += 1;
        // Check for `@prefix` and `@base` (Turtle-style).
        let rest = &self.src[self.pos..];
        if rest.starts_with(b"prefix") && !rest.get(6).is_some_and(|&c| is_pn_chars(c)) {
            self.pos += 6;
            // Reuse KwPrefix token — grammar distinguishes by context.
            return Ok(Some(Spanned {
                tok: Tok::KwPrefix,
                start,
                end: self.pos,
            }));
        }
        if rest.starts_with(b"base") && !rest.get(4).is_some_and(|&c| is_pn_chars(c)) {
            self.pos += 4;
            return Ok(Some(Spanned {
                tok: Tok::KwBase,
                start,
                end: self.pos,
            }));
        }
        // Language tag or shape reference marker.
        // A language tag is `@alpha(-alphanum)*` with NO trailing `:`.
        // A shape reference is `@<iri>` or `@prefix:local` — we emit `At`
        // and let the grammar consume the following token.
        //
        // Disambiguation: scan the alphabetic part and peek whether `:` follows.
        // If yes → pname → emit `At`. If no → language tag.
        if self.pos < self.src.len() && self.src[self.pos].is_ascii_alphabetic() {
            let tag_start = self.pos;
            // Scan the first alphabetic segment.
            while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphabetic() {
                self.pos += 1;
            }
            // If the next byte is `:` this is a pname prefix — reset and emit bare `@`.
            if self.pos < self.src.len() && self.src[self.pos] == b':' {
                self.pos = tag_start; // reset — grammar will re-lex pname
                return Ok(Some(Spanned {
                    tok: Tok::At,
                    start,
                    end: self.pos,
                }));
            }
            // Otherwise try to consume subtags separated by `-`.
            while self.pos < self.src.len() && self.src[self.pos] == b'-' {
                self.pos += 1;
                let sub_start = self.pos;
                while self.pos < self.src.len() && self.src[self.pos].is_ascii_alphanumeric() {
                    self.pos += 1;
                }
                if self.pos == sub_start {
                    return Err(LexError::new(self.pos, "empty subtag in language tag"));
                }
            }
            let tag = std::str::from_utf8(&self.src[tag_start..self.pos])
                .map_err(|_| LexError::new(tag_start, "non-UTF-8 language tag"))?
                .to_owned();
            return Ok(Some(Spanned {
                tok: Tok::LangTag(tag),
                start,
                end: self.pos,
            }));
        }
        // Bare `@` — shape reference prefix for `@<iri>`.
        Ok(Some(Spanned {
            tok: Tok::At,
            start,
            end: self.pos,
        }))
    }

    fn lex_integer(&mut self, start: usize) -> Spanned {
        let mut val: u32 = 0;
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_digit() {
            let d = u32::from(self.src[self.pos] - b'0');
            val = val.saturating_mul(10).saturating_add(d);
            self.pos += 1;
        }
        Spanned {
            tok: Tok::Integer(val),
            start,
            end: self.pos,
        }
    }

    fn lex_bnode(&mut self, start: usize) -> Result<(String, usize), LexError> {
        debug_assert_eq!(self.src[self.pos], b'_');
        self.pos += 2; // skip "_:"
        if self.pos >= self.src.len() {
            return Err(LexError::new(start, "empty blank-node label"));
        }
        let label_start = self.pos;
        let first = self.src[self.pos];
        if !(is_pn_chars_u(first) || first.is_ascii_digit()) {
            return Err(LexError::new(self.pos, "invalid blank-node label start"));
        }
        self.pos += 1;
        while self.pos < self.src.len() && (is_pn_chars(self.src[self.pos]) || self.src[self.pos] == b'.') {
            self.pos += 1;
        }
        // Trim trailing dots.
        while self.pos > label_start + 1 && self.src[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        let label = std::str::from_utf8(&self.src[label_start..self.pos])
            .map_err(|_| LexError::new(label_start, "non-UTF-8 blank-node label"))?
            .to_owned();
        Ok((label, self.pos))
    }

    fn lex_pname_or_keyword(&mut self, start: usize) -> Result<Option<Spanned>, LexError> {
        // PN_PREFIX part.
        let prefix_start = self.pos;
        if self.pos < self.src.len() && self.src[self.pos] != b':' {
            if !is_pn_chars_base(self.src[self.pos]) {
                return Err(LexError::new(start, "expected identifier start"));
            }
            self.pos += 1;
            while self.pos < self.src.len()
                && (is_pn_chars(self.src[self.pos]) || self.src[self.pos] == b'.')
            {
                self.pos += 1;
            }
            // No trailing dot.
            while self.pos > prefix_start + 1 && self.src[self.pos - 1] == b'.' {
                self.pos -= 1;
            }
        }

        let word = std::str::from_utf8(&self.src[prefix_start..self.pos])
            .map_err(|_| LexError::new(prefix_start, "non-UTF-8 identifier"))?;

        // Check for colon (pname) vs bare keyword.
        if self.pos < self.src.len() && self.src[self.pos] == b':' {
            // Pname.
            let prefix = word.to_owned();
            self.pos += 1; // consume ':'
            let mut local = String::new();
            let mut first_char = true;
            while self.pos < self.src.len() {
                let b = self.src[self.pos];
                let ok_start = is_pn_chars_u(b) || b == b':' || b.is_ascii_digit();
                let ok_cont = is_pn_chars(b) || b == b':' || b == b'.';
                if (first_char && ok_start) || (!first_char && ok_cont) {
                    let ch = self
                        .next_utf8_char()
                        .ok_or_else(|| LexError::new(self.pos, "invalid UTF-8 in local part"))?;
                    local.push(ch);
                    first_char = false;
                } else {
                    break;
                }
            }
            // No trailing dot.
            while local.ends_with('.') {
                local.pop();
                self.pos -= 1;
            }
            let end = self.pos;
            return Ok(Some(Spanned {
                tok: Tok::Pname { prefix, local },
                start,
                end,
            }));
        }

        // Bare word — match keywords (case-sensitive for ShEx, except PREFIX/BASE).
        let tok = match word {
            "a" => Tok::KwA,
            "AND" => Tok::KwAnd,
            "OR" => Tok::KwOr,
            "NOT" => Tok::KwNot,
            "IRI" => Tok::KwIri,
            "LITERAL" => Tok::KwLiteral,
            "NONLITERAL" => Tok::KwNonLiteral,
            "BNODE" => Tok::KwBNode,
            "CLOSED" => Tok::KwClosed,
            "EXTENDS" => Tok::KwExtends,
            "INVERSE" => Tok::KwInverse,
            "EXTRA" => Tok::KwExtra,
            "START" => Tok::KwStart,
            "ABSTRACT" => Tok::KwAbstract,
            // ANY is a keyword; bare `.` is also lexed as KwAny but by the
            // grammar dot-handler separately.
            "ANY" => Tok::KwAny,
            w if w.eq_ignore_ascii_case("prefix") => Tok::KwPrefix,
            w if w.eq_ignore_ascii_case("base") => Tok::KwBase,
            _ => {
                return Err(LexError::new(
                    start,
                    format!("unrecognised keyword or bare identifier '{word}'; expected 'prefix:local' or a keyword"),
                ));
            }
        };
        Ok(Some(Spanned {
            tok,
            start,
            end: self.pos,
        }))
    }

    /// Advance past one UTF-8 scalar from the current position.
    fn next_utf8_char(&mut self) -> Option<char> {
        let remaining = self.src.get(self.pos..)?;
        let s = std::str::from_utf8(remaining).ok()?;
        let ch = s.chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
}

// -----------------------------------------------------------------------
// Character class helpers (SPARQL/ShEx grammar).
// -----------------------------------------------------------------------

fn is_pn_chars_base(b: u8) -> bool {
    b.is_ascii_alphabetic() || b >= 0x80
}

fn is_pn_chars_u(b: u8) -> bool {
    is_pn_chars_base(b) || b == b'_'
}

fn is_pn_chars(b: u8) -> bool {
    is_pn_chars_u(b) || b.is_ascii_digit() || b == b'-' || b == 0xB7
}
