//! Adversary IRI fixtures — cohort-B `v1-adv-iri`.
//!
//! Each test in this module corresponds to one of the 8 IRI failure modes
//! documented in `docs/verification/adversary-findings/iri.md`.  The fixtures
//! live in `tests/adversary-iri/`; this module enumerates them and
//! registers a compile-time test per fixture.
//!
//! ## Lifecycle
//!
//! All tests are `#[ignore]`-gated following the same convention as
//! `properties.rs` and `snapshots.rs`: they compile and run the
//! structural checks today (file present, bytes non-empty), but the
//! diff-level assertions are deferred until both a conformant parser and
//! a reference oracle exist.  Once `v1-diff-core` and at least one
//! shadow parser land, the `#[ignore]` attributes come off per the
//! handoff checklist.
//!
//! ## Label for `xtask verify`
//!
//! `xtask verify --adversary-iri` (to be wired by `v1-ci-wiring`) runs
//! this module by filtering `adversary_iri`.
//!
//! ADR references: ADR-0019 §4, ADR-0020 §6.5
//! Spec: RFC 3987 (IRI), RFC 3986 (URI), RDF Concepts §3.1

#![allow(clippy::missing_panics_doc)]

use std::path::{Path, PathBuf};

/// Return the absolute path to the `adversary-iri` fixture directory.
fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("adversary-iri")
}

/// Assert a fixture file exists and is non-empty.  This is the
/// always-on structural check that runs even before parsers land.
fn assert_fixture_present(name: &str) {
    let path = fixture_dir().join(name);
    assert!(
        path.exists(),
        "adversary-iri fixture missing: {}",
        path.display()
    );
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display()));
    assert!(
        !bytes.is_empty(),
        "adversary-iri fixture is empty: {}",
        path.display()
    );
}

// ---------------------------------------------------------------------------
// Structural always-on tests (run on every `cargo test --workspace`)
// ---------------------------------------------------------------------------

/// Verify all 9 adversary-iri fixture files are present and non-empty.
/// This runs without any parser dependency and catches accidental deletion.
#[test]
fn all_adversary_iri_fixtures_present() {
    for name in [
        "iri-001-remove-dots-above-root.ttl",
        "iri-002-pure-fragment-resolution.ttl",
        "iri-003-surrogate-and-private-use.ttl",
        "iri-003-surrogate-rejection.nt",
        "iri-004-percent-encoding-case.nt",
        "iri-005-urn-absoluteness.nt",
        "iri-006-empty-base-path-merge.ttl",
        "iri-007-host-case-folding.nt",
        "iri-008-nfc-normalization.nt",
    ] {
        assert_fixture_present(name);
    }
}

/// README is present in the fixture directory.
#[test]
fn adversary_iri_readme_present() {
    let readme = fixture_dir().join("README.md");
    assert!(
        readme.exists(),
        "adversary-iri README.md missing at {}",
        readme.display()
    );
}

// ---------------------------------------------------------------------------
// IRI-001 — remove-dots: above-root path clamping
//
// RFC 3986 §5.2.4 step 2C: `..` segments that would escape the root are
// treated as `/`.  `../../../d` against `http://example/a/b/c` must
// resolve to `http://example/d`, never to `http://d`.
//
// Expected resolved subjects (parsers fill these once wired):
//   <../d>         → http://example/a/d
//   <../../d>      → http://example/d
//   <../../..>     → http://example/
//   <../../../d>   → http://example/d  (clamped — cannot go above root)
// ---------------------------------------------------------------------------

/// **IRI-001 (structural)** — fixture compiles and is loadable.
#[test]
fn iri_001_remove_dots_above_root_fixture_present() {
    assert_fixture_present("iri-001-remove-dots-above-root.ttl");
}

/// **IRI-001 (diff)** — above-root `..` does not escape to a bare authority.
///
/// Unignore once `rdf-turtle` + `rdf-turtle-shadow` (or reference oracle)
/// exist and `Facts::canonicalise` / `diff` are implemented by `v1-diff-core`.
///
/// Wiring sketch:
/// ```ignore
/// let input = include_bytes!("adversary-iri/iri-001-remove-dots-above-root.ttl");
/// let main   = rdf_turtle::Parser::default().parse(input)?;
/// let oracle = rdf_turtle_oracle::Parser::default().parse(input)?;
/// let report = diff(&main.facts, &oracle.facts).unwrap();
/// // Surface: assert divergence count — any collapse of `../../../d`
/// // into a wrong authority produces a FactOnlyIn divergence.
/// assert!(report.is_clean(), "IRI-001 divergence: {:?}", report.divergences);
/// ```
#[test]
#[ignore = "unignore once rdf-turtle + oracle + v1-diff-core land (IRI-001)"]
fn iri_001_remove_dots_above_root_no_divergence() {
    // Load fixture bytes as a smoke-check that the ignore compiles.
    let input =
        std::fs::read(fixture_dir().join("iri-001-remove-dots-above-root.ttl")).unwrap();
    assert!(!input.is_empty());
    // Diff wiring goes here once parsers exist.
    todo!("wire rdf-turtle + oracle once v1-diff-core lands")
}

// ---------------------------------------------------------------------------
// IRI-002 — pure fragment reference resolution
//
// `#section2` against `http://example/doc#section1`
// must resolve to `http://example/doc#section2`.
// Base path is preserved; only the fragment is replaced.
// ---------------------------------------------------------------------------

/// **IRI-002 (structural)**
#[test]
fn iri_002_pure_fragment_resolution_fixture_present() {
    assert_fixture_present("iri-002-pure-fragment-resolution.ttl");
}

/// **IRI-002 (diff)** — pure fragment reference preserves base path and replaces fragment.
///
/// Divergence hypothesis: double-fragment (`#section1section2`) or dropped
/// base path (`http://example#section2` instead of `http://example/doc#section2`).
#[test]
#[ignore = "unignore once rdf-turtle + oracle + v1-diff-core land (IRI-002)"]
fn iri_002_pure_fragment_resolution_no_divergence() {
    let input =
        std::fs::read(fixture_dir().join("iri-002-pure-fragment-resolution.ttl")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-turtle + oracle once v1-diff-core lands")
}

// ---------------------------------------------------------------------------
// IRI-003 — surrogate (invalid) and private-use (valid) code points
// ---------------------------------------------------------------------------

/// **IRI-003a (structural)**
#[test]
fn iri_003a_private_use_fixture_present() {
    assert_fixture_present("iri-003-surrogate-and-private-use.ttl");
}

/// **IRI-003a (diff)** — private-use U+E001 in IRI path accepted by both parsers.
///
/// Divergence hypothesis: an overly restrictive implementation rejects a
/// private-use code point that RFC 3987 §2.2 explicitly permits.
#[test]
#[ignore = "unignore once rdf-turtle + oracle + v1-diff-core land (IRI-003a)"]
fn iri_003a_private_use_accepted_by_both() {
    let input =
        std::fs::read(fixture_dir().join("iri-003-surrogate-and-private-use.ttl")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-turtle + oracle once v1-diff-core lands")
}

/// **IRI-003b (structural)**
#[test]
fn iri_003b_surrogate_rejection_fixture_present() {
    assert_fixture_present("iri-003-surrogate-rejection.nt");
}

/// **IRI-003b (diff)** — surrogate `%ED%A0%80` in IRI rejected by both parsers.
///
/// Divergence hypothesis: a UTF-16 internal implementation admits the lone
/// surrogate U+D800 encoded as `%ED%A0%80`, while a strict implementation
/// rejects it.  Expected: `AcceptRejectSplit` if they disagree, or both
/// `Diagnostics { fatal: true }` if they agree on rejection.
///
/// A divergence here is the *expected* finding — one parser accepts, one
/// rejects → `AcceptRejectSplit`.
#[test]
#[ignore = "unignore once rdf-ntriples + oracle + v1-diff-core land (IRI-003b)"]
fn iri_003b_surrogate_rejected_by_strict_parser() {
    let input =
        std::fs::read(fixture_dir().join("iri-003-surrogate-rejection.nt")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-ntriples + oracle once v1-diff-core lands")
}

// ---------------------------------------------------------------------------
// IRI-004 — percent-encoding case: %c3%a9 vs %C3%A9 are distinct IRIs
// ---------------------------------------------------------------------------

/// **IRI-004 (structural)**
#[test]
fn iri_004_percent_encoding_case_fixture_present() {
    assert_fixture_present("iri-004-percent-encoding-case.nt");
}

/// **IRI-004 (diff)** — lowercase and uppercase percent-encoding are NOT unified.
///
/// Divergence hypothesis: a normalizing parser uppercases hex digits and
/// merges both subjects into one, causing one `FactOnlyIn` for each of the
/// two labels (one disappears, one doubled).
#[test]
#[ignore = "unignore once rdf-ntriples + oracle + v1-diff-core land (IRI-004)"]
fn iri_004_percent_encoding_case_not_unified() {
    let input =
        std::fs::read(fixture_dir().join("iri-004-percent-encoding-case.nt")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-ntriples + oracle once v1-diff-core lands")
}

// ---------------------------------------------------------------------------
// IRI-005 — authority-less absolute IRIs: urn:, tag:, data:
// ---------------------------------------------------------------------------

/// **IRI-005 (structural)**
#[test]
fn iri_005_urn_absoluteness_fixture_present() {
    assert_fixture_present("iri-005-urn-absoluteness.nt");
}

/// **IRI-005 (diff)** — `urn:`, `tag:`, and `data:` IRIs accepted as absolute.
///
/// Divergence hypothesis: a `://`-checking absoluteness guard rejects one
/// or more of these triples with a fatal diagnostic.  An `AcceptRejectSplit`
/// would surface here.
#[test]
#[ignore = "unignore once rdf-ntriples + oracle + v1-diff-core land (IRI-005)"]
fn iri_005_authority_less_iris_accepted() {
    let input =
        std::fs::read(fixture_dir().join("iri-005-urn-absoluteness.nt")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-ntriples + oracle once v1-diff-core lands")
}

// ---------------------------------------------------------------------------
// IRI-006 — empty base path: slash insertion in merge-paths algorithm
// ---------------------------------------------------------------------------

/// **IRI-006 (structural)**
#[test]
fn iri_006_empty_base_path_merge_fixture_present() {
    assert_fixture_present("iri-006-empty-base-path-merge.ttl");
}

/// **IRI-006 (diff)** — `<foo>` against `<http://example>` resolves to `http://example/foo`.
///
/// Divergence hypothesis: string-concatenation resolver produces
/// `http://examplefoo` (wrong authority) instead of `http://example/foo`.
/// Manifests as `FactOnlyIn` divergences for all three relative references
/// in the fixture.
#[test]
#[ignore = "unignore once rdf-turtle + oracle + v1-diff-core land (IRI-006)"]
fn iri_006_empty_base_path_slash_inserted() {
    let input =
        std::fs::read(fixture_dir().join("iri-006-empty-base-path-merge.ttl")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-turtle + oracle once v1-diff-core lands")
}

// ---------------------------------------------------------------------------
// IRI-007 — host case-folding must NOT unify distinct RDF IRIs
// ---------------------------------------------------------------------------

/// **IRI-007 (structural)**
#[test]
fn iri_007_host_case_folding_fixture_present() {
    assert_fixture_present("iri-007-host-case-folding.nt");
}

/// **IRI-007 (diff)** — `http://EXAMPLE.COM/s` and `http://example.com/s` are distinct.
///
/// Divergence hypothesis: a parser that lowercases the host at parse time
/// unifies both subjects.  In the diff, one `FactOnlyIn` divergence appears
/// for the `"uppercase-host"` label because both facts land on the same
/// lowercased subject.
#[test]
#[ignore = "unignore once rdf-ntriples + oracle + v1-diff-core land (IRI-007)"]
fn iri_007_host_case_folding_not_unified() {
    let input =
        std::fs::read(fixture_dir().join("iri-007-host-case-folding.nt")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-ntriples + oracle once v1-diff-core lands")
}

// ---------------------------------------------------------------------------
// IRI-008 — NFC normalization must NOT unify NFD vs NFC IRI characters
// ---------------------------------------------------------------------------

/// **IRI-008 (structural)**
#[test]
fn iri_008_nfc_normalization_fixture_present() {
    assert_fixture_present("iri-008-nfc-normalization.nt");
}

/// **IRI-008 (diff)** — NFC and NFD percent-encoded forms are distinct RDF IRIs.
///
/// Divergence hypothesis: a parser using an NFC-normalizing string library
/// will unify `%C3%A9` (NFC) and `%65%CC%81` (NFD) into the same subject,
/// producing an `ObjectMismatch` or `FactOnlyIn` divergence.
#[test]
#[ignore = "unignore once rdf-ntriples + oracle + v1-diff-core land (IRI-008)"]
fn iri_008_nfc_normalization_not_unified() {
    let input =
        std::fs::read(fixture_dir().join("iri-008-nfc-normalization.nt")).unwrap();
    assert!(!input.is_empty());
    todo!("wire rdf-ntriples + oracle once v1-diff-core lands")
}
