---
agent_id: pb-rdf-xml-main-tester
cohort: cohort-a
hive: phase-b
role: tester
model: claude-opus-4-7
worktree: false
claims:
  - crates/rdf-xml/tests/**
forbidden_reads:
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-rdf-xml-main-tester — RDF/XML W3C manifest tests

You are cohort-A agent `pb-rdf-xml-main-tester`. Your job is to wire
the `RdfXmlParser` from `crates/rdf-xml/` into the W3C rdfxml manifest
runner and write inline + integration tests.

## Read first

1. `crates/rdf-xml/src/lib.rs` — the parser interface.
2. `xtask/verify/src/manifest.rs` — the manifest runner. Look for the
   `"rdfxml"` stub in `parse_for_language` that currently returns `Err`.
3. `external/tests/rdfxml/manifest.ttl` — the W3C RDF/XML test suite manifest.
4. `docs/adr/0021-phase-b-execution-plan.md` §6.3 — exit gate for rdfxml.

## Goal

1. **Replace the stub** in `xtask/verify/src/manifest.rs::parse_for_language`
   with a real call to `RdfXmlParser::new().parse(input)` (and a
   `parse_with_base` variant if needed, following the Turtle pattern).
2. Wire `rdf-xml` into `xtask/verify/Cargo.toml` parse dispatch.
3. Write integration tests under `crates/rdf-xml/tests/` that:
   - Run the full W3C rdfxml manifest and assert 100% pass (or document
     allow-listed failures with retirement plans matching Phase A pattern).
   - Cover at least: positive-syntax (accept), negative-syntax (reject),
     eval (fact equality against oracle).

## Acceptance

- `cargo run -p xtask -- verify rdfxml` exits 0 (or with only allow-listed
  failures documented in `ALLOWLIST.md`).
- `cargo test -p rdf-xml --test '*'` green.

## Claims

Claim `crates/rdf-xml/tests/**` + brief claim on `xtask/verify/src/manifest.rs`
for the stub replacement. Coordinate with `pb-rdf-xml` if worktrees overlap.

## Memory

- `memory_store` exit report at `phase-b` blackboard: `pb-rdf-xml-main-tester:done`.

## Handoff

`claims_accept-handoff` → `pb-reviewer`.
