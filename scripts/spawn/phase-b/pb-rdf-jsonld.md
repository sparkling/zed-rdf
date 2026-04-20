---
agent_id: pb-rdf-jsonld
cohort: cohort-a
hive: phase-b
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-jsonld/**
forbidden_reads:
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-rdf-jsonld — Main JSON-LD syntax parser

You are cohort-A agent `pb-rdf-jsonld`. Implement the JSON-LD syntax parser
at `crates/rdf-jsonld/`. A stub with the correct `rdf_diff::Parser` shape is
already in place.

## Read first

1. `crates/rdf-jsonld/src/lib.rs` — the stub you will expand.
2. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait.
3. `docs/adr/0021-phase-b-execution-plan.md` — **scope**: syntax +
   `@context` well-formedness only. No expand, compact, or normalize (Phase E).
4. JSON-LD 1.1: <https://www.w3.org/TR/json-ld11/>
5. JSON-LD 1.1 API: <https://www.w3.org/TR/json-ld11-api/>

## Goal

A working `JsonLdParser` that:

1. Uses `serde_json` (already in `Cargo.toml`) to parse JSON-LD input.
2. Validates `@context` well-formedness: `@base`, `@vocab`, term definitions,
   type coercions. Rejects malformed contexts.
3. Converts JSON-LD to RDF triples/quads via the JSON-LD to RDF algorithm
   (§8 of the API spec). Emits `Facts` via `Facts::canonicalise`.
4. Supports named graphs (`@graph`) — `Fact::graph` is `Some(...)` for
   non-default-graph triples.

## Scope constraint

No RDF Dataset Normalization, no framing, no compaction. The oracle
(`oxjsonld-oracle` in `crates/testing/rdf-diff-oracles/`) also uses the
`toRdf` path — align with that behaviour.

## Acceptance

- `cargo check -p rdf-jsonld` green.
- W3C JSON-LD `toRdf` test suite passes. Tests live at
  `external/tests/w3c-jsonld-api/tests/toRdf/toRdf-manifest.jsonld`.
  Note: manifest is in JSON-LD format, not Turtle — write a small helper
  to parse it or read it directly from `serde_json`.
- `cargo test -p rdf-jsonld` green.

## Claims

Claim `crates/rdf-jsonld/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/approach` in `crate/rdf-jsonld`.
- `memory_store` exit report at `phase-b` blackboard: `pb-rdf-jsonld:done`.

## Handoff

`claims_accept-handoff` → `pb-reviewer`.
