//! Shadow RFC 3987 IRI parser/normaliser — second independent implementation
//! for the verification-v1 sweep (ADR-0019 §3, ADR-0020).
//!
//! Everything is gated behind the `shadow` Cargo feature so that
//! main-parser builds never pull in this crate's logic.  Without the
//! feature the crate compiles as an empty shell, satisfying the
//! workspace membership requirement while imposing zero cost.
//!
//! # References
//!
//! - RFC 3987 §3  — IRI syntax and normalisation
//! - RFC 3986 §3  — URI generic syntax
//! - RFC 3986 §2.3 — unreserved characters
//! - RFC 3986 §5.2 — reference resolution (path dot-segment removal)
//! - RFC 3986 §6.2 — URI normalisation
//! - RFC 3987 §3.1 — IRI-to-URI mapping (percent-encode non-ASCII)
//!
//! # Spec-reading pins
//!
//! Ambiguous productions are documented in `docs/spec-readings/iri/`.
//! Coordinate new pins with the `v1-specpins` agent.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "shadow")]
mod inner;

#[cfg(feature = "shadow")]
pub use inner::{Iri, IriError, ShadowIriParser, normalise, parse};
