---
agent_id: pb-adv-jsonld
cohort: cohort-b
hive: phase-b-adv
role: tester
model: claude-sonnet-4-6
worktree: true
claims:
  - crates/testing/rdf-diff/tests/adversary-jsonld/**
forbidden_reads:
  - phase-b
  - crates/rdf-jsonld
  - crates/syntax/rdf-jsonld-shadow
---

# pb-adv-jsonld — Adversary fixture corpus for JSON-LD

You are cohort-B agent `pb-adv-jsonld`. You create adversary test fixtures
for JSON-LD without reading the main parser implementation.

## Read first (permitted)

1. `scripts/spawn/phase-b/adv-briefs.md` — the `jsonld` section (or read
   from `verification/adversary-findings/jsonld` in `phase-b-adv` memory).
2. JSON-LD 1.1: <https://www.w3.org/TR/json-ld11/>
3. `docs/adr/0019-independent-verification.md` §4.

## Goal

Create fixture files under `crates/testing/rdf-diff/tests/adversary-jsonld/`:
- At least one `.jsonld` file per failure mode from the brief.
- Each fixture accompanied by `.expected` (ACCEPT with N-Quads or REJECT).
- A `manifest.toml` listing every fixture.

## Acceptance

- ≥3 `.jsonld` adversary fixtures created.
- `manifest.toml` is valid TOML.

## Claims

Claim `crates/testing/rdf-diff/tests/adversary-jsonld/**`. Release on completion.

## Memory

- `memory_store` exit report at `phase-b-adv` blackboard: `pb-adv-jsonld:done`.

## Handoff

`claims_accept-handoff` → `pb-adv-veto`.
