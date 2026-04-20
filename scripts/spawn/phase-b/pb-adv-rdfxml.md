---
agent_id: pb-adv-rdfxml
cohort: cohort-b
hive: phase-b-adv
role: tester
model: claude-sonnet-4-6
worktree: true
claims:
  - crates/testing/rdf-diff/tests/adversary-rdfxml/**
forbidden_reads:
  - phase-b
  - crates/rdf-xml
  - crates/syntax/rdf-xml-shadow
---

# pb-adv-rdfxml — Adversary fixture corpus for RDF/XML

You are cohort-B agent `pb-adv-rdfxml`. You create adversary test fixtures
for RDF/XML without reading the main parser implementation.

## Read first (permitted)

1. `scripts/spawn/phase-b/adv-briefs.md` — `pb-adv-redteam`'s brief for
   rdfxml failure modes (or read from `verification/adversary-findings/rdfxml`
   in memory namespace `phase-b-adv`).
2. W3C RDF/XML Specification: <https://www.w3.org/TR/rdf-syntax-grammar/>
3. `docs/adr/0019-independent-verification.md` §4.

## Goal

Create fixture files under `crates/testing/rdf-diff/tests/adversary-rdfxml/`:
- At least one `.rdf` file per failure mode from the brief.
- Each fixture is accompanied by a `.expected` file: either `ACCEPT` (with
  the canonical N-Triples expected output) or `REJECT` (parser must reject).
- A `manifest.toml` listing every fixture with its expected outcome.

## Acceptance

- ≥3 `.rdf` adversary fixtures created.
- Each fixture independently exercises a distinct failure mode.
- `manifest.toml` is syntactically valid TOML.

## Claims

Claim `crates/testing/rdf-diff/tests/adversary-rdfxml/**`. Release on completion.

## Memory

- `memory_store` exit report at `phase-b-adv` blackboard: `pb-adv-rdfxml:done`.

## Handoff

`claims_accept-handoff` → `pb-adv-veto`.
