//! RFC 3987 IRI-reference parser.
//!
//! Strategy: split on the ABNF structural separators (`:` for scheme,
//! `//` for authority, `?` for query, `#` for fragment), then validate
//! each component's character set. We deliberately do **not** build a
//! character-by-character state machine; RFC 3987 is unambiguous enough
//! that split+validate suffices and is easier to audit.
//!
//! Character classes come straight from RFC 3986 §A + RFC 3987 §2.2.

use crate::{Components, Diagnostic, DiagnosticCode, Iri};

/// Entry point for [`Iri::parse`].
pub fn parse(input: &str) -> Result<Iri, Diagnostic> {
    // Empty string is a valid relative reference (RFC 3986 §4.2).
    if input.is_empty() {
        return Ok(Iri::from_raw(
            String::new(),
            Components {
                path: (0, 0),
                ..Components::default()
            },
        ));
    }

    let mut parts = Components::default();
    let bytes = input.as_bytes();

    // 1. Scheme (optional). RFC 3986 §3.1:
    //      scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
    //    Must be followed by ':'.
    let mut cursor = scan_scheme(bytes).map_or(0, |scheme_end| {
        parts.scheme = Some((0, scheme_end));
        scheme_end + 1 // skip ':'
    });

    // 2. Hier-part / relative-part.
    //    If the next two bytes are "//", we have an authority.
    if bytes.get(cursor..cursor + 2) == Some(b"//") {
        let auth_start = cursor + 2;
        // Authority ends at first '/', '?', '#', or end of input.
        let auth_end = bytes[auth_start..]
            .iter()
            .position(|&b| matches!(b, b'/' | b'?' | b'#'))
            .map_or(bytes.len(), |p| auth_start + p);
        parts.authority = Some((auth_start, auth_end));
        split_authority(input, auth_start, auth_end, &mut parts)?;
        cursor = auth_end;
    }

    // 3. Path. Spans from cursor up to first '?' or '#'.
    let path_end = bytes[cursor..]
        .iter()
        .position(|&b| matches!(b, b'?' | b'#'))
        .map_or(bytes.len(), |p| cursor + p);
    parts.path = (cursor, path_end);
    cursor = path_end;

    // 4. Query (optional).
    if bytes.get(cursor) == Some(&b'?') {
        let q_start = cursor + 1;
        let q_end = bytes[q_start..]
            .iter()
            .position(|&b| b == b'#')
            .map_or(bytes.len(), |p| q_start + p);
        parts.query = Some((q_start, q_end));
        cursor = q_end;
    }

    // 5. Fragment (optional).
    if bytes.get(cursor) == Some(&b'#') {
        let f_start = cursor + 1;
        let f_end = bytes.len();
        parts.fragment = Some((f_start, f_end));
    }

    // Character-class validation for each component.
    validate_scheme(input, parts.scheme)?;
    validate_userinfo(input, parts.userinfo)?;
    validate_host(input, parts.host)?;
    validate_port(input, parts.port)?;
    validate_path(
        input,
        parts.path,
        parts.authority.is_some(),
        parts.scheme.is_some(),
    )?;
    validate_query_or_fragment(input, parts.query, "query")?;
    validate_query_or_fragment(input, parts.fragment, "fragment")?;

    Ok(Iri::from_raw(input.to_owned(), parts))
}

fn scan_scheme(bytes: &[u8]) -> Option<usize> {
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            return if i == 0 { None } else { Some(i) };
        }
        let ok = b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.');
        if !ok {
            return None;
        }
    }
    None
}

fn split_authority(
    input: &str,
    start: usize,
    end: usize,
    parts: &mut Components,
) -> Result<(), Diagnostic> {
    let slice = &input.as_bytes()[start..end];

    // Userinfo: span up to first '@' (if any).
    let at_rel = slice.iter().position(|&b| b == b'@');
    let host_start_rel = at_rel.map_or(0, |p| p + 1);
    if let Some(at) = at_rel {
        parts.userinfo = Some((start, start + at));
    }

    // Port: RFC 3986 §3.2.3. The `:` that separates host from port is
    // unambiguous for reg-name and IPv4 but **ambiguous** for IPv6
    // (which contains `:`). RFC 3986 brackets IPv6 literals in `[..]`,
    // so: if the host starts with `[`, find the matching `]` and look
    // for `:` after that; otherwise, use the last `:`.
    let host_rel_slice = &slice[host_start_rel..];
    let (host_rel_end, port_rel_start) = if host_rel_slice.first() == Some(&b'[') {
        let close = host_rel_slice.iter().position(|&b| b == b']').ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::Authority,
                "IP-literal is missing closing ']'",
                Some(start + host_start_rel),
            )
        })?;
        // Port (if any) must follow ']' immediately as ":".
        match host_rel_slice.get(close + 1) {
            Some(&b':') => (close + 1, Some(close + 2)),
            Some(_) => {
                return Err(Diagnostic::new(
                    DiagnosticCode::Authority,
                    "unexpected character after IP-literal ']'",
                    Some(start + host_start_rel + close + 1),
                ));
            }
            None => (close + 1, None),
        }
    } else {
        host_rel_slice
            .iter()
            .rposition(|&b| b == b':')
            .map_or((host_rel_slice.len(), None), |c| (c, Some(c + 1)))
    };

    let host_abs_start = start + host_start_rel;
    let host_abs_end = host_abs_start + host_rel_end;
    parts.host = Some((host_abs_start, host_abs_end));

    if let Some(p_rel) = port_rel_start {
        let p_abs_start = start + host_start_rel + p_rel;
        parts.port = Some((p_abs_start, end));
    }

    Ok(())
}

// -----------------------------------------------------------------------
// Character-class validation. Kept in small focused helpers.
// -----------------------------------------------------------------------

fn validate_scheme(input: &str, range: Option<(usize, usize)>) -> Result<(), Diagnostic> {
    let Some((a, b)) = range else { return Ok(()) };
    let slice = &input[a..b];
    let mut chars = slice.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::Scheme,
                format!("scheme must start with ALPHA, got {slice:?}"),
                Some(a),
            ));
        }
    }
    for (i, c) in chars.enumerate() {
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
            return Err(Diagnostic::new(
                DiagnosticCode::Scheme,
                format!("scheme contains invalid character {c:?}"),
                Some(a + 1 + i),
            ));
        }
    }
    Ok(())
}

fn validate_userinfo(input: &str, range: Option<(usize, usize)>) -> Result<(), Diagnostic> {
    let Some((a, b)) = range else { return Ok(()) };
    // iuserinfo = *( iunreserved / pct-encoded / sub-delims / ":" )
    validate_run(input, a, b, |c| is_iunreserved(c) || is_sub_delim(c) || c == ':')
}

fn validate_host(input: &str, range: Option<(usize, usize)>) -> Result<(), Diagnostic> {
    let Some((a, b)) = range else { return Ok(()) };
    let slice = &input[a..b];
    if slice.starts_with('[') {
        // IP-literal: RFC 3986 §3.2.2. We accept the bracketed body
        // opaquely (any iunreserved/sub-delim/":" plus hex digits);
        // a full parse of IPv6 literals is out of scope for Phase A.
        if !slice.ends_with(']') {
            return Err(Diagnostic::new(
                DiagnosticCode::Authority,
                "IP-literal must be bracketed",
                Some(a),
            ));
        }
        return validate_run(input, a + 1, b - 1, |c| {
            c.is_ascii_hexdigit()
                || matches!(c, ':' | '.' | 'v' | 'V')
                || is_iunreserved(c)
                || is_sub_delim(c)
        });
    }
    // ireg-name = *( iunreserved / pct-encoded / sub-delims )
    validate_run(input, a, b, |c| is_iunreserved(c) || is_sub_delim(c))
}

fn validate_port(input: &str, range: Option<(usize, usize)>) -> Result<(), Diagnostic> {
    let Some((a, b)) = range else { return Ok(()) };
    for (i, c) in input[a..b].char_indices() {
        if !c.is_ascii_digit() {
            return Err(Diagnostic::new(
                DiagnosticCode::Port,
                format!("port must be DIGIT, got {c:?}"),
                Some(a + i),
            ));
        }
    }
    Ok(())
}

fn validate_path(
    input: &str,
    (a, b): (usize, usize),
    has_authority: bool,
    has_scheme: bool,
) -> Result<(), Diagnostic> {
    if a == b {
        return Ok(());
    }
    // RFC 3986 §3.3: if authority is present, path-abempty begins with
    // '/' or is empty. If authority is absent, a path may NOT begin
    // with "//".
    let slice = &input[a..b];
    if has_authority && !slice.starts_with('/') {
        return Err(Diagnostic::new(
            DiagnosticCode::Syntax,
            "path must start with '/' when authority is present",
            Some(a),
        ));
    }
    if !has_authority && slice.starts_with("//") {
        return Err(Diagnostic::new(
            DiagnosticCode::Syntax,
            "path must not start with '//' when authority is absent",
            Some(a),
        ));
    }
    // ipath-* = *( ipchar / "/" ). ipchar = iunreserved / pct-encoded /
    //            sub-delims / ":" / "@".
    validate_run(input, a, b, |c| {
        is_iunreserved(c) || is_sub_delim(c) || matches!(c, ':' | '@' | '/')
    })?;

    // RFC 3986 §4.2: a **relative-reference** (no scheme) with a
    // relative-path (does not start with '/') must not have a ':' in
    // its first segment — that would ambiguate with a scheme-prefixed
    // absolute reference. Once a scheme has already been parsed, the
    // IRI is absolute and §4.2 no longer applies: path-rootless and
    // path-noscheme both accept `ipchar` which includes ':'. See
    // RFC 3986 §3.3 and `docs/verification/adversary-findings/iri/divergences.md`
    // bug #1.
    if !has_scheme && !has_authority && !slice.starts_with('/') {
        let first_seg = slice.split('/').next().unwrap_or("");
        if first_seg.contains(':') {
            let offset = a + first_seg.find(':').unwrap_or(0);
            return Err(Diagnostic::new(
                DiagnosticCode::Syntax,
                "first segment of a relative-path reference must not contain ':' \
                 (RFC 3986 §4.2)",
                Some(offset),
            ));
        }
    }
    Ok(())
}

fn validate_query_or_fragment(
    input: &str,
    range: Option<(usize, usize)>,
    label: &str,
) -> Result<(), Diagnostic> {
    let Some((a, b)) = range else { return Ok(()) };
    // iquery = *( ipchar / iprivate / "/" / "?" )
    // ifragment = *( ipchar / "/" / "?" )
    // The distinction (iprivate) does not affect acceptance; both
    // productions accept '/' and '?' on top of ipchar.
    let allow_iprivate = label == "query";
    validate_run(input, a, b, |c| {
        is_iunreserved(c)
            || is_sub_delim(c)
            || matches!(c, ':' | '@' | '/' | '?')
            || (allow_iprivate && is_iprivate(c))
    })
}

/// Walk a slice of `input[a..b]` and validate that every character is
/// either `allow(c)` or the start of a valid `%HH` pct-encoded triplet.
///
/// Additionally enforces the `IRI-SURROGATE-001` pin
/// (`docs/spec-readings/iri/lone-surrogate-rejection.md`): when two
/// consecutive `%HH` triplets form the first two bytes of a UTF-8
/// encoding in the surrogate range `U+D800..U+DFFF` (first byte
/// `0xED`, second `0xA0..=0xBF`), the run is rejected with a fatal
/// diagnostic. RFC 3987 Errata 3937 forbids these byte sequences from
/// appearing in IRI references.
fn validate_run<F: Fn(char) -> bool>(
    input: &str,
    a: usize,
    b: usize,
    allow: F,
) -> Result<(), Diagnostic> {
    let bytes = &input.as_bytes()[a..b];
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // Require two following hex digits.
            let h1 = bytes.get(i + 1).copied();
            let h2 = bytes.get(i + 2).copied();
            match (h1, h2) {
                (Some(c1), Some(c2))
                    if c1.is_ascii_hexdigit() && c2.is_ascii_hexdigit() =>
                {
                    let decoded = hex_byte(c1, c2);
                    // IRI-SURROGATE-001: if this triplet and the next
                    // form a UTF-8 surrogate encoding
                    // (0xED followed by 0xA0..=0xBF), reject.
                    if decoded == 0xED
                        && let Some(next_decoded) = peek_pct(bytes, i + 3)
                        && (0xA0..=0xBF).contains(&next_decoded)
                    {
                        return Err(Diagnostic::new(
                            DiagnosticCode::SurrogatePct,
                            "IRI-SURROGATE-001: pct-encoded byte sequence decodes to a \
                             UTF-16 surrogate (U+D800..U+DFFF); forbidden by RFC 3987 \
                             Errata 3937",
                            Some(a + i),
                        ));
                    }
                    i += 3;
                    continue;
                }
                _ => {
                    return Err(Diagnostic::percent_encoding(
                        "malformed pct-encoded triplet (IRI-PCT-001: expected '%HH')",
                        Some(a + i),
                    ));
                }
            }
        }
        // Non-'%' byte: decode a full UTF-8 char to check `allow` and
        // advance by its length. `input` is known-valid UTF-8.
        let c = input[a + i..].chars().next().expect("non-empty slice");
        if !allow(c) {
            return Err(Diagnostic::new(
                DiagnosticCode::Syntax,
                format!("character {c:?} is not permitted in this subcomponent"),
                Some(a + i),
            ));
        }
        i += c.len_utf8();
    }
    Ok(())
}

/// Decode two ASCII hex digits into a byte. Caller must ensure both are
/// valid hex digits.
const fn hex_byte(h1: u8, h2: u8) -> u8 {
    const fn nyb(b: u8) -> u8 {
        if b.is_ascii_digit() {
            b - b'0'
        } else if b >= b'a' {
            b - b'a' + 10
        } else {
            b - b'A' + 10
        }
    }
    (nyb(h1) << 4) | nyb(h2)
}

/// If `bytes[pos..pos+3]` is a valid `%HH` triplet, return the decoded
/// byte; otherwise `None`.
fn peek_pct(bytes: &[u8], pos: usize) -> Option<u8> {
    let pct = *bytes.get(pos)?;
    let h1 = *bytes.get(pos + 1)?;
    let h2 = *bytes.get(pos + 2)?;
    if pct == b'%' && h1.is_ascii_hexdigit() && h2.is_ascii_hexdigit() {
        Some(hex_byte(h1, h2))
    } else {
        None
    }
}

// -----------------------------------------------------------------------
// RFC 3987 §2.2 character classes. We keep them as `fn(char) -> bool`
// so the validator above can compose them.
// -----------------------------------------------------------------------

const fn is_sub_delim(c: char) -> bool {
    matches!(c, '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=')
}

const fn is_unreserved_ascii(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~')
}

/// iunreserved = ALPHA / DIGIT / "-" / "." / "_" / "~" / ucschar  (RFC 3987 §2.2)
fn is_iunreserved(c: char) -> bool {
    is_unreserved_ascii(c) || is_ucschar(c)
}

/// ucschar, as spelled out in RFC 3987 §2.2.
fn is_ucschar(c: char) -> bool {
    let n = c as u32;
    (0xA0..=0xD7FF).contains(&n)
        || (0xF900..=0xFDCF).contains(&n)
        || (0xFDF0..=0xFFEF).contains(&n)
        || (0x10000..=0x1FFFD).contains(&n)
        || (0x20000..=0x2FFFD).contains(&n)
        || (0x30000..=0x3FFFD).contains(&n)
        || (0x40000..=0x4FFFD).contains(&n)
        || (0x50000..=0x5FFFD).contains(&n)
        || (0x60000..=0x6FFFD).contains(&n)
        || (0x70000..=0x7FFFD).contains(&n)
        || (0x80000..=0x8FFFD).contains(&n)
        || (0x90000..=0x9FFFD).contains(&n)
        || (0xA0000..=0xAFFFD).contains(&n)
        || (0xB0000..=0xBFFFD).contains(&n)
        || (0xC0000..=0xCFFFD).contains(&n)
        || (0xD0000..=0xDFFFD).contains(&n)
        || (0xE1000..=0xEFFFD).contains(&n)
}

/// iprivate, as spelled out in RFC 3987 §2.2.
fn is_iprivate(c: char) -> bool {
    let n = c as u32;
    (0xE000..=0xF8FF).contains(&n)
        || (0xF0000..=0xFFFFD).contains(&n)
        || (0x100_000..=0x10_FFFD).contains(&n)
}
