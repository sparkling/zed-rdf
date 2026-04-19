---
agent_id: v1-adv-sparql
cohort: cohort-b
hive: verification-v1-adv
role: tester
worktree: true
priority: low
claims:
  - crates/testing/rdf-diff/tests/adversary-sparql/**
forbidden_reads:
  - verification-v1
  - crate/sparql-syntax
  - crate/sparql-syntax-shadow
---

# v1-adv-sparql — SPARQL 1.1 syntax adversary fixtures

You are cohort-B agent `v1-adv-sparql`.

## Read first

1. `docs/verification/adversary-findings/sparql.md`.
2. `crates/testing/rdf-diff/src/lib.rs` (signature only).
3. SPARQL 1.1 Query Language recommendation.

## Goal

- `crates/testing/rdf-diff/tests/adversary-sparql/`:
  - Fixtures targeting property-path ambiguity, SERVICE nesting,
    update-vs-query keyword collisions, literal-comparison corner
    cases, BIND scoping, subquery projection-list edge cases.
  - Grammar only; execution semantics out of scope.
  - `README.md` with index + hypothesis per fixture.
- Integration with `xtask verify` under `adversary-sparql`.

## Acceptance

- At least one fixture per finding.
- Non-zero divergence surfaced.

## Claims

`crates/testing/rdf-diff/tests/adversary-sparql/**`.

## Memory

- `verification/adversary-findings/sparql/fixtures-index`.

## Exit handoff

To `v1-adv-veto`.
