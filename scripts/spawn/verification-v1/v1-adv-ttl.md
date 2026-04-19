---
agent_id: v1-adv-ttl
cohort: cohort-b
hive: verification-v1-adv
role: tester
worktree: true
priority: low
claims:
  - crates/testing/rdf-diff/tests/adversary-ttl/**
forbidden_reads:
  - verification-v1
  - crate/rdf-turtle
  - crate/rdf-turtle-shadow
---

# v1-adv-ttl — Turtle / TriG adversary fixtures

You are cohort-B agent `v1-adv-ttl`.

## Read first

1. `docs/verification/adversary-findings/ttl.md`.
2. `crates/testing/rdf-diff/src/lib.rs` (signature only).
3. W3C Turtle 1.1 and TriG recommendations.

## Goal

- `crates/testing/rdf-diff/tests/adversary-ttl/`:
  - Fixtures targeting `@prefix` redefinition scoping, long-literal
    escape boundaries, BOM handling, numeric-literal typing edges,
    nested collection forms, IRI base resolution in chained `@base`.
  - `README.md` with index + hypothesis per fixture.
- Integration with `xtask verify` under `adversary-ttl`.

## Acceptance

- At least one fixture per finding in the brief.
- Non-zero divergence surfaced in at least one fixture.

## Claims

`crates/testing/rdf-diff/tests/adversary-ttl/**`. Adversary fixture
paths never overlap non-adversary test paths by construction.

## Memory

- `verification/adversary-findings/ttl/fixtures-index`.

## Exit handoff

To `v1-adv-veto`.
