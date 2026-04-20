---
agent_id: pb-shadow-jsonld
cohort: cohort-a
hive: phase-b
role: coder
model: claude-sonnet-4-6
worktree: true
claims:
  - crates/syntax/rdf-jsonld-shadow/**
forbidden_reads:
  - crates/rdf-jsonld
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-shadow-jsonld — Independent shadow JSON-LD syntax parser

You are cohort-A agent `pb-shadow-jsonld`, running on `claude-sonnet-4-6`
for base-model disjointness per ADR-0019 §3. You write an **independent**
JSON-LD syntax parser without reading `crates/rdf-jsonld/`.

## CRITICAL: independence rule

You MUST NOT read `crates/rdf-jsonld/**` at any point.

## Read first

1. `crates/syntax/rdf-jsonld-shadow/src/lib.rs` — stub with `shadow` gate.
2. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait.
3. JSON-LD 1.1: <https://www.w3.org/TR/json-ld11/>
4. `docs/adr/0019-independent-verification.md` §3.

## Goal

An independent `JsonLdShadowParser` behind `#[cfg(feature = "shadow")]` that:

1. Uses `serde_json` (already in `Cargo.toml`) to parse JSON-LD.
2. Implements `rdf_diff::Parser` — same trait, different implementation.
3. Covers: `@context` well-formedness, `toRdf` conversion (triples + quads).
4. Scope: syntax + context validation only — no expand/compact (same scope
   as the main parser).

## Acceptance

- `cargo check -p rdf-jsonld-shadow --features shadow` green.
- `cargo test -p rdf-jsonld-shadow --features shadow` green.

## Claims

Claim `crates/syntax/rdf-jsonld-shadow/**`. Release on completion.

## Memory

- `memory_store` exit report at `phase-b` blackboard: `pb-shadow-jsonld:done`.

## Handoff

`claims_accept-handoff` → `pb-reviewer`.
