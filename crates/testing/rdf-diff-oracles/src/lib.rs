//! Reference-parser oracle adapters for the verification-v1 diff harness.
//!
//! This crate wraps permitted reference parsers (ADR-0019 §1) and exposes
//! each through the frozen [`rdf_diff::Parser`] trait (ADR-0020 §1.4).
//! Every upstream parser listed below is pulled **strictly as a
//! `[dev-dependency]`**; the crate's `[dependencies]` section references
//! only the frozen trait crate, and `cargo tree -p rdf-diff-oracles -e
//! normal` must show zero `ox*` / `sophia_*` edges.
//!
//! ## Layout
//!
//! | Adapter                | Upstream crate    | Formats                                |
//! |------------------------|-------------------|----------------------------------------|
//! | [`oxttl_adapter`]      | `oxttl`           | `N-Triples`, `N-Quads`, `Turtle`, `TriG`       |
//! | [`oxrdfxml_adapter`]   | `oxrdfxml`        | RDF/XML                                |
//! | [`oxjsonld_adapter`]   | `oxjsonld`        | JSON-LD                                |
//! | [`oxsparql_adapter`]   | `spargebra`*      | SPARQL 1.1 query + update syntax       |
//! | [`sophia_adapter`]     | `sophia_turtle`   | Turtle family (secondary reference)    |
//!
//! *ADR-0019 §1 names this role `oxsparql-syntax`; the crate is
//! unpublished, so we pin oxigraph's `spargebra` crate (same source
//! tree, same maintainer). The role, not the crate name, is what the
//! ADR governs.
//!
//! ## Test-scope adapters
//!
//! Because ADR-0019 §1 forbids the upstream parsers from appearing in
//! `[dependencies]`, the adapter modules below are compiled under
//! `#[cfg(test)]` together with the per-oracle Cargo feature gate. In a
//! normal `cargo check` the modules are empty; in `cargo check --tests
//! --all-features` they compile against the dev-deps. The crate's
//! smoke test (`mod smoke`) exercises the adapters from within the
//! same test build.
//!
//! Downstream harness crates consume adapters by depending on
//! `rdf-diff-oracles` as a **dev-dependency** of their own, enabling
//! the specific `oracle-*` features they need. This is the only graph
//! shape permitted by the carve-out.
//!
//! ## References
//!
//! - `crates/testing/rdf-diff/src/lib.rs` — frozen [`Parser`] trait.
//! - `docs/adr/0019-independent-verification.md` §1 — oracle carve-out.
//! - `docs/adr/0020-verification-implementation-plan.md` §1.4 — sweep
//!   integration contract.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// ---------------------------------------------------------------------------
// Adapter modules.
//
// Each is `pub mod` and gated on `#[cfg(test)]` + the per-oracle feature.
// The `cfg(test)` gate is what keeps the upstream crates inside the
// `[dev-dependencies]` graph — the lib's public surface outside of test
// builds is the empty re-export below.
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "oracle-oxttl"))]
pub mod oxttl_adapter;

#[cfg(all(test, feature = "oracle-oxrdfxml"))]
pub mod oxrdfxml_adapter;

#[cfg(all(test, feature = "oracle-oxjsonld"))]
pub mod oxjsonld_adapter;

#[cfg(all(test, feature = "oracle-oxsparql"))]
pub mod oxsparql_adapter;

#[cfg(all(test, feature = "oracle-sophia"))]
pub mod sophia_adapter;

// ---------------------------------------------------------------------------
// Re-exports for downstream consumers. Kept intentionally minimal; the
// crate is a thin adapter layer.
// ---------------------------------------------------------------------------

pub use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

/// Helper that translates any adapter-local fatal error into the frozen
/// [`Diagnostics`] shape. Adapters call this when the upstream parser
/// rejects the input; the resulting [`Diagnostics`] has `fatal = true`.
///
/// This helper is `pub(crate)` and lives here rather than in each
/// adapter to guarantee every adapter reports rejections identically —
/// a prerequisite for the `AcceptRejectSplit` divergence category in
/// [`rdf_diff::Divergence`].
#[cfg(test)]
pub(crate) fn fatal<E: core::fmt::Display>(parser_id: &str, err: E) -> Diagnostics {
    Diagnostics {
        messages: vec![format!("{parser_id}: {err}")],
        fatal: true,
    }
}

// ---------------------------------------------------------------------------
// Smoke test — round-trips a trivial Turtle document through each
// enabled adapter and asserts the resulting `Facts` are canonically
// equal. Per ADR-0020 §1.4 the canonical form is produced by
// `Facts::canonicalise` (now implemented in `rdf-diff`); the smoke
// assertions below run unconditionally.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    //! Trivial round-trip smoke test.
    //!
    //! The document is the smallest non-trivial Turtle that exercises
    //! one IRI subject, one IRI predicate, and one plain-literal
    //! object. Every adapter must accept it, produce exactly one fact,
    //! and canonicalise to the same `Facts` set.

    use super::{Facts, Parser};

    /// Minimal Turtle input — single triple, ASCII, no prefixes. Any
    /// conforming parser must round-trip this.
    const TRIVIAL_TTL: &[u8] =
        b"<http://example.org/s> <http://example.org/p> \"o\" .\n";

    /// Helper: parse with `parser`, panic on reject, return the facts.
    /// Test-internal — keeps each `#[test]` body to a single assertion.
    fn parse_or_panic(parser: &dyn Parser, input: &[u8]) -> Facts {
        match parser.parse(input) {
            Ok(outcome) => outcome.facts,
            Err(diag) => panic!(
                "{} rejected trivial Turtle input: {:?}",
                parser.id(),
                diag.messages
            ),
        }
    }

    #[cfg(feature = "oracle-oxttl")]
    #[test]
    fn oxttl_round_trips_trivial_turtle() {
        let parser = super::oxttl_adapter::Adapter::new();
        let facts = parse_or_panic(&parser, TRIVIAL_TTL);
        assert_eq!(facts.set.len(), 1, "expected exactly one fact");
    }

    #[cfg(feature = "oracle-sophia")]
    #[test]
    fn sophia_round_trips_trivial_turtle() {
        let parser = super::sophia_adapter::Adapter::new();
        let facts = parse_or_panic(&parser, TRIVIAL_TTL);
        assert_eq!(facts.set.len(), 1, "expected exactly one fact");
    }

    /// Cross-oracle canonical equality on the Turtle family. Needs both
    /// `oxttl` and `sophia` adapters available; other combinations live
    /// in the harness crate, not here.
    #[cfg(all(feature = "oracle-oxttl", feature = "oracle-sophia"))]
    #[test]
    fn oxttl_and_sophia_agree_on_trivial_turtle() {
        let oxttl = super::oxttl_adapter::Adapter::new();
        let sophia = super::sophia_adapter::Adapter::new();

        let a = parse_or_panic(&oxttl, TRIVIAL_TTL);
        let b = parse_or_panic(&sophia, TRIVIAL_TTL);

        // Compare canonical fact keys only. `FactProvenance` values
        // carry the emitting parser id and are expected to differ
        // between oracles by construction.
        let keys_a: std::collections::BTreeSet<_> = a.set.keys().collect();
        let keys_b: std::collections::BTreeSet<_> = b.set.keys().collect();
        assert_eq!(
            keys_a, keys_b,
            "oxttl and sophia disagree on canonical facts for trivial Turtle"
        );
    }

    /// Compile-shape check that never panics, analogous to
    /// `api_shape_compiles` in the `rdf-diff` property harness. Keeps
    /// `cargo test` green across the sweep even before `canonicalise`
    /// is filled.
    #[test]
    fn adapter_trait_objects_compile() {
        let _parsers: Vec<Box<dyn Parser>> = vec![
            #[cfg(feature = "oracle-oxttl")]
            Box::new(super::oxttl_adapter::Adapter::new()),
            #[cfg(feature = "oracle-oxrdfxml")]
            Box::new(super::oxrdfxml_adapter::Adapter::new()),
            #[cfg(feature = "oracle-oxjsonld")]
            Box::new(super::oxjsonld_adapter::Adapter::new()),
            #[cfg(feature = "oracle-oxsparql")]
            Box::new(super::oxsparql_adapter::Adapter::new()),
            #[cfg(feature = "oracle-sophia")]
            Box::new(super::sophia_adapter::Adapter::new()),
        ];
    }
}
