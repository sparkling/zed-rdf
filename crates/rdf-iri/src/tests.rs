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
// IRI-SURROGATE-001 pin (RFC 3987 Errata 3937).
// A pct-encoded byte sequence that decodes to a value in the
// surrogate range U+D800..U+DFFF (UTF-8: `0xED 0xA0..=0xBF 0x80..=0xBF`)
// must be rejected as fatal. Boundary `%ED%9F%BF` (= U+D7FF, just
// below surrogate range) must still be accepted.
// See docs/spec-readings/iri/lone-surrogate-rejection.md.
// ----------------------------------------------------------------------

#[test]
fn parse_rejects_pct_encoded_lone_high_surrogate() {
    // %ED%A0%80 = UTF-8 encoding of U+D800 (lone high surrogate).
    let err = Iri::parse("http://example/%ED%A0%80")
        .expect_err("lone high surrogate must reject");
    assert_eq!(err.code, DiagnosticCode::SurrogatePct);
    assert_eq!(err.code.as_str(), "IRI-SURROGATE-001");
}

#[test]
fn parse_rejects_pct_encoded_lone_low_surrogate() {
    // %ED%B0%80 = UTF-8 encoding of U+DC00 (lone low surrogate).
    let err = Iri::parse("http://example/%ED%B0%80")
        .expect_err("lone low surrogate must reject");
    assert_eq!(err.code, DiagnosticCode::SurrogatePct);
}

#[test]
fn parse_rejects_pct_encoded_surrogate_pair() {
    // A surrogate pair pct-encoded byte-for-byte is still invalid per
    // Errata 3937 — the rejection is on each surrogate scalar
    // independently, not on whether callers "meant" a supplementary
    // code point.
    let err = Iri::parse("http://example/%ED%A0%80%ED%B0%80")
        .expect_err("pct-encoded surrogate pair must reject");
    assert_eq!(err.code, DiagnosticCode::SurrogatePct);
}

#[test]
fn parse_accepts_pct_encoded_just_below_surrogate() {
    // %ED%9F%BF = UTF-8 encoding of U+D7FF, the last non-surrogate
    // code point before the range. Boundary case — must be accepted.
    let iri = Iri::parse("http://example/%ED%9F%BF")
        .expect("U+D7FF boundary accepts");
    assert_eq!(iri.path(), "/%ED%9F%BF");
}

#[test]
fn parse_rejects_surrogate_in_query_and_fragment() {
    let qerr = Iri::parse("http://example/?q=%ED%A0%80")
        .expect_err("surrogate in query must reject");
    assert_eq!(qerr.code, DiagnosticCode::SurrogatePct);
    let ferr = Iri::parse("http://example/#%ED%A0%80")
        .expect_err("surrogate in fragment must reject");
    assert_eq!(ferr.code, DiagnosticCode::SurrogatePct);
}

// ----------------------------------------------------------------------
// RFC 3986 §4.2 first-segment-colon rule: applies only to
// **relative-references** (no scheme). Absolute IRIs with a scheme may
// include ':' freely in path segments. See
// docs/verification/adversary-findings/iri/divergences.md bug #1.
// ----------------------------------------------------------------------

#[test]
fn parse_accepts_urn_example_foo() {
    // Classic URN: scheme=urn, path=example:foo. The two colons in the
    // path are legal per RFC 3986 §3.3 because the IRI is absolute.
    let iri = Iri::parse("urn:example:foo").expect("urn:example:foo must parse");
    assert_eq!(iri.scheme(), Some("urn"));
    assert_eq!(iri.authority(), None);
    assert_eq!(iri.path(), "example:foo");
    assert!(iri.is_absolute());
}

#[test]
fn parse_accepts_urn_isbn() {
    // RFC 3187 ISBN URN.
    let iri = Iri::parse("urn:isbn:0-486-27557-4").expect("ISBN URN must parse");
    assert_eq!(iri.scheme(), Some("urn"));
    assert_eq!(iri.path(), "isbn:0-486-27557-4");
}

#[test]
fn parse_accepts_tag_uri() {
    // RFC 4151 tag URI with comma + colon in the path.
    let iri = Iri::parse("tag:example.com,2026:bar").expect("tag URI must parse");
    assert_eq!(iri.scheme(), Some("tag"));
    assert_eq!(iri.path(), "example.com,2026:bar");
}

#[test]
fn parse_rejects_relative_ref_with_colon_in_first_segment() {
    // Regression guard: §4.2 still applies when there is no scheme.
    // A scheme must start with ALPHA (RFC 3986 §3.1), so `1a:b/c`
    // cannot be parsed as scheme + path — the whole input is a
    // relative-path reference whose first segment `1a:b` contains a
    // colon. This is exactly the ambiguity §4.2 forbids.
    let err = Iri::parse("1a:b/c").expect_err(
        "relative reference with ':' in first path segment must be rejected (RFC 3986 §4.2)",
    );
    assert_eq!(err.code, DiagnosticCode::Syntax);
}

#[test]
fn parse_accepts_relative_ref_dot_segment_workaround() {
    // RFC 3986 §4.2 notes that `./foo:bar` is the documented workaround
    // for getting a colon into the first path segment of a relative
    // reference: the `.` is a distinct first segment, so `foo:bar`
    // sits in the second segment where colons are unrestricted.
    let iri = Iri::parse("./foo:bar").expect("./foo:bar is a valid relative reference");
    assert!(!iri.is_absolute());
    assert_eq!(iri.path(), "./foo:bar");
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
fn to_uri_preserves_pct_hex_case_in_path() {
    // Pin: no hex case fold.
    let iri = Iri::parse("http://example/a%2f%2F").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://example/a%2f%2F");
}

// ----------------------------------------------------------------------
// IDN ToASCII (RFC 3490 / UTS 46) — ADR-0004 runtime IETF-RFC carve-out.
// Covered in docs/spec-readings/iri/idna-host-normalisation-pin.md.
// ----------------------------------------------------------------------

#[test]
fn to_uri_plain_ascii_host_unchanged() {
    // ASCII-only host: must round-trip through our local lowercase
    // path, not through `idna` (which would reject e.g. underscores
    // our parser legitimately accepts).
    let iri = Iri::parse("http://example.com/a").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://example.com/a");
}

#[test]
fn to_uri_lowercase_folds_ascii_host() {
    // Host case-fold is RFC 3490 §4 step 2. Already covered elsewhere
    // but repeated here to keep the IDN-group complete.
    let iri = Iri::parse("http://EXAMPLE.COM/").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://example.com/");
}

#[test]
fn to_uri_single_label_unicode_to_ace() {
    // Single Unicode label becomes a single ACE label.
    let iri = Iri::parse("http://münchen.de/").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://xn--mnchen-3ya.de/");
}

#[test]
fn to_uri_mixed_unicode_and_ascii_labels() {
    // Only the Unicode label becomes ACE; the ASCII labels stay ASCII.
    let iri = Iri::parse("http://api.münchen.de/v1").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://api.xn--mnchen-3ya.de/v1");
}

#[test]
fn to_uri_idn_reject_falls_back_to_pct_encode() {
    // Exercising the `idna::domain_to_ascii_strict` → `Err` fallback
    // path via the public API requires a code point that (a) our
    // IRI parser admits in a host via RFC 3987 §2.2 `ucschar`, and
    // (b) UTS 46 strict rejects. That intersection is empty in
    // practice: the `ucschar` range excludes bidi/format/unassigned
    // code points that UTS 46 disallows, so every non-ASCII host we
    // can parse is one `idna` accepts.
    //
    // The fallback code path in `encode_host` is therefore not
    // reachable from this end. It remains load-bearing as a defence
    // against direct `to_uri` callers that bypass `Iri::parse` with
    // hand-crafted `Iri` values. When that API arrives, this test
    // grows teeth. For now it documents the invariant.
    let iri = Iri::parse("http://münchen.de/").unwrap();
    // No fallback fired; idna succeeded and emitted the ACE form.
    assert_eq!(iri.to_uri().unwrap(), "http://xn--mnchen-3ya.de/");
}

#[test]
fn to_uri_idn_round_trip_is_idempotent() {
    // Running `to_uri` on the already-URI output must be a no-op in
    // the host component.
    let iri = Iri::parse("http://münchen.de/path").unwrap();
    let once = iri.to_uri().unwrap();
    let again = Iri::parse(&once).unwrap().to_uri().unwrap();
    assert_eq!(once, again);
    assert_eq!(again, "http://xn--mnchen-3ya.de/path");
}

#[test]
fn to_uri_empty_host_falls_back() {
    // file:/// has an empty authority. `idna::domain_to_ascii_strict`
    // rejects the empty string, so our fallback takes over and emits
    // the empty host verbatim — no `xn--` on a phantom label.
    let iri = Iri::parse("file:///tmp/x").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "file:///tmp/x");
}

#[test]
fn to_uri_already_ace_host_preserved() {
    // Pre-encoded ACE labels must round-trip exactly; the ASCII
    // short-circuit in `encode_host` skips the `idna` call entirely
    // so there is no decode/encode drift.
    let iri = Iri::parse("http://xn--mnchen-3ya.de/").unwrap();
    assert_eq!(iri.to_uri().unwrap(), "http://xn--mnchen-3ya.de/");
}

#[test]
fn normalise_preserves_ace_host_verbatim() {
    // Already-ACE hosts are ASCII by construction; `normalise`'s
    // ASCII-lowercase pass is a no-op on them.
    let iri = Iri::parse("http://xn--mnchen-3ya.de/a/./b").unwrap().normalise();
    assert_eq!(iri.host(), Some("xn--mnchen-3ya.de"));
    assert_eq!(iri.path(), "/a/b");
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
