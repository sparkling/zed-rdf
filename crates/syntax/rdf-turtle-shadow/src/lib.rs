//! Independent shadow implementation of Turtle 1.1 and `TriG` parsers.
//!
//! This crate is a **disjoint** second implementation written for the
//! verification-v1 sweep (ADR-0020). It intentionally uses a different
//! internal structure from the main `rdf-turtle` crate so that the
//! diff-harness can catch bugs in either independently.
//!
//! Both parsers implement [`rdf_diff::Parser`] and are gated behind the
//! `shadow` feature flag. Without the `shadow` feature the crate compiles
//! as an empty shell, satisfying workspace membership at zero cost.
//!
//! # Conformance notes
//!
//! - W3C Turtle 1.1 recommendation: <https://www.w3.org/TR/turtle/>
//! - W3C `TriG` recommendation: <https://www.w3.org/TR/trig/>
//! - `@prefix` and `@base` directives (and their SPARQL-style `PREFIX` /
//!   `BASE` forms) are fully resolved before emitting facts.
//! - Long-string literals `"""…"""` and `'''…'''` are fully supported,
//!   including embedded newlines and escaped characters.
//! - Numeric literals are typed per the XSD datatype rules:
//!   integers → `xsd:integer`, decimals → `xsd:decimal`,
//!   doubles → `xsd:double`.
//! - Blank-node scoping: blank-node labels are local to a document; re-defining
//!   `@prefix` does not reset the blank-node label space (per W3C Turtle 1.1 §6).
//! - Collections `( … )` are expanded to `rdf:first` / `rdf:rest` chains.
//! - Blank-node property lists `[ … ]` allocate a fresh blank node.
//! - Blank-node labels are re-mapped to a deterministic per-document sequence
//!   for the diff harness.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "shadow")]
pub mod trig;
#[cfg(feature = "shadow")]
pub mod turtle;

#[cfg(feature = "shadow")]
mod bnode;
#[cfg(feature = "shadow")]
mod diagnostics;
#[cfg(feature = "shadow")]
mod iri;
#[cfg(feature = "shadow")]
mod lexer;
#[cfg(feature = "shadow")]
mod literal;
#[cfg(feature = "shadow")]
mod unescape;

#[cfg(feature = "shadow")]
pub use diagnostics::DiagnosticsBuilder;
