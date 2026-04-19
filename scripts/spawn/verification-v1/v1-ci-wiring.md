---
agent_id: v1-ci-wiring
cohort: cohort-a
hive: verification-v1
role: cicd-engineer
worktree: true
priority: normal
claims:
  - .github/workflows/verification.yml
  - xtask/verify/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
---

# v1-ci-wiring — PR-gate wiring for the verification sweep

You are cohort-A agent `v1-ci-wiring`. You bolt the diff harness,
shadow-vs-main, cargo-deny check, and oracle fact-corpus consumption
into the PR gate.

## Read first

1. `docs/adr/0019-independent-verification.md` §Validation.
2. `docs/adr/0020-verification-implementation-plan.md` §5.
3. `docs/adr/0006-testing-strategy.md`.
4. Existing `.github/workflows/` directory (if any).

## Goal

- `.github/workflows/verification.yml`:
  - Runs on every PR.
  - `cargo test --workspace --all-features`
  - `cargo clippy --workspace --all-features -- -D warnings`
  - `cargo deny check`
  - `xtask verify` (the harness entry point below).
  - On failure, uploads the `DiffReport` JSON as a build artifact.
- `xtask/verify/`: a new xtask binary that:
  - Loads W3C manifests + edge-case corpora + fact-oracle JSON.
  - Runs the diff harness across main + all shadow parsers + all
    enabled oracles.
  - Emits a `DiffReport` per format + a summary.
  - Exits non-zero on any non-allow-listed divergence.

## Acceptance

- `cargo run -p xtask -- verify` green on a smoke fixture.
- PR gate fails as designed on a deliberately-broken shadow (verify,
  then revert).
- No JVM at test-path time; JVM artifacts are consumed as JSON only.

## Claims

`.github/workflows/verification.yml`, `xtask/verify/**`, workspace
member add for `xtask` (coordinate with other member adds).

## Memory

- `verification/ci/pr-gate/version` — record the pipeline's git sha
  hash after wiring.

## Exit handoff

`v1-reviewer`.
