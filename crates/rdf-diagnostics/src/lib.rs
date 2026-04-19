//! Shared diagnostic-reporting infrastructure for the zed-rdf parser
//! family.
//!
//! This crate is the single upstream source of truth for how parsers
//! (`rdf-iri`, `rdf-ntriples`, `rdf-turtle`, `rdf-xml`, `rdf-jsonld`, …)
//! report problems.  Its public surface is deliberately small and
//! stable — downstream crates read the types here; no parser invents
//! its own diagnostic shape.
//!
//! # Model
//!
//! - [`Severity`] — one of `Error` / `Warning` / `Info` / `Hint`.
//! - [`Span`] — a half-open byte-offset range into the original source
//!   (`[start, end)`).  Byte offsets (not char offsets) so they round-
//!   trip losslessly to LSP positions via UTF-8 index math in the LSP
//!   layer.
//! - [`Diagnostic`] — severity + stable code (e.g. `NT-LITESC-001` per
//!   `docs/spec-readings/`) + human-readable message + primary
//!   [`Span`], plus optional [`Diagnostic::hint`] and
//!   [`Diagnostic::related`] notes.
//! - [`DiagnosticBag`] — an ordered accumulator with
//!   [`DiagnosticBag::push`], [`DiagnosticBag::extend`],
//!   [`DiagnosticBag::into_vec`], and [`DiagnosticBag::is_fatal`].
//!
//! # Rendering
//!
//! [`render`] turns a [`Diagnostic`] into a human-readable,
//! LSP-independent string with line/column and a caret indicator, for
//! CLI / test-snapshot use.  The LSP layer has its own renderer; see
//! the optional `lsp` feature below.
//!
//! # Feature flags
//!
//! - `lsp` (off by default) — adds `impl From<Diagnostic> for
//!   lsp_types::Diagnostic`.  The `lsp-types` dependency is only
//!   compiled when this feature is enabled; parser crates depend on
//!   `rdf-diagnostics` with `default-features = false` and the LSP
//!   crate opts in.
//!
//! # Stability
//!
//! Per ADR-0017 §4 this crate is load-bearing for every Phase A
//! parser.  Adding a new variant to [`Severity`] or a new required
//! field to [`Diagnostic`] is a workspace-wide breaking change and
//! requires an ADR amendment.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod bag;
mod diagnostic;
mod render;
mod severity;
mod span;

#[cfg(feature = "lsp")]
mod lsp;

pub use bag::DiagnosticBag;
pub use diagnostic::{Diagnostic, Related};
pub use render::render;
pub use severity::Severity;
pub use span::Span;
