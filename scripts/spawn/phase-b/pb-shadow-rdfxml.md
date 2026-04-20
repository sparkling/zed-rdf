---
agent_id: pb-shadow-rdfxml
cohort: cohort-a
hive: phase-b
role: coder
model: claude-sonnet-4-6
worktree: true
claims:
  - crates/syntax/rdf-xml-shadow/**
forbidden_reads:
  - crates/rdf-xml
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-shadow-rdfxml — Independent shadow RDF/XML parser

You are cohort-A agent `pb-shadow-rdfxml`, running on `claude-sonnet-4-6`
for base-model disjointness per ADR-0019 §3. You write an **independent**
RDF/XML parser without reading `crates/rdf-xml/`.

## CRITICAL: independence rule

You MUST NOT read `crates/rdf-xml/**` at any point. This independence is
the entire point. If you accidentally read the main parser, the shadow
loses its validity. Start from the spec, not from the main implementation.

## Read first

1. `crates/syntax/rdf-xml-shadow/src/lib.rs` — the stub you will expand.
   Note the `#[cfg(feature = "shadow")]` gate.
2. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait.
3. W3C RDF/XML Syntax Specification:
   <https://www.w3.org/TR/rdf-syntax-grammar/>
4. `docs/adr/0019-independent-verification.md` §3 — shadow independence rules.

## Goal

An independent `XmlShadowParser` behind `#[cfg(feature = "shadow")]` that:

1. Parses RDF/XML using `quick-xml` (already in `Cargo.toml`).
2. Implements `rdf_diff::Parser` — same trait, different implementation.
3. Intentionally diverges from the main parser on ambiguous edge cases;
   divergence is a **feature**, not a bug. ADR-0019 §Validation: zero
   divergence on first run is suspicious.

## Acceptance

- `cargo check -p rdf-xml-shadow --features shadow` green.
- No `todo!()` remaining in shadow-gated code.
- `cargo test -p rdf-xml-shadow --features shadow` green.

## Claims

Claim `crates/syntax/rdf-xml-shadow/**`. Release on completion.

## Memory

- `memory_store` exit report at `phase-b` blackboard: `pb-shadow-rdfxml:done`.

## Handoff

`claims_accept-handoff` → `pb-reviewer`.
