//! Unit tests for the main `rdf-iri` implementation.
//!
//! Coverage:
//!
//! - Parse acceptance + rejection for each subcomponent.
//! - `IRI-PCT-001` pin: no percent-encoding hex case fold, no
//!   percent-decode, no host case fold, no NFC/NFD.
//! - RFC 3986 §5.2.4 `remove_dot_segments` including errata 4005.
//! - RFC 3986 §5 strict resolution table.
//! - RFC 3987 §3.1 IRI → URI mapping with ASCII-lowercase IDN carve-out.
//! - `rdf_diff::Parser` integration smoke test.

use rdf_diff::Parser;

use super::*;

// ----------------------------------------------------------------------
// Parsing
// ----------------------------------------------------------------------

#[test]
fn parse_minimal_absolute() {
    let iri = Iri::parse("http://example/").expect("parse");
    assert_eq!(iri.scheme(), Some("http"));
    assert_eq!(iri.authority(), Some("example"));
    assert_eq!(iri.host(), Some("example"));
    assert_eq!(iri.path(), "/");
    assert!(iri.is_absolute());
}

#[test]
fn parse_authority_userinfo_host_port() {
    let iri = Iri::parse("https://alice:secret@Example.COM:8080/a?q=1#frag").expect("parse");
    assert_eq!(iri.scheme(), Some("https"));
    assert_eq!(iri.authority(), Some("alice:secret@Example.COM:8080"));
    assert_eq!(iri.host(), Some("Example.COM"));
    assert_eq!(iri.path(), "/a");
    assert_eq!(iri.query(), Some("q=1"));
    assert_eq!(iri.fragment(), Some("frag"));
}

#[test]
fn parse_ipv6_literal() {
    let iri = Iri::parse("http://[2001:db8::1]:80/").expect("parse ipv6");
    assert_eq!(iri.host(), Some("[2001:db8::1]"));
}

#[test]
fn parse_relative_reference() {
    let iri = Iri::parse("../foo/bar").expect("parse relative");
    assert!(!iri.is_absolute());
    assert_eq!(iri.path(), "../foo/bar");
}

#[test]
fn parse_empty_is_valid_relative_ref() {
    let iri = Iri::parse("").expect("empty is a relative reference (RFC 3986 §4.2)");
    assert!(!iri.is_absolute());
    assert_eq!(iri.path(), "");
}

#[test]
fn parse_rejects_malformed_pct_encoded() {
    let err = Iri::parse("http://example/a%2").expect_err("truncated %HH");
    assert_eq!(err.code, DiagnosticCode::PercentEncoding);
    assert_eq!(err.code.as_str(), "IRI-PCT-001");
}

#[test]
fn parse_rejects_non_hex_pct_encoded() {
    let err = Iri::parse("http://example/a%GG").expect_err("non-hex pct");
    assert_eq!(err.code, DiagnosticCode::PercentEncoding);
}

#[test]
fn parse_rejects_invalid_scheme_char() {
    let err = Iri::parse("ht!tp://example/").expect_err("invalid scheme");
    // Note: '!' in position 2 makes the scheme scan fail, so the parser
    // treats the prefix as a relative reference, then the ':' in the
    // path (forbidden without authority before it) triggers a different
    // failure path — either `Scheme` or `Syntax`. Accept either.
    assert!(matches!(
        err.code,
        DiagnosticCode::Scheme | DiagnosticCode::Syntax
    ));
}

#[test]
fn parse_accepts_ucschar_non_ascii() {
    let iri = Iri::parse("http://example/café").expect("non-ASCII path");
    assert_eq!(iri.path(), "/café");
}

#[test]
fn parse_rejects_control_char_in_path() {
    let err = Iri::parse("http://example/\x07bel").expect_err("BEL in path");
    assert_eq!(err.code, DiagnosticCode::Syntax);
}

// ----------------------------------------------------------------------
// IRI-PCT-001 pin: no silent normalisation at parse time.
// ----------------------------------------------------------------------

#[test]
fn pct_pin_hex_case_is_preserved_at_parse() {
    let lower = Iri::parse("http://example/caf%c3%a9").unwrap();
    let upper = Iri::parse("http://example/caf%C3%A9").unwrap();
    // Parse preserves byte-for-byte; the two are distinct strings.
    assert_ne!(lower.as_str(), upper.as_str());
}

#[test]
fn pct_pin_host_case_is_preserved_at_parse() {
    let a = Iri::parse("http://EXAMPLE.COM/").unwrap();
    let b = Iri::parse("http://example.com/").unwrap();
    assert_ne!(a, b);
    assert_eq!(a.host(), Some("EXAMPLE.COM"));
}

#[test]
fn pct_pin_nfc_nfd_preserved_at_parse() {
    let nfc = Iri::parse("http://example/caf\u{00E9}").unwrap();
    let nfd = Iri::parse("http://example/cafe\u{0301}").unwrap();
    assert_ne!(nfc, nfd);
}

// ----------------------------------------------------------------------
// Normalisation — the narrow subset the pin permits.
// ----------------------------------------------------------------------

#[test]
fn normalise_lowercases_scheme() {
    let iri = Iri::parse("HTTP://example/").unwrap().normalise();
    assert_eq!(iri.scheme(), Some("http"));
}

#[test]
fn normalise_lowercases_host_ascii_only() {
    let iri = Iri::parse("http://Example.COM/").unwrap().normalise();
    assert_eq!(iri.host(), Some("example.com"));
}

#[test]
fn normalise_does_not_fold_hex_case_in_pct() {
    // The pin forbids hex case folding even during normalisation.
    let iri = Iri::parse("http://example/caf%c3%a9").unwrap().normalise();
    assert_eq!(iri.path(), "/caf%c3%a9");
}

#[test]
fn normalise_removes_dot_segments() {
    let iri = Iri::parse("http://example/a/./b/../c").unwrap().normalise();
    assert_eq!(iri.path(), "/a/c");
}

#[test]
fn normalise_errata_4005_at_root() {
    // `/../` at root must collapse to `/`, not below.
    let iri = Iri::parse("http://example/../../a").unwrap().normalise();
    assert_eq!(iri.path(), "/a");
}

#[test]
fn normalise_preserves_non_ascii_host() {
    // Full IDNA is deferred; non-ASCII host bytes survive.
    let iri = Iri::parse("http://münchen.de/").unwrap().normalise();
    assert_eq!(iri.host(), Some("münchen.de"));
}

// ----------------------------------------------------------------------
// Resolve (RFC 3986 §5.3 strict).
// ----------------------------------------------------------------------

fn base() -> Iri {
    Iri::parse("http://a/b/c/d;p?q").unwrap()
}

fn resolve_str(r: &str) -> String {
    let base = base();
    let r_iri = Iri::parse(r).unwrap();
    r_iri.resolve(&base).as_str().to_owned()
}

#[test]
fn resolve_rfc3986_normal_examples() {
    // From RFC 3986 §5.4.1. A few representative cases.
    assert_eq!(resolve_str("g"), "http://a/b/c/g");
    assert_eq!(resolve_str("./g"), "http://a/b/c/g");
    assert_eq!(resolve_str("g/"), "http://a/b/c/g/");
    assert_eq!(resolve_str("/g"), "http://a/g");
    assert_eq!(resolve_str("?y"), "http://a/b/c/d;p?y");
    assert_eq!(resolve_str("g?y"), "http://a/b/c/g?y");
    assert_eq!(resolve_str("#s"), "http://a/b/c/d;p?q#s");
    assert_eq!(resolve_str("g#s"), "http://a/b/c/g#s");
    assert_eq!(resolve_str("g?y#s"), "http://a/b/c/g?y#s");
    assert_eq!(resolve_str(""), "http://a/b/c/d;p?q");
    assert_eq!(resolve_str("."), "http://a/b/c/");
    assert_eq!(resolve_str("./"), "http://a/b/c/");
    assert_eq!(resolve_str(".."), "http://a/b/");
    assert_eq!(resolve_str("../"), "http://a/b/");
    assert_eq!(resolve_str("../g"), "http://a/b/g");
    assert_eq!(resolve_str("../.."), "http://a/");
    assert_eq!(resolve_str("../../g"), "http://a/g");
}

#[test]
fn resolve_rfc3986_abnormal_examples_errata_4005() {
    // Errata 4005: /../ at root collapses to /, not error.
    assert_eq!(resolve_str("../../../g"), "http://a/g");
    assert_eq!(resolve_str("../../../../g"), "http://a/g");
    assert_eq!(resolve_str("/./g"), "http://a/g");
    assert_eq!(resolve_str("/../g"), "http://a/g");
}

#[test]
fn resolve_absolute_reference_overrides_base() {
    let r = Iri::parse("https://other/x").unwrap();
    let out = r.resolve(&base());
    assert_eq!(out.as_str(), "https://other/x");
}

// ----------------------------------------------------------------------
// IRI → URI mapping (RFC 3987 §3.1).
// ----------------------------------------------------------------------

#[test]
fn to_uri_pct_encodes_non_ascii_path() {
    let iri = Iri::parse("http://example/café").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://example/caf%C3%A9");
}

#[test]
fn to_uri_lowercases_host_ascii_only() {
    // Per IDN pin, non-ASCII host labels are pct-encoded (not Puny).
    let iri = Iri::parse("http://Example.COM/").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://example.com/");
}

#[test]
fn to_uri_non_ascii_host_is_pct_encoded_not_punycode() {
    let iri = Iri::parse("http://münchen.de/").unwrap();
    // Full ToASCII would produce `xn--mnchen-3ya.de`; we emit pct-
    // encoded UTF-8 and document this as a known deferral.
    assert_eq!(iri.to_uri().unwrap(), "http://m%C3%BCnchen.de/");
}

#[test]
fn to_uri_preserves_pct_hex_case_in_path() {
    // Pin: no hex case fold.
    let iri = Iri::parse("http://example/a%2f%2F").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://example/a%2f%2F");
}

// ----------------------------------------------------------------------
// rdf_diff::Parser integration.
// ----------------------------------------------------------------------

#[test]
fn parser_impl_emits_single_fact() {
    let parser = IriParser;
    let out = parser
        .parse(b"http://example/a")
        .expect("parse ok");
    assert_eq!(out.facts.set.len(), 1);
    let fact = out.facts.set.keys().next().unwrap();
    assert_eq!(fact.subject, "<http://example/a>");
    assert_eq!(fact.predicate, "<urn:x-rdf-iri:parses-to>");
    assert!(fact.object.starts_with('<') && fact.object.ends_with('>'));
}

#[test]
fn parser_impl_rejects_relative_ref() {
    let parser = IriParser;
    let err = parser.parse(b"../foo").expect_err("relative ref fatal");
    assert!(err.fatal);
    assert!(err.messages.iter().any(|m| m.contains("IRI-SYNTAX-001")));
}

#[test]
fn parser_impl_rejects_bad_pct() {
    let parser = IriParser;
    let err = parser
        .parse(b"http://example/a%2")
        .expect_err("bad pct fatal");
    assert!(err.fatal);
    assert!(err.messages.iter().any(|m| m.contains("IRI-PCT-001")));
}

#[test]
fn parser_impl_id_matches_const() {
    let parser = IriParser;
    assert_eq!(parser.id(), IriParser::ID);
}
