---
agent_id: pb-rdf-xml
cohort: cohort-a
hive: phase-b
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-xml/**
forbidden_reads:
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-rdf-xml — Main RDF/XML parser

You are cohort-A agent `pb-rdf-xml`. Implement the RDF/XML parser at
`crates/rdf-xml/`. A stub with the correct `rdf_diff::Parser` shape is
already in place — your job is to fill the implementation.

## Read first

1. `crates/rdf-xml/src/lib.rs` — the stub you will expand.
2. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait + `Facts`,
   `ParseOutcome`, `Diagnostics`.
3. `docs/adr/0021-phase-b-execution-plan.md` — scope, exit gate, Phase B rules.
4. `docs/adr/0007-parser-technology.md` — hand-roll default (Accepted).
5. `docs/spec-readings/` — any RDF/XML pins already landed there.
6. W3C RDF/XML Syntax Specification (2004-02-10):
   <https://www.w3.org/TR/rdf-syntax-grammar/>

## Goal

A working `RdfXmlParser` that:

1. Parses any valid W3C rdfxml test-suite document via `quick-xml` (streaming
   SAX-style events — see `Cargo.toml` dep).
2. Implements `rdf_diff::Parser` so the diff harness can compare it against
   the `oxrdfxml-oracle` adapter in `crates/testing/rdf-diff-oracles/`.
3. Emits triples as `Facts` via `Facts::canonicalise`. No named-graph support
   (RDF/XML is triples-only). `Fact::graph` is always `None`.
4. Rejects invalid inputs (unmatched element types, malformed IRIs) with
   `Err(Diagnostics { fatal: true, messages: […] })`.

## Acceptance

- `cargo check -p rdf-xml` green with no `todo!()` in `lib.rs`.
- W3C rdfxml positive-syntax tests pass; negative-syntax tests are correctly
  rejected. Target: 100% of `external/tests/rdfxml/manifest.ttl`.
- `cargo test -p rdf-xml` green.

## Test-suite location

`external/tests/rdfxml/manifest.ttl` — the W3C RDF/XML test suite (already
vendored). Run via `cargo run -p xtask -- verify rdfxml` once `xtask/verify`
is updated to call `RdfXmlParser::new()`.

## Claims

Claim `crates/rdf-xml/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/approach` in `crate/rdf-xml` describing
  your grammar strategy, any divergences from the spec, and open questions.
- `memory_store` exit report at `phase-b` blackboard: `pb-rdf-xml:done` with
  W3C pass/fail counts.

## Handoff

`claims_accept-handoff` → `pb-reviewer` when `cargo check` + tests are green.
