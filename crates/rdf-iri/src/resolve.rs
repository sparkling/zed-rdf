//! RFC 3986 §5 reference resolution (strict algorithm).
//!
//! We implement the `T.scheme = …` decision table from §5.3 verbatim.
//! The strict algorithm (no scheme-inheritance shortcut) is mandated by
//! `docs/spec-readings/iri/percent-encoding-3986-vs-3987.md` via its
//! reference to RFC 3986 §5.2.2 + errata 4005.

use crate::{Iri, normalise, parse};

#[derive(Clone)]
struct Parts<'a> {
    scheme: Option<&'a str>,
    authority: Option<&'a str>,
    path: &'a str,
    query: Option<&'a str>,
    fragment: Option<&'a str>,
}

fn view(iri: &Iri) -> Parts<'_> {
    Parts {
        scheme: iri.scheme(),
        authority: iri.authority(),
        path: iri.path(),
        query: iri.query(),
        fragment: iri.fragment(),
    }
}

pub fn resolve(r: &Iri, base: &Iri) -> Iri {
    assert!(base.is_absolute(), "base IRI must be absolute (RFC 3986 §5.1)");

    let rv = view(r);
    let bv = view(base);

    // RFC 3986 §5.3 Transform References (strict).
    let (t_scheme, t_authority, t_path_owned, t_query);
    if let Some(rs) = rv.scheme {
        t_scheme = Some(rs.to_owned());
        t_authority = rv.authority.map(str::to_owned);
        t_path_owned = normalise::remove_dot_segments(rv.path);
        t_query = rv.query.map(str::to_owned);
    } else if rv.authority.is_some() {
        t_scheme = bv.scheme.map(str::to_owned);
        t_authority = rv.authority.map(str::to_owned);
        t_path_owned = normalise::remove_dot_segments(rv.path);
        t_query = rv.query.map(str::to_owned);
    } else if rv.path.is_empty() {
        t_scheme = bv.scheme.map(str::to_owned);
        t_authority = bv.authority.map(str::to_owned);
        t_path_owned = bv.path.to_owned();
        t_query = rv.query.or(bv.query).map(str::to_owned);
    } else {
        t_scheme = bv.scheme.map(str::to_owned);
        t_authority = bv.authority.map(str::to_owned);
        let merged = if rv.path.starts_with('/') {
            rv.path.to_owned()
        } else {
            merge(bv.authority.is_some(), bv.path, rv.path)
        };
        t_path_owned = normalise::remove_dot_segments(&merged);
        t_query = rv.query.map(str::to_owned);
    }
    let t_fragment = rv.fragment.map(str::to_owned);

    // Re-compose per §5.3 recomposition.
    let mut out = String::new();
    if let Some(s) = t_scheme.as_deref() {
        out.push_str(s);
        out.push(':');
    }
    if let Some(a) = t_authority.as_deref() {
        out.push_str("//");
        out.push_str(a);
    }
    out.push_str(&t_path_owned);
    if let Some(q) = t_query.as_deref() {
        out.push('?');
        out.push_str(q);
    }
    if let Some(f) = t_fragment.as_deref() {
        out.push('#');
        out.push_str(f);
    }

    // Re-parse the recomposed string so component offsets refer to the
    // new buffer.  The composition above yields a grammar-valid IRI by
    // construction; falling back to a minimal reparse of `out` is safe.
    parse::parse(&out).expect("resolved IRI is well-formed by construction")
}

/// RFC 3986 §5.2.3 `merge` algorithm.
fn merge(has_authority: bool, base_path: &str, ref_path: &str) -> String {
    if has_authority && base_path.is_empty() {
        let mut out = String::with_capacity(ref_path.len() + 1);
        out.push('/');
        out.push_str(ref_path);
        out
    } else {
        // Take all bytes up to and including the last '/'; append ref.
        let split = base_path.rfind('/').map_or(0, |i| i + 1);
        let mut out = String::with_capacity(split + ref_path.len());
        out.push_str(&base_path[..split]);
        out.push_str(ref_path);
        out
    }
}
