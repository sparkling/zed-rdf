---
agent_id: v1-adv-iri
cohort: cohort-b
hive: verification-v1-adv
role: tester
worktree: true
priority: low
claims:
  - crates/testing/rdf-diff/tests/adversary-iri/**
forbidden_reads:
  - verification-v1
  - crate/rdf-iri
  - crate/rdf-iri-shadow
---

# v1-adv-iri — IRI normalisation / resolution adversary fixtures

You are cohort-B agent `v1-adv-iri`.

## Read first

1. `docs/verification/adversary-findings/iri.md`.
2. `crates/testing/rdf-diff/src/lib.rs` (signature only).
3. **RFC 3987** (IRI) + **RFC 3986** (URI). The divergence between
   these is fertile adversary territory.

## Goal

- `crates/testing/rdf-diff/tests/adversary-iri/`:
  - Fixtures targeting percent-encoding order-of-operations, IDN host
    handling, path-segment resolution (`..` relative to an empty path),
    scheme case folding, reserved-character behaviour in queries vs
    fragments.
  - `README.md` with index + hypothesis per fixture.
- Integration with `xtask verify` under `adversary-iri`.

## Acceptance

- At least one fixture per finding.
- Non-zero divergence surfaced.

## Claims

`crates/testing/rdf-diff/tests/adversary-iri/**`.

## Memory

- `verification/adversary-findings/iri/fixtures-index`.

## Exit handoff

To `v1-adv-veto`.
