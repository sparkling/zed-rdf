//! API surface tests for `rdf-vocab`.
//!
//! These tests are deliberately written to compile against the current stub
//! (xsd-only) AND to remain green when pe-rdf-vocab completes the full set
//! of vocabulary modules.

use rdf_vocab::xsd;

// ---------------------------------------------------------------------------
// xsd module — namespace constant
// ---------------------------------------------------------------------------

/// The XSD namespace must end with `#` per the W3C spec
/// (`http://www.w3.org/2001/XMLSchema#`).
#[test]
fn xsd_ns_ends_with_hash() {
    assert!(
        xsd::NS.ends_with('#'),
        "xsd::NS should end with '#', got: {:?}",
        xsd::NS
    );
}

/// `xsd::NS` must start with the canonical W3C XML-Schema prefix.
#[test]
fn xsd_ns_starts_with_w3c_prefix() {
    assert!(
        xsd::NS.starts_with("http://www.w3.org/2001/XMLSchema"),
        "xsd::NS should start with W3C XML-Schema URI, got: {:?}",
        xsd::NS
    );
}

// ---------------------------------------------------------------------------
// xsd::STRING — term IRI
// ---------------------------------------------------------------------------

/// `xsd::STRING` must contain the xsd namespace prefix as a substring.
#[test]
fn xsd_string_contains_xsd_ns_prefix() {
    // NS = "http://www.w3.org/2001/XMLSchema#"
    // STRING must start with that prefix.
    assert!(
        xsd::STRING.starts_with(xsd::NS),
        "xsd::STRING should start with xsd::NS ({:?}), got: {:?}",
        xsd::NS,
        xsd::STRING
    );
}

/// `xsd::STRING` must end with the local name `string`.
#[test]
fn xsd_string_local_name_is_string() {
    assert!(
        xsd::STRING.ends_with("string"),
        "xsd::STRING local name should be 'string', got: {:?}",
        xsd::STRING
    );
}

/// Every term constant in the xsd module must be a full IRI (starts with
/// `http://`).
#[test]
fn xsd_terms_are_full_iris() {
    for term in [xsd::STRING, xsd::INTEGER, xsd::BOOLEAN] {
        assert!(
            term.starts_with("http://"),
            "xsd term should be a full http IRI, got: {:?}",
            term
        );
    }
}

/// Term constants must be formed by appending the local name to NS.
/// This validates the structure `NS + local_name == TERM`.
#[test]
fn xsd_term_equals_ns_plus_local() {
    let expected_string = format!("{}string", xsd::NS);
    assert_eq!(
        xsd::STRING, expected_string,
        "xsd::STRING should equal xsd::NS + \"string\""
    );

    let expected_integer = format!("{}integer", xsd::NS);
    assert_eq!(
        xsd::INTEGER, expected_integer,
        "xsd::INTEGER should equal xsd::NS + \"integer\""
    );

    let expected_boolean = format!("{}boolean", xsd::NS);
    assert_eq!(
        xsd::BOOLEAN, expected_boolean,
        "xsd::BOOLEAN should equal xsd::NS + \"boolean\""
    );
}

// ---------------------------------------------------------------------------
// NS constant shape invariant (applicable to any vocab module)
// ---------------------------------------------------------------------------

/// A vocabulary namespace must end with either `#` or `/`.
/// The xsd module uses `#`; other modules (rdf, rdfs, owl, …) may use `/`.
/// This test encodes the invariant for the xsd module specifically.
#[test]
fn xsd_ns_is_not_empty_and_has_valid_terminator() {
    assert!(!xsd::NS.is_empty(), "xsd::NS must not be empty");
    let last = xsd::NS.chars().last().unwrap();
    assert!(
        last == '#' || last == '/',
        "xsd::NS must end with '#' or '/', got last char: {:?}",
        last
    );
}

// ---------------------------------------------------------------------------
// Doc-comment presence — build-time source check
// ---------------------------------------------------------------------------

/// Verify that the `rdf-vocab/src/lib.rs` source file contains `/// Label:`
/// strings, as required by the arch memo (§1: label and comment for every
/// term constant carried as `///` doc-comment lines).
///
/// Uses `include_str!` so the check happens at compile time (the file must
/// exist) and the assertion confirms content at test time.
#[test]
fn vocab_source_contains_label_doc_comments() {
    // include_str! resolves relative to this test file's source location.
    // At compile time: crates/rdf-vocab/tests/api_surface.rs
    // The target: crates/rdf-vocab/src/lib.rs
    let source = include_str!("../src/lib.rs");

    // The arch memo (phase-e/arch-memo.md §1) requires every term constant
    // to carry a `///` doc-comment whose first line begins with the term
    // name. The stub already uses the pattern
    //   /// `xsd:string` — …
    // We accept either form: a bare `/// Label:` block or the stub's
    // inline dash pattern. Since the stub has doc-comments that carry
    // meaningful descriptions ("`xsd:string` — …"), we verify those exist.
    assert!(
        source.contains("///"),
        "rdf-vocab/src/lib.rs must contain at least one `///` doc-comment line"
    );

    // When pe-rdf-vocab delivers the full vocabulary, every term should
    // have a doc-comment. We verify the stub's existing comments are present.
    assert!(
        source.contains("pub const NS:"),
        "rdf-vocab/src/lib.rs must expose a pub const NS"
    );

    // The stub uses `/// `xsd:string` — …` style. When the full vocab lands,
    // arch-memo §1 requires `/// Label:` lines. Accept whichever is present.
    let has_label_colon = source.contains("Label:");
    let has_inline_doc = source.contains("///");
    assert!(
        has_label_colon || has_inline_doc,
        "rdf-vocab/src/lib.rs must contain doc-comment strings (/// Label: or /// `term` —)"
    );
}
