//! Narrow normalisation + IRI → URI mapping.
//!
//! The `IRI-PCT-001` pin forbids most normalisations at parse / compare
//! time. [`normalise`] applies only the subset that the pin explicitly
//! allows:
//!
//! - Scheme ASCII lowercase (RFC 3986 §6.2.2.1).
//! - Host ASCII lowercase (RFC 3490 §4 step 2). Already-ACE labels
//!   (`xn--…`) are ASCII by construction, so lowercasing preserves
//!   them verbatim — see
//!   `docs/spec-readings/iri/idna-host-normalisation-pin.md`.
//! - Path dot-segment removal (RFC 3986 §5.2.4, errata 4005) for
//!   hierarchical IRIs only (i.e., when an authority or an absolute
//!   path is present).
//!
//! [`to_uri`] implements RFC 3987 §3.1 step 3 via the `idna` crate's
//! `domain_to_ascii_strict` (RFC 3490 `ToASCII` — Punycode + UTS 46
//! mapping). When `idna` rejects the input (empty host, disallowed
//! code points, malformed `xn--` label), we fall back to the
//! pre-patch path: ASCII-lowercase the host and percent-encode any
//! remaining non-ASCII UTF-8 bytes. The fallback is documented in the
//! spec-reading pin.

use crate::{Diagnostic, DiagnosticCode, Iri, parse};

pub fn normalise(iri: &Iri) -> Iri {
    let scheme = iri.scheme().map(str::to_ascii_lowercase);
    let host = iri.host().map(str::to_ascii_lowercase);

    // Remove-dot-segments on the path when we have a hierarchical IRI.
    // RFC 3986 §5.2.4 with errata 4005 for the `/../` at root case.
    let path = if iri.authority().is_some() || iri.path().starts_with('/') {
        remove_dot_segments(iri.path())
    } else {
        iri.path().to_owned()
    };

    let authority = iri.authority().map(|_| AuthorityView {
        userinfo: iri.parts.userinfo.map(|(a, b)| &iri.as_str()[a..b]),
        host: host.as_deref().unwrap_or(""),
        port: iri.parts.port.map(|(a, b)| &iri.as_str()[a..b]),
    });

    // Re-serialise into an IRI string, then re-parse. Re-parsing makes
    // component offsets consistent with the new byte sequence.
    let raw = reassemble(
        scheme.as_deref(),
        authority,
        &path,
        iri.query(),
        iri.fragment(),
    );

    // Invariant: normalisation yields a well-formed IRI; tests cover the
    // happy paths. If re-parse fails, fall back to the input rather
    // than panicking in a `#[must_use]` API.
    parse::parse(&raw).unwrap_or_else(|_| iri.clone())
}

/// RFC 3987 §3.1: Converting IRIs to URIs.
///
/// Procedure:
///
/// 1. Replace each non-ASCII character in an `iunreserved` position by
///    its UTF-8 byte sequence, each byte percent-encoded.
/// 2. In `ireg-name`, pass each label through `ToASCII` — we do the
///    ASCII-lowercase subset only (see IDN pin); labels with non-ASCII
///    octets are percent-encoded rather than Punycode-escaped.
///
/// Control characters (`%00`–`%1F`, `%7F`) are forbidden in URIs per
/// RFC 3986 §2.2; we reject them with `IRI-URI-001`.
pub fn to_uri(iri: &Iri) -> Result<String, Diagnostic> {
    let mut out = String::with_capacity(iri.as_str().len());

    if let Some((a, b)) = iri.parts.scheme {
        out.push_str(&iri.as_str()[a..b].to_ascii_lowercase());
        out.push(':');
    }

    if iri.authority().is_some() {
        out.push_str("//");
        if let Some((a, b)) = iri.parts.userinfo {
            encode_non_ascii(&iri.as_str()[a..b], &mut out, 0)?;
            out.push('@');
        }
        if let Some((a, b)) = iri.parts.host {
            let host = &iri.as_str()[a..b];
            if host.starts_with('[') {
                // IP-literal: already ASCII, pass through verbatim.
                out.push_str(host);
            } else {
                encode_host(host, &mut out, a)?;
            }
        }
        if let Some((a, b)) = iri.parts.port {
            out.push(':');
            out.push_str(&iri.as_str()[a..b]);
        }
    }

    encode_non_ascii(iri.path(), &mut out, iri.parts.path.0)?;

    if let Some(q) = iri.query() {
        out.push('?');
        encode_non_ascii(q, &mut out, iri.parts.query.map_or(0, |(a, _)| a))?;
    }
    if let Some(f) = iri.fragment() {
        out.push('#');
        encode_non_ascii(f, &mut out, iri.parts.fragment.map_or(0, |(a, _)| a))?;
    }

    Ok(out)
}

/// RFC 3987 §3.1 step 3: map an `ireg-name` host to its ASCII form.
///
/// Strategy:
///
/// 1. If the host is pure ASCII, ASCII-lowercase it and emit. This
///    covers already-ACE labels (`xn--…`) without re-running them
///    through the `idna` decode/encode round trip.
/// 2. Otherwise, run `idna::domain_to_ascii_strict` (RFC 3490 `ToASCII`
///    with the UTS 46 strict profile). A successful result is emitted
///    verbatim — it is already lowercase ASCII by construction.
/// 3. If `idna` rejects the input (disallowed code points, malformed
///    labels, empty string, bad existing `xn--` decode), fall back to
///    the pre-patch path: ASCII-lowercase, then percent-encode any
///    non-ASCII UTF-8 bytes. This keeps the function total so parsing
///    never fails because of an IDNA-hostile host — the rejection is
///    visible at the URI level as a `%`-heavy host rather than a
///    structural error.
///
/// Trade-off documented in
/// `docs/spec-readings/iri/idna-host-normalisation-pin.md`.
fn encode_host(host: &str, out: &mut String, base_offset: usize) -> Result<(), Diagnostic> {
    if host.is_ascii() {
        // ASCII host: lowercase locally. Covers plain reg-names and
        // already-ACE (`xn--…`) labels without a round-trip through
        // `idna` — which in strict mode would reject underscores and
        // other chars that our parser allows per RFC 3986 §3.2.2.
        let lower = host.to_ascii_lowercase();
        out.push_str(&lower);
        return Ok(());
    }
    // Non-ASCII host: try full `ToASCII`.
    if let Ok(ace) = idna::domain_to_ascii_strict(host) {
        // `idna` returns lowercase ASCII on success; emit verbatim.
        out.push_str(&ace);
        Ok(())
    } else {
        // Fallback per the pin: lowercase ASCII bytes in the host,
        // percent-encode the rest.
        let lower = host.to_ascii_lowercase();
        encode_non_ascii(&lower, out, base_offset)
    }
}

fn encode_non_ascii(slice: &str, out: &mut String, base_offset: usize) -> Result<(), Diagnostic> {
    for (i, c) in slice.char_indices() {
        if c.is_ascii() {
            // Reject C0 controls + DEL; RFC 3986 §2.2 forbids them
            // outside pct-encoding, and we got here only if the parser
            // let them in (it did, via ucschar, for non-ASCII — ASCII
            // controls would already be rejected by `validate_run`).
            // Defensive check for `.to_uri()` direct callers.
            if (c as u32) < 0x20 || c == '\x7F' {
                return Err(Diagnostic::new(
                    DiagnosticCode::UriMapping,
                    format!("control character U+{:04X} forbidden in URI form", c as u32),
                    Some(base_offset + i),
                ));
            }
            out.push(c);
        } else {
            // Percent-encode the UTF-8 octets.
            let mut buf = [0u8; 4];
            for &b in c.encode_utf8(&mut buf).as_bytes() {
                out.push('%');
                push_hex_upper(out, b);
            }
        }
    }
    Ok(())
}

fn push_hex_upper(out: &mut String, b: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    out.push(HEX[(b >> 4) as usize] as char);
    out.push(HEX[(b & 0x0F) as usize] as char);
}

// -----------------------------------------------------------------------
// Re-serialisation helper.
// -----------------------------------------------------------------------

struct AuthorityView<'a> {
    userinfo: Option<&'a str>,
    host: &'a str,
    port: Option<&'a str>,
}

fn reassemble(
    scheme: Option<&str>,
    authority: Option<AuthorityView<'_>>,
    path: &str,
    query: Option<&str>,
    fragment: Option<&str>,
) -> String {
    let mut out = String::new();
    if let Some(s) = scheme {
        out.push_str(s);
        out.push(':');
    }
    if let Some(a) = authority {
        out.push_str("//");
        if let Some(u) = a.userinfo {
            out.push_str(u);
            out.push('@');
        }
        out.push_str(a.host);
        if let Some(p) = a.port {
            out.push(':');
            out.push_str(p);
        }
    }
    out.push_str(path);
    if let Some(q) = query {
        out.push('?');
        out.push_str(q);
    }
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

// -----------------------------------------------------------------------
// Dot-segment removal: RFC 3986 §5.2.4 + errata 4005.
// -----------------------------------------------------------------------

/// Apply the `remove_dot_segments` algorithm from RFC 3986 §5.2.4.
///
/// The algorithm maintains an output buffer and repeatedly inspects the
/// leading characters of the remaining input, transferring complete
/// segments. Errata 4005 clarifies that `/../` at the root of an
/// absolute path collapses to `/` rather than being an error.
pub fn remove_dot_segments(input: &str) -> String {
    // Work in bytes; all relevant characters are ASCII.
    let bytes = input.as_bytes();
    let mut input_pos = 0usize;
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());

    while input_pos < bytes.len() {
        let rest = &bytes[input_pos..];
        // A. If the input begins with "../" or "./" — remove.
        if starts_with(rest, b"../") {
            input_pos += 3;
        } else if starts_with(rest, b"./") {
            input_pos += 2;
        // B. If the input begins with "/./" or is "/.", replace with "/".
        } else if starts_with(rest, b"/./") {
            input_pos += 2;
            // Leave a leading '/' in the input (already there after advance).
        } else if rest == b"/." {
            // Replace with "/". Achieved by consuming ".", leaving "/".
            input_pos += 2;
            out.push(b'/');
        // C. If the input begins with "/../" or is "/..", replace with "/"
        //    AND remove the last segment from out.
        } else if starts_with(rest, b"/../") {
            input_pos += 3; // Leaves "/" at input head.
            pop_segment(&mut out);
        } else if rest == b"/.." {
            input_pos += 3;
            pop_segment(&mut out);
            out.push(b'/');
        // D. If the input consists of "." or ".." — remove.
        } else if rest == b"." || rest == b".." {
            input_pos += rest.len();
        // E. Otherwise, move the first path segment (including any
        //    leading "/") from input to the end of output.
        } else {
            // Find next '/' (not at position 0 if '/' is first).
            let first_is_slash = rest[0] == b'/';
            let search_from = usize::from(first_is_slash);
            let seg_end = rest[search_from..]
                .iter()
                .position(|&b| b == b'/')
                .map_or(rest.len(), |p| search_from + p);
            out.extend_from_slice(&rest[..seg_end]);
            input_pos += seg_end;
        }
    }

    // The algorithm preserves ASCII / UTF-8 validity because input is
    // UTF-8 and we only copy whole segments.
    String::from_utf8(out).expect("remove_dot_segments preserves UTF-8")
}

fn starts_with(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len() && &s[..prefix.len()] == prefix
}

/// Remove the last segment *and* its preceding `/` from `out`, if any.
/// Per errata 4005 this is a no-op when `out` is empty or contains only
/// a leading `/` — i.e., `/..` at the root resolves to `/`.
fn pop_segment(out: &mut Vec<u8>) {
    // Find the last '/'. Drop everything from there to the end.
    if let Some(idx) = out.iter().rposition(|&b| b == b'/') {
        out.truncate(idx);
    } else {
        out.clear();
    }
}
