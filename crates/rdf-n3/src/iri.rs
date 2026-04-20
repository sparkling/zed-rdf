//! Minimal IRI resolver for the N3 parser.
//!
//! Mirrors the implementation in `rdf-turtle::iri` — same RFC 3986 §5
//! reference resolution, same pin (`IRI-PCT-001`).

/// `true` iff `s` looks like an absolute IRI (has a scheme).
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

/// Decomposition of an IRI reference into its five RFC 3986 components.
struct Components<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
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
    let (rest, fragment) = if let Some(i) = rest.find('#') {
        (&rest[..i], Some(&rest[i + 1..]))
    } else {
        (rest, None)
    };
    let (rest, query) = if let Some(i) = rest.find('?') {
        (&rest[..i], Some(&rest[i + 1..]))
    } else {
        (rest, None)
    };
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
pub(crate) fn resolve(reference: &str, base: &str) -> String {
    let r = split_components(reference);
    let b = split_components(base);
    let (scheme, authority, path, query) = if r.scheme.is_some() {
        (r.scheme, r.authority, remove_dot_segments(r.path), r.query)
    } else if r.authority.is_some() {
        (b.scheme, r.authority, remove_dot_segments(r.path), r.query)
    } else if r.path.is_empty() {
        let q = r.query.or(b.query);
        (b.scheme, b.authority, b.path.to_owned(), q)
    } else if r.path.starts_with('/') {
        (b.scheme, b.authority, remove_dot_segments(r.path), r.query)
    } else {
        let merged = merge(&b, r.path);
        (b.scheme, b.authority, remove_dot_segments(&merged), r.query)
    };
    recompose(&Components {
        scheme,
        authority,
        path: &path,
        query,
        fragment: r.fragment,
    })
}
