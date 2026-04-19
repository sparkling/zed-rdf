//! Minimal IRI resolver self-stub.
//!
//! Implements just enough of RFC 3986 §5 (reference resolution) for the
//! Turtle / TriG parser to build absolute IRIs from `@base` / `@prefix` +
//! relative reference. Intentionally narrow — swap for `rdf-iri::Iri`
//! when that crate lands in the workspace graph (see ADR-0017 Phase A).
//!
//! Equality semantics follow pin `IRI-PCT-001`: byte-for-byte, no
//! percent-encoding case folding, no NFC/NFD, no host casing.

use crate::diag::{Diag, DiagnosticCode};

/// Decomposition of an IRI reference into its five RFC 3986 components.
/// Empty (not absent) components are represented by `Some("")`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Components<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

/// `true` iff `s` looks like an absolute IRI (has a scheme). A scheme is
/// an ASCII-alpha followed by `[A-Za-z0-9+.-]*` and a `:` — RFC 3986
/// §3.1.
pub(crate) fn is_absolute(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    for (i, &b) in bytes.iter().enumerate() {
        if b == b':' {
            return i > 0;
        }
        if !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) {
            return false;
        }
    }
    false
}

fn split_components(s: &str) -> Components<'_> {
    // Scheme.
    let (scheme, rest) = {
        let bytes = s.as_bytes();
        if !bytes.is_empty() && bytes[0].is_ascii_alphabetic() {
            let mut end = None;
            for (i, &b) in bytes.iter().enumerate() {
                if b == b':' {
                    if i > 0 {
                        end = Some(i);
                    }
                    break;
                }
                if !(b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.')) {
                    break;
                }
            }
            if let Some(e) = end {
                (Some(&s[..e]), &s[e + 1..])
            } else {
                (None, s)
            }
        } else {
            (None, s)
        }
    };
    // Fragment.
    let (rest, fragment) = if let Some(i) = rest.find('#') {
        (&rest[..i], Some(&rest[i + 1..]))
    } else {
        (rest, None)
    };
    // Query.
    let (rest, query) = if let Some(i) = rest.find('?') {
        (&rest[..i], Some(&rest[i + 1..]))
    } else {
        (rest, None)
    };
    // Authority.
    let (authority, path) = if let Some(stripped) = rest.strip_prefix("//") {
        let end = stripped.find('/').unwrap_or(stripped.len());
        (Some(&stripped[..end]), &stripped[end..])
    } else {
        (None, rest)
    };
    Components {
        scheme,
        authority,
        path,
        query,
        fragment,
    }
}

fn recompose(c: &Components<'_>) -> String {
    let mut out = String::new();
    if let Some(s) = c.scheme {
        out.push_str(s);
        out.push(':');
    }
    if let Some(a) = c.authority {
        out.push_str("//");
        out.push_str(a);
    }
    out.push_str(c.path);
    if let Some(q) = c.query {
        out.push('?');
        out.push_str(q);
    }
    if let Some(f) = c.fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// RFC 3986 §5.2.4 `remove_dot_segments`.
fn remove_dot_segments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut input = input.to_owned();
    while !input.is_empty() {
        if let Some(rest) = input.strip_prefix("../") {
            input = rest.to_owned();
        } else if let Some(rest) = input.strip_prefix("./") {
            input = rest.to_owned();
        } else if let Some(rest) = input.strip_prefix("/./") {
            input = format!("/{rest}");
        } else if input == "/." {
            input = "/".to_owned();
        } else if let Some(rest) = input.strip_prefix("/../") {
            input = format!("/{rest}");
            pop_last_segment(&mut output);
        } else if input == "/.." {
            input = "/".to_owned();
            pop_last_segment(&mut output);
        } else if input == "." || input == ".." {
            input.clear();
        } else {
            // Move the first path segment (including leading '/') to output.
            let split = if input.starts_with('/') {
                input[1..].find('/').map(|i| i + 1).unwrap_or(input.len())
            } else {
                input.find('/').unwrap_or(input.len())
            };
            output.push_str(&input[..split]);
            input = input[split..].to_owned();
        }
    }
    output
}

fn pop_last_segment(s: &mut String) {
    if let Some(i) = s.rfind('/') {
        s.truncate(i);
    } else {
        s.clear();
    }
}

/// RFC 3986 §5.2.3 merge.
fn merge(base: &Components<'_>, ref_path: &str) -> String {
    if base.authority.is_some() && base.path.is_empty() {
        format!("/{ref_path}")
    } else {
        let base_path = base.path;
        if let Some(i) = base_path.rfind('/') {
            format!("{}{}", &base_path[..=i], ref_path)
        } else {
            ref_path.to_owned()
        }
    }
}

/// Resolve `reference` against `base` per RFC 3986 §5.2, strict mode.
/// `base` is assumed absolute; the caller validates.
pub(crate) fn resolve(reference: &str, base: &str) -> String {
    let r = split_components(reference);
    let b = split_components(base);
    let (scheme, authority, path, query) = if r.scheme.is_some() {
        (
            r.scheme,
            r.authority,
            remove_dot_segments(r.path),
            r.query,
        )
    } else if r.authority.is_some() {
        (b.scheme, r.authority, remove_dot_segments(r.path), r.query)
    } else if r.path.is_empty() {
        let q = r.query.or(b.query);
        (b.scheme, b.authority, b.path.to_owned(), q)
    } else if r.path.starts_with('/') {
        (b.scheme, b.authority, remove_dot_segments(r.path), r.query)
    } else {
        let merged = merge(&b, r.path);
        (
            b.scheme,
            b.authority,
            remove_dot_segments(&merged),
            r.query,
        )
    };
    recompose(&Components {
        scheme,
        authority,
        path: &path,
        query,
        fragment: r.fragment,
    })
}

/// Shape-validate an IRI body (the text between `<` and `>`). Rejects
/// disallowed control characters per Turtle §6.3 `IRIREF`.
pub(crate) fn validate_iri_body(body: &str, offset: usize) -> Result<(), Diag> {
    for (i, c) in body.char_indices() {
        if matches!(c, '\u{00}'..='\u{20}' | '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\') {
            // `\\` inside an IRIREF is only valid if it introduces a UCHAR,
            // but UCHARs are already decoded before this call; leftover
            // backslashes are an error.
            return Err(Diag {
                code: DiagnosticCode::Syntax,
                message: format!("invalid character {c:?} in IRI reference"),
                offset: offset + i,
                fatal: true,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_detection() {
        assert!(is_absolute("http://example/"));
        assert!(is_absolute("urn:x"));
        // `ex:` is syntactically absolute (scheme + empty path) per
        // RFC 3986 §3.3 — the Turtle grammar still distinguishes it
        // from pname `ex:` at the token level.
        assert!(is_absolute("ex:"));
        assert!(!is_absolute("/relative"));
        assert!(!is_absolute(""));
    }

    #[test]
    fn resolve_rfc3986_examples() {
        let base = "http://a/b/c/d;p?q";
        // RFC 3986 §5.4.1 normal examples — subset.
        assert_eq!(resolve("g", base), "http://a/b/c/g");
        assert_eq!(resolve("./g", base), "http://a/b/c/g");
        assert_eq!(resolve("g/", base), "http://a/b/c/g/");
        assert_eq!(resolve("/g", base), "http://a/g");
        assert_eq!(resolve("//g", base), "http://g");
        assert_eq!(resolve("?y", base), "http://a/b/c/d;p?y");
        assert_eq!(resolve("#s", base), "http://a/b/c/d;p?q#s");
        assert_eq!(resolve("../g", base), "http://a/b/g");
        assert_eq!(resolve("../../g", base), "http://a/g");
        assert_eq!(resolve("", base), "http://a/b/c/d;p?q");
    }
}
