//! Hand-written SPARQL 1.1 lexer.
//!
//! Produces a flat `Vec<Token>` from a UTF-8 source string. Whitespace and
//! comments are consumed and discarded between tokens; their byte offsets are
//! not preserved.
//!
//! Token classification follows the SPARQL 1.1 grammar terminals (§19.5 of the
//! W3C SPARQL 1.1 Query Language recommendation).

use crate::ParseError;

/// A single lexical token with its byte span in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Raw source text for this token.
    pub text: String,
    /// 0-indexed byte offset of the first byte of this token.
    pub offset: usize,
}

/// All token kinds recognised by the SPARQL 1.1 grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TokenKind {
    // ----- Keywords -----
    /// `SELECT`
    Select,
    /// `CONSTRUCT`
    Construct,
    /// `ASK`
    Ask,
    /// `DESCRIBE`
    Describe,
    /// `WHERE`
    Where,
    /// `FROM`
    From,
    /// `NAMED`
    Named,
    /// `DISTINCT`
    Distinct,
    /// `REDUCED`
    Reduced,
    /// `AS`
    As,
    /// `GROUP`
    Group,
    /// `BY`
    By,
    /// `HAVING`
    Having,
    /// `ORDER`
    Order,
    /// `ASC`
    Asc,
    /// `DESC`
    Desc,
    /// `LIMIT`
    Limit,
    /// `OFFSET`
    Offset,
    /// `VALUES`
    Values,
    /// `UNDEF`
    Undef,
    /// `GRAPH`
    Graph,
    /// `OPTIONAL`
    Optional,
    /// `UNION`
    Union,
    /// `FILTER`
    Filter,
    /// `BIND`
    Bind,
    /// `MINUS` (graph pattern MINUS keyword)
    MinusKw,
    /// `SERVICE`
    Service,
    /// `SILENT`
    Silent,
    /// `EXISTS`
    Exists,
    /// `NOT`
    Not,
    /// `IN`
    In,
    /// `STR`
    Str,
    /// `LANG`
    Lang,
    /// `LANGMATCHES`
    LangMatches,
    /// `DATATYPE`
    Datatype,
    /// `BOUND`
    Bound,
    /// `IRI`
    IriFunc,
    /// `URI`
    UriFunc,
    /// `BNODE`
    BnodeFunc,
    /// `RAND`
    Rand,
    /// `ABS`
    Abs,
    /// `CEIL`
    Ceil,
    /// `FLOOR`
    Floor,
    /// `ROUND`
    Round,
    /// `CONCAT`
    Concat,
    /// `STRLEN`
    Strlen,
    /// `SUBSTR`
    Substr,
    /// `UCASE`
    Ucase,
    /// `LCASE`
    Lcase,
    /// `ENCODE_FOR_URI`
    EncodeForUri,
    /// `CONTAINS`
    Contains,
    /// `STRSTARTS`
    Strstarts,
    /// `STRENDS`
    Strends,
    /// `STRBEFORE`
    Strbefore,
    /// `STRAFTER`
    Strafter,
    /// `YEAR`
    Year,
    /// `MONTH`
    Month,
    /// `DAY`
    Day,
    /// `HOURS`
    Hours,
    /// `MINUTES`
    Minutes,
    /// `SECONDS`
    Seconds,
    /// `TIMEZONE`
    Timezone,
    /// `TZ`
    Tz,
    /// `NOW`
    Now,
    /// `UUID`
    Uuid,
    /// `STRUUID`
    Struuid,
    /// `MD5`
    Md5,
    /// `SHA1`
    Sha1,
    /// `SHA256`
    Sha256,
    /// `SHA384`
    Sha384,
    /// `SHA512`
    Sha512,
    /// `COALESCE`
    Coalesce,
    /// `IF`
    If,
    /// `STRLANG`
    Strlang,
    /// `STRDT`
    Strdt,
    /// `SAMETERM`
    Sameterm,
    /// `ISIRI`
    IsIri,
    /// `ISURI`
    IsUri,
    /// `ISBLANK`
    IsBlank,
    /// `ISLITERAL`
    IsLiteral,
    /// `ISNUMERIC`
    IsNumeric,
    /// `REGEX`
    Regex,
    /// `SUBSTR` (already listed, but used as function too)
    /// `REPLACE`
    Replace,
    /// `COUNT`
    Count,
    /// `SUM`
    Sum,
    /// `MIN`
    Min,
    /// `MAX`
    Max,
    /// `AVG`
    Avg,
    /// `SAMPLE`
    Sample,
    /// `GROUP_CONCAT`
    GroupConcat,
    /// `SEPARATOR`
    Separator,
    /// `TRUE`
    True,
    /// `FALSE`
    False,
    /// `BASE`
    Base,
    /// `PREFIX`
    Prefix,
    // ----- SPARQL Update keywords -----
    /// `INSERT`
    Insert,
    /// `DELETE`
    Delete,
    /// `DATA`
    Data,
    /// `LOAD`
    Load,
    /// `CLEAR`
    Clear,
    /// `CREATE`
    Create,
    /// `DROP`
    Drop,
    /// `COPY`
    Copy,
    /// `MOVE`
    Move,
    /// `ADD`
    Add,
    /// `INTO`
    Into,
    /// `ALL`
    All,
    /// `DEFAULT`
    Default,
    /// `WITH`
    With,
    /// `USING`
    Using,

    // ----- Punctuation -----
    /// `{`
    LBrace,
    /// `}`
    RBrace,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `;`
    Semi,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `|`
    Pipe,
    /// `^`
    Caret,
    /// `?`
    Question,
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `!`
    Bang,
    /// `=`
    Eq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `^^`
    DatatypeSep,

    // ----- Literals -----
    /// `<...>` absolute IRI reference
    IriRef,
    /// `prefix:local` prefixed name
    PrefixedName,
    /// `?name` or `$name` variable
    Variable,
    /// `_:label` blank-node label
    BlankNodeLabel,
    /// `[]` anonymous blank node
    Anon,
    /// `"..."` or `'...'` or `"""..."""` or `'''...'''` string literal
    StringLiteral,
    /// Integer literal, e.g. `42`
    IntegerLiteral,
    /// Decimal literal, e.g. `3.14`
    DecimalLiteral,
    /// Double literal, e.g. `1.0e10`
    DoubleLiteral,
    /// `@lang`
    LangTag,
    /// `a` (shorthand for `rdf:type`)
    A,
    /// End-of-file sentinel
    Eof,
}

/// Tokenise `src` and return the token sequence (never includes `Eof` — the
/// parser adds that sentinel itself).
///
/// # Errors
///
/// Returns [`ParseError::Lex`] for any unrecognised character or malformed
/// token.
pub fn tokenise(src: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut pos = 0_usize;

    while pos < len {
        // Skip whitespace and comments
        pos = skip_ws_and_comments(bytes, pos);
        if pos >= len {
            break;
        }
        let start = pos;
        let b = bytes[pos];

        // IRI reference
        if b == b'<' {
            // Check for <=
            if pos + 1 < len && bytes[pos + 1] == b'=' {
                tokens.push(Token {
                    kind: TokenKind::Le,
                    text: "<=".to_owned(),
                    offset: start,
                });
                pos += 2;
                continue;
            }
            let (text, end) = lex_iri_ref(bytes, pos, src)?;
            tokens.push(Token {
                kind: TokenKind::IriRef,
                text,
                offset: start,
            });
            pos = end;
            continue;
        }

        // Variable
        if b == b'?' || b == b'$' {
            let (text, end) = lex_var(bytes, pos, src)?;
            tokens.push(Token {
                kind: TokenKind::Variable,
                text,
                offset: start,
            });
            pos = end;
            continue;
        }

        // Blank node label or anonymous blank node
        if b == b'_' && pos + 1 < len && bytes[pos + 1] == b':' {
            let (text, end) = lex_blank_label(bytes, pos, src)?;
            tokens.push(Token {
                kind: TokenKind::BlankNodeLabel,
                text,
                offset: start,
            });
            pos = end;
            continue;
        }

        // Anonymous blank node []
        if b == b'[' {
            // Peek ahead to see if it's [ ] (anon)
            let mut p2 = pos + 1;
            while p2 < len && (bytes[p2] == b' ' || bytes[p2] == b'\t') {
                p2 += 1;
            }
            if p2 < len && bytes[p2] == b']' {
                tokens.push(Token {
                    kind: TokenKind::Anon,
                    text: src[pos..=p2].to_owned(),
                    offset: start,
                });
                pos = p2 + 1;
                continue;
            }
            tokens.push(Token {
                kind: TokenKind::LBracket,
                text: "[".to_owned(),
                offset: start,
            });
            pos += 1;
            continue;
        }

        // String literals
        if b == b'"' || b == b'\'' {
            let (text, end) = lex_string(bytes, pos, src)?;
            tokens.push(Token {
                kind: TokenKind::StringLiteral,
                text,
                offset: start,
            });
            pos = end;
            continue;
        }

        // Numeric literals
        if b.is_ascii_digit() || (b == b'-' && pos + 1 < len && bytes[pos + 1].is_ascii_digit())
            || (b == b'+' && pos + 1 < len && bytes[pos + 1].is_ascii_digit())
            || (b == b'.'
                && pos + 1 < len
                && bytes[pos + 1].is_ascii_digit())
        {
            let (kind, text, end) = lex_number(bytes, pos)?;
            tokens.push(Token {
                kind,
                text,
                offset: start,
            });
            pos = end;
            continue;
        }

        // Language tag @lang
        if b == b'@' {
            let (text, end) = lex_lang_tag(bytes, pos, src)?;
            tokens.push(Token {
                kind: TokenKind::LangTag,
                text,
                offset: start,
            });
            pos = end;
            continue;
        }

        // Datatype separator ^^
        if b == b'^' {
            if pos + 1 < len && bytes[pos + 1] == b'^' {
                tokens.push(Token {
                    kind: TokenKind::DatatypeSep,
                    text: "^^".to_owned(),
                    offset: start,
                });
                pos += 2;
                continue;
            }
            tokens.push(Token {
                kind: TokenKind::Caret,
                text: "^".to_owned(),
                offset: start,
            });
            pos += 1;
            continue;
        }

        // Identifiers and keywords (ASCII alpha, underscore, or non-ASCII)
        if b.is_ascii_alphabetic() || b == b'_' || b >= 0x80 {
            let (text, end) = lex_ident(bytes, pos, src);
            let kind = keyword_or_ident(&text);
            tokens.push(Token {
                kind,
                text,
                offset: start,
            });
            pos = end;
            continue;
        }

        // Multi-char punctuation
        match b {
            b'!' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::NotEq,
                        text: "!=".to_owned(),
                        offset: start,
                    });
                    pos += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Bang,
                        text: "!".to_owned(),
                        offset: start,
                    });
                    pos += 1;
                }
            }
            b'>' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    tokens.push(Token {
                        kind: TokenKind::Ge,
                        text: ">=".to_owned(),
                        offset: start,
                    });
                    pos += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Gt,
                        text: ">".to_owned(),
                        offset: start,
                    });
                    pos += 1;
                }
            }
            b'&' => {
                if pos + 1 < len && bytes[pos + 1] == b'&' {
                    tokens.push(Token {
                        kind: TokenKind::And,
                        text: "&&".to_owned(),
                        offset: start,
                    });
                    pos += 2;
                } else {
                    return Err(ParseError::lex(pos, "bare '&' is not valid SPARQL"));
                }
            }
            b'|' => {
                if pos + 1 < len && bytes[pos + 1] == b'|' {
                    tokens.push(Token {
                        kind: TokenKind::Or,
                        text: "||".to_owned(),
                        offset: start,
                    });
                    pos += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Pipe,
                        text: "|".to_owned(),
                        offset: start,
                    });
                    pos += 1;
                }
            }
            b'{' => {
                tokens.push(Token {
                    kind: TokenKind::LBrace,
                    text: "{".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'}' => {
                tokens.push(Token {
                    kind: TokenKind::RBrace,
                    text: "}".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'(' => {
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    text: "(".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b')' => {
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    text: ")".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b']' => {
                tokens.push(Token {
                    kind: TokenKind::RBracket,
                    text: "]".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'.' => {
                tokens.push(Token {
                    kind: TokenKind::Dot,
                    text: ".".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b',' => {
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    text: ",".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b';' => {
                tokens.push(Token {
                    kind: TokenKind::Semi,
                    text: ";".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'*' => {
                tokens.push(Token {
                    kind: TokenKind::Star,
                    text: "*".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'/' => {
                tokens.push(Token {
                    kind: TokenKind::Slash,
                    text: "/".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'+' => {
                tokens.push(Token {
                    kind: TokenKind::Plus,
                    text: "+".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'-' => {
                tokens.push(Token {
                    kind: TokenKind::Minus,
                    text: "-".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            b'=' => {
                tokens.push(Token {
                    kind: TokenKind::Eq,
                    text: "=".to_owned(),
                    offset: start,
                });
                pos += 1;
            }
            _ => {
                return Err(ParseError::lex(
                    pos,
                    format!("unexpected character {:?}", char::from(b)),
                ));
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
        offset: len,
    });

    Ok(tokens)
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn skip_ws_and_comments(bytes: &[u8], mut pos: usize) -> usize {
    loop {
        // Skip ASCII whitespace
        while pos < bytes.len()
            && (bytes[pos] == b' '
                || bytes[pos] == b'\t'
                || bytes[pos] == b'\r'
                || bytes[pos] == b'\n')
        {
            pos += 1;
        }
        // Skip # comment to end of line
        if pos < bytes.len() && bytes[pos] == b'#' {
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }
        break;
    }
    pos
}

fn lex_iri_ref(
    bytes: &[u8],
    pos: usize,
    src: &str,
) -> Result<(String, usize), ParseError> {
    // pos is at '<'
    let mut i = pos + 1;
    while i < bytes.len() && bytes[i] != b'>' {
        if bytes[i] == b'<' || bytes[i] == b'"' || bytes[i] == b'{' || bytes[i] == b'}' {
            return Err(ParseError::lex(i, "illegal character in IRI reference"));
        }
        i += 1;
    }
    if i >= bytes.len() {
        return Err(ParseError::lex(pos, "unterminated IRI reference"));
    }
    let text = src[pos..=i].to_owned(); // includes < and >
    Ok((text, i + 1))
}

fn lex_var(
    bytes: &[u8],
    pos: usize,
    src: &str,
) -> Result<(String, usize), ParseError> {
    // pos is at '?' or '$'
    let mut i = pos + 1;
    while i < bytes.len() && is_varname_char(bytes[i]) {
        i += 1;
    }
    if i == pos + 1 {
        return Err(ParseError::lex(pos, "empty variable name"));
    }
    Ok((src[pos..i].to_owned(), i))
}

fn is_varname_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

fn lex_blank_label(
    bytes: &[u8],
    pos: usize,
    src: &str,
) -> Result<(String, usize), ParseError> {
    // pos at '_', next is ':'
    let mut i = pos + 2;
    // First char: PN_CHARS_U or digit
    if i >= bytes.len() {
        return Err(ParseError::lex(pos, "empty blank-node label"));
    }
    if !is_pn_chars_u(bytes[i]) && !bytes[i].is_ascii_digit() {
        return Err(ParseError::lex(i, "invalid start of blank-node label"));
    }
    i += 1;
    while i < bytes.len() && (is_pn_chars(bytes[i]) || bytes[i] == b'.') {
        i += 1;
    }
    // Trailing dot is not part of the label
    while i > pos + 2 && bytes[i - 1] == b'.' {
        i -= 1;
    }
    Ok((src[pos..i].to_owned(), i))
}

fn is_pn_chars_u(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b >= 0x80
}

fn is_pn_chars(b: u8) -> bool {
    is_pn_chars_u(b) || b == b'-' || b.is_ascii_digit()
}

fn lex_string(
    bytes: &[u8],
    pos: usize,
    src: &str,
) -> Result<(String, usize), ParseError> {
    let delim = bytes[pos];
    let mut i = pos + 1;
    // Check for triple-quote
    let triple = i + 1 < bytes.len() && bytes[i] == delim && bytes[i + 1] == delim;
    if triple {
        i += 2;
        loop {
            if i + 2 >= bytes.len() {
                return Err(ParseError::lex(pos, "unterminated triple-quoted string"));
            }
            if bytes[i] == delim && bytes[i + 1] == delim && bytes[i + 2] == delim {
                i += 3;
                break;
            }
            if bytes[i] == b'\\' {
                i += 2; // skip escape
            } else {
                i += 1;
            }
        }
    } else {
        loop {
            if i >= bytes.len() {
                return Err(ParseError::lex(pos, "unterminated string literal"));
            }
            if bytes[i] == delim {
                i += 1;
                break;
            }
            if bytes[i] == b'\n' || bytes[i] == b'\r' {
                return Err(ParseError::lex(
                    i,
                    "unescaped newline in single-quoted string literal",
                ));
            }
            if bytes[i] == b'\\' {
                i += 2;
            } else {
                i += 1;
            }
        }
    }
    Ok((src[pos..i].to_owned(), i))
}

fn lex_number(
    bytes: &[u8],
    pos: usize,
) -> Result<(TokenKind, String, usize), ParseError> {
    let mut i = pos;
    let mut is_decimal = false;
    let mut is_double = false;

    // Optional sign
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }

    // Integer part
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }

    // Fractional part
    if i < bytes.len() && bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit()
    {
        is_decimal = true;
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    // Exponent
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        is_double = true;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }

    let text = std::str::from_utf8(&bytes[pos..i])
        .unwrap_or("")
        .to_owned();

    let kind = if is_double {
        TokenKind::DoubleLiteral
    } else if is_decimal {
        TokenKind::DecimalLiteral
    } else {
        TokenKind::IntegerLiteral
    };

    Ok((kind, text, i))
}

fn lex_lang_tag(
    bytes: &[u8],
    pos: usize,
    src: &str,
) -> Result<(String, usize), ParseError> {
    // pos at '@'
    let mut i = pos + 1;
    if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
        return Err(ParseError::lex(pos, "empty language tag"));
    }
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    while i < bytes.len() && bytes[i] == b'-' {
        i += 1;
        if i >= bytes.len() || !bytes[i].is_ascii_alphanumeric() {
            return Err(ParseError::lex(i, "malformed language tag subtag"));
        }
        while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
            i += 1;
        }
    }
    Ok((src[pos..i].to_owned(), i))
}

fn lex_ident(bytes: &[u8], pos: usize, src: &str) -> (String, usize) {
    let mut i = pos;
    // First char: alpha, _, or non-ASCII (possible Unicode PN_CHARS_BASE)
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-' || bytes[i] >= 0x80) {
        i += 1;
    }
    // Prefixed names: if followed by ':' and then a local part
    if i < bytes.len() && bytes[i] == b':' {
        // Could be a prefixed name like "prefix:local"
        let colon = i;
        i += 1;
        // local part may be empty (e.g. "ex:" is valid)
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric()
                || bytes[i] == b'_'
                || bytes[i] == b'-'
                || bytes[i] == b'.'
                || bytes[i] >= 0x80
                || bytes[i] == b'%'   // percent-encoding
                || bytes[i] == b'\\') // local escape
        {
            if bytes[i] == b'.' && (i + 1 >= bytes.len() || !is_local_part_char(bytes[i + 1])) {
                break;
            }
            i += 1;
        }
        // Trailing dot not part of local
        while i > colon + 1 && bytes[i - 1] == b'.' {
            i -= 1;
        }
        return (src[pos..i].to_owned(), i);
    }
    (src[pos..i].to_owned(), i)
}

fn is_local_part_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b >= 0x80 || b == b'%' || b == b'\\'
}

#[allow(clippy::too_many_lines)]
fn keyword_or_ident(text: &str) -> TokenKind {
    // Check for `a` shorthand
    if text == "a" {
        return TokenKind::A;
    }
    match text.to_ascii_uppercase().as_str() {
        "SELECT" => TokenKind::Select,
        "CONSTRUCT" => TokenKind::Construct,
        "ASK" => TokenKind::Ask,
        "DESCRIBE" => TokenKind::Describe,
        "WHERE" => TokenKind::Where,
        "FROM" => TokenKind::From,
        "NAMED" => TokenKind::Named,
        "DISTINCT" => TokenKind::Distinct,
        "REDUCED" => TokenKind::Reduced,
        "AS" => TokenKind::As,
        "GROUP" => TokenKind::Group,
        "BY" => TokenKind::By,
        "HAVING" => TokenKind::Having,
        "ORDER" => TokenKind::Order,
        "ASC" => TokenKind::Asc,
        "DESC" => TokenKind::Desc,
        "LIMIT" => TokenKind::Limit,
        "OFFSET" => TokenKind::Offset,
        "VALUES" => TokenKind::Values,
        "UNDEF" => TokenKind::Undef,
        "GRAPH" => TokenKind::Graph,
        "OPTIONAL" => TokenKind::Optional,
        "UNION" => TokenKind::Union,
        "FILTER" => TokenKind::Filter,
        "BIND" => TokenKind::Bind,
        "MINUS" => TokenKind::MinusKw,
        "SERVICE" => TokenKind::Service,
        "SILENT" => TokenKind::Silent,
        "EXISTS" => TokenKind::Exists,
        "NOT" => TokenKind::Not,
        "IN" => TokenKind::In,
        "STR" => TokenKind::Str,
        "LANG" => TokenKind::Lang,
        "LANGMATCHES" => TokenKind::LangMatches,
        "DATATYPE" => TokenKind::Datatype,
        "BOUND" => TokenKind::Bound,
        "IRI" => TokenKind::IriFunc,
        "URI" => TokenKind::UriFunc,
        "BNODE" => TokenKind::BnodeFunc,
        "RAND" => TokenKind::Rand,
        "ABS" => TokenKind::Abs,
        "CEIL" => TokenKind::Ceil,
        "FLOOR" => TokenKind::Floor,
        "ROUND" => TokenKind::Round,
        "CONCAT" => TokenKind::Concat,
        "STRLEN" => TokenKind::Strlen,
        "SUBSTR" => TokenKind::Substr,
        "UCASE" => TokenKind::Ucase,
        "LCASE" => TokenKind::Lcase,
        "ENCODE_FOR_URI" => TokenKind::EncodeForUri,
        "CONTAINS" => TokenKind::Contains,
        "STRSTARTS" => TokenKind::Strstarts,
        "STRENDS" => TokenKind::Strends,
        "STRBEFORE" => TokenKind::Strbefore,
        "STRAFTER" => TokenKind::Strafter,
        "YEAR" => TokenKind::Year,
        "MONTH" => TokenKind::Month,
        "DAY" => TokenKind::Day,
        "HOURS" => TokenKind::Hours,
        "MINUTES" => TokenKind::Minutes,
        "SECONDS" => TokenKind::Seconds,
        "TIMEZONE" => TokenKind::Timezone,
        "TZ" => TokenKind::Tz,
        "NOW" => TokenKind::Now,
        "UUID" => TokenKind::Uuid,
        "STRUUID" => TokenKind::Struuid,
        "MD5" => TokenKind::Md5,
        "SHA1" => TokenKind::Sha1,
        "SHA256" => TokenKind::Sha256,
        "SHA384" => TokenKind::Sha384,
        "SHA512" => TokenKind::Sha512,
        "COALESCE" => TokenKind::Coalesce,
        "IF" => TokenKind::If,
        "STRLANG" => TokenKind::Strlang,
        "STRDT" => TokenKind::Strdt,
        "SAMETERM" => TokenKind::Sameterm,
        "ISIRI" => TokenKind::IsIri,
        "ISURI" => TokenKind::IsUri,
        "ISBLANK" => TokenKind::IsBlank,
        "ISLITERAL" => TokenKind::IsLiteral,
        "ISNUMERIC" => TokenKind::IsNumeric,
        "REGEX" => TokenKind::Regex,
        "REPLACE" => TokenKind::Replace,
        "COUNT" => TokenKind::Count,
        "SUM" => TokenKind::Sum,
        "MIN" => TokenKind::Min,
        "MAX" => TokenKind::Max,
        "AVG" => TokenKind::Avg,
        "SAMPLE" => TokenKind::Sample,
        "GROUP_CONCAT" => TokenKind::GroupConcat,
        "SEPARATOR" => TokenKind::Separator,
        "TRUE" => TokenKind::True,
        "FALSE" => TokenKind::False,
        "BASE" => TokenKind::Base,
        "PREFIX" => TokenKind::Prefix,
        "INSERT" => TokenKind::Insert,
        "DELETE" => TokenKind::Delete,
        "DATA" => TokenKind::Data,
        "LOAD" => TokenKind::Load,
        "CLEAR" => TokenKind::Clear,
        "CREATE" => TokenKind::Create,
        "DROP" => TokenKind::Drop,
        "COPY" => TokenKind::Copy,
        "MOVE" => TokenKind::Move,
        "ADD" => TokenKind::Add,
        "INTO" => TokenKind::Into,
        "ALL" => TokenKind::All,
        "DEFAULT" => TokenKind::Default,
        "WITH" => TokenKind::With,
        "USING" => TokenKind::Using,
        _ => {
            // Check if it contains ':', treat as prefixed name
            if text.contains(':') {
                TokenKind::PrefixedName
            } else {
                // Bare identifier — treat as prefixed name with empty prefix
                TokenKind::PrefixedName
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenise_simple_select() {
        let src = "SELECT ?x WHERE { ?x a <http://example.org/T> }";
        let tokens = tokenise(src).unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert!(kinds.contains(&&TokenKind::Select));
        assert!(kinds.contains(&&TokenKind::Variable));
        assert!(kinds.contains(&&TokenKind::Where));
        assert!(kinds.contains(&&TokenKind::A));
        assert!(kinds.contains(&&TokenKind::IriRef));
    }

    #[test]
    fn tokenise_insert_data() {
        let src = "INSERT DATA { <s> <p> <o> . }";
        let tokens = tokenise(src).unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Insert)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Data)));
    }

    #[test]
    fn tokenise_string_literal() {
        let tokens = tokenise("\"hello world\"").unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::StringLiteral)));
    }

    #[test]
    fn tokenise_integer() {
        let tokens = tokenise("42").unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::IntegerLiteral)));
    }

    #[test]
    fn tokenise_prefixed_name() {
        let tokens = tokenise("ex:Foo").unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::PrefixedName)));
    }

    #[test]
    fn tokenise_blank_node() {
        let tokens = tokenise("_:b0").unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::BlankNodeLabel)));
    }

    #[test]
    fn tokenise_lang_tag() {
        let tokens = tokenise("\"hello\"@en").unwrap();
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::LangTag)));
    }
}
