//! Regression guard for ADR-0019 §1's `[dev-dependencies]`-only
//! oracle carve-out.
//!
//! This crate is test-only. The real work lives in
//! `tests/no_banned_runtime_deps.rs`, which shells out to
//! `cargo metadata`, walks the resolved dependency graph, and asserts
//! that none of the banned third-party RDF/SPARQL parser crates appear
//! on any normal (non-dev, non-build) edge reachable from a
//! non-test library or binary target.
//!
//! The library target exists only so that `cargo test -p
//! deny-regression` has something to link against; there is no
//! runtime code. See ADR-0019 §1 "Validation" and ADR-0020 §1.
//!
//! Keeping the check in a regular integration test (rather than a
//! `build.rs` or an `xtask`) means it participates in `cargo test
//! --workspace` and therefore in every CI run without any extra
//! tooling.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The canonical banned-crate set, shared with `deny.toml`.
///
/// Exposed as `pub const` so tests can reference a single source of
/// truth. If this list drifts from `deny.toml` the regression test is
/// free to also assert that invariant, but at minimum it is what
/// callers compare against.
///
/// The carve-out from ADR-0019 §1 permits these crates in
/// `[dev-dependencies]` only; runtime / build edges are rejected.
pub const BANNED_RUNTIME_CRATES: &[&str] = &[
    "oxrdf",
    "oxttl",
    "oxrdfio",
    "oxsparql-syntax",
    "oxigraph",
    "oxjsonld",
    "oxrdfxml",
    "sophia",
    "sophia_api",
    "sophia_iri",
    "sophia_inmem",
    "sophia_term",
    "sophia_turtle",
    "sophia_xml",
    "sophia_jsonld",
    "rio_turtle",
    "rio_xml",
    "rio_api",
    "rdftk_core",
    "rdftk_io",
    "horned-owl",
];
