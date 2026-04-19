//! Unicode escape decoding for N-Triples / N-Quads string literals and IRIs.
//!
//! Handles `\uXXXX` (4 hex digits) and `\UXXXXXXXX` (8 hex digits) sequences
//! as defined in the W3C N-Triples recommendation §3 "ECHAR and UCHAR".
//!
//! Also handles the standard single-character escapes defined in ECHAR:
//! `\t`, `\b`, `\n`, `\r`, `\f`, `\"`, `\'`, `\\`.

/// Decode all recognised escape sequences in `raw` into `out`.
///
/// Returns `Err(message)` on any malformed escape.
pub fn decode_string_escapes(raw: &str, out: &mut String) -> Result<(), String> {
    let mut chars = raw.char_indices();
    while let Some((_, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            None => return Err("trailing backslash in string literal".into()),
            Some((_, 't')) => out.push('\t'),
            Some((_, 'b')) => out.push('\x08'),
            Some((_, 'n')) => out.push('\n'),
            Some((_, 'r')) => out.push('\r'),
            Some((_, 'f')) => out.push('\x0C'),
            Some((_, '"')) => out.push('"'),
            Some((_, '\'')) => out.push('\''),
            Some((_, '\\')) => out.push('\\'),
            Some((_, 'u')) => {
                let scalar = read_hex_escape(&mut chars, 4, "\\uXXXX")?;
                out.push(scalar);
            }
            Some((_, 'U')) => {
                let scalar = read_hex_escape(&mut chars, 8, "\\UXXXXXXXX")?;
                out.push(scalar);
            }
            Some((_, bad)) => {
                return Err(format!("unrecognised escape sequence '\\{bad}'"));
            }
        }
    }
    Ok(())
}

/// Decode only `\uXXXX` / `\UXXXXXXXX` escapes inside an IRI.
///
/// IRIs permit only these two escape forms; single-character escapes are
/// not valid inside `<…>`.
pub fn decode_iri_escapes(raw: &str, out: &mut String) -> Result<(), String> {
    let mut chars = raw.char_indices();
    while let Some((_, ch)) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            None => return Err("trailing backslash in IRI".into()),
            Some((_, 'u')) => {
                let scalar = read_hex_escape(&mut chars, 4, "\\uXXXX")?;
                out.push(scalar);
            }
            Some((_, 'U')) => {
                let scalar = read_hex_escape(&mut chars, 8, "\\UXXXXXXXX")?;
                out.push(scalar);
            }
            Some((_, bad)) => {
                return Err(format!("invalid escape '\\{bad}' inside IRI"));
            }
        }
    }
    Ok(())
}

/// Read exactly `n` hex digits from `chars` and return the decoded `char`.
fn read_hex_escape<I>(chars: &mut I, n: usize, label: &str) -> Result<char, String>
where
    I: Iterator<Item = (usize, char)>,
{
    let mut value: u32 = 0;
    for i in 0..n {
        match chars.next() {
            None => {
                return Err(format!(
                    "incomplete {label} escape: only {i} hex digit(s) found"
                ));
            }
            Some((_, d)) => {
                let digit = d
                    .to_digit(16)
                    .ok_or_else(|| format!("non-hex digit '{d}' in {label} escape"))?;
                value = (value << 4) | digit;
            }
        }
    }
    char::from_u32(value)
        .ok_or_else(|| format!("U+{value:X} is not a valid Unicode scalar value (in {label})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unescape(s: &str) -> String {
        let mut out = String::new();
        decode_string_escapes(s, &mut out).expect("should decode");
        out
    }

    fn unescape_iri(s: &str) -> String {
        let mut out = String::new();
        decode_iri_escapes(s, &mut out).expect("should decode");
        out
    }

    #[test]
    fn basic_escapes() {
        assert_eq!(unescape(r"\t"), "\t");
        assert_eq!(unescape(r"\n"), "\n");
        assert_eq!(unescape(r"\r"), "\r");
        assert_eq!(unescape(r#"\""#), "\"");
        assert_eq!(unescape(r"\\"), "\\");
    }

    #[test]
    fn small_u_escape() {
        // \u0041 = 'A'
        assert_eq!(unescape(r"\u0041"), "A");
        // \u00E9 = 'é'
        assert_eq!(unescape(r"\u00E9"), "\u{00E9}");
    }

    #[test]
    fn large_u_escape() {
        // \U0001F600 = emoji 😀
        assert_eq!(unescape(r"\U0001F600"), "\u{1F600}");
    }

    #[test]
    fn iri_unicode_escape() {
        assert_eq!(unescape_iri(r"\u0041"), "A");
        assert_eq!(unescape_iri(r"\U0001F600"), "\u{1F600}");
    }

    #[test]
    fn iri_rejects_string_escapes() {
        let mut out = String::new();
        assert!(decode_iri_escapes(r"\n", &mut out).is_err());
    }

    #[test]
    fn invalid_scalar() {
        let mut out = String::new();
        // U+D800 is a surrogate — not a valid scalar
        assert!(decode_string_escapes(r"\uD800", &mut out).is_err());
    }
}
