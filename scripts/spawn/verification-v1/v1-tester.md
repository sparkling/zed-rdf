---
agent_id: v1-tester
cohort: cohort-a
hive: verification-v1
role: tester
worktree: false
priority: normal
claims:
  - crates/testing/rdf-diff/tests/**
  - crates/syntax/*-shadow/tests/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
---

# v1-tester — property + snapshot tests for diff harness and shadows

You are cohort-A agent `v1-tester`. No worktree: you add tests to
existing crates as they land, integrating on the main branch without
spawning your own.

## Read first

1. `crates/testing/rdf-diff/src/lib.rs`.
2. `docs/adr/0006-testing-strategy.md`.
3. Each shadow crate's source as it lands.

## Goal

- Property tests (proptest or quickcheck) for:
  - `Facts::canonicalise` idempotence.
  - `diff(a, a)` is clean for any canonical `a`.
  - `diff(a, b) == diff(b, a)` (commutativity at the divergence-set
    level, not list-order).
- Snapshot tests for shadow-vs-main on curated inputs (initially small;
  grow as `v1-adv-*` produce findings).
- Integration tests that exercise `xtask verify` on smoke corpora.

## Acceptance

- `cargo test --workspace` green.
- Coverage measurement exists for `rdf-diff` (integrate with
  `cargo-llvm-cov` target added by `v1-ci-wiring`).

## Claims

No worktree. Claim file paths directly before edit and release after.
Do not claim any file currently held by another cohort-A agent; wait
for their release.

## Memory

- `verification/tests/catalogue` — index of what tests cover which
  invariant.

## Exit handoff

`v1-reviewer`.
