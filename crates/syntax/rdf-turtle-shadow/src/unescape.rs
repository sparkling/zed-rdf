//! String-escape and IRI-escape decoding for Turtle 1.1.
//!
//! Handles:
//! - String escape sequences: `\\`, `\"`, `\'`, `\n`, `\r`, `\t`, `\b`, `\f`
//! - Unicode escapes: `\uXXXX` and `\UXXXXXXXX`
//! - IRI local-name percent encoding passthrough (we preserve `%XX` in IRIs)

use thiserror::Error;

/// Errors produced during escape sequence decoding.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnescapeError {
    /// A `\uXXXX` or `\UXXXXXXXX` sequence contained a non-hex digit.
    #[error("invalid unicode escape at byte {offset}: {message}")]
    InvalidUnicodeEscape {
        /// Byte offset of the backslash.
        offset: usize,
        /// Human-readable message.
        message: String,
    },
    /// The code point decoded from a unicode escape is not valid Unicode.
    #[error("unicode code point U+{code_point:04X} at byte {offset} is not a valid scalar")]
    InvalidCodePoint {
        /// Byte offset of the backslash.
        offset: usize,
        /// The code point value.
        code_point: u32,
    },
    /// An unrecognised escape sequence.
    #[error("unknown escape sequence '\\{ch}' at byte {offset}")]
    UnknownEscape {
        /// Byte offset of the backslash.
        offset: usize,
        /// The character after the backslash.
        ch: char,
    },
    /// A backslash at end of input.
    #[error("backslash at end of input at byte {offset}")]
    TrailingBackslash {
        /// Byte offset of the backslash.
        offset: usize,
    },
}

/// Decode Turtle string escape sequences in `input`, writing the result into
/// `out`. Returns an error on invalid escapes.
pub fn unescape_string(input: &str) -> Result<String, UnescapeError> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        // backslash — read next char
        match chars.next() {
            None => return Err(UnescapeError::TrailingBackslash { offset: i }),
            Some((_, 't')) => out.push('\t'),
            Some((_, 'b')) => out.push('\x08'),
            Some((_, 'n')) => out.push('\n'),
            Some((_, 'r')) => out.push('\r'),
            Some((_, 'f')) => out.push('\x0C'),
            Some((_, '"')) => out.push('"'),
            Some((_, '\'')) => out.push('\''),
            Some((_, '\\')) => out.push('\\'),
            Some((_, 'u')) => {
                let cp = read_hex_digits(&mut chars, i, 4)?;
                let c = char::from_u32(cp).ok_or(UnescapeError::InvalidCodePoint {
                    offset: i,
                    code_point: cp,
                })?;
                out.push(c);
            }
            Some((_, 'U')) => {
                let cp = read_hex_digits(&mut chars, i, 8)?;
                let c = char::from_u32(cp).ok_or(UnescapeError::InvalidCodePoint {
                    offset: i,
                    code_point: cp,
                })?;
                out.push(c);
            }
            Some((_, other)) => {
                return Err(UnescapeError::UnknownEscape {
                    offset: i,
                    ch: other,
                });
            }
        }
    }
    Ok(out)
}

/// Decode IRI escape sequences (`\uXXXX` and `\UXXXXXXXX` only).
/// Other backslash sequences are errors in IRI context.
pub fn unescape_iri(input: &str) -> Result<String, UnescapeError> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            None => return Err(UnescapeError::TrailingBackslash { offset: i }),
            Some((_, 'u')) => {
                let cp = read_hex_digits(&mut chars, i, 4)?;
                let c = char::from_u32(cp).ok_or(UnescapeError::InvalidCodePoint {
                    offset: i,
                    code_point: cp,
                })?;
                out.push(c);
            }
            Some((_, 'U')) => {
                let cp = read_hex_digits(&mut chars, i, 8)?;
                let c = char::from_u32(cp).ok_or(UnescapeError::InvalidCodePoint {
                    offset: i,
                    code_point: cp,
                })?;
                out.push(c);
            }
            Some((_, other)) => {
                return Err(UnescapeError::UnknownEscape {
                    offset: i,
                    ch: other,
                });
            }
        }
    }
    Ok(out)
}

fn read_hex_digits<I>(chars: &mut I, backslash_offset: usize, count: usize) -> Result<u32, UnescapeError>
where
    I: Iterator<Item = (usize, char)>,
{
    let mut value: u32 = 0;
    for k in 0..count {
        match chars.next() {
            None => {
                return Err(UnescapeError::InvalidUnicodeEscape {
                    offset: backslash_offset,
                    message: format!("expected {count} hex digits, got only {k}"),
                });
            }
            Some((_, h)) => {
                let digit = h.to_digit(16).ok_or_else(|| UnescapeError::InvalidUnicodeEscape {
                    offset: backslash_offset,
                    message: format!("'{h}' is not a hex digit"),
                })?;
                value = (value << 4) | digit;
            }
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_escapes() {
        assert_eq!(unescape_string(r"\n").unwrap(), "\n");
        assert_eq!(unescape_string(r"\t").unwrap(), "\t");
        assert_eq!(unescape_string(r"\\").unwrap(), "\\");
        assert_eq!(unescape_string(r#"\""#).unwrap(), "\"");
        assert_eq!(unescape_string(r"\'").unwrap(), "'");
    }

    #[test]
    fn unicode_4() {
        assert_eq!(unescape_string(r"\u0041").unwrap(), "A");
        assert_eq!(unescape_string(r"\u00E9").unwrap(), "é");
    }

    #[test]
    fn unicode_8() {
        // U+1F600 GRINNING FACE
        assert_eq!(unescape_string(r"\U0001F600").unwrap(), "\u{1F600}");
    }

    #[test]
    fn unknown_escape_error() {
        assert!(unescape_string(r"\x").is_err());
    }
}
