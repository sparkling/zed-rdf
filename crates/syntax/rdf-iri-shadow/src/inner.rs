//! Core RFC 3987 / RFC 3986 IRI implementation, compiled only under the
//! `shadow` feature.
//!
//! Design overview
//! ---------------
//!
//! 1. **Parse** — split a byte string into the five components defined in
//!    RFC 3986 §3: scheme, authority (host + optional userinfo/port), path,
//!    query, fragment.
//!
//! 2. **Normalise** — apply the following transformations in order, each
//!    derived directly from the RFC text (section citations in-line):
//!
//!    - Scheme case-folding to ASCII lowercase (RFC 3986 §6.2.2.1).
//!    - Host case-folding to ASCII lowercase (RFC 3986 §6.2.2.1).
//!    - Percent-encoding normalisation: decode unreserved-char octets;
//!      uppercase hex digits in remaining sequences (RFC 3986 §6.2.2).
//!    - Path dot-segment removal (RFC 3986 §5.2.4).
//!    - IRI-to-URI mapping: percent-encode non-ASCII bytes using UTF-8
//!      (RFC 3987 §3.1 step 2).
//!
//! 3. **`rdf_diff::Parser` impl** — treat the entire input bytes as a
//!    single IRI.  On success emit one canonical `Fact` whose subject,
//!    predicate, and object are derived from the normalised IRI string
//!    (single-IRI fact convention documented in the handoff notes).

use std::collections::BTreeMap;

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};
use thiserror::Error;

// ──────────────────────────────────────────────────────────────────────────────
// Public types
// ──────────────────────────────────────────────────────────────────────────────

/// A parsed and normalised IRI (RFC 3987).
///
/// All five components are stored in their *normalised* form.  The raw
/// input is not retained; callers needing the original should keep it
/// themselves.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Iri {
    /// Scheme component, lowercased (RFC 3986 §3.1).
    pub scheme: String,
    /// Authority component, if present.  Includes host (lowercased),
    /// optional userinfo, and optional port.
    pub authority: Option<String>,
    /// Path component after dot-segment removal.
    pub path: String,
    /// Query component, without leading `?`.
    pub query: Option<String>,
    /// Fragment component, without leading `#`.
    pub fragment: Option<String>,
}

impl Iri {
    /// Return the normalised IRI as a `String`.
    ///
    /// Reconstructs the five-component form per RFC 3986 §5.3.
    #[must_use]
    pub fn to_iri_string(&self) -> String {
        let mut out = String::with_capacity(128);
        out.push_str(&self.scheme);
        out.push(':');
        if let Some(auth) = &self.authority {
            out.push_str("//");
            out.push_str(auth);
        }
        out.push_str(&self.path);
        if let Some(q) = &self.query {
            out.push('?');
            out.push_str(q);
        }
        if let Some(f) = &self.fragment {
            out.push('#');
            out.push_str(f);
        }
        out
    }
}

/// Errors produced by the IRI parser.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IriError {
    /// Input is not valid UTF-8 and cannot be treated as an IRI.
    #[error("input is not valid UTF-8")]
    NotUtf8,

    /// No scheme component found; RFC 3987 §2.2 requires at least one.
    #[error("missing scheme (no ':' found after scheme start)")]
    MissingScheme,

    /// Scheme contains an illegal character (RFC 3986 §3.1:
    /// `ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`).
    #[error("invalid character in scheme: {0:?}")]
    InvalidSchemeChar(char),

    /// Percent-encoding sequence is malformed (e.g. `%GG`, `%2`, `%`).
    #[error("malformed percent-encoding at offset {offset}: {detail}")]
    BadPercentEncoding {
        /// Byte offset in the original string.
        offset: usize,
        /// Human-readable description.
        detail: String,
    },

    /// Pct-encoded byte sequence decodes to a value in the UTF-16
    /// surrogate range `U+D800..U+DFFF`. RFC 3987 Errata 3937 forbids
    /// this — the decoded scalar cannot represent a valid Unicode code
    /// point. See `docs/spec-readings/iri/lone-surrogate-rejection.md`
    /// (`IRI-SURROGATE-001`).
    #[error(
        "IRI-SURROGATE-001: pct-encoded byte sequence at offset {offset} decodes to a \
         UTF-16 surrogate (U+D800..U+DFFF); forbidden by RFC 3987 Errata 3937"
    )]
    SurrogatePctEncoding {
        /// Byte offset of the offending `%ED` triplet in the original
        /// string.
        offset: usize,
    },
}

// ──────────────────────────────────────────────────────────────────────────────
// Top-level public functions
// ──────────────────────────────────────────────────────────────────────────────

/// Parse a byte slice as an RFC 3987 IRI.
///
/// The bytes must be valid UTF-8 (RFC 3987 §1.5 mandates this for IRIs).
/// On success returns an [`Iri`] with components split but **not** yet
/// normalised — call [`normalise`] for the full canonical form.
///
/// # Errors
///
/// Returns [`IriError`] when:
/// - the bytes are not valid UTF-8,
/// - no scheme separator (`:`) is present,
/// - the scheme contains a character outside `ALPHA / DIGIT / "+" / "-" / "."`,
/// - a `%XX` sequence has non-hex digits.
pub fn parse(input: &[u8]) -> Result<Iri, IriError> {
    let s = std::str::from_utf8(input).map_err(|_| IriError::NotUtf8)?;
    parse_str(s)
}

/// Normalise an [`Iri`] into canonical form.
///
/// Applies, in order:
/// 1. Scheme lowercasing (RFC 3986 §6.2.2.1).
/// 2. Host lowercasing (RFC 3986 §6.2.2.1).
/// 3. Percent-encoding normalisation (decode unreserved, uppercase hex).
/// 4. Path dot-segment removal (RFC 3986 §5.2.4).
/// 5. IRI→URI mapping: percent-encode non-ASCII bytes (RFC 3987 §3.1).
///
/// # Errors
///
/// Returns [`IriError::BadPercentEncoding`] if a `%XX` sequence in the
/// authority or path uses non-hex digits (should have been caught by
/// [`parse`], but normalise is callable independently).
pub fn normalise(iri: Iri) -> Result<Iri, IriError> {
    // Step 1 — scheme already lowercased by parse_str; idempotent.
    let scheme = iri.scheme.to_ascii_lowercase();

    // Step 2 + 3 — authority: lowercase host, normalise pct-encoding.
    let authority = match iri.authority {
        None => None,
        Some(auth) => Some(normalise_authority(&auth)?),
    };

    // Step 3 — path: normalise pct-encoding, then dot-segment removal.
    let path_pct = normalise_pct_encoding(&iri.path)?;
    // Step 4 — dot segments.
    let path = remove_dot_segments(&path_pct);

    // Step 3 — query and fragment: normalise pct-encoding only.
    let query = iri.query.map(|q| normalise_pct_encoding(&q)).transpose()?;
    let fragment = iri
        .fragment
        .map(|f| normalise_pct_encoding(&f))
        .transpose()?;

    // Step 5 — IRI→URI: percent-encode remaining non-ASCII bytes.
    let scheme_uri = encode_non_ascii(&scheme);
    let authority_uri = authority.map(|a| encode_non_ascii(&a));
    let path_uri = encode_non_ascii(&path);
    let query_uri = query.map(|q| encode_non_ascii(&q));
    let fragment_uri = fragment.map(|f| encode_non_ascii(&f));

    Ok(Iri {
        scheme: scheme_uri,
        authority: authority_uri,
        path: path_uri,
        query: query_uri,
        fragment: fragment_uri,
    })
}

// ──────────────────────────────────────────────────────────────────────────────
// rdf_diff::Parser implementation
// ──────────────────────────────────────────────────────────────────────────────

/// Shadow IRI parser implementing the [`rdf_diff::Parser`] contract.
///
/// Treats the entire input byte slice as a single IRI.  On a successful
/// parse-and-normalise it emits one canonical `Fact`:
///
/// - `subject`   = normalised IRI string
/// - `predicate` = `http://www.w3.org/1999/02/22-rdf-syntax-ns#type`
/// - `object`    = `http://www.w3.org/2002/07/owl#Thing`
/// - `graph`     = `None`
///
/// This convention is minimal but sufficient for the diff harness to
/// compare two IRI normalisations: any difference in the `subject` field
/// surfaces as an `ObjectMismatch` when the main `rdf-iri` crate uses a
/// different canonical form.
#[derive(Debug, Default)]
pub struct ShadowIriParser;

impl Parser for ShadowIriParser {
    fn id(&self) -> &'static str {
        "rdf-iri-shadow"
    }

    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        // Strip leading/trailing ASCII whitespace for convenience.
        let trimmed = trim_ascii(input);

        let iri = parse(trimmed).map_err(|e| Diagnostics {
            messages: vec![format!("parse error: {e}")],
            fatal: true,
        })?;

        let normalised = normalise(iri).map_err(|e| Diagnostics {
            messages: vec![format!("normalise error: {e}")],
            fatal: true,
        })?;

        let iri_string = normalised.to_iri_string();

        let fact = Fact {
            subject: iri_string,
            predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            object: "http://www.w3.org/2002/07/owl#Thing".to_string(),
            graph: None,
        };

        let prov = FactProvenance {
            offset: Some(0),
            parser: self.id().to_string(),
        };

        let mut set = std::collections::BTreeMap::new();
        set.insert(fact, prov);

        let facts = Facts {
            set,
            prefixes: BTreeMap::new(),
        };

        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics {
                messages: vec![],
                fatal: false,
            },
        })
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────────────────────

/// Trim ASCII whitespace (SP, HT, CR, LF) from both ends of a byte slice.
fn trim_ascii(input: &[u8]) -> &[u8] {
    let start = input
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        .unwrap_or(input.len());
    let end = input
        .iter()
        .rposition(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        .map_or(0, |i| i + 1);
    if start >= end {
        &input[..0]
    } else {
        &input[start..end]
    }
}

/// Parse a `&str` into an [`Iri`] by splitting the five RFC 3986 §3
/// components.
///
/// Parsing is deliberately lenient on path/query/fragment content so that
/// normalisation can apply uniform percent-encoding rules afterwards.  The
/// only strict validation is on the scheme.
fn parse_str(s: &str) -> Result<Iri, IriError> {
    // RFC 3986 §3.1: scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    // Find the first ':' that can serve as the scheme/rest separator.
    let colon = s
        .char_indices()
        .find(|&(_, c)| c == ':')
        .map(|(i, _)| i)
        .ok_or(IriError::MissingScheme)?;

    let scheme_raw = &s[..colon];
    let rest = &s[colon + 1..];

    // Validate scheme characters.
    for (i, c) in scheme_raw.char_indices() {
        if i == 0 {
            if !c.is_ascii_alphabetic() {
                return Err(IriError::InvalidSchemeChar(c));
            }
        } else if !c.is_ascii_alphanumeric() && !matches!(c, '+' | '-' | '.') {
            return Err(IriError::InvalidSchemeChar(c));
        }
    }
    // Empty scheme is invalid.
    if scheme_raw.is_empty() {
        return Err(IriError::MissingScheme);
    }

    // Lowercase scheme (RFC 3986 §6.2.2.1).
    let scheme = scheme_raw.to_ascii_lowercase();

    // Split authority, path, query, fragment from `rest`.
    // RFC 3986 §3:
    //   hier-part = "//" authority path-abempty
    //             / path-absolute / path-rootless / path-empty
    let (authority, path_qf) = if let Some(after_slashes) = rest.strip_prefix("//") {
        // Authority ends at the first '/', '?', '#', or end of string.
        let auth_end = after_slashes
            .char_indices()
            .find(|&(_, c)| matches!(c, '/' | '?' | '#'))
            .map_or(after_slashes.len(), |(i, _)| i);
        let auth_raw = &after_slashes[..auth_end];
        let path_qf = &after_slashes[auth_end..];
        // Validate percent-encoding in authority eagerly.
        validate_pct_encoding(auth_raw)?;
        (Some(auth_raw.to_string()), path_qf)
    } else {
        (None, rest)
    };

    // Split path from query/fragment.
    let (path_raw, qf) = split_at_char(path_qf, |c| matches!(c, '?' | '#'));

    // Validate percent-encoding in path.
    validate_pct_encoding(path_raw)?;

    // Split query from fragment.
    let (query_raw, fragment_raw) = if let Some(qf_after) = qf.strip_prefix('?') {
        let (q, f) = split_at_char(qf_after, |c| c == '#');
        validate_pct_encoding(q)?;
        let frag = if let Some(fr) = f.strip_prefix('#') {
            validate_pct_encoding(fr)?;
            Some(fr.to_string())
        } else {
            None
        };
        (Some(q.to_string()), frag)
    } else if let Some(fr) = qf.strip_prefix('#') {
        validate_pct_encoding(fr)?;
        (None, Some(fr.to_string()))
    } else {
        (None, None)
    };

    Ok(Iri {
        scheme,
        authority,
        path: path_raw.to_string(),
        query: query_raw,
        fragment: fragment_raw,
    })
}

/// Split `s` at the first character matching `pred`, returning
/// `(before, from_match_inclusive)`.
fn split_at_char<F>(s: &str, pred: F) -> (&str, &str)
where
    F: Fn(char) -> bool,
{
    match s.char_indices().find(|&(_, c)| pred(c)) {
        Some((i, _)) => (&s[..i], &s[i..]),
        None => (s, ""),
    }
}

/// Validate that every `%XX` sequence in `s` uses two valid hex digits,
/// and that no triplet pair decodes into the UTF-16 surrogate range
/// `U+D800..U+DFFF`.
///
/// A pct-encoded UTF-8 surrogate has the shape `%ED %Ax-%Bx %8x-%Bx`:
/// RFC 3987 Errata 3937 forbids this sequence in IRI references because
/// surrogate scalars are not valid Unicode code points. Detected via
/// `IRI-SURROGATE-001` (see `docs/spec-readings/iri/lone-surrogate-rejection.md`).
/// Returns at the first violation.
fn validate_pct_encoding(s: &str) -> Result<(), IriError> {
    let bytes = s.as_bytes();
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if bytes[cursor] != b'%' {
            cursor += 1;
            continue;
        }
        // Must have two following hex digits.
        if cursor + 2 >= bytes.len() {
            return Err(IriError::BadPercentEncoding {
                offset: cursor,
                detail: "percent sign at end of string or too close to end".to_string(),
            });
        }
        let hi = bytes[cursor + 1];
        let lo = bytes[cursor + 2];
        if !is_hex_digit(hi) || !is_hex_digit(lo) {
            return Err(IriError::BadPercentEncoding {
                offset: cursor,
                detail: format!(
                    "non-hex digits after '%': '{}''{}'",
                    hi as char, lo as char
                ),
            });
        }
        let decoded = hex_to_byte(hi, lo);
        // IRI-SURROGATE-001: pct-encoded UTF-8 surrogate starts with
        // `%ED` and is followed by a triplet whose decoded byte is in
        // 0xA0..=0xBF. We look one triplet ahead; if the next three
        // bytes are a well-formed %HH, decode and check.
        if decoded == 0xED {
            let next_start = cursor + 3;
            if next_start + 2 < bytes.len()
                && bytes[next_start] == b'%'
                && is_hex_digit(bytes[next_start + 1])
                && is_hex_digit(bytes[next_start + 2])
            {
                let nxt = hex_to_byte(bytes[next_start + 1], bytes[next_start + 2]);
                if (0xA0..=0xBF).contains(&nxt) {
                    return Err(IriError::SurrogatePctEncoding { offset: cursor });
                }
            }
        }
        cursor += 3;
    }
    Ok(())
}

/// `true` if `b` is an ASCII hexadecimal digit.
const fn is_hex_digit(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

/// `true` if `b` is an RFC 3986 §2.3 *unreserved* character:
/// `ALPHA / DIGIT / "-" / "." / "_" / "~"`.
const fn is_unreserved(b: u8) -> bool {
    matches!(b,
        b'A'..=b'Z'
        | b'a'..=b'z'
        | b'0'..=b'9'
        | b'-' | b'.' | b'_' | b'~'
    )
}

/// Parse two ASCII hex digits into a `u8`.
///
/// Caller must ensure `hi` and `lo` are both valid hex digits.
const fn hex_to_byte(hi: u8, lo: u8) -> u8 {
    let h = if hi.is_ascii_digit() {
        hi - b'0'
    } else if hi.is_ascii_uppercase() {
        hi - b'A' + 10
    } else {
        hi - b'a' + 10
    };
    let l = if lo.is_ascii_digit() {
        lo - b'0'
    } else if lo.is_ascii_uppercase() {
        lo - b'A' + 10
    } else {
        lo - b'a' + 10
    };
    (h << 4) | l
}

/// Format a byte as two uppercase hex digits into `out`.
fn push_hex_upper(out: &mut String, b: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push(HEX[(b >> 4) as usize] as char);
    out.push(HEX[(b & 0x0F) as usize] as char);
}

/// Normalise percent-encoding in a string component:
/// - Decode `%XX` where `XX` encodes an RFC 3986 §2.3 unreserved char.
/// - Uppercase hex digits in all remaining `%XX` sequences.
///
/// The input must already have passed [`validate_pct_encoding`].
///
/// Non-`%` characters (including multi-byte UTF-8 sequences) are
/// preserved verbatim in the output.
fn normalise_pct_encoding(s: &str) -> Result<String, IriError> {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut byte_pos = 0;
    // `char_indices` gives us (byte_offset, char) pairs; we consume the
    // char iterator for non-'%' code points so multi-byte UTF-8 sequences
    // are preserved without corruption.
    let mut chars = s.char_indices().peekable();

    while byte_pos < bytes.len() {
        if bytes[byte_pos] == b'%' {
            // Bounds already guaranteed by validate_pct_encoding, but we
            // re-check to produce a good error if called independently.
            if byte_pos + 2 >= bytes.len() {
                return Err(IriError::BadPercentEncoding {
                    offset: byte_pos,
                    detail: "truncated percent sequence".to_string(),
                });
            }
            let h1 = bytes[byte_pos + 1];
            let h2 = bytes[byte_pos + 2];
            if !is_hex_digit(h1) || !is_hex_digit(h2) {
                return Err(IriError::BadPercentEncoding {
                    offset: byte_pos,
                    detail: format!("non-hex: '{}''{}'", h1 as char, h2 as char),
                });
            }
            let decoded = hex_to_byte(h1, h2);
            if is_unreserved(decoded) {
                // RFC 3986 §6.2.2.2: decode unreserved characters.
                // These are all ASCII so the cast is safe.
                out.push(decoded as char);
            } else {
                // RFC 3986 §6.2.2.1: uppercase the hex digits.
                out.push('%');
                push_hex_upper(&mut out, decoded);
            }
            byte_pos += 3;
            // Advance the char iterator past the three ASCII bytes we just
            // consumed (`%`, hex1, hex2).
            while chars
                .peek()
                .is_some_and(|&(off, _)| off < byte_pos)
            {
                chars.next();
            }
        } else {
            // Non-'%' code point: take the next char from the iterator (which
            // correctly handles multi-byte UTF-8) and advance byte_pos by its
            // encoded length.
            if let Some((_, ch)) = chars.next() {
                let ch_len = ch.len_utf8();
                out.push(ch);
                byte_pos += ch_len;
            } else {
                // Should not happen for valid UTF-8 strings.
                byte_pos += 1;
            }
        }
    }
    Ok(out)
}

/// Normalise the authority component:
/// 1. Separate userinfo (`user:pass@`) from host+port.
/// 2. Lowercase the host (RFC 3986 §6.2.2.1).
/// 3. Normalise percent-encoding in userinfo and host separately.
/// 4. Strip the default port for well-known schemes (not applicable here
///    because the authority is normalised without scheme context; callers
///    that want port-stripping should do so after full normalisation).
///
/// Pin: RFC 3987 §3.1 step 3 says to map the host using the `ToASCII`
/// operation defined in RFC 3490 (IDNA). We apply ASCII lowercasing only,
/// which is the intersection safe for non-DNS contexts, and document that
/// full IDNA is *not* implemented. See `docs/spec-readings/iri/idna-pin.md`.
fn normalise_authority(auth: &str) -> Result<String, IriError> {
    // Split userinfo from host[:port].
    let (userinfo, hostport) = auth
        .rfind('@')
        .map_or((None, auth), |at| (Some(&auth[..at]), &auth[at + 1..]));

    // Normalise percent-encoding in userinfo.
    let userinfo_norm = userinfo.map(normalise_pct_encoding).transpose()?;

    // Split host from optional port.
    // RFC 3986 §3.2.2: IP-literal = "[" *( IPvFuture / IPv6 ) "]"
    let (host_raw, port) = if hostport.starts_with('[') {
        // IPv6 / IP-literal: everything up to and including ']'.
        hostport
            .find(']')
            .map_or((hostport, None), |end| {
                let ip_lit = &hostport[..=end];
                let rest = &hostport[end + 1..];
                let port = rest.strip_prefix(':').map(str::to_string);
                (ip_lit, port)
            })
    } else {
        hostport.rfind(':').map_or((hostport, None), |colon| {
            let h = &hostport[..colon];
            let p = &hostport[colon + 1..];
            // Only treat as port if it is all digits (RFC 3986 §3.2.3).
            if p.bytes().all(|b| b.is_ascii_digit()) {
                (h, Some(p.to_string()))
            } else {
                (hostport, None)
            }
        })
    };

    // Percent-normalise host, then lowercase.
    let host_pct = normalise_pct_encoding(host_raw)?;
    let host_lower = host_pct.to_ascii_lowercase();

    // Reconstruct authority.
    let mut out = String::with_capacity(auth.len());
    if let Some(ui) = userinfo_norm {
        out.push_str(&ui);
        out.push('@');
    }
    out.push_str(&host_lower);
    if let Some(p) = port {
        out.push(':');
        out.push_str(&p);
    }
    Ok(out)
}

/// Remove dot segments from a path per RFC 3986 §5.2.4.
///
/// Implements the algorithm directly: split on `/`, push non-dot
/// components onto a stack, pop for `..`, ignore `.`.  Preserves whether
/// the original path was absolute (leading `/`) and whether it had a
/// trailing `/` (trailing empty component).
///
/// RFC 3986 §5.2.4 specifies the algorithm for both absolute and
/// relative paths; we call this only on path components so the
/// relative-path rules (strip leading `../` / `./`) are included for
/// completeness.
fn remove_dot_segments(path: &str) -> String {
    let is_absolute = path.starts_with('/');
    // A trailing slash means the last component was empty; we preserve it.
    let trailing_slash = path.len() > 1 && path.ends_with('/');

    // Split into components.  For an absolute path "/a/b/c" this gives
    // ["", "a", "b", "c"]; we skip the leading empty string from the split.
    let mut stack: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            // RFC 3986 §5.2.4 rule C: `..` pops the last component.
            ".." => {
                stack.pop();
            }
            // RFC 3986 §5.2.4 rule A: discard lone `.` segments.
            // Empty strings arise from leading/double `/`; skip them too.
            "." | "" => {}
            // Normal segment.
            other => {
                stack.push(other);
            }
        }
    }

    // Reconstruct.
    let mut out = String::with_capacity(path.len());
    if is_absolute {
        out.push('/');
    }
    let joined = stack.join("/");
    out.push_str(&joined);
    if trailing_slash && !out.ends_with('/') {
        out.push('/');
    }
    out
}

/// Percent-encode any non-ASCII byte in `s` using the UTF-8 encoding
/// per RFC 3987 §3.1 step 2.
///
/// ASCII bytes are passed through unchanged; each non-ASCII byte is
/// replaced by `%XX` with uppercase hex.
fn encode_non_ascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        if *b > 0x7F {
            out.push('%');
            push_hex_upper(&mut out, *b);
        } else {
            out.push(*b as char);
        }
    }
    out
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── scheme case-folding ────────────────────────────────────────────────

    #[test]
    fn scheme_is_lowercased() {
        let iri = parse(b"HTTP://example.org/path").unwrap();
        assert_eq!(iri.scheme, "http");
    }

    #[test]
    fn scheme_mixed_case_lowercased() {
        let iri = parse(b"FtP://ftp.example.org/file").unwrap();
        assert_eq!(iri.scheme, "ftp");
    }

    // ── authority / host lowercasing ───────────────────────────────────────

    #[test]
    fn host_is_lowercased_after_normalise() {
        let iri = parse(b"http://EXAMPLE.ORG/").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.authority.as_deref(), Some("example.org"));
    }

    #[test]
    fn host_mixed_case_normalised() {
        let iri = parse(b"http://ExAmPlE.CoM/path").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.authority.as_deref(), Some("example.com"));
    }

    // ── percent-encoding normalisation ─────────────────────────────────────

    /// RFC 3986 §6.2.2.2: %41 is 'A', unreserved — must be decoded.
    #[test]
    fn pct_unreserved_decoded() {
        let iri = parse(b"http://example.org/%41BC").unwrap();
        let n = normalise(iri).unwrap();
        // 'A' is unreserved, so %41 -> 'A'
        assert_eq!(n.path, "/ABC");
    }

    /// %7E is '~', which is unreserved — must be decoded.
    #[test]
    fn pct_tilde_unreserved_decoded() {
        let iri = parse(b"http://example.org/%7Euser").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.path, "/~user");
    }

    /// %2F is '/', which is reserved — hex digits uppercased, NOT decoded.
    #[test]
    fn pct_reserved_kept_uppercase() {
        let iri = parse(b"http://example.org/seg%2fpath").unwrap();
        let n = normalise(iri).unwrap();
        // '/' is reserved; kept as %2F (uppercased).
        assert_eq!(n.path, "/seg%2Fpath");
    }

    /// %20 is SP, reserved — must remain encoded, hex uppercased.
    #[test]
    fn pct_space_kept_uppercase() {
        let iri = parse(b"http://example.org/hello%20world").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.path, "/hello%20world");
    }

    /// Lowercase hex in existing pct-encoding gets uppercased.
    #[test]
    fn pct_lowercase_hex_uppercased() {
        let iri = parse(b"http://example.org/seg%2fpath").unwrap();
        let n = normalise(iri).unwrap();
        assert!(n.path.contains("%2F"), "expected %2F, got {}", n.path);
    }

    // ── path dot-segment removal ───────────────────────────────────────────

    #[test]
    fn dot_segment_single_dot_removed() {
        let iri = parse(b"http://example.org/a/./b").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.path, "/a/b");
    }

    #[test]
    fn dot_segment_double_dot_goes_up() {
        let iri = parse(b"http://example.org/a/b/../c").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.path, "/a/c");
    }

    #[test]
    fn dot_segment_multiple() {
        let iri = parse(b"http://example.org/a/b/c/../../d").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.path, "/a/d");
    }

    #[test]
    fn dot_segment_leading_double_dot() {
        // RFC 3986 §5.2.4 rule A: leading "../" is stripped.
        let iri = parse(b"http://example.org/../a/b").unwrap();
        let n = normalise(iri).unwrap();
        // Leading "/../" at root pops nothing but the input prefix is consumed.
        // Expected: /a/b
        assert_eq!(n.path, "/a/b");
    }

    // ── IRI → URI mapping ─────────────────────────────────────────────────

    /// Non-ASCII chars in the path must be percent-encoded in the URI form.
    #[test]
    fn non_ascii_encoded_in_uri() {
        // U+00E9 LATIN SMALL LETTER E WITH ACUTE encodes as %C3%A9 in UTF-8.
        let input = "http://example.org/caf\u{00E9}";
        let iri = parse(input.as_bytes()).unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.path, "/caf%C3%A9");
    }

    /// ASCII-only IRI should pass through the IRI→URI step unchanged.
    #[test]
    fn ascii_iri_unchanged_by_uri_mapping() {
        let iri = parse(b"http://example.org/hello/world").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.path, "/hello/world");
    }

    // ── query and fragment ────────────────────────────────────────────────

    #[test]
    fn query_is_preserved() {
        let iri = parse(b"http://example.org/path?q=1").unwrap();
        assert_eq!(iri.query.as_deref(), Some("q=1"));
    }

    #[test]
    fn fragment_is_preserved() {
        let iri = parse(b"http://example.org/path#sec1").unwrap();
        assert_eq!(iri.fragment.as_deref(), Some("sec1"));
    }

    #[test]
    fn query_and_fragment_preserved() {
        let iri = parse(b"http://example.org/path?q=1#frag").unwrap();
        assert_eq!(iri.query.as_deref(), Some("q=1"));
        assert_eq!(iri.fragment.as_deref(), Some("frag"));
    }

    // ── parse errors ──────────────────────────────────────────────────────

    #[test]
    fn missing_scheme_errors() {
        assert_eq!(parse(b"no-scheme-here"), Err(IriError::MissingScheme));
    }

    #[test]
    fn not_utf8_errors() {
        assert_eq!(parse(b"\xFF\xFE"), Err(IriError::NotUtf8));
    }

    #[test]
    fn invalid_scheme_char_errors() {
        // Scheme must start with ALPHA (RFC 3986 §3.1); '1' is not ALPHA.
        let r = parse(b"1nvalid://host/");
        assert!(matches!(r, Err(IriError::InvalidSchemeChar('1'))));
    }

    #[test]
    fn bad_pct_encoding_errors() {
        let r = parse(b"http://example.org/%GG");
        assert!(matches!(r, Err(IriError::BadPercentEncoding { .. })));
    }

    // ── IRI-SURROGATE-001 (RFC 3987 Errata 3937) ──────────────────────────
    // pct-encoded byte sequences that decode to the UTF-16 surrogate
    // range (U+D800..U+DFFF) must be rejected as fatal. The boundary
    // case U+D7FF (encoded as %ED%9F%BF) is the last scalar before the
    // range and must still be accepted.
    // See docs/spec-readings/iri/lone-surrogate-rejection.md.

    #[test]
    fn surrogate_rejects_lone_high() {
        // U+D800 UTF-8 = ED A0 80
        let r = parse(b"http://example.org/%ED%A0%80");
        assert!(
            matches!(r, Err(IriError::SurrogatePctEncoding { .. })),
            "expected SurrogatePctEncoding, got {r:?}",
        );
    }

    #[test]
    fn surrogate_rejects_lone_low() {
        // U+DC00 UTF-8 = ED B0 80
        let r = parse(b"http://example.org/%ED%B0%80");
        assert!(matches!(r, Err(IriError::SurrogatePctEncoding { .. })));
    }

    #[test]
    fn surrogate_rejects_encoded_pair() {
        // Each half of a pct-encoded surrogate pair is independently
        // invalid; reject at the first one.
        let r = parse(b"http://example.org/%ED%A0%80%ED%B0%80");
        assert!(matches!(r, Err(IriError::SurrogatePctEncoding { .. })));
    }

    #[test]
    fn surrogate_accepts_just_below_range() {
        // U+D7FF UTF-8 = ED 9F BF; the second byte (9F) is outside
        // the surrogate window (A0..=BF) so the sequence is accepted.
        let r = parse(b"http://example.org/%ED%9F%BF").unwrap();
        assert_eq!(r.path, "/%ED%9F%BF");
    }

    #[test]
    fn surrogate_rejected_in_query_and_fragment() {
        assert!(matches!(
            parse(b"http://example.org/?q=%ED%A0%80"),
            Err(IriError::SurrogatePctEncoding { .. }),
        ));
        assert!(matches!(
            parse(b"http://example.org/#%ED%A0%80"),
            Err(IriError::SurrogatePctEncoding { .. }),
        ));
    }

    // ── authority parsing ─────────────────────────────────────────────────

    #[test]
    fn authority_no_port() {
        let iri = parse(b"http://example.org/").unwrap();
        assert_eq!(iri.authority.as_deref(), Some("example.org"));
    }

    #[test]
    fn authority_with_port() {
        let iri = parse(b"http://example.org:8080/").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.authority.as_deref(), Some("example.org:8080"));
    }

    #[test]
    fn authority_with_userinfo() {
        let iri = parse(b"ftp://user:pass@ftp.example.org/").unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.authority.as_deref(), Some("user:pass@ftp.example.org"));
    }

    #[test]
    fn urn_no_authority() {
        let iri = parse(b"urn:isbn:0451450523").unwrap();
        assert!(iri.authority.is_none());
        assert_eq!(iri.path, "isbn:0451450523");
    }

    // ── to_iri_string roundtrip ────────────────────────────────────────────

    #[test]
    fn to_iri_string_roundtrip() {
        let input = "http://example.org/path?q=1#frag";
        let iri = parse(input.as_bytes()).unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.to_iri_string(), input);
    }

    #[test]
    fn to_iri_string_no_authority() {
        let input = "urn:example:a123";
        let iri = parse(input.as_bytes()).unwrap();
        let n = normalise(iri).unwrap();
        assert_eq!(n.to_iri_string(), input);
    }

    // ── rdf_diff::Parser impl ─────────────────────────────────────────────

    #[test]
    fn parser_id_is_correct() {
        let p = ShadowIriParser;
        assert_eq!(p.id(), "rdf-iri-shadow");
    }

    #[test]
    fn parser_emits_one_fact_for_valid_iri() {
        let p = ShadowIriParser;
        let outcome = p.parse(b"http://example.org/resource").unwrap();
        assert_eq!(outcome.facts.set.len(), 1);
        let fact = outcome.facts.set.keys().next().unwrap();
        assert_eq!(fact.subject, "http://example.org/resource");
        assert_eq!(
            fact.predicate,
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
        );
        assert_eq!(fact.object, "http://www.w3.org/2002/07/owl#Thing");
        assert!(fact.graph.is_none());
    }

    #[test]
    fn parser_returns_diagnostics_for_invalid_input() {
        let p = ShadowIriParser;
        let result = p.parse(b"not-an-iri");
        assert!(result.is_err());
        let diag = result.unwrap_err();
        assert!(diag.fatal);
    }

    /// Whitespace trimming: leading/trailing whitespace around a valid IRI
    /// should not cause a parse failure.
    #[test]
    fn parser_trims_whitespace() {
        let p = ShadowIriParser;
        let outcome = p.parse(b"  http://example.org/resource  ").unwrap();
        assert_eq!(outcome.facts.set.len(), 1);
    }

    // ── normalise idempotency ─────────────────────────────────────────────

    /// Normalising an already-normalised IRI must be a no-op.
    #[test]
    fn normalise_idempotent() {
        let raw = "http://example.org/path/to/resource?key=val#frag";
        let iri = parse(raw.as_bytes()).unwrap();
        let once = normalise(iri).unwrap();
        let once_str = once.to_iri_string();

        let iri2 = parse(once_str.as_bytes()).unwrap();
        let twice = normalise(iri2).unwrap();
        let twice_str = twice.to_iri_string();

        assert_eq!(once_str, twice_str);
    }
}
