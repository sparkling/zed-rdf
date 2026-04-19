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
//!
//! # Canonical form
//!
//! The agreed-upon canonical form (see `canonical-form/decisions` in the
//! `crate/rdf-diff` memory namespace) is:
//!
//! - **IRIs** are stored wrapped in angle brackets (`<http://ex/>`). Prefix
//!   names (`ex:foo`) are rejected at the front door of [`diff`] /
//!   [`diff_many`] with [`DiffError::NonCanonical`]. [`Facts::prefixes`] is
//!   preserved for diagnostics only and is NOT consulted by the diff.
//! - **Blank nodes** are relabelled deterministically to `_:c0`, `_:c1`, …
//!   in first-encounter order after sorting facts with all blank-node labels
//!   unified under a single placeholder. This is a simplified positional
//!   canonicalisation (not URDNA2015): adequate for the verification-v1
//!   sweep where each [`Facts`] set is consumed against another derived
//!   from the same byte input.
//! - **Plain literals** canonicalise to `"<lex>"` with implicit datatype
//!   `xsd:string` (RDF 1.1 §3.3). Datatyped literals canonicalise to
//!   `"<lex>"^^<iri>`. Language-tagged literals canonicalise to
//!   `"<lex>"@<bcp47>` with the language tag normalised per BCP-47 §2.1.1
//!   (language base lowercase, script title-case, region uppercase,
//!   variant lowercase). Lexical forms are preserved byte-for-byte — no
//!   trimming, no Unicode normalisation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap};

use thiserror::Error;

/// A single logical fact emitted by a parser, in a canonical form suitable
/// for set-diffing across implementations.
///
/// The exact canonicalisation rules (blank-node relabelling, IRI normalisation,
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
    /// The rules applied, per the crate-level "Canonical form" section:
    ///
    /// 1. Every component (subject / predicate / object / graph) is passed
    ///    through [`canonicalise_term`]; literals have their language tag
    ///    BCP-47 case-folded, lexical form is preserved verbatim.
    /// 2. Blank-node labels are collected across every position, then
    ///    relabelled to `_:c<N>` in first-encounter order under a
    ///    deterministic sort that treats all blank nodes as identical.
    ///    Duplicate facts (after relabelling) collapse; the earliest
    ///    provenance wins.
    /// 3. `prefixes` is attached to the returned [`Facts`] untouched — it
    ///    is diagnostic context, not part of the diff.
    ///
    /// The signature is frozen (ADR-0020 §1.4).
    #[must_use]
    pub fn canonicalise<I: IntoIterator<Item = (Fact, FactProvenance)>>(
        raw: I,
        prefixes: BTreeMap<String, String>,
    ) -> Self {
        // Stage 1: normalise every term in place. At this point BNode
        // labels are still whatever the parser emitted.
        let normalised: Vec<(Fact, FactProvenance)> = raw
            .into_iter()
            .map(|(f, p)| {
                let fact = Fact {
                    subject: canonicalise_term(&f.subject),
                    predicate: canonicalise_term(&f.predicate),
                    object: canonicalise_term(&f.object),
                    graph: f.graph.as_deref().map(canonicalise_term),
                };
                (fact, p)
            })
            .collect();

        // Stage 2: collect BNode labels in a deterministic discovery order.
        // We sort by a BNode-anonymised key so that the label assignment
        // is independent of which parser-internal label the input used.
        let mut indexed: Vec<(usize, &(Fact, FactProvenance))> = normalised
            .iter()
            .enumerate()
            .collect();
        indexed.sort_by(|(_, a), (_, b)| {
            let ka = bnode_blind_key(&a.0);
            let kb = bnode_blind_key(&b.0);
            ka.cmp(&kb)
        });

        let mut bnode_map: HashMap<String, String> = HashMap::new();
        let mut counter: usize = 0;
        let record = |label: &str, map: &mut HashMap<String, String>, ctr: &mut usize| {
            if !map.contains_key(label) {
                let canonical = format!("_:c{}", *ctr);
                map.insert(label.to_owned(), canonical);
                *ctr += 1;
            }
        };
        for (_, (fact, _)) in &indexed {
            if is_bnode(&fact.subject) {
                record(&fact.subject, &mut bnode_map, &mut counter);
            }
            if is_bnode(&fact.object) {
                record(&fact.object, &mut bnode_map, &mut counter);
            }
            if let Some(g) = fact.graph.as_deref()
                && is_bnode(g)
            {
                record(g, &mut bnode_map, &mut counter);
            }
        }

        // Stage 3: apply the relabelling and fold into the canonical set.
        // Duplicate facts collapse; the earliest provenance (by input
        // order) wins, matching the "first writer wins" convention used
        // elsewhere in the harness.
        let mut set: BTreeMap<Fact, FactProvenance> = BTreeMap::new();
        for (fact, prov) in normalised {
            let relabelled = Fact {
                subject: rewrite_bnode(&fact.subject, &bnode_map),
                predicate: fact.predicate,
                object: rewrite_bnode(&fact.object, &bnode_map),
                graph: fact.graph.map(|g| rewrite_bnode(&g, &bnode_map)),
            };
            set.entry(relabelled).or_insert(prov);
        }

        Self { set, prefixes }
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
        /// (`parser_id`, object) pairs from each side.
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
    pub const fn is_clean(&self) -> bool {
        self.divergences.is_empty()
    }
}

/// Compare two canonical fact sets and produce a [`DiffReport`].
///
/// The inputs **must** have been produced by [`Facts::canonicalise`] (or
/// by an implementation that agrees on the canonical form); the harness
/// enforces that at the front door with [`DiffError::NonCanonical`].
///
/// Divergences surface as:
///
/// - [`Divergence::FactOnlyIn`] — a fact present on one side only, with no
///   matching `(subject, predicate, graph)` key on the other side.
/// - [`Divergence::ObjectMismatch`] — a `(subject, predicate, graph)` key
///   present on both sides but with non-equal object sets; both sides'
///   objects are listed in `sides` tagged by parser id.
///
/// [`Divergence::AcceptRejectSplit`] is **not** produced here:
/// accept/reject is a property of the [`Parser`] call, not of a [`Facts`]
/// set. Harness code above this layer that knows both parse results emits
/// that variant.
///
/// Parser ids are taken from the first fact's [`FactProvenance::parser`]
/// on each side. If a side is empty, the id falls back to the string
/// `"(unknown)"` — the diff is still sound, only the label is degraded.
///
/// # Errors
///
/// Returns [`DiffError::NonCanonical`] if either side contains a term that
/// is not in the canonical form documented at the crate root.
pub fn diff(a: &Facts, b: &Facts) -> Result<DiffReport, DiffError> {
    check_canonical(a, "lhs")?;
    check_canonical(b, "rhs")?;

    let id_a = parser_id(a);
    let id_b = parser_id(b);

    // Index each side by (subject, predicate, graph) -> set of objects.
    // BTreeMap/BTreeSet so divergence emission order is deterministic.
    let key_index_a = index_by_key(a);
    let key_index_b = index_by_key(b);

    let mut divergences: Vec<Divergence> = Vec::new();
    let mut keys: BTreeSet<&(String, String, Option<String>)> = BTreeSet::new();
    keys.extend(key_index_a.keys());
    keys.extend(key_index_b.keys());

    for key in keys {
        let empty: BTreeSet<String> = BTreeSet::new();
        let objs_a = key_index_a.get(key).unwrap_or(&empty);
        let objs_b = key_index_b.get(key).unwrap_or(&empty);

        if objs_a == objs_b {
            continue;
        }

        // Objects present only on A → FactOnlyIn(A).
        for obj in objs_a.difference(objs_b) {
            if objs_b.is_empty() {
                divergences.push(Divergence::FactOnlyIn {
                    parser_id: id_a.clone(),
                    fact: fact_from_key(key, obj),
                });
            } else {
                divergences.push(Divergence::ObjectMismatch {
                    subject: key.0.clone(),
                    predicate: key.1.clone(),
                    graph: key.2.clone(),
                    sides: vec![(id_a.clone(), obj.clone())],
                });
            }
        }

        // Objects present only on B → symmetric.
        for obj in objs_b.difference(objs_a) {
            if objs_a.is_empty() {
                divergences.push(Divergence::FactOnlyIn {
                    parser_id: id_b.clone(),
                    fact: fact_from_key(key, obj),
                });
            } else {
                divergences.push(Divergence::ObjectMismatch {
                    subject: key.0.clone(),
                    predicate: key.1.clone(),
                    graph: key.2.clone(),
                    sides: vec![(id_b.clone(), obj.clone())],
                });
            }
        }
    }

    Ok(DiffReport {
        divergences,
        triage_hint: triage_hint_for(&id_a, &id_b),
    })
}

/// N-way diff. Convenience over pairwise [`diff`] for oracle ensembles.
///
/// `sets` is a sequence of `(parser_id, facts)` pairs. The harness builds
/// a per-key tally across all parsers and emits only the **dissenting**
/// views, collapsing any `(s, p, g) -> object` shared by a strict
/// majority of parsers into a single consensus. This matches ADR-0019 §2:
/// "any divergence is a CI failure", but with a useful collapse for
/// ensembles of three or more oracles.
///
/// Semantics:
///
/// - For each `(subject, predicate, graph)` key seen by any parser, the
///   set of objects claimed by each parser is recorded. Where a strict
///   majority (> n/2) of parsers agree on the object set, the consensus
///   is silent; parsers whose object set differs emit
///   [`Divergence::ObjectMismatch`] with their divergent objects.
/// - Where fewer than a strict majority converge, every distinct
///   object set emits a divergence — no consensus can be formed.
///
/// # Errors
///
/// Returns [`DiffError::NonCanonical`] on the first input that fails the
/// canonical-form check. The harness short-circuits; later inputs are not
/// checked.
pub fn diff_many<'a, I>(sets: I) -> Result<DiffReport, DiffError>
where
    I: IntoIterator<Item = (&'a str, &'a Facts)>,
{
    let collected: Vec<(&str, &Facts)> = sets.into_iter().collect();
    for (id, facts) in &collected {
        check_canonical(facts, id)?;
    }

    let n = collected.len();
    if n < 2 {
        return Ok(DiffReport::default());
    }
    let majority_threshold = n / 2; // strict majority == more than this.

    // Build a per-key, per-parser object set.
    let mut tally: ManyTally = BTreeMap::new();
    // First pass: record every (parser -> objects) observation.
    for (id, facts) in &collected {
        for fact in facts.set.keys() {
            let key = (
                fact.subject.clone(),
                fact.predicate.clone(),
                fact.graph.clone(),
            );
            tally
                .entry(key)
                .or_default()
                .entry((*id).to_owned())
                .or_default()
                .insert(fact.object.clone());
        }
    }
    // Second pass: absence is modelled as the empty object set so that
    // the majority logic can compare fairly across every parser.
    let all_keys: Vec<_> = tally.keys().cloned().collect();
    for key in all_keys {
        let per_parser = tally.get_mut(&key).unwrap_or_else(|| {
            unreachable!("key came from tally.keys() above")
        });
        for (id, _) in &collected {
            per_parser.entry((*id).to_owned()).or_default();
        }
    }

    let mut divergences: Vec<Divergence> = Vec::new();
    for (key, per_parser) in &tally {
        // Collapse equal object sets into groups; count group sizes.
        let mut groups: Vec<(BTreeSet<String>, Vec<String>)> = Vec::new();
        for (pid, objs) in per_parser {
            if let Some(entry) = groups.iter_mut().find(|(o, _)| o == objs) {
                entry.1.push(pid.clone());
            } else {
                groups.push((objs.clone(), vec![pid.clone()]));
            }
        }

        // If exactly one group, every parser agrees → no divergence.
        if groups.len() == 1 {
            continue;
        }

        // Find the (possibly absent) strict-majority group.
        let majority = groups
            .iter()
            .find(|(_, members)| members.len() > majority_threshold)
            .cloned();

        for (objs, members) in &groups {
            if let Some((maj_objs, _)) = majority.as_ref()
                && objs == maj_objs
            {
                continue; // Consensus branch — silent.
            }
            emit_group_divergences(&mut divergences, key, objs, members);
        }
    }

    Ok(DiffReport {
        divergences,
        triage_hint: format!("diff_many across {n} parsers"),
    })
}

// -----------------------------------------------------------------------
// Internal helpers — not part of the frozen surface.
// -----------------------------------------------------------------------

/// Per-key, per-parser tally used inside [`diff_many`]. Top-level key is
/// `(subject, predicate, graph)`; inner key is the parser id; inner value
/// is the set of objects that parser claims for that key.
type ManyTally = BTreeMap<(String, String, Option<String>), BTreeMap<String, BTreeSet<String>>>;

/// Emit divergences for one group in the `diff_many` tally.
fn emit_group_divergences(
    out: &mut Vec<Divergence>,
    key: &(String, String, Option<String>),
    objs: &BTreeSet<String>,
    members: &[String],
) {
    if objs.is_empty() {
        // This parser produced no fact for this key — record it once.
        for pid in members {
            out.push(Divergence::FactOnlyIn {
                parser_id: pid.clone(),
                fact: Fact {
                    subject: key.0.clone(),
                    predicate: key.1.clone(),
                    object: String::new(),
                    graph: key.2.clone(),
                },
            });
        }
        return;
    }

    let sides: Vec<(String, String)> = members
        .iter()
        .flat_map(|pid| objs.iter().map(move |o| (pid.clone(), o.clone())))
        .collect();
    out.push(Divergence::ObjectMismatch {
        subject: key.0.clone(),
        predicate: key.1.clone(),
        graph: key.2.clone(),
        sides,
    });
}

/// Canonicalise one term string. The rules are applied by term kind:
///
/// - Angle-bracketed IRI (`<...>`): left untouched.
/// - Blank node (`_:label`): left untouched at this stage; the global
///   relabelling pass rewrites it.
/// - Literal (`"..."`, `"..."@tag`, `"..."^^<iri>`): lexical form preserved
///   verbatim; the language tag is BCP-47 case-folded if present.
/// - Anything else is passed through untouched; the canonical-form
///   validator in [`check_canonical`] will catch it later.
fn canonicalise_term(term: &str) -> String {
    if term.starts_with('"')
        && let Some((lex, suffix)) = split_literal(term)
    {
        if let Some(tag) = suffix.strip_prefix('@') {
            let normalised = bcp47_case_fold(tag);
            return format!("\"{lex}\"@{normalised}");
        }
        return format!("\"{lex}\"{suffix}");
    }
    // Angle-bracketed IRIs and blank-node labels are already canonical.
    if term.starts_with('<') || term.starts_with("_:") {
        return term.to_owned();
    }
    // A bare absolute IRI (scheme ":" something) is wrapped into angle
    // brackets so the diff-side validator accepts it. Prefix-names (pnames
    // like `ex:foo`) would also match this shape; they are rejected at the
    // front door of `diff` / `diff_many` via [`check_term`]. We do **not**
    // attempt to expand pnames against `Facts::prefixes` — pname
    // expansion is a parser concern per the canonical-form decision
    // (see `canonical-form/decisions` in `crate/rdf-diff` memory).
    if looks_like_absolute_iri(term) {
        return format!("<{term}>");
    }
    term.to_owned()
}

/// `true` if `term` looks like an RFC-3986 absolute IRI. Requires an
/// ASCII-alpha scheme followed by `:` and at least one further character.
/// Rejects empty schemes and schemes containing `/`, `#`, or whitespace
/// before the `:`; those are pnames or junk, and canonicalisation must
/// not silently promote them.
fn looks_like_absolute_iri(term: &str) -> bool {
    let Some(colon) = term.find(':') else {
        return false;
    };
    if colon == 0 || colon + 1 >= term.len() {
        return false;
    }
    let scheme = &term[..colon];
    if !scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    if !scheme
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return false;
    }
    // Path starts after the colon. Require it to look like a hierarchical
    // IRI (`//` authority, `/` path, or `#` fragment). Bare
    // `prefix:local` stays unwrapped so the front-door validator rejects
    // it as a pname. URN-style schemes (`urn:isbn:...`) are recognised by
    // scheme length >= 3 — pnames rarely use 3+-char prefixes, and a
    // collision here still produces a deterministic canonical form.
    let rest = &term[colon + 1..];
    rest.starts_with("//") || rest.starts_with('/') || rest.starts_with('#')
}

/// Split a literal canonical form into `(lexical, suffix)`, where suffix
/// is `""`, `"@en-US"`, or `"^^<iri>"`. Returns `None` when the literal
/// shape is unparseable; the caller leaves it untouched and the front-door
/// validator rejects it.
fn split_literal(term: &str) -> Option<(&str, &str)> {
    if !term.starts_with('"') {
        return None;
    }
    // Walk from the end to find the closing quote of the lexical form.
    // The lexical form can contain escaped `\"`, so we scan.
    let bytes = term.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2, // skip escape
            b'"' => break,
            _ => i += 1,
        }
    }
    if i >= bytes.len() {
        return None;
    }
    // `term[1..i]` is the lexical form (between the quotes).
    // `term[i+1..]` is the suffix (`""`, `"@tag"` or `"^^<iri>"`).
    let lex = term.get(1..i)?;
    let suffix = term.get(i + 1..)?;
    Some((lex, suffix))
}

/// Case-fold a BCP-47 language tag per §2.1.1:
/// - primary language subtag lowercase
/// - script subtag title-case (4 letters)
/// - region subtag uppercase (2 letters or 3 digits)
/// - variant subtags lowercase
/// - extension / private-use singletons lowercase
///
/// No registry validation is performed; unknown subtag shapes are passed
/// through lowercased.
fn bcp47_case_fold(tag: &str) -> String {
    let parts: Vec<&str> = tag.split('-').collect();
    let mut out: Vec<String> = Vec::with_capacity(parts.len());
    for (idx, part) in parts.iter().enumerate() {
        let folded = if idx == 0 {
            part.to_ascii_lowercase()
        } else if part.len() == 4 && part.chars().all(|c| c.is_ascii_alphabetic()) {
            // Script: Xxxx
            let mut s = String::with_capacity(4);
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                s.push(first.to_ascii_uppercase());
            }
            for c in chars {
                s.push(c.to_ascii_lowercase());
            }
            s
        } else if (part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic()))
            || (part.len() == 3 && part.chars().all(|c| c.is_ascii_digit()))
        {
            // Region
            part.to_ascii_uppercase()
        } else {
            part.to_ascii_lowercase()
        };
        out.push(folded);
    }
    out.join("-")
}

/// `true` if `term` is a blank-node label.
fn is_bnode(term: &str) -> bool {
    term.starts_with("_:")
}

/// Rewrite a blank-node label if it appears in `map`; otherwise return the
/// term unchanged.
fn rewrite_bnode(term: &str, map: &HashMap<String, String>) -> String {
    if is_bnode(term)
        && let Some(canon) = map.get(term)
    {
        return canon.clone();
    }
    term.to_owned()
}

/// Compute a blank-node-blind sort key: every blank-node label is replaced
/// by a single placeholder so that two fact lists that differ only in their
/// parser-chosen blank-node labels sort identically.
fn bnode_blind_key(fact: &Fact) -> (String, String, String, Option<String>) {
    (
        blind(&fact.subject),
        fact.predicate.clone(),
        blind(&fact.object),
        fact.graph.as_deref().map(blind),
    )
}

fn blind(term: &str) -> String {
    if is_bnode(term) {
        "_:?".to_owned()
    } else {
        term.to_owned()
    }
}

/// Validate that every term in every fact is in canonical form. Called at
/// the front door of [`diff`] and [`diff_many`].
fn check_canonical(facts: &Facts, side_label: &str) -> Result<(), DiffError> {
    for fact in facts.set.keys() {
        check_term(&fact.subject, TermKind::SubjectOrObject, side_label, "subject")?;
        check_term(&fact.predicate, TermKind::Iri, side_label, "predicate")?;
        check_term(&fact.object, TermKind::SubjectOrObject, side_label, "object")?;
        if let Some(g) = fact.graph.as_deref() {
            check_term(g, TermKind::GraphName, side_label, "graph")?;
        }
    }
    Ok(())
}

#[derive(Copy, Clone)]
enum TermKind {
    /// Must be an IRI (e.g., predicate).
    Iri,
    /// IRI, blank node, or (for objects) a literal.
    SubjectOrObject,
    /// IRI or blank node.
    GraphName,
}

fn check_term(
    term: &str,
    kind: TermKind,
    side_label: &str,
    position: &str,
) -> Result<(), DiffError> {
    if term.is_empty() {
        return Err(DiffError::NonCanonical(format!(
            "{side_label}: empty {position}"
        )));
    }

    let is_iri = term.starts_with('<') && term.ends_with('>');
    let is_bnode = is_bnode(term);
    let is_literal = term.starts_with('"');

    let ok = match kind {
        TermKind::Iri => is_iri,
        TermKind::SubjectOrObject => is_iri || is_bnode || is_literal,
        TermKind::GraphName => is_iri || is_bnode,
    };

    if !ok {
        return Err(DiffError::NonCanonical(format!(
            "{side_label}: {position} {term:?} is not in canonical form \
             (expected angle-bracketed IRI{extra})",
            extra = match kind {
                TermKind::Iri => "",
                TermKind::SubjectOrObject => ", blank node (_:label), or literal (\"…\")",
                TermKind::GraphName => " or blank node (_:label)",
            }
        )));
    }

    // Additionally: IRIs must not contain an unescaped pname-style
    // `prefix:localname` leakage. We accept the IRI body opaquely but
    // assert the wrapping is intact.
    if (matches!(kind, TermKind::Iri) || (matches!(kind, TermKind::SubjectOrObject) && is_iri))
        && (term.len() < 2 || !term.ends_with('>'))
    {
        return Err(DiffError::NonCanonical(format!(
            "{side_label}: {position} {term:?} has malformed IRI wrapping"
        )));
    }

    Ok(())
}

/// Build a `(subject, predicate, graph) -> {objects}` index for a side.
fn index_by_key(
    facts: &Facts,
) -> BTreeMap<(String, String, Option<String>), BTreeSet<String>> {
    let mut out: BTreeMap<(String, String, Option<String>), BTreeSet<String>> =
        BTreeMap::new();
    for fact in facts.set.keys() {
        out.entry((
            fact.subject.clone(),
            fact.predicate.clone(),
            fact.graph.clone(),
        ))
        .or_default()
        .insert(fact.object.clone());
    }
    out
}

/// Recover the parser id for a side from the first fact's provenance.
fn parser_id(facts: &Facts) -> String {
    facts
        .set
        .values()
        .next()
        .map(|p| p.parser.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(unknown)".to_owned())
}

fn fact_from_key(key: &(String, String, Option<String>), object: &str) -> Fact {
    Fact {
        subject: key.0.clone(),
        predicate: key.1.clone(),
        object: object.to_owned(),
        graph: key.2.clone(),
    }
}

fn triage_hint_for(a: &str, b: &str) -> String {
    format!("diff({a}, {b})")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prov(parser: &str, offset: usize) -> FactProvenance {
        FactProvenance {
            offset: Some(offset),
            parser: parser.to_owned(),
        }
    }

    fn iri_fact(s: &str, p: &str, o: &str) -> Fact {
        Fact {
            subject: format!("<{s}>"),
            predicate: format!("<{p}>"),
            object: format!("<{o}>"),
            graph: None,
        }
    }

    #[test]
    fn canonicalise_preserves_iri_facts() {
        let raw = vec![(
            iri_fact("http://ex/s", "http://ex/p", "http://ex/o"),
            prov("test", 0),
        )];
        let facts = Facts::canonicalise(raw, BTreeMap::new());
        assert_eq!(facts.set.len(), 1);
    }

    #[test]
    fn canonicalise_is_idempotent_on_simple_input() {
        let raw = vec![(
            iri_fact("http://ex/s", "http://ex/p", "http://ex/o"),
            prov("test", 0),
        )];
        let first = Facts::canonicalise(raw, BTreeMap::new());
        let second = Facts::canonicalise(
            first.set.iter().map(|(f, p)| (f.clone(), p.clone())),
            first.prefixes.clone(),
        );
        assert_eq!(first, second);
    }

    #[test]
    fn canonicalise_relabels_bnodes_deterministically() {
        let raw_a = vec![
            (
                Fact {
                    subject: "_:alpha".to_owned(),
                    predicate: "<http://ex/p>".to_owned(),
                    object: "_:beta".to_owned(),
                    graph: None,
                },
                prov("A", 0),
            ),
            (
                Fact {
                    subject: "_:beta".to_owned(),
                    predicate: "<http://ex/p>".to_owned(),
                    object: "<http://ex/o>".to_owned(),
                    graph: None,
                },
                prov("A", 1),
            ),
        ];
        let raw_b = vec![
            (
                Fact {
                    subject: "_:xyzzy".to_owned(),
                    predicate: "<http://ex/p>".to_owned(),
                    object: "_:plugh".to_owned(),
                    graph: None,
                },
                prov("B", 0),
            ),
            (
                Fact {
                    subject: "_:plugh".to_owned(),
                    predicate: "<http://ex/p>".to_owned(),
                    object: "<http://ex/o>".to_owned(),
                    graph: None,
                },
                prov("B", 1),
            ),
        ];
        let a = Facts::canonicalise(raw_a, BTreeMap::new());
        let b = Facts::canonicalise(raw_b, BTreeMap::new());
        // Same abstract graph; BNode relabelling should make them equal.
        let keys_a: Vec<_> = a.set.keys().cloned().collect();
        let keys_b: Vec<_> = b.set.keys().cloned().collect();
        assert_eq!(keys_a, keys_b, "BNode relabelling is not canonical");
    }

    #[test]
    fn canonicalise_lang_tag_case_folds() {
        let raw = vec![(
            Fact {
                subject: "<http://ex/s>".to_owned(),
                predicate: "<http://ex/p>".to_owned(),
                object: "\"Hello\"@EN-us".to_owned(),
                graph: None,
            },
            prov("t", 0),
        )];
        let facts = Facts::canonicalise(raw, BTreeMap::new());
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"Hello\"@en-US");
    }

    #[test]
    fn canonicalise_preserves_literal_lexical_form() {
        let raw = vec![(
            Fact {
                subject: "<http://ex/s>".to_owned(),
                predicate: "<http://ex/p>".to_owned(),
                object: "\"  spaced  \\\"inner\\\"  \"".to_owned(),
                graph: None,
            },
            prov("t", 0),
        )];
        let facts = Facts::canonicalise(raw, BTreeMap::new());
        let fact = facts.set.keys().next().unwrap();
        assert_eq!(fact.object, "\"  spaced  \\\"inner\\\"  \"");
    }

    #[test]
    fn diff_self_is_clean() {
        let raw = vec![(
            iri_fact("http://ex/s", "http://ex/p", "http://ex/o"),
            prov("t", 0),
        )];
        let facts = Facts::canonicalise(raw, BTreeMap::new());
        let report = diff(&facts, &facts).expect("canonical input");
        assert!(report.is_clean());
    }

    #[test]
    fn diff_fact_only_in_side() {
        let a = Facts::canonicalise(
            vec![(
                iri_fact("http://ex/s", "http://ex/p", "http://ex/o"),
                prov("A", 0),
            )],
            BTreeMap::new(),
        );
        let b = Facts::canonicalise(Vec::<(Fact, FactProvenance)>::new(), BTreeMap::new());
        let report = diff(&a, &b).expect("canonical input");
        assert_eq!(report.divergences.len(), 1);
        match &report.divergences[0] {
            Divergence::FactOnlyIn { parser_id, .. } => assert_eq!(parser_id, "A"),
            other => panic!("expected FactOnlyIn, got {other:?}"),
        }
    }

    #[test]
    fn diff_object_mismatch() {
        let a = Facts::canonicalise(
            vec![(
                Fact {
                    subject: "<http://ex/s>".to_owned(),
                    predicate: "<http://ex/p>".to_owned(),
                    object: "\"foo\"".to_owned(),
                    graph: None,
                },
                prov("A", 0),
            )],
            BTreeMap::new(),
        );
        let b = Facts::canonicalise(
            vec![(
                Fact {
                    subject: "<http://ex/s>".to_owned(),
                    predicate: "<http://ex/p>".to_owned(),
                    object: "\"bar\"".to_owned(),
                    graph: None,
                },
                prov("B", 0),
            )],
            BTreeMap::new(),
        );
        let report = diff(&a, &b).expect("canonical input");
        assert!(
            report
                .divergences
                .iter()
                .any(|d| matches!(d, Divergence::ObjectMismatch { .. }))
        );
    }

    #[test]
    fn diff_rejects_non_canonical_iri() {
        let mut set = BTreeMap::new();
        set.insert(
            Fact {
                subject: "ex:s".to_owned(), // prefix name — forbidden
                predicate: "<http://ex/p>".to_owned(),
                object: "<http://ex/o>".to_owned(),
                graph: None,
            },
            prov("P", 0),
        );
        let bad = Facts {
            set,
            prefixes: BTreeMap::new(),
        };
        let good = Facts::default();
        assert!(matches!(diff(&bad, &good), Err(DiffError::NonCanonical(_))));
    }

    #[test]
    fn diff_commutative_at_set_level_for_disjoint_inputs() {
        let a = Facts::canonicalise(
            vec![(
                iri_fact("http://ex/s1", "http://ex/p", "http://ex/o1"),
                prov("A", 0),
            )],
            BTreeMap::new(),
        );
        let b = Facts::canonicalise(
            vec![(
                iri_fact("http://ex/s2", "http://ex/p", "http://ex/o2"),
                prov("B", 0),
            )],
            BTreeMap::new(),
        );
        let ab: BTreeSet<_> = diff(&a, &b)
            .unwrap()
            .divergences
            .iter()
            .map(|d| format!("{d:?}"))
            .collect();
        let ba: BTreeSet<_> = diff(&b, &a)
            .unwrap()
            .divergences
            .iter()
            .map(|d| format!("{d:?}"))
            .collect();
        assert_eq!(ab, ba);
    }

    #[test]
    fn diff_many_collapses_majority() {
        let a = Facts::canonicalise(
            vec![(
                iri_fact("http://ex/s", "http://ex/p", "http://ex/o"),
                prov("A", 0),
            )],
            BTreeMap::new(),
        );
        let b = Facts::canonicalise(
            vec![(
                iri_fact("http://ex/s", "http://ex/p", "http://ex/o"),
                prov("B", 0),
            )],
            BTreeMap::new(),
        );
        let c = Facts::canonicalise(
            vec![(
                iri_fact("http://ex/s", "http://ex/p", "http://ex/other"),
                prov("C", 0),
            )],
            BTreeMap::new(),
        );
        let report = diff_many(vec![("A", &a), ("B", &b), ("C", &c)]).expect("canonical");
        // A and B agree (majority); only C dissents.
        assert_eq!(report.divergences.len(), 1);
        match &report.divergences[0] {
            Divergence::ObjectMismatch { sides, .. } => {
                assert!(sides.iter().any(|(pid, _)| pid == "C"));
                assert!(!sides.iter().any(|(pid, _)| pid == "A" || pid == "B"));
            }
            other => panic!("expected ObjectMismatch, got {other:?}"),
        }
    }

    #[test]
    fn bcp47_case_fold_examples() {
        assert_eq!(bcp47_case_fold("EN"), "en");
        assert_eq!(bcp47_case_fold("en-us"), "en-US");
        assert_eq!(bcp47_case_fold("ZH-hant-tw"), "zh-Hant-TW");
        assert_eq!(bcp47_case_fold("de-CH-1901"), "de-CH-1901");
    }
}
