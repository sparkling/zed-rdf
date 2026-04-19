//! Independent shadow implementation of N-Triples and N-Quads parsers.
//!
//! This crate is a **disjoint** second implementation written for the
//! verification-v1 sweep (ADR-0020). It intentionally uses a different
//! internal structure from the main `rdf-ntriples` crate so that the
//! diff-harness can catch bugs in either independently.
//!
//! Both parsers implement [`rdf_diff::Parser`] and are gated behind the
//! `shadow` feature flag. Without the `shadow` feature the crate compiles
//! as an empty shell, satisfying workspace membership at zero cost.
//!
//! # Conformance notes
//!
//! - W3C N-Triples recommendation (2014): <https://www.w3.org/TR/n-triples/>
//! - W3C N-Quads recommendation (2014): <https://www.w3.org/TR/n-quads/>
//! - Unicode escapes `\uXXXX` and `\UXXXXXXXX` are decoded during parsing.
//! - Literal lexical forms are preserved exactly as written (no trimming or
//!   normalisation beyond escape decoding).
//! - Line terminators: LF (`\n`), CRLF (`\r\n`), and bare CR (`\r`) are all
//!   accepted.
//! - A leading UTF-8 BOM (`U+FEFF`) on the first line is silently consumed.
//! - Blank-node labels are re-mapped to a deterministic per-document sequence
//!   for the diff harness.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[cfg(feature = "shadow")]
pub mod nquads;
#[cfg(feature = "shadow")]
pub mod ntriples;

#[cfg(feature = "shadow")]
mod diagnostics;
#[cfg(feature = "shadow")]
mod lexer;
#[cfg(feature = "shadow")]
mod unescape;

#[cfg(feature = "shadow")]
pub use diagnostics::DiagnosticsBuilder;
