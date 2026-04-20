//! Hand-rolled lexer for Datalog syntax.

/// Token kinds produced by the Datalog lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// An identifier that starts with an uppercase letter (variable).
    Uppercase(String),
    /// An identifier that starts with a lowercase letter (constant or relname).
    Lowercase(String),
    /// A quoted string constant, e.g. `"ice cream"`. Value is the raw content
    /// between the quotes (no surrounding quotes).
    QuotedStr(String),
    /// `:-`
    ColonDash,
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `not` keyword (lowercase identifier matched separately).
    Not,
    /// End of input.
    Eof,
}

/// A single token with its byte offset in the source.
#[derive(Debug, Clone)]
pub struct Token {
    /// Token kind.
    pub kind: TokenKind,
    /// Byte offset of the first byte of this token in the source.
    pub offset: usize,
}

/// Errors that can be produced during lexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    /// Human-readable message.
    pub message: String,
    /// Byte offset where the error was detected.
    pub offset: usize,
}

/// Tokenise the full input, returning either a token list (including a
/// terminal `Eof` token) or the first lex error.
///
/// # Errors
///
/// Returns a [`LexError`] on the first byte that cannot be interpreted as
/// part of a valid Datalog token (unexpected byte, unterminated string
/// literal, lone `:` not followed by `-`, or invalid UTF-8).
#[allow(clippy::too_many_lines)]
pub fn tokenise(input: &[u8]) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    let len = input.len();

    while pos < len {
        let start = pos;
        let ch = input[pos];

        match ch {
            // Skip whitespace.
            b' ' | b'\t' | b'\r' | b'\n' => {
                pos += 1;
            }

            // Comment: `% ... \n` — skip the entire line.
            b'%' => {
                while pos < len && input[pos] != b'\n' {
                    pos += 1;
                }
            }

            // Single-character punctuation.
            b'.' => {
                tokens.push(Token { kind: TokenKind::Dot, offset: start });
                pos += 1;
            }
            b',' => {
                tokens.push(Token { kind: TokenKind::Comma, offset: start });
                pos += 1;
            }
            b'(' => {
                tokens.push(Token { kind: TokenKind::LParen, offset: start });
                pos += 1;
            }
            b')' => {
                tokens.push(Token { kind: TokenKind::RParen, offset: start });
                pos += 1;
            }

            // `:-`
            b':' => {
                if pos + 1 < len && input[pos + 1] == b'-' {
                    tokens.push(Token { kind: TokenKind::ColonDash, offset: start });
                    pos += 2;
                } else {
                    return Err(LexError {
                        message: format!(
                            "unexpected character ':' at offset {start}; expected ':-'"
                        ),
                        offset: start,
                    });
                }
            }

            // Quoted string.
            b'"' => {
                pos += 1; // skip opening quote
                let str_start = pos;
                while pos < len && input[pos] != b'"' {
                    pos += 1;
                }
                if pos >= len {
                    return Err(LexError {
                        message: format!("unterminated string literal starting at offset {start}"),
                        offset: start,
                    });
                }
                let raw = std::str::from_utf8(&input[str_start..pos]).map_err(|_| LexError {
                    message: format!("invalid UTF-8 in string literal at offset {start}"),
                    offset: start,
                })?;
                tokens.push(Token {
                    kind: TokenKind::QuotedStr(raw.to_owned()),
                    offset: start,
                });
                pos += 1; // skip closing quote
            }

            // Uppercase identifier → variable.
            b'A'..=b'Z' => {
                let id_start = pos;
                while pos < len && is_ident_continue(input[pos]) {
                    pos += 1;
                }
                let raw = std::str::from_utf8(&input[id_start..pos]).map_err(|_| LexError {
                    message: format!("invalid UTF-8 in identifier at offset {start}"),
                    offset: start,
                })?;
                tokens.push(Token {
                    kind: TokenKind::Uppercase(raw.to_owned()),
                    offset: start,
                });
            }

            // Lowercase identifier → relname, constant, or `not`.
            b'a'..=b'z' => {
                let id_start = pos;
                while pos < len && is_ident_continue(input[pos]) {
                    pos += 1;
                }
                let raw = std::str::from_utf8(&input[id_start..pos]).map_err(|_| LexError {
                    message: format!("invalid UTF-8 in identifier at offset {start}"),
                    offset: start,
                })?;
                let kind = if raw == "not" {
                    TokenKind::Not
                } else {
                    TokenKind::Lowercase(raw.to_owned())
                };
                tokens.push(Token { kind, offset: start });
            }

            // Anything else is unexpected.
            other => {
                return Err(LexError {
                    message: format!(
                        "unexpected byte 0x{other:02X} ({}) at offset {start}",
                        char::from(other),
                    ),
                    offset: start,
                });
            }
        }
    }

    tokens.push(Token { kind: TokenKind::Eof, offset: len });
    Ok(tokens)
}

/// `true` for bytes that may continue an identifier (`[a-zA-Z0-9_]`).
#[inline]
const fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
