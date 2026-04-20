---
agent_id: pb-rdf-trix
cohort: cohort-a
hive: phase-b
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-trix/**
forbidden_reads:
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-rdf-trix — Main TriX parser

You are cohort-A agent `pb-rdf-trix`. Implement the TriX parser at
`crates/rdf-trix/`. A stub with the correct `rdf_diff::Parser` shape is
already in place.

## Read first

1. `crates/rdf-trix/src/lib.rs` — stub.
2. `crates/testing/rdf-diff/src/lib.rs` — frozen `Parser` trait.
3. `docs/adr/0021-phase-b-execution-plan.md`.
4. TriX: Triples in XML (HP Labs 2004):
   <https://www.hpl.hp.com/techreports/2004/HPL-2004-56.html>

## Goal

A working `TriXParser` that:

1. Uses `quick-xml` (already in `Cargo.toml`) for streaming XML tokenisation.
2. Parses the TriX format: `<TriX>` → `<graph>` → `<triple>` elements
   containing `<uri>`, `<bnode>`, `<plainLiteral>`, `<typedLiteral>`.
3. Emits facts via `Facts::canonicalise`. Named graphs are supported
   (`Fact::graph` is `Some(graph_uri)` for non-default graphs).

## Acceptance

- `cargo check -p rdf-trix` green.
- Snapshot test corpus green (no W3C conformance suite for TriX — create
  fixtures under `crates/rdf-trix/tests/snapshots/` that cover: empty graph,
  named graph, blank nodes, literals with language tags and datatypes, and
  at least one negative-syntax case).
- `cargo test -p rdf-trix` green.

## Claims

Claim `crates/rdf-trix/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/approach` in `crate/rdf-trix`.
- `memory_store` exit report at `phase-b` blackboard: `pb-rdf-trix:done`.

## Handoff

`claims_accept-handoff` → `pb-reviewer`.
