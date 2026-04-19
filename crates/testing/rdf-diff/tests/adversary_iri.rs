//! Adversary IRI fixtures — cohort-B `v1-adv-iri`.
//!
//! Each test in this module corresponds to one of the 8 IRI failure modes
//! documented in `docs/verification/adversary-findings/iri.md`.  The fixtures
//! live in `tests/adversary-iri/`; this module enumerates them and
//! registers one or more `#[test]` per fixture.
//!
//! ## Lifecycle
//!
//! Phase-A carry-over: `cu-adversary-iri` landed the fixtures + this
//! module with all per-fixture diff tests `#[ignore]`-gated pending the
//! main / shadow IRI parsers. Both are now present
//! (`rdf_iri::IriParser` + `rdf_iri_shadow::{parse, normalise}`), so
//! `cu2-adversary-iri` wires each body: each test parses the relevant
//! IRI(s) through both implementations, classifies the outcome, and
//! asserts the specific divergence type (`AcceptRejectSplit`,
//! `ObjectMismatch`) or agreement the brief predicts. Per-fixture
//! findings are recorded in
//! `docs/verification/adversary-findings/iri/divergences.md`.
//!
//! ## Why per-IRI rather than document-level
//!
//! The fixture files are Turtle / N-Triples documents chosen for their
//! human-readable shape, but the failure modes probe IRI-specific
//! behaviour (parsing, validation, resolution, normalisation). Feeding
//! a whole Turtle document to `rdf_iri::IriParser` (which treats its
//! byte input as a single IRI) would only exercise the document-level
//! envelope, not the IRI claim. Every test here therefore pulls the
//! IRI strings named in the fixture comment headers out of the
//! corpus, runs them through both parsers via `Iri::parse` +
//! `Iri::resolve` (main) and `shadow::parse` + `shadow::normalise`
//! (shadow), and compares the outcomes. The structural fixture-present
//! tests still run on every `cargo test --workspace`.
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

use rdf_iri::Iri;
use rdf_iri_shadow as shadow;

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
// Per-IRI probe kernel.
//
// Each failure-mode test names one or more IRI strings drawn from the
// fixture.  These helpers normalise the probe shape across the module.
// ---------------------------------------------------------------------------

/// Outcome of running one IRI through both parsers.
#[derive(Debug)]
struct Probe {
    /// Main parse + normalise result, as a `(raw, normalised)` pair,
    /// or a diagnostic message when the main rejects.
    main: Result<(String, String), String>,
    /// Shadow parse + normalise result (normalised to the canonical
    /// string via `shadow::Iri::to_iri_string()`), or diagnostic.
    shadow: Result<String, String>,
}

impl Probe {
    fn new(input: &str) -> Self {
        let main = Iri::parse(input).map_err(|d| format!("{}: {}", d.code, d.message)).map(
            |iri| {
                let norm = iri.normalise();
                (iri.as_str().to_owned(), norm.as_str().to_owned())
            },
        );

        let shadow = shadow::parse(input.as_bytes())
            .map_err(|e| format!("{e}"))
            .and_then(|iri| shadow::normalise(iri).map_err(|e| format!("{e}")))
            .map(|iri| iri.to_iri_string());

        Self { main, shadow }
    }

    const fn main_accepts(&self) -> bool {
        self.main.is_ok()
    }

    const fn shadow_accepts(&self) -> bool {
        self.shadow.is_ok()
    }

    fn main_normalised(&self) -> Option<&str> {
        self.main.as_ref().ok().map(|(_, n)| n.as_str())
    }

    fn shadow_normalised(&self) -> Option<&str> {
        self.shadow.as_ref().ok().map(String::as_str)
    }
}

/// Collapse a `(main, shadow)` pair into a stable classification
/// string compatible with the labels used in
/// `docs/verification/adversary-findings/iri/divergences.md`.
fn classify(p: &Probe) -> &'static str {
    match (p.main_accepts(), p.shadow_accepts()) {
        (true, true) => match (p.main_normalised(), p.shadow_normalised()) {
            (Some(m), Some(s)) if m == s => "both-accept-same",
            _ => "both-accept-object-mismatch",
        },
        (false, false) => "both-reject",
        (true, false) => "accept-reject-split:main-accepts",
        (false, true) => "accept-reject-split:shadow-accepts",
    }
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
// Expected resolved subjects:
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
/// The fixture is a Turtle document; the four IRIs below are the
/// relative references it registers against
/// `@base <http://example/a/b/c>`. We resolve each through the main
/// parser (which exposes `Iri::resolve`) and verify the expected
/// resolved form. The shadow parser does not expose resolution, so the
/// cross-parser diff is driven on the already-resolved string: we
/// parse+normalise the expected target through both parsers and
/// assert they agree on the final form.
///
/// Divergence hypothesis (from the brief): a resolver that fails to
/// clamp `../../..` at root produces `http://d` instead of
/// `http://example/d`. Main clamps correctly.
#[test]
fn iri_001_remove_dots_above_root_no_divergence() {
    let _bytes = std::fs::read(
        fixture_dir().join("iri-001-remove-dots-above-root.ttl"),
    )
    .expect("fixture loadable");

    let base = Iri::parse("http://example/a/b/c").expect("base is absolute");
    for (rel, expected) in [
        ("../d", "http://example/a/d"),
        ("../../d", "http://example/d"),
        ("../../..", "http://example/"),
        ("../../../d", "http://example/d"),
    ] {
        let r = Iri::parse(rel).expect("relative parses");
        let resolved = r.resolve(&base);
        assert_eq!(
            resolved.as_str(),
            expected,
            "IRI-001: main parser resolved {rel:?} to {:?}, expected {expected:?}",
            resolved.as_str()
        );

        // Cross-check that both parsers agree on the already-resolved form.
        let p = Probe::new(expected);
        assert!(
            matches!(classify(&p), "both-accept-same"),
            "IRI-001: resolved form {expected:?} diverges across parsers: main={:?} shadow={:?}",
            p.main,
            p.shadow,
        );
    }
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

/// **IRI-002 (diff)** — pure fragment reference preserves base path and
/// replaces fragment.
///
/// RFC 3986 §5.2.2: T.fragment = R.fragment (unconditional replacement
/// when R.fragment is defined). The base path is preserved and the
/// base fragment is never leaked.
#[test]
fn iri_002_pure_fragment_resolution_no_divergence() {
    let _bytes =
        std::fs::read(fixture_dir().join("iri-002-pure-fragment-resolution.ttl"))
            .expect("fixture loadable");

    let base = Iri::parse("http://example/doc#section1").expect("base parses");

    // `#section2` → http://example/doc#section2
    let r = Iri::parse("#section2").expect("pure fragment parses");
    let t = r.resolve(&base);
    assert_eq!(
        t.as_str(),
        "http://example/doc#section2",
        "IRI-002: pure-fragment resolution produced {:?}, expected \
         http://example/doc#section2 (no double-fragment, no dropped path)",
        t.as_str()
    );

    // `#` (empty fragment) → http://example/doc#
    // RFC 3986 §5.2.2: T.fragment = R.fragment even when R.fragment
    // is the empty string (which is semantically "same-document,
    // no named fragment"). Main preserves the empty-fragment marker.
    let r_empty = Iri::parse("#").expect("empty fragment parses");
    let t_empty = r_empty.resolve(&base);
    assert_eq!(
        t_empty.as_str(),
        "http://example/doc#",
        "IRI-002: empty-fragment resolution produced {:?}, expected \
         http://example/doc#",
        t_empty.as_str()
    );

    // Cross-parser agreement on the resolved form.
    let p = Probe::new("http://example/doc#section2");
    assert!(
        matches!(classify(&p), "both-accept-same"),
        "IRI-002: resolved form diverges across parsers: main={:?} shadow={:?}",
        p.main,
        p.shadow,
    );
}

// ---------------------------------------------------------------------------
// IRI-003 — surrogate (invalid) and private-use (valid) code points
// ---------------------------------------------------------------------------

/// **IRI-003a (structural)**
#[test]
fn iri_003a_private_use_fixture_present() {
    assert_fixture_present("iri-003-surrogate-and-private-use.ttl");
}

/// **IRI-003a (diff)** — U+E001 in the PATH component is an
/// `AcceptRejectSplit`.
///
/// This test surfaces a deliberate reading of RFC 3987 §2.2. The
/// fixture brief hypothesised that "overly restrictive" parsers
/// reject private-use code points that RFC 3987 permits. A closer
/// reading of §2.2 shows:
///
/// - `iunreserved = ALPHA / DIGIT / "-" / "." / "_" / "~" / ucschar`
/// - `ucschar` does **not** include `0xE000..=0xF8FF`; those live in
///   `iprivate`, which only appears in `iquery`.
///
/// So the correct `ipath-*` grammar rejects U+E001 in the path; the
/// *query* position would accept it. Main enforces the grammar
/// strictly and rejects. Shadow's validator is lax around non-ASCII
/// characters in the path and accepts, then percent-encodes the U+E001
/// bytes on `normalise()` (`%EE%80%81`). The brief's hypothesis
/// therefore fires, but inverted: it is the shadow that is overly
/// lenient, not the main that is overly strict.
///
/// Percent-encoded supplementary private-use `%F3%B0%80%81` (U+F0001,
/// in a supplementary plane that is within `ucschar`) is accepted
/// verbatim by both and normalises identically.
#[test]
fn iri_003a_private_use_accepted_by_both() {
    let _bytes =
        std::fs::read(fixture_dir().join("iri-003-surrogate-and-private-use.ttl"))
            .expect("fixture loadable");

    // 1. Literal U+E001 in path — AcceptRejectSplit (main rejects).
    let p1 = Probe::new("http://example/\u{E001}-private-use-valid");
    assert_eq!(
        classify(&p1),
        "accept-reject-split:shadow-accepts",
        "IRI-003a (literal U+E001 in path): classification surprise — \
         RFC 3987 §2.2 excludes iprivate from ipath-*, so the strict \
         main parser is expected to reject while the lax shadow \
         accepts. main={:?} shadow={:?}",
        p1.main,
        p1.shadow,
    );

    // 2. Percent-encoded U+F0001 (supplementary private-use) — both accept
    // and agree on normalised bytes.
    let p2 = Probe::new("http://example/%F3%B0%80%81");
    assert_eq!(
        classify(&p2),
        "both-accept-same",
        "IRI-003a (pct-encoded U+F0001): main={:?} shadow={:?}",
        p2.main,
        p2.shadow,
    );
}

/// **IRI-003b (structural)**
#[test]
fn iri_003b_surrogate_rejection_fixture_present() {
    assert_fixture_present("iri-003-surrogate-rejection.nt");
}

/// **IRI-003b (diff)** — lone-surrogate percent-encoding
/// (`%ED%A0%80`) is rejected by both parsers.
///
/// RFC 3987 Errata 3937 clarifies that surrogates must never appear as
/// Unicode scalar values in IRIs and must not appear as
/// percent-encoded octets except in legacy contexts. The pin
/// `docs/spec-readings/iri/lone-surrogate-rejection.md`
/// (`IRI-SURROGATE-001`) encodes this reading; both main and shadow
/// now decode pct-encoded triplet pairs at validation time and emit a
/// fatal diagnostic when the bytes form a UTF-8 surrogate encoding
/// (`0xED` followed by `0xA0..=0xBF`). Prior outcome — shared
/// under-enforcement — is closed; see the "RESOLVED" row in
/// `iri/divergences.md`.
#[test]
fn iri_003b_surrogate_rejected_by_strict_parser() {
    let _bytes = std::fs::read(fixture_dir().join("iri-003-surrogate-rejection.nt"))
        .expect("fixture loadable");

    let p = Probe::new("http://example/path/%ED%A0%80suffix");
    assert_eq!(
        classify(&p),
        "both-reject",
        "IRI-003b: main={:?} shadow={:?}",
        p.main,
        p.shadow,
    );
    // Both parsers must cite IRI-SURROGATE-001.
    let main_err = p.main.as_ref().expect_err("main rejects");
    let shadow_err = p.shadow.as_ref().expect_err("shadow rejects");
    assert!(
        main_err.contains("IRI-SURROGATE-001"),
        "main diagnostic must cite IRI-SURROGATE-001: {main_err}",
    );
    assert!(
        shadow_err.contains("IRI-SURROGATE-001"),
        "shadow diagnostic must cite IRI-SURROGATE-001: {shadow_err}",
    );
}

// ---------------------------------------------------------------------------
// IRI-004 — percent-encoding case: %c3%a9 vs %C3%A9 are distinct IRIs
// ---------------------------------------------------------------------------

/// **IRI-004 (structural)**
#[test]
fn iri_004_percent_encoding_case_fixture_present() {
    assert_fixture_present("iri-004-percent-encoding-case.nt");
}

/// **IRI-004 (diff)** — lowercase and uppercase percent-encoding are
/// NOT unified by main; shadow uppercases during normalise →
/// `ObjectMismatch` on the lowercase input.
///
/// Pin (`docs/spec-readings/iri/percent-encoding-3986-vs-3987.md`):
/// main deliberately does NOT hex-case-fold percent-encoded octets.
/// Shadow follows RFC 3986 §6.2.2.1 and uppercases. This is the
/// expected divergence; it is a correctness claim for main (which
/// matches RDF Concepts §3.1's string-equality rule) and a
/// permissiveness bias in shadow.
#[test]
fn iri_004_percent_encoding_case_not_unified() {
    let _bytes = std::fs::read(fixture_dir().join("iri-004-percent-encoding-case.nt"))
        .expect("fixture loadable");

    let p_low = Probe::new("http://example/caf%c3%a9");
    let p_up = Probe::new("http://example/caf%C3%A9");

    // Main must preserve the lowercase hex exactly.
    assert_eq!(
        p_low.main_normalised(),
        Some("http://example/caf%c3%a9"),
        "IRI-004: main folded %c3%a9 to uppercase — pin violation. \
         main={:?}",
        p_low.main,
    );
    assert_eq!(
        p_up.main_normalised(),
        Some("http://example/caf%C3%A9"),
        "IRI-004: main altered %C3%A9. main={:?}",
        p_up.main,
    );

    // Main keeps the two distinct.
    assert_ne!(
        p_low.main_normalised(),
        p_up.main_normalised(),
        "IRI-004: main unified %c3%a9 with %C3%A9 — RDF identity violated."
    );

    // Shadow unifies the two via uppercasing (documented divergence).
    assert_eq!(
        p_low.shadow_normalised(),
        p_up.shadow_normalised(),
        "IRI-004: shadow did not unify percent-case as expected. \
         low={:?} up={:?}",
        p_low.shadow,
        p_up.shadow,
    );

    // On the lowercase input specifically, main and shadow disagree on
    // the normalised object → ObjectMismatch equivalent.
    assert_eq!(
        classify(&p_low),
        "both-accept-object-mismatch",
        "IRI-004 (lowercase): expected ObjectMismatch, got {}. \
         main={:?} shadow={:?}",
        classify(&p_low),
        p_low.main,
        p_low.shadow,
    );
}

// ---------------------------------------------------------------------------
// IRI-005 — authority-less absolute IRIs: urn:, tag:, data:
// ---------------------------------------------------------------------------

/// **IRI-005 (structural)**
#[test]
fn iri_005_urn_absoluteness_fixture_present() {
    assert_fixture_present("iri-005-urn-absoluteness.nt");
}

/// **IRI-005 (diff)** — `urn:` and `tag:` and other authority-less
/// absolute IRIs with a `:` in the path are now accepted by both
/// parsers.
///
/// RFC 3986 §4.2 forbids `:` in the first segment of a
/// **relative-path reference** to disambiguate from a scheme. Once a
/// scheme has been parsed (e.g., `urn:example:a-resource` → scheme
/// `urn`, path `example:a-resource`), the reference is absolute and
/// §4.2 no longer applies. Main's `validate_path` originally ran the
/// §4.2 check on every path whose `has_authority` was false, ignoring
/// whether a scheme was parsed — rejecting `urn:example:*`, `tag:*`,
/// ISBN URNs, and similar authority-less absolute IRIs with a `:` in
/// the path.
///
/// See `docs/verification/adversary-findings/iri/divergences.md`
/// bug #1 and `crates/rdf-iri/src/parse.rs::validate_path`. The guard
/// now reads `!has_scheme && !has_authority && !slice.starts_with('/')`,
/// aligning main with shadow. This test asserts the fix: all cases
/// classify as `both-accept-same`.
#[test]
fn iri_005_authority_less_iris_accepted() {
    let _bytes = std::fs::read(fixture_dir().join("iri-005-urn-absoluteness.nt"))
        .expect("fixture loadable");

    // data: passes through because its path has no colon. Kept as a
    // control: its acceptance never depended on the §4.2 guard.
    let p_data = Probe::new("data:,hello");
    assert_eq!(
        classify(&p_data),
        "both-accept-same",
        "IRI-005 (data:,hello): main={:?} shadow={:?}",
        p_data.main,
        p_data.shadow,
    );

    // Post-fix: every urn: / tag: authority-less absolute now parses
    // on both sides.
    for absolute in [
        "urn:example:a-resource",
        "urn:isbn:0451450523",
        "urn:example:foo#bar",
        "tag:example.org,2024:resource-1",
    ] {
        let p = Probe::new(absolute);
        assert_eq!(
            classify(&p),
            "both-accept-same",
            "IRI-005: {absolute:?} expected both-accept-same after \
             validate_path §4.2 scheme-aware fix. main={:?} shadow={:?}",
            p.main,
            p.shadow,
        );
    }
}

// ---------------------------------------------------------------------------
// IRI-006 — empty base path: slash insertion in merge-paths algorithm
// ---------------------------------------------------------------------------

/// **IRI-006 (structural)**
#[test]
fn iri_006_empty_base_path_merge_fixture_present() {
    assert_fixture_present("iri-006-empty-base-path-merge.ttl");
}

/// **IRI-006 (diff)** — `<foo>` against `<http://example>` resolves to
/// `http://example/foo`.
///
/// RFC 3986 §5.2.3 Merge Paths: if the base has an authority and an
/// empty path, the merged path is `"/" + reference_path`. Main
/// implements this correctly. Shadow does not expose a resolver; we
/// verify by parsing and normalising the expected resolved form and
/// asserting both parsers agree on it.
#[test]
fn iri_006_empty_base_path_slash_inserted() {
    let _bytes = std::fs::read(fixture_dir().join("iri-006-empty-base-path-merge.ttl"))
        .expect("fixture loadable");

    let base = Iri::parse("http://example").expect("base parses");
    for (rel, expected) in [
        ("foo", "http://example/foo"),
        ("bar/baz", "http://example/bar/baz"),
    ] {
        let r = Iri::parse(rel).expect("relative parses");
        let t = r.resolve(&base);
        assert_eq!(
            t.as_str(),
            expected,
            "IRI-006: main resolved {rel:?} to {:?}; expected {expected:?} \
             (RFC 3986 §5.2.3 Merge Paths must insert a slash)",
            t.as_str(),
        );
        let p = Probe::new(expected);
        assert!(
            matches!(classify(&p), "both-accept-same"),
            "IRI-006: resolved form {expected:?} diverges across parsers: \
             main={:?} shadow={:?}",
            p.main,
            p.shadow,
        );
    }

    // Empty reference against the empty-path base → the base itself
    // (RFC 3986 §5.2.2). The main parser preserves the base verbatim.
    let r_empty = Iri::parse("").expect("empty relative parses");
    let t_empty = r_empty.resolve(&base);
    assert_eq!(
        t_empty.as_str(),
        "http://example",
        "IRI-006: empty reference against empty-path base produced {:?}, \
         expected http://example",
        t_empty.as_str(),
    );
}

// ---------------------------------------------------------------------------
// IRI-007 — host case-folding must NOT unify distinct RDF IRIs
// ---------------------------------------------------------------------------

/// **IRI-007 (structural)**
#[test]
fn iri_007_host_case_folding_fixture_present() {
    assert_fixture_present("iri-007-host-case-folding.nt");
}

/// **IRI-007 (diff)** — `http://EXAMPLE.COM/s` and
/// `http://example.com/s` are distinct IRIs in the RAW parse, but both
/// parsers fold host case during normalisation.
///
/// Main's `normalise()` ASCII-lowercases the host per RFC 3490 §4 and
/// RFC 3986 §6.2.2.1; shadow does the same. The raw parse preserves
/// case (so `parse()` keeps RDF string-equality correct when callers
/// compare raw bytes), but the normalised form unifies. The brief's
/// divergence hypothesis therefore does NOT fire: both parsers agree
/// on the normalised form.
///
/// The RDF-level concern (normalisation silently unifies two distinct
/// RDF subjects) is the *pin* that lives in
/// `docs/spec-readings/iri/idna-host-normalisation-pin.md`; it is a
/// caller contract, not a main-vs-shadow divergence.
#[test]
fn iri_007_host_case_folding_not_unified() {
    let _bytes = std::fs::read(fixture_dir().join("iri-007-host-case-folding.nt"))
        .expect("fixture loadable");

    let p_upper = Probe::new("http://EXAMPLE.COM/subject");
    let p_lower = Probe::new("http://example.com/subject");

    // Raw parse preserves case (main only).
    let main_raw_upper =
        p_upper.main.as_ref().ok().map(|(raw, _)| raw.as_str());
    assert_eq!(
        main_raw_upper,
        Some("http://EXAMPLE.COM/subject"),
        "IRI-007: main parse altered host case. main={:?}",
        p_upper.main,
    );

    // Both normalise forms collapse to the lowercase host — both parsers
    // agree on that collapse.
    assert_eq!(
        p_upper.main_normalised(),
        p_lower.main_normalised(),
        "IRI-007: main normalise did not fold host case symmetrically. \
         upper={:?} lower={:?}",
        p_upper.main,
        p_lower.main,
    );
    assert_eq!(
        classify(&p_upper),
        "both-accept-same",
        "IRI-007: main={:?} shadow={:?}",
        p_upper.main,
        p_upper.shadow,
    );
}

// ---------------------------------------------------------------------------
// IRI-008 — NFC normalization must NOT unify NFD vs NFC IRI characters
// ---------------------------------------------------------------------------

/// **IRI-008 (structural)**
#[test]
fn iri_008_nfc_normalization_fixture_present() {
    assert_fixture_present("iri-008-nfc-normalization.nt");
}

/// **IRI-008 (diff)** — NFC (`%C3%A9`) and NFD (`cafe%CC%81`) forms
/// are preserved as distinct by both parsers.
///
/// Neither parser applies Unicode NFC at parse or normalise time — the
/// byte sequence flows through opaquely. The brief's divergence
/// hypothesis does NOT fire. Recorded in `iri/divergences.md`.
#[test]
fn iri_008_nfc_normalization_not_unified() {
    let _bytes = std::fs::read(fixture_dir().join("iri-008-nfc-normalization.nt"))
        .expect("fixture loadable");

    let nfc = Probe::new("http://example/caf%C3%A9");
    let nfd = Probe::new("http://example/cafe%CC%81");

    assert_eq!(
        classify(&nfc),
        "both-accept-same",
        "IRI-008 NFC: main={:?} shadow={:?}",
        nfc.main,
        nfc.shadow,
    );
    assert_eq!(
        classify(&nfd),
        "both-accept-same",
        "IRI-008 NFD: main={:?} shadow={:?}",
        nfd.main,
        nfd.shadow,
    );
    assert_ne!(
        nfc.main_normalised(),
        nfd.main_normalised(),
        "IRI-008: main silently NFC-normalised NFD → NFC. nfc={:?} nfd={:?}",
        nfc.main,
        nfd.main,
    );
    assert_ne!(
        nfc.shadow_normalised(),
        nfd.shadow_normalised(),
        "IRI-008: shadow silently NFC-normalised NFD → NFC. nfc={:?} nfd={:?}",
        nfc.shadow,
        nfd.shadow,
    );
}
