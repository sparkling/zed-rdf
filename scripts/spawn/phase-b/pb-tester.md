---
agent_id: pb-tester
cohort: cohort-a
hive: phase-b
role: tester
model: claude-opus-4-7
worktree: false
claims:
  - crates/testing/rdf-diff/tests/**
forbidden_reads:
  - phase-b-adv
  - verification/adversary-findings/rdfxml
  - verification/adversary-findings/jsonld
  - verification/adversary-findings/trix
  - verification/adversary-findings/n3
---

# pb-tester — Cross-format adversary + snapshot wiring

You are cohort-A agent `pb-tester`. Your job is to extend the diff-harness
test suite for all four Phase B formats.

## Read first

1. `crates/testing/rdf-diff/src/lib.rs` — frozen trait + harness types.
2. `crates/testing/rdf-diff/tests/` — existing Phase A test patterns.
3. `crates/testing/rdf-diff-oracles/src/lib.rs` — oracle adapters for
   `oracle-oxrdfxml` and `oracle-oxjsonld`.
4. `docs/adr/0019-independent-verification.md` §2-3 — test layer rules.
5. `docs/adr/0021-phase-b-execution-plan.md` §6.3 — exit gate.

## Goal

1. Add snapshot tests for all four Phase B parsers against their respective
   oracle adapters (rdf-xml vs oxrdfxml-oracle; rdf-jsonld vs oxjsonld-oracle).
2. Un-ignore any snapshot tests that are currently `#[ignore]` pending
   Phase B implementation.
3. Extend the diff harness to wire adversary fixtures from
   `crates/testing/rdf-diff/tests/adversary-{rdfxml,jsonld,trix,n3}/` once
   those dirs are populated by the adversary agents.
4. Confirm that shadow diff runs (rdf-xml vs rdf-xml-shadow, rdf-jsonld vs
   rdf-jsonld-shadow) are hooked in with `oracle-oxrdfxml` / `oracle-oxjsonld`
   as the third comparator.

## Acceptance

- `cargo test -p rdf-diff --all-features` green.
- At least one passing snapshot per format (rdfxml, jsonld, trix, n3).

## Claims

Claim `crates/testing/rdf-diff/tests/**`. Release on completion.

## Memory

- `memory_store` exit report at `phase-b` blackboard: `pb-tester:done`.

## Handoff

`claims_accept-handoff` → `pb-reviewer`.
