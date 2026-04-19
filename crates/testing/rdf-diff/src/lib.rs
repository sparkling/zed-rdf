//! Frozen trait surface for the verification-v1 sweep.
//!
//! This crate is the integration contract referenced by ADR-0020 §1.4. Every
//! shadow implementation, reference oracle, and the adversary hive write
//! against the types defined here. Bodies are deliberately left as
//! [`todo!`]; the `v1-diff-core` agent fills them.
//!
//! **Stability contract:** the public API in this file is frozen for the
//! duration of the verification-v1 sweep. Changing a signature requires an
//! amendment to ADR-0020 because cohort-A worktrees depend on the shape.
//!
//! See:
//!
//! - [ADR-0019 §2](../../../../docs/adr/0019-independent-verification.md) —
//!   differential test harness responsibilities.
//! - [ADR-0020 §1.4](../../../../docs/adr/0020-verification-implementation-plan.md) —
//!   the freeze rationale and cohort integration contract.
//! - [ADR-0006](../../../../docs/adr/0006-testing-strategy.md) — where the
//!   diff harness sits in the test pyramid.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use thiserror::Error;

/// A single logical fact emitted by a parser, in a canonical form suitable
/// for set-diffing across implementations.
///
/// The exact canonicalisation rules (BNode relabelling, IRI normalisation,
/// literal lexical form, datatype defaulting) are defined by
/// [`Facts::canonicalise`] and must agree across every implementer of
/// [`Parser`]. The `v1-diff-core` agent fills this in per ADR-0019 §2.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fact {
    /// Opaque canonical subject form. Real shape is decided by
    /// `v1-diff-core`; treated as an opaque string by consumers.
    pub subject: String,
    /// Canonical predicate IRI.
    pub predicate: String,
    /// Canonical object form. Literals carry datatype + language tag inline.
    pub object: String,
    /// Optional canonical graph name. `None` for default graph.
    pub graph: Option<String>,
}

/// A canonicalised collection of facts plus minimal provenance.
///
/// Canonicalisation is **prefix-free** (no pname-shortening survives) and
/// **BNode-canonical** (blank-node labels are relabelled to a deterministic
/// lexicographic form). Implementations must not leak parser-internal state
/// into this struct; if they do, cross-implementation diff will report
/// false positives.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    /// The canonical fact set. `BTreeMap` over the fact's canonical form to
    /// preserve deterministic ordering in diff reports.
    pub set: BTreeMap<Fact, FactProvenance>,
    /// Parser-reported prefix declarations, captured for diagnostic context
    /// only. Not part of the diff. `BTreeMap` for deterministic ordering.
    pub prefixes: BTreeMap<String, String>,
}

/// Per-fact provenance carried alongside [`Facts`] for debuggability only.
///
/// Provenance is **not** considered by [`diff`]; it is surfaced in the
/// [`DiffReport`] for human triage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FactProvenance {
    /// 1-indexed byte offset of the first byte that produced this fact in
    /// the source input. `None` when the parser cannot report it.
    pub offset: Option<usize>,
    /// Human-readable parser identifier, e.g. `"rdf-turtle"` or
    /// `"oxttl-oracle"`. Used in diff reports only.
    pub parser: String,
}

impl Facts {
    /// Canonicalise an unordered iterator of [`Fact`] into a [`Facts`]
    /// with the sweep's agreed canonical form.
    ///
    /// Bodies filled by `v1-diff-core`. The signature is frozen.
    #[must_use]
    pub fn canonicalise<I: IntoIterator<Item = (Fact, FactProvenance)>>(
        _raw: I,
        _prefixes: BTreeMap<String, String>,
    ) -> Self {
        todo!("v1-diff-core: canonical form per ADR-0019 §2")
    }
}

/// A structured diagnostic emitted by a parser when it rejects an input or
/// when it accepts-with-warnings. The shape deliberately avoids leaking the
/// implementer's internal error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostics {
    /// Parser-reported messages. Empty means "accepted cleanly".
    pub messages: Vec<String>,
    /// Whether the parser considered this input fatal (i.e., rejected).
    /// Non-fatal diagnostics are warnings.
    pub fatal: bool,
}

/// Errors the diff harness itself can emit. Parser errors are carried in
/// [`Diagnostics`], **not** here.
#[derive(Debug, Error)]
pub enum DiffError {
    /// The two [`Facts`] sets could not be compared because they disagree
    /// on a required canonical invariant (e.g., prefix-free form).
    #[error("canonical-form invariant violated: {0}")]
    NonCanonical(String),
}

/// The frozen integration contract.
///
/// Shadow crates (`crates/syntax/*-shadow`), reference oracles
/// (`crates/testing/rdf-diff-oracles`), and main parsers
/// (`crates/syntax/*`) all implement this trait. See ADR-0020 §1.4.
pub trait Parser {
    /// Parse `input` into a canonical [`Facts`] set, or return structured
    /// [`Diagnostics`] when the parser rejects the input.
    ///
    /// # Errors
    ///
    /// Returns [`Diagnostics`] with `fatal: true` when the parser rejects
    /// the input. Returns `Ok(Facts)` with `Diagnostics { fatal: false }`
    /// accompanying a successful parse when warnings are present — see
    /// [`ParseOutcome`] for that case.
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics>;

    /// A short identifier for this parser, used only in [`DiffReport`]
    /// rendering. Example values: `"rdf-turtle"`, `"rdf-turtle-shadow"`,
    /// `"oxttl-oracle"`.
    fn id(&self) -> &'static str;
}

/// Result of a successful parse. Carries both the canonical facts and any
/// non-fatal diagnostics the parser raised while producing them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutcome {
    /// The canonical fact set.
    pub facts: Facts,
    /// Non-fatal diagnostics (warnings). Fatal diagnostics are returned as
    /// `Err(Diagnostics)` from [`Parser::parse`].
    pub warnings: Diagnostics,
}

/// A single divergence surfaced by [`diff`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Divergence {
    /// Only one side produced this fact.
    FactOnlyIn {
        /// Parser id that produced it.
        parser_id: String,
        /// The divergent fact.
        fact: Fact,
    },
    /// Both sides produced a fact with the same subject+predicate+graph
    /// but a different object. Usually a literal datatype / language tag /
    /// IRI-normalisation disagreement.
    ObjectMismatch {
        /// Subject component, canonicalised.
        subject: String,
        /// Predicate component, canonicalised.
        predicate: String,
        /// Optional graph component.
        graph: Option<String>,
        /// (parser_id, object) pairs from each side.
        sides: Vec<(String, String)>,
    },
    /// One parser accepted and another rejected.
    AcceptRejectSplit {
        /// Parser ids that accepted.
        accepted_by: Vec<String>,
        /// Parser ids that rejected.
        rejected_by: Vec<String>,
    },
}

/// The output of a diff across two or more [`Facts`] sets.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffReport {
    /// All divergences found. Empty means the inputs agreed fact-for-fact.
    /// ADR-0019 §Validation treats an empty report on Phase-A inputs as
    /// *suspicious*, not successful.
    pub divergences: Vec<Divergence>,
    /// Free-form human triage hint, populated by the harness.
    pub triage_hint: String,
}

impl DiffReport {
    /// `true` iff there are no divergences.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.divergences.is_empty()
    }
}

/// Compare two canonical fact sets and produce a [`DiffReport`].
///
/// Bodies filled by `v1-diff-core`. The signature is frozen.
///
/// # Errors
///
/// Returns [`DiffError::NonCanonical`] if either side was not produced via
/// [`Facts::canonicalise`] (e.g., mixed prefix-form facts snuck in).
pub fn diff(_a: &Facts, _b: &Facts) -> Result<DiffReport, DiffError> {
    todo!("v1-diff-core: set-diff + ObjectMismatch detection per ADR-0019 §2")
}

/// N-way diff. Convenience over pairwise [`diff`] for oracle ensembles.
///
/// # Errors
///
/// Same conditions as [`diff`]. The harness short-circuits on the first
/// non-canonical input.
pub fn diff_many<'a, I>(_sets: I) -> Result<DiffReport, DiffError>
where
    I: IntoIterator<Item = (&'a str, &'a Facts)>,
{
    todo!("v1-diff-core: N-way diff per ADR-0019 §2")
}
