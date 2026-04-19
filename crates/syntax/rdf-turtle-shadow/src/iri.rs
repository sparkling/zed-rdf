//! IRI resolution for Turtle 1.1 / `TriG`.
//!
//! Implements RFC 3986 §5.2 reference resolution against a base IRI.  We use a
//! simplified subset: Turtle IRIs may only be absolute-IRI or relative
//! references; the base is always absolute (set by `@base` / `BASE`).
//!
//! Key spec notes (W3C Turtle 1.1 §2.4 and §2.5):
//! - `@base` / `BASE` declarations update the *current* base IRI for all
//!   subsequent relative IRI references.
//! - IRIs wrapped in `<…>` may contain `\uXXXX` / `\UXXXXXXXX` escapes which
//!   are decoded before resolution.
//! - Prefixed names are expanded to an absolute IRI using the prefix mapping
//!   declared by `@prefix` / `PREFIX`.  The local part may contain escape
//!   sequences `\x` for reserved characters.
//! - The default base IRI is `""` (empty string), i.e., no base is set; any
//!   relative reference in that state is an error.

use thiserror::Error;

use crate::unescape::{UnescapeError, unescape_iri};

/// Errors during IRI processing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IriError {
    /// A relative IRI was encountered with no base IRI set.
    #[error("relative IRI reference '{iri}' found but no base IRI is set")]
    NoBase {
        /// The relative IRI.
        iri: String,
    },
    /// Escape decoding inside an IRI failed.
    #[error("IRI escape error: {0}")]
    Unescape(#[from] UnescapeError),
    /// An undefined prefix was used.
    #[error("undefined prefix '{prefix}:'")]
    UndefinedPrefix {
        /// The offending prefix.
        prefix: String,
    },
}

/// Resolve a raw IRI reference string (the content between `<` and `>`,
/// already stripped of delimiters) against the current base IRI.
///
/// Performs unicode-escape decoding then RFC 3986 §5.2 resolution.
pub fn resolve_iri(raw: &str, base: Option<&str>) -> Result<String, IriError> {
    let decoded = unescape_iri(raw)?;
    if is_absolute(&decoded) {
        return Ok(decoded);
    }
    if decoded.is_empty() {
        // <> resolves to base
        return base
            .map(ToOwned::to_owned)
            .ok_or(IriError::NoBase { iri: decoded });
    }
    let base_str = base.ok_or_else(|| IriError::NoBase { iri: decoded.clone() })?;
    Ok(merge(base_str, &decoded))
}

/// Expand a prefixed name `prefix:local` using `prefix_map`.
///
/// The local part allows escape sequences `\x` for reserved characters.
pub fn expand_pname(
    prefix: &str,
    local: &str,
    prefix_map: &std::collections::HashMap<String, String>,
) -> Result<String, IriError> {
    let iri_prefix = prefix_map
        .get(prefix)
        .ok_or_else(|| IriError::UndefinedPrefix { prefix: prefix.to_owned() })?;
    let local_decoded = decode_pname_local(local);
    Ok(format!("{iri_prefix}{local_decoded}"))
}

/// Decode percent-style local-name escapes in a `PN_LOCAL` value.
/// In Turtle, `PN_LOCAL_ESC` allows `\` followed by: `_~.-!$&'()*+,;=/?#@%`
fn decode_pname_local(local: &str) -> String {
    let mut out = String::with_capacity(local.len());
    let mut chars = local.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                chars.next();
                out.push(next);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Return `true` if the IRI has a scheme (is absolute).
fn is_absolute(iri: &str) -> bool {
    // A scheme is [a-zA-Z][a-zA-Z0-9+\-.]*:
    let mut chars = iri.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for c in chars.by_ref() {
        if c == ':' {
            return true;
        }
        if !c.is_ascii_alphanumeric() && c != '+' && c != '-' && c != '.' {
            return false;
        }
    }
    false
}

/// Minimal RFC 3986 §5.2.2 reference resolution.
fn merge(base: &str, reference: &str) -> String {
    let (base_scheme_auth, base_path, base_query) = split_base(base);

    if reference.starts_with("//") {
        return format!("{base_scheme_auth}{reference}");
    }
    if reference.starts_with('?') {
        return format!("{base_scheme_auth}{base_path}{reference}");
    }
    if reference.starts_with('#') {
        return format!("{base_scheme_auth}{base_path}{base_query}{reference}");
    }

    let merged_path = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let base_dir = base_path.rfind('/').map_or("/", |i| &base_path[..=i]);
        format!("{base_dir}{reference}")
    };
    let normalised = remove_dot_segments(&merged_path);
    format!("{base_scheme_auth}{normalised}")
}

/// Split a base IRI into `(scheme_and_authority, path, query_and_fragment)`.
fn split_base(base: &str) -> (&str, &str, &str) {
    let path_start = base.find("://").map_or(0, |cs| {
        let after_auth = cs + 3;
        base[after_auth..].find('/').map_or(base.len(), |i| after_auth + i)
    });

    let scheme_auth = &base[..path_start];
    let rest = &base[path_start..];

    let (path, query_frag) = rest.find('?').map_or_else(
        || {
            rest.find('#').map_or((rest, ""), |fi| (&rest[..fi], &rest[fi..]))
        },
        |qi| (&rest[..qi], &rest[qi..]),
    );

    (scheme_auth, path, query_frag)
}

/// Remove `.` and `..` segments from a path per RFC 3986 §5.2.4.
fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    let mut input = path;

    while !input.is_empty() {
        if input.starts_with("../") {
            input = &input[3..];
        } else if input.starts_with("./") || input.starts_with("/./") {
            input = &input[2..];
        } else if input == "/." {
            input = "/";
        } else if input.starts_with("/../") {
            input = &input[3..];
            output.pop();
        } else if input == "/.." {
            input = "/";
            output.pop();
        } else if input == "." || input == ".." {
            input = "";
        } else {
            let seg_end = input.strip_prefix('/').map_or_else(
                || input.find('/').unwrap_or(input.len()),
                |stripped| stripped.find('/').map_or(input.len(), |i| i + 1),
            );
            output.push(&input[..seg_end]);
            input = &input[seg_end..];
        }
    }

    output.concat()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_iri_passthrough() {
        let r = resolve_iri("http://example.org/foo", None).unwrap();
        assert_eq!(r, "http://example.org/foo");
    }

    #[test]
    fn empty_ref_resolves_to_base() {
        let r = resolve_iri("", Some("http://example.org/foo")).unwrap();
        assert_eq!(r, "http://example.org/foo");
    }

    #[test]
    fn relative_ref_resolved() {
        let r = resolve_iri("bar", Some("http://example.org/foo/baz")).unwrap();
        assert_eq!(r, "http://example.org/foo/bar");
    }

    #[test]
    fn absolute_path_ref() {
        let r = resolve_iri("/other", Some("http://example.org/foo/baz")).unwrap();
        assert_eq!(r, "http://example.org/other");
    }

    #[test]
    fn unicode_escape_in_iri() {
        let r = resolve_iri(r"http://example.org/\u00E9", None).unwrap();
        assert_eq!(r, "http://example.org/é");
    }

    #[test]
    fn no_base_error() {
        assert!(resolve_iri("relative", None).is_err());
    }

    #[test]
    fn remove_dot_segments_basic() {
        assert_eq!(remove_dot_segments("/a/b/../c"), "/a/c");
        assert_eq!(remove_dot_segments("/a/./b"), "/a/b");
        assert_eq!(remove_dot_segments("/a/b/../../c"), "/c");
    }
}
