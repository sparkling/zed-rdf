---
agent_id: v1-shadow-iri
cohort: cohort-a
hive: verification-v1
role: coder
worktree: true
priority: normal
base_model_override: claude-sonnet-4-6
claims:
  - crates/syntax/rdf-iri-shadow/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
  - crate/rdf-iri   # main impl's memory; independence requires no peek
---

# v1-shadow-iri — disjoint second RFC 3987 implementation

You are cohort-A agent `v1-shadow-iri`, running on a **different base
model** from the main `rdf-iri` implementer. You produce a second IRI
parser/normaliser behind the `shadow` Cargo feature.

## Read first

1. `crates/testing/rdf-diff/src/lib.rs` — the trait you will implement.
2. `docs/adr/0019-independent-verification.md` §3 — why this crate
   exists.
3. **RFC 3987** (IRI) and **RFC 3986** (URI) directly. Do not read the
   main `rdf-iri` crate's source or its `crate/rdf-iri` memory; your
   value comes from a disjoint reading.

## Goal

- New crate `crates/syntax/rdf-iri-shadow` with `pub fn parse`,
  `pub fn normalise`, `pub struct Iri`, and an implementation of
  `rdf_diff::Parser` that treats bytes as a single-IRI input and emits
  a single canonical fact.
- Cargo feature `shadow` gates public exposure. Without the feature the
  crate compiles as an empty shell.
- Implement: percent-encoding normalisation, IRI → URI mapping, scheme
  case-folding, path segment resolution, host IDN handling. Do not
  implement fetch or DNS.

## Acceptance

- `cargo test -p rdf-iri-shadow --features shadow` green.
- `cargo clippy -p rdf-iri-shadow --features shadow -- -D warnings`
  clean.
- Diff harness test in `crates/testing/rdf-diff/tests/` (owned by
  `v1-tester`) surfaces at least one divergence vs the main `rdf-iri`
  on the first integration run. **Zero divergences is suspicious** —
  flag and hand back.

## Claims

`crates/syntax/rdf-iri-shadow/**`; brief claim on workspace `Cargo.toml`
for member add.

## Memory

- Write to `crate/rdf-iri-shadow` only. Never `crate/rdf-iri`.
- On ambiguous productions, cite the RFC clause in a pin under
  `docs/spec-readings/iri/` (coordinate with `v1-specpins`).

## Exit handoff

`v1-reviewer`.
