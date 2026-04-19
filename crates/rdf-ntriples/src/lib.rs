//! Main Phase-A N-Triples and N-Quads parser.
//!
//! This crate provides the `NTriplesParser` and `NQuadsParser` types.
//! Each implements the frozen [`rdf_diff::Parser`] contract from the
//! verification-v1 sweep (ADR-0020 §1.4) so the same parser drives both
//! the in-tree consumers and the differential diff harness.
//!
//! Grammar target:
//!
//! - W3C RDF 1.1 N-Triples — <https://www.w3.org/TR/n-triples/>
//! - W3C RDF 1.1 N-Quads — <https://www.w3.org/TR/n-quads/>
//!
//! Spec-reading pins honoured (see `docs/spec-readings/`):
//!
//! - `NT-LITESC-001` — `\uXXXX` / `\UXXXXXXXX` decoded at parse time,
//!   hex is case-insensitive, surrogate code points rejected.
//! - `ANY-BOM-001` — leading UTF-8 BOM (`EF BB BF`) at byte offset 0 is
//!   silently skipped; a BOM anywhere else is a fatal error.
//!
//! Independence note: written from the W3C recommendations and the
//! spec-reading pins alone; the `rdf-ntriples-shadow` source is
//! deliberately not consulted (ADR-0019 §3 — disjoint implementations).

#![forbid(unsafe_code)]
#![warn(missing_docs)]
// The parser is fundamentally a big state machine; a handful of pedantic
// lints would make it worse rather than better.
#![allow(
    clippy::too_many_lines,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::multiple_crate_versions
)]

use std::collections::BTreeMap;

use rdf_diff::{
    Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser as ParserTrait,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Main N-Triples (RDF 1.1) parser.
///
/// Accepts only the triple form (three terms + `.`). Rejects the fourth
/// "graph name" term; use [`NQuadsParser`] for that.
#[derive(Debug, Default, Clone, Copy)]
pub struct NTriplesParser;

/// Main N-Quads (RDF 1.1) parser.
///
/// Accepts the quad form (three terms + optional graph-name term + `.`).
/// Triples without a graph name default to `graph: None`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NQuadsParser;

impl ParserTrait for NTriplesParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        run_parse(input, Mode::NTriples, self.id())
    }

    fn id(&self) -> &'static str {
        "rdf-ntriples"
    }
}

impl ParserTrait for NQuadsParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        run_parse(input, Mode::NQuads, self.id())
    }

    fn id(&self) -> &'static str {
        "rdf-nquads"
    }
}

// ---------------------------------------------------------------------------
// Parse driver
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    NTriples,
    NQuads,
}

fn run_parse(
    input: &[u8],
    mode: Mode,
    parser_id: &'static str,
) -> Result<ParseOutcome, Diagnostics> {
    // Stage 0: UTF-8 validation. Both NT and NQ fix the encoding to UTF-8.
    let text = match std::str::from_utf8(input) {
        Ok(s) => s,
        Err(e) => {
            return Err(fatal(format!(
                "NT-UTF8-001: input is not valid UTF-8 at byte {}",
                e.valid_up_to()
            )));
        }
    };

    // Stage 1: BOM handling (ANY-BOM-001). Leading BOM is skipped and the
    // byte offset is tracked so FactProvenance lines up with the original
    // input (per the pin's byte-offset accounting clause).
    let (body, base_offset, bom_warning) = strip_leading_bom(text);

    // Stage 2: statement-by-statement parse. The parser is a simple
    // hand-written state machine over `char`s with byte-offset bookkeeping.
    let mut p = ParseState::new(body, base_offset, mode, parser_id);
    let mut facts: Vec<(Fact, FactProvenance)> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    warnings.extend(bom_warning);

    loop {
        p.skip_ws_and_comments()?;
        if p.eof() {
            break;
        }
        match p.parse_statement() {
            Ok(Some((fact, prov))) => facts.push((fact, prov)),
            Ok(None) => {} // empty line tolerated
            Err(diag) => return Err(diag),
        }
    }

    let facts = Facts::canonicalise(facts, BTreeMap::new());
    Ok(ParseOutcome {
        facts,
        warnings: Diagnostics {
            messages: warnings,
            fatal: false,
        },
    })
}

fn fatal(message: String) -> Diagnostics {
    Diagnostics {
        messages: vec![message],
        fatal: true,
    }
}

/// Strip a leading UTF-8 BOM (`U+FEFF`, bytes `EF BB BF`) and report the
/// number of bytes skipped. A BOM elsewhere is handled as a normal
/// character by the grammar (which will reject it); we only peel the
/// byte-0 BOM off here.
fn strip_leading_bom(text: &str) -> (&str, usize, Option<String>) {
    text.strip_prefix('\u{FEFF}').map_or((text, 0, None), |rest| {
        (
            rest,
            '\u{FEFF}'.len_utf8(),
            Some("ANY-BOM-001: leading UTF-8 BOM skipped at byte offset 0".to_owned()),
        )
    })
}

// ---------------------------------------------------------------------------
// Hand-written parser state machine
// ---------------------------------------------------------------------------

struct ParseState<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,       // byte offset into `src`
    base_offset: usize, // bytes skipped before `src` (e.g. BOM)
    mode: Mode,
    parser_id: &'static str,
}

impl<'a> ParseState<'a> {
    const fn new(
        src: &'a str,
        base_offset: usize,
        mode: Mode,
        parser_id: &'static str,
    ) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            base_offset,
            mode,
            parser_id,
        }
    }

    const fn eof(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    /// Current absolute byte offset (matches `FactProvenance::offset`'s
    /// "offset into the original input" contract).
    const fn abs_offset(&self) -> usize {
        self.pos + self.base_offset
    }

    /// Peek the next byte without advancing.
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    /// Bump one ASCII byte. Multi-byte UTF-8 must go through `consume_char`.
    const fn bump_byte(&mut self) {
        self.pos += 1;
    }

    /// Consume one code point (handles multi-byte UTF-8). Returns `None` at
    /// EOF. The fast-path (ASCII) does not touch the char decoder.
    fn consume_char(&mut self) -> Option<char> {
        let remaining = &self.src[self.pos..];
        let mut chars = remaining.chars();
        let c = chars.next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    // -----------------------------------------------------------------
    // Whitespace / comments / EOL
    // -----------------------------------------------------------------

    /// Skip inter-token whitespace and comments. The N-Triples grammar
    /// permits SP + HT inside a statement and the `EOL` production
    /// (`[#xD#xA]+`) between statements. Comments start with `#` and
    /// run to EOL.
    ///
    /// BOM mid-stream is a fatal error (ANY-BOM-001 clause 2).
    fn skip_ws_and_comments(&mut self) -> Result<(), Diagnostics> {
        loop {
            let Some(b) = self.peek() else { return Ok(()); };
            match b {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.bump_byte();
                }
                b'#' => {
                    // Consume `#` then everything up to the next CR or LF.
                    self.bump_byte();
                    while let Some(c) = self.peek() {
                        if c == b'\n' || c == b'\r' {
                            break;
                        }
                        self.bump_byte();
                    }
                }
                // Non-ASCII: could be a stray U+FEFF (BOM). Detect and
                // surface the dedicated diagnostic.
                0xEF => {
                    if self.bytes.get(self.pos..self.pos + 3) == Some(b"\xEF\xBB\xBF") {
                        return Err(fatal(format!(
                            "ANY-BOM-001: stray U+FEFF at byte offset {}",
                            self.abs_offset()
                        )));
                    }
                    // Otherwise it's some other character; fall through
                    // and let the statement parser reject it.
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }
    }

    // -----------------------------------------------------------------
    // Statement
    // -----------------------------------------------------------------

    fn parse_statement(
        &mut self,
    ) -> Result<Option<(Fact, FactProvenance)>, Diagnostics> {
        let start_offset = self.abs_offset();

        // Subject: IRI or blank node.
        let subject = self.parse_subject()?;
        self.skip_inline_ws();

        // Predicate: IRI only.
        let predicate = self.parse_iri_term()?;
        self.skip_inline_ws();

        // Object: IRI, blank node, or literal.
        let object = self.parse_object()?;
        self.skip_inline_ws();

        // Graph (N-Quads only, optional).
        let graph = if self.mode == Mode::NQuads {
            match self.peek() {
                Some(b'<' | b'_') => {
                    let g = self.parse_graph_term()?;
                    self.skip_inline_ws();
                    Some(g)
                }
                _ => None,
            }
        } else {
            None
        };

        // Statement terminator.
        match self.peek() {
            Some(b'.') => self.bump_byte(),
            Some(c) => {
                return Err(fatal(format!(
                    "NT-STMT-001: expected '.' at byte offset {}, found {:?}",
                    self.abs_offset(),
                    c as char
                )));
            }
            None => {
                return Err(fatal(format!(
                    "NT-STMT-001: expected '.' at EOF (byte offset {})",
                    self.abs_offset()
                )));
            }
        }

        let fact = Fact {
            subject,
            predicate,
            object,
            graph,
        };
        let prov = FactProvenance {
            offset: Some(start_offset),
            parser: self.parser_id.to_owned(),
        };
        Ok(Some((fact, prov)))
    }

    /// Skip spaces and tabs but not EOL — EOL is the statement separator
    /// handled by `skip_ws_and_comments` after `.`.
    fn skip_inline_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' {
                self.bump_byte();
            } else {
                break;
            }
        }
    }

    // -----------------------------------------------------------------
    // Terms
    // -----------------------------------------------------------------

    fn parse_subject(&mut self) -> Result<String, Diagnostics> {
        match self.peek() {
            Some(b'<') => self.parse_iri_term(),
            Some(b'_') => self.parse_bnode_term(),
            Some(c) => Err(fatal(format!(
                "NT-SUBJ-001: expected IRI or blank node at byte offset {}, found {:?}",
                self.abs_offset(),
                c as char
            ))),
            None => Err(fatal(format!(
                "NT-SUBJ-001: unexpected EOF at byte offset {}",
                self.abs_offset()
            ))),
        }
    }

    fn parse_object(&mut self) -> Result<String, Diagnostics> {
        match self.peek() {
            Some(b'<') => self.parse_iri_term(),
            Some(b'_') => self.parse_bnode_term(),
            Some(b'"') => self.parse_literal_term(),
            Some(c) => Err(fatal(format!(
                "NT-OBJ-001: expected IRI, blank node, or literal at byte offset {}, found {:?}",
                self.abs_offset(),
                c as char
            ))),
            None => Err(fatal(format!(
                "NT-OBJ-001: unexpected EOF at byte offset {}",
                self.abs_offset()
            ))),
        }
    }

    fn parse_graph_term(&mut self) -> Result<String, Diagnostics> {
        match self.peek() {
            Some(b'<') => self.parse_iri_term(),
            Some(b'_') => self.parse_bnode_term(),
            _ => unreachable!("caller checks lead byte"),
        }
    }

    /// Parse an IRI term into canonical `<...>` form. Enforces the
    /// absolute-IRI requirement from NT §2 (relative IRIs are a Turtle
    /// feature).
    fn parse_iri_term(&mut self) -> Result<String, Diagnostics> {
        let open_offset = self.abs_offset();
        match self.peek() {
            Some(b'<') => self.bump_byte(),
            _ => {
                return Err(fatal(format!(
                    "NT-IRI-001: expected '<' at byte offset {}",
                    self.abs_offset()
                )));
            }
        }

        // Collect body code points, decoding UCHAR escapes as we go.
        let mut body = String::new();
        loop {
            let Some(c) = self.peek_char() else {
                return Err(fatal(format!(
                    "NT-IRI-002: unterminated IRI starting at byte offset {open_offset}"
                )));
            };
            if c == '>' {
                self.consume_char();
                break;
            }
            if c == '\\' {
                // Only UCHAR escapes are permitted inside IRIREF.
                self.consume_char();
                let decoded = self.read_uchar_escape()?;
                body.push(decoded);
                continue;
            }
            // Forbid chars disallowed inside IRIREF per the grammar:
            //   IRIREF excludes: U+00..=U+20, <, >, ", {, }, |, ^, `, \
            // (`\` is allowed only as the start of a UCHAR above).
            if (c as u32) <= 0x20
                || matches!(c, '<' | '"' | '{' | '}' | '|' | '^' | '`')
            {
                return Err(fatal(format!(
                    "NT-IRI-003: illegal character {:?} in IRI at byte offset {}",
                    c,
                    self.abs_offset()
                )));
            }
            self.consume_char();
            body.push(c);
        }

        validate_absolute_iri(&body, open_offset)?;
        Ok(format!("<{body}>"))
    }

    fn parse_bnode_term(&mut self) -> Result<String, Diagnostics> {
        let start = self.abs_offset();
        // Consume the `_:`.
        if self.bytes.get(self.pos..self.pos + 2) != Some(b"_:") {
            return Err(fatal(format!(
                "NT-BN-001: expected '_:' at byte offset {start}"
            )));
        }
        self.pos += 2;

        // Lex the label. BLANK_NODE_LABEL per NT §2:
        //   BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9])
        //                         ((PN_CHARS | '.')* PN_CHARS)?
        //
        // Keys:
        //   - First char: PN_CHARS_U | [0-9]
        //   - Middle chars: PN_CHARS | '.'
        //   - Last char MUST NOT be '.'  (enforced post-lex).
        //
        // The simplest safe lexer: accept any PN_CHARS | '.' greedily,
        // then peel back a trailing '.' if present.
        let label_start = self.pos;
        match self.peek_char() {
            Some(c) if is_pn_chars_u(c) || c.is_ascii_digit() => {
                self.consume_char();
            }
            _ => {
                return Err(fatal(format!(
                    "NT-BN-002: illegal first character in blank-node label at byte offset {}",
                    self.abs_offset()
                )));
            }
        }
        while let Some(c) = self.peek_char() {
            if is_pn_chars(c) || c == '.' {
                self.consume_char();
            } else {
                break;
            }
        }
        // Strip any trailing '.': grammar forbids a label ending in '.'.
        // Importantly, we back off **into** the input so the `.` becomes
        // the statement terminator (FM4-b fixture relies on this).
        while self.pos > label_start && self.bytes[self.pos - 1] == b'.' {
            self.pos -= 1;
        }
        if self.pos == label_start {
            return Err(fatal(format!(
                "NT-BN-002: empty blank-node label at byte offset {start}"
            )));
        }

        let label = &self.src[label_start..self.pos];
        Ok(format!("_:{label}"))
    }

    fn parse_literal_term(&mut self) -> Result<String, Diagnostics> {
        let start = self.abs_offset();
        // Consume opening '"'.
        self.bump_byte();

        // Read the lexical form, decoding ECHAR and UCHAR, forbidding
        // raw CR/LF/quote/backslash.
        let mut lex = String::new();
        loop {
            let Some(c) = self.peek_char() else {
                return Err(fatal(format!(
                    "NT-LIT-001: unterminated literal starting at byte offset {start}"
                )));
            };
            match c {
                '"' => {
                    self.consume_char();
                    break;
                }
                '\\' => {
                    self.consume_char();
                    let decoded = self.read_string_escape()?;
                    lex.push_str(&decoded);
                }
                '\r' | '\n' => {
                    return Err(fatal(format!(
                        "NT-LIT-002: unescaped control character 0x{:02X} in literal at byte offset {}",
                        c as u32,
                        self.abs_offset()
                    )));
                }
                _ => {
                    self.consume_char();
                    lex.push(c);
                }
            }
        }

        // Optional language tag or datatype.
        let suffix = match self.peek() {
            Some(b'@') => {
                self.bump_byte();
                self.parse_lang_tag()?
            }
            Some(b'^') => {
                if self.bytes.get(self.pos..self.pos + 2) != Some(b"^^") {
                    return Err(fatal(format!(
                        "NT-LIT-003: expected '^^' at byte offset {}",
                        self.abs_offset()
                    )));
                }
                self.pos += 2;
                let iri = self.parse_iri_term()?;
                format!("^^{iri}")
            }
            _ => String::new(),
        };

        // Re-emit the literal in canonical form. Only `\` and `"` need to
        // be re-escaped so that `rdf_diff::split_literal` can recover the
        // lexical form.
        let escaped = escape_literal_lex(&lex);
        Ok(format!("\"{escaped}\"{suffix}"))
    }

    fn parse_lang_tag(&mut self) -> Result<String, Diagnostics> {
        // LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*
        let start = self.pos;
        // First subtag: [a-zA-Z]+
        let head = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphabetic() {
                self.bump_byte();
            } else {
                break;
            }
        }
        if self.pos == head {
            return Err(fatal(format!(
                "NT-LIT-004: empty language tag at byte offset {}",
                self.abs_offset()
            )));
        }
        // ('-' [a-zA-Z0-9]+)*
        while self.peek() == Some(b'-') {
            self.bump_byte();
            let sub = self.pos;
            while let Some(b) = self.peek() {
                if b.is_ascii_alphanumeric() {
                    self.bump_byte();
                } else {
                    break;
                }
            }
            if self.pos == sub {
                return Err(fatal(format!(
                    "NT-LIT-004: empty language subtag at byte offset {}",
                    self.abs_offset()
                )));
            }
        }
        Ok(format!("@{}", &self.src[start..self.pos]))
    }

    // -----------------------------------------------------------------
    // Escape decoding
    // -----------------------------------------------------------------

    /// Consume the body of a `\uXXXX` or `\UXXXXXXXX` escape **after**
    /// the leading backslash has already been consumed. This is the
    /// NT-LITESC-001 pin's decoding path.
    fn read_uchar_escape(&mut self) -> Result<char, Diagnostics> {
        let Some(marker) = self.peek() else {
            return Err(fatal(format!(
                "NT-LITESC-001: truncated UCHAR escape at byte offset {}",
                self.abs_offset()
            )));
        };
        let width = match marker {
            b'u' => 4,
            b'U' => 8,
            other => {
                return Err(fatal(format!(
                    "NT-LITESC-001: invalid UCHAR escape at byte offset {}: \\{:?}",
                    self.abs_offset(),
                    other as char
                )));
            }
        };
        self.bump_byte();
        let hex_start = self.pos;
        if self.bytes.len() < hex_start + width {
            return Err(fatal(format!(
                "NT-LITESC-001: truncated UCHAR escape at byte offset {}",
                self.abs_offset()
            )));
        }
        let hex = &self.src[hex_start..hex_start + width];
        for c in hex.chars() {
            if !c.is_ascii_hexdigit() {
                return Err(fatal(format!(
                    "NT-LITESC-001: non-hex digit {:?} in UCHAR escape at byte offset {}",
                    c,
                    self.abs_offset()
                )));
            }
        }
        self.pos += width;
        let code = u32::from_str_radix(hex, 16).map_err(|_| {
            fatal(format!(
                "NT-LITESC-001: invalid UCHAR escape at byte offset {}",
                self.abs_offset()
            ))
        })?;
        // Surrogates are not scalar values — the pin marks this fatal.
        if (0xD800..=0xDFFF).contains(&code) {
            return Err(fatal(format!(
                "NT-LITESC-001: surrogate U+{code:04X} in UCHAR escape at byte offset {}",
                self.abs_offset()
            )));
        }
        char::from_u32(code).ok_or_else(|| {
            fatal(format!(
                "NT-LITESC-001: U+{code:X} is not a valid Unicode scalar value at byte offset {}",
                self.abs_offset()
            ))
        })
    }

    /// ECHAR / UCHAR inside string literal. Backslash already consumed.
    fn read_string_escape(&mut self) -> Result<String, Diagnostics> {
        let Some(marker) = self.peek() else {
            return Err(fatal(format!(
                "NT-LIT-005: truncated string escape at byte offset {}",
                self.abs_offset()
            )));
        };
        let decoded = match marker {
            b't' => {
                self.bump_byte();
                '\t'.to_string()
            }
            b'b' => {
                self.bump_byte();
                '\u{0008}'.to_string()
            }
            b'n' => {
                self.bump_byte();
                '\n'.to_string()
            }
            b'r' => {
                self.bump_byte();
                '\r'.to_string()
            }
            b'f' => {
                self.bump_byte();
                '\u{000C}'.to_string()
            }
            b'"' => {
                self.bump_byte();
                '"'.to_string()
            }
            b'\'' => {
                self.bump_byte();
                '\''.to_string()
            }
            b'\\' => {
                self.bump_byte();
                '\\'.to_string()
            }
            b'u' | b'U' => self.read_uchar_escape()?.to_string(),
            other => {
                return Err(fatal(format!(
                    "NT-LIT-005: unknown escape \\{:?} at byte offset {}",
                    other as char,
                    self.abs_offset()
                )));
            }
        };
        Ok(decoded)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Escape a decoded lexical form into NT literal syntax. Only `\` and `"`
/// need to be re-escaped — the canonical form stored in `Fact::object`
/// keeps every other byte verbatim (no trimming, no NFC normalisation).
fn escape_literal_lex(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

/// Reject relative IRIs; only absolute IRIs (with a scheme) are legal at
/// the N-Triples grammar level. This guards FM2 and FM5.
fn validate_absolute_iri(body: &str, open_offset: usize) -> Result<(), Diagnostics> {
    if body.is_empty() {
        return Err(fatal(format!(
            "NT-IRI-004: empty IRI at byte offset {open_offset}"
        )));
    }
    // RFC-3986 style scheme: ALPHA *( ALPHA / DIGIT / '+' / '-' / '.' ) ':'
    let mut chars = body.chars();
    let first = chars.next().expect("non-empty checked above");
    if !first.is_ascii_alphabetic() {
        return Err(fatal(format!(
            "NT-IRI-005: relative IRI reference at byte offset {open_offset}"
        )));
    }
    let mut saw_colon = false;
    for c in body.chars().skip(1) {
        if c == ':' {
            saw_colon = true;
            break;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
            return Err(fatal(format!(
                "NT-IRI-005: relative IRI reference at byte offset {open_offset}"
            )));
        }
    }
    if !saw_colon {
        return Err(fatal(format!(
            "NT-IRI-005: relative IRI reference at byte offset {open_offset}"
        )));
    }
    Ok(())
}

// --- PN_CHARS helpers (N-Triples §2.3 grammar) ------------------------------
//
// These tables are simplified from the grammar's `PN_CHARS_BASE` /
// `PN_CHARS_U` / `PN_CHARS` productions. The ranges match exactly; we
// write them out rather than pulling a regex crate in.

const fn is_pn_chars_base(c: char) -> bool {
    matches!(c,
        'A'..='Z' | 'a'..='z'
        | '\u{00C0}'..='\u{00D6}'
        | '\u{00D8}'..='\u{00F6}'
        | '\u{00F8}'..='\u{02FF}'
        | '\u{0370}'..='\u{037D}'
        | '\u{037F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}'
    )
}

const fn is_pn_chars_u(c: char) -> bool {
    // N-Triples / N-Quads grammar §2.3:
    //   PN_CHARS_U ::= PN_CHARS_BASE | '_'
    // Unlike Turtle, ':' is deliberately **excluded** because N-Triples has
    // no prefixed-name syntax — a ':' inside a blank-node label is always
    // a syntax error (see W3C negative tests `nt-syntax-bad-bnode-01` and
    // `nt-syntax-bad-bnode-02`).
    is_pn_chars_base(c) || c == '_'
}

const fn is_pn_chars(c: char) -> bool {
    is_pn_chars_u(c)
        || c.is_ascii_digit()
        || c == '-'
        || c == '\u{00B7}'
        || matches!(c, '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_nt(input: &str) -> Result<Facts, Diagnostics> {
        NTriplesParser.parse(input.as_bytes()).map(|p| p.facts)
    }

    fn parse_nq(input: &str) -> Result<Facts, Diagnostics> {
        NQuadsParser.parse(input.as_bytes()).map(|p| p.facts)
    }

    // --- happy path -------------------------------------------------

    #[test]
    fn accepts_simple_triple() {
        let facts = parse_nt(
            "<http://ex/s> <http://ex/p> <http://ex/o> .\n",
        ).expect("accept");
        assert_eq!(facts.set.len(), 1);
    }

    #[test]
    fn accepts_plain_literal() {
        let facts = parse_nt("<http://ex/s> <http://ex/p> \"hello\" .\n").expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"hello\"");
    }

    #[test]
    fn accepts_typed_literal() {
        let facts = parse_nt(
            "<http://ex/s> <http://ex/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
        )
        .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(
            fact.object,
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn accepts_lang_literal_and_case_folds() {
        // EN is canonicalised to 'en' by rdf-diff's BCP-47 folder.
        let facts = parse_nt("<http://ex/s> <http://ex/p> \"Hello\"@EN .\n").expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"Hello\"@en");
    }

    #[test]
    fn accepts_blank_node_subject() {
        let facts =
            parse_nt("_:alice <http://ex/p> <http://ex/o> .\n").expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert!(fact.subject.starts_with("_:"));
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let facts = parse_nt(concat!(
            "# leading comment\n",
            "\n",
            "<http://ex/s> <http://ex/p> <http://ex/o> .\n",
            "# trailing\n"
        ))
        .expect("accept");
        assert_eq!(facts.set.len(), 1);
    }

    // --- BOM ------------------------------------------------------

    #[test]
    fn leading_bom_is_skipped_with_warning() {
        let mut s = String::from("\u{FEFF}");
        s.push_str("<http://ex/s> <http://ex/p> <http://ex/o> .\n");
        let outcome = NTriplesParser.parse(s.as_bytes()).expect("accept");
        assert_eq!(outcome.facts.set.len(), 1);
        assert!(
            outcome
                .warnings
                .messages
                .iter()
                .any(|m| m.starts_with("ANY-BOM-001"))
        );
    }

    #[test]
    fn stray_bom_mid_stream_rejected() {
        let mut s = String::from("<http://ex/s> <http://ex/p> <http://ex/o> .\n");
        s.push('\u{FEFF}');
        s.push_str("<http://ex/s> <http://ex/p> <http://ex/o> .\n");
        let err = NTriplesParser.parse(s.as_bytes()).expect_err("reject");
        assert!(err.fatal);
        assert!(err.messages[0].starts_with("ANY-BOM-001"));
    }

    // --- EOL variants --------------------------------------------

    #[test]
    fn crlf_line_endings() {
        let facts = parse_nt(
            "<http://ex/s1> <http://ex/p> \"a\" .\r\n<http://ex/s2> <http://ex/p> \"b\" .\r\n",
        )
        .expect("accept");
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn bare_cr_line_endings() {
        // NT EOL ::= [#xD#xA]+, so a bare CR terminator is legal.
        // Note: we achieve this by using `.` followed directly by the
        // next statement; the `skip_ws_and_comments` path happily
        // consumes CR.
        let facts = parse_nt(
            "<http://ex/s1> <http://ex/p> \"a\" .\r<http://ex/s2> <http://ex/p> \"b\" .\r",
        )
        .expect("accept");
        assert_eq!(facts.set.len(), 2);
    }

    #[test]
    fn missing_final_newline_ok() {
        let facts =
            parse_nt("<http://ex/s> <http://ex/p> <http://ex/o> . # comment")
                .expect("accept");
        assert_eq!(facts.set.len(), 1);
    }

    // --- UCHAR escapes (NT-LITESC-001) ---------------------------

    #[test]
    fn uchar_upper_decodes() {
        let facts = parse_nt("<http://ex/s> <http://ex/p> \"\\u00E9\" .\n").expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"\u{00E9}\"");
    }

    #[test]
    fn uchar_lower_decodes_identically() {
        let facts_upper =
            parse_nt("<http://ex/s> <http://ex/p> \"\\u00E9\" .\n").expect("accept");
        let facts_lower =
            parse_nt("<http://ex/s> <http://ex/p> \"\\u00e9\" .\n").expect("accept");
        assert_eq!(facts_upper, facts_lower);
    }

    #[test]
    fn uchar_big_u_decodes() {
        let facts = parse_nt(
            "<http://ex/s> <http://ex/p> \"\\U0001F600\" .\n",
        )
        .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"\u{1F600}\"");
    }

    #[test]
    fn surrogate_escape_rejected() {
        let err = NTriplesParser
            .parse(b"<http://ex/s> <http://ex/p> \"\\uD800\" .\n")
            .expect_err("reject");
        assert!(err.fatal);
        assert!(err.messages[0].starts_with("NT-LITESC-001"));
    }

    #[test]
    fn truncated_uchar_rejected() {
        let err = NTriplesParser
            .parse(b"<http://ex/s> <http://ex/p> \"\\u00E\" .\n")
            .expect_err("reject");
        assert!(err.fatal);
        assert!(err.messages[0].starts_with("NT-LITESC-001"));
    }

    // --- Relative IRI rejection (FM2/FM5) -------------------------

    #[test]
    fn relative_iri_predicate_rejected() {
        let err = NTriplesParser
            .parse(b"<http://ex/s> <p> <http://ex/o> .\n")
            .expect_err("reject");
        assert!(err.fatal);
        assert!(err.messages[0].starts_with("NT-IRI-"));
    }

    #[test]
    fn relative_datatype_iri_rejected() {
        let err = NTriplesParser
            .parse(b"<http://ex/s> <http://ex/p> \"42\"^^<integer> .\n")
            .expect_err("reject");
        assert!(err.fatal);
    }

    // --- Blank node trailing dot (FM4) ----------------------------

    #[test]
    fn bnode_dot_in_middle_accepted() {
        let facts = parse_nt("_:b.1 <http://ex/p> <http://ex/o> .\n").expect("accept");
        assert_eq!(facts.set.len(), 1);
        let fact = facts.set.keys().next().unwrap();
        // Canonicalisation relabels to _:c0; the point is it parsed.
        assert!(fact.subject.starts_with("_:"));
    }

    #[test]
    fn bnode_trailing_dot_lex_does_not_absorb_terminator() {
        // `_:b1.` must lex as bnode `_:b1` + `.`. A greedy lexer that
        // absorbed the trailing `.` would produce an invalid label and
        // then complain about a missing terminator.
        //
        // We verify by constructing the pathological form the FM4-b
        // fixture targets: `_:b1.` immediately followed by the statement
        // body on the SAME statement is invalid regardless — the test
        // instead checks that the dot is restored so that a second
        // statement-ending `.` is reachable.
        let err = NTriplesParser
            .parse(b"_:b1. <http://ex/p> <http://ex/o> .\n")
            .expect_err("reject: two dots in a row is malformed");
        assert!(err.fatal);
        // And the well-formed baseline still succeeds.
        let ok = parse_nt("_:b1 <http://ex/p> <http://ex/o> .\n").expect("accept");
        assert_eq!(ok.set.len(), 1);
    }

    // --- Colon in blank-node label (W3C nt-syntax-bad-bnode-01/02) ---

    #[test]
    fn bnode_label_first_char_colon_rejected() {
        // W3C negative-syntax test `nt-syntax-bad-bnode-01`:
        //   _::a  <http://example/p> <http://example/o> .
        // Manifest comment: "Colon in bnode label not allowed".
        // In N-Triples, PN_CHARS_U = PN_CHARS_BASE | '_' (no ':'),
        // so the ':' after the mandatory `_:` prefix is illegal.
        let err = NTriplesParser
            .parse(b"_::a <http://example/p> <http://example/o> .\n")
            .expect_err("reject");
        assert!(err.fatal);
        assert!(
            err.messages[0].starts_with("NT-BN-002"),
            "expected NT-BN-002, got: {:?}",
            err.messages
        );
    }

    #[test]
    fn bnode_label_interior_colon_rejected() {
        // W3C negative-syntax test `nt-syntax-bad-bnode-02`:
        //   _:abc:def  <http://example/p> <http://example/o> .
        // The ':' after `abc` is not in PN_CHARS for N-Triples, so the
        // label ends at `abc` and the parser must then reject the `:` as
        // an unexpected character before the predicate.
        let err = NTriplesParser
            .parse(b"_:abc:def <http://example/p> <http://example/o> .\n")
            .expect_err("reject");
        assert!(err.fatal);
    }

    #[test]
    fn nquads_bnode_label_first_char_colon_rejected() {
        // Same fixture as NT but via NQuads (the NQ corpus duplicates the
        // NT negative tests verbatim).
        let err = NQuadsParser
            .parse(b"_::a <http://example/p> <http://example/o> .\n")
            .expect_err("reject");
        assert!(err.fatal);
        assert!(err.messages[0].starts_with("NT-BN-002"));
    }

    #[test]
    fn nquads_bnode_label_interior_colon_rejected() {
        let err = NQuadsParser
            .parse(b"_:abc:def <http://example/p> <http://example/o> .\n")
            .expect_err("reject");
        assert!(err.fatal);
    }

    // --- Missing terminator --------------------------------------

    #[test]
    fn missing_terminator_rejected() {
        let err = NTriplesParser
            .parse(b"<http://ex/s> <http://ex/p> <http://ex/o>\n")
            .expect_err("reject");
        assert!(err.fatal);
    }

    // --- N-Quads --------------------------------------------------

    #[test]
    fn nquads_with_graph_name() {
        let facts = parse_nq(
            "<http://ex/s> <http://ex/p> <http://ex/o> <http://ex/g> .\n",
        )
        .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.graph.as_deref(), Some("<http://ex/g>"));
    }

    #[test]
    fn nquads_without_graph_name() {
        let facts = parse_nq(
            "<http://ex/s> <http://ex/p> <http://ex/o> .\n",
        )
        .expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.graph, None);
    }

    #[test]
    fn nquads_relative_graph_iri_rejected() {
        let err = NQuadsParser
            .parse(b"<http://ex/s> <http://ex/p> <http://ex/o> <g> .\n")
            .expect_err("reject");
        assert!(err.fatal);
    }

    #[test]
    fn ntriples_rejects_fourth_term() {
        // The fourth term is a graph name, forbidden in N-Triples.
        let err = NTriplesParser
            .parse(b"<http://ex/s> <http://ex/p> <http://ex/o> <http://ex/g> .\n")
            .expect_err("reject");
        assert!(err.fatal);
    }

    // --- literal lexical form preservation ------------------------

    #[test]
    fn literal_lexical_form_is_not_trimmed() {
        let facts =
            parse_nt("<http://ex/s> <http://ex/p> \"  spaced  \" .\n").expect("accept");
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"  spaced  \"");
    }

    #[test]
    fn echar_newline_in_literal_decodes() {
        let facts =
            parse_nt("<http://ex/s> <http://ex/p> \"a\\nb\" .\n").expect("accept");
        let fact = facts.set.keys().next().unwrap();
        // Raw newline survives — rdf_diff stores the lexical form opaquely.
        assert_eq!(fact.object, "\"a\nb\"");
    }

    #[test]
    fn raw_newline_in_literal_rejected() {
        let err = NTriplesParser
            .parse(b"<http://ex/s> <http://ex/p> \"a\nb\" .\n")
            .expect_err("reject");
        assert!(err.fatal);
    }

    // --- parser ids ----------------------------------------------

    #[test]
    fn parser_ids_are_stable() {
        assert_eq!(NTriplesParser.id(), "rdf-ntriples");
        assert_eq!(NQuadsParser.id(), "rdf-nquads");
    }

}
