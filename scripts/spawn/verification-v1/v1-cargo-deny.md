---
agent_id: v1-cargo-deny
cohort: cohort-a
hive: verification-v1
role: coder
worktree: true
priority: normal
claims:
  - deny.toml
  - crates/testing/deny-regression/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
---

# v1-cargo-deny — enforce the oracle-only `[dev-dependencies]` carve-out

You are cohort-A agent `v1-cargo-deny`. ADR-0019 §1 amends ADR-0004 to
permit `ox*` and `sophia_*` as `[dev-dependencies]` only. You land the
mechanical guard.

## Read first

1. `deny.toml` (current state).
2. `docs/adr/0019-independent-verification.md` §1 — exact permitted
   list.
3. `docs/adr/0004-third-party-crate-policy.md` — current policy.

## Goal

- Update `deny.toml` so that `oxttl`, `oxrdfxml`, `oxjsonld`,
  `oxsparql-syntax`, and any `sophia_*` are **banned** in normal
  dependency edges but explicitly allowed under dev edges. Use
  `[bans.workspace-dependencies]` or the `deny` table's allowance
  mechanism per the current `cargo-deny` schema.
- New crate `crates/testing/deny-regression`:
  - A standalone integration test that invokes `cargo metadata` and
    asserts no banned crate appears in the normal dependency closure
    of any non-test binary or library.
  - Runs under `cargo test -p deny-regression` without network.

## Acceptance

- `cargo deny check` green locally.
- `cargo test -p deny-regression` green, with a deliberately inserted
  bad dependency causing the test to **fail** (verify by transient
  edit, then revert).
- Paste the proof run into your memory summary at
  `verification/cargo-deny/proof-of-failure` in the
  `verification/cargo-deny` namespace.

## Claims

`deny.toml`, `crates/testing/deny-regression/**`, plus a short claim on
workspace `Cargo.toml` for the member add.

## Memory

- Read `verification/cargo-deny` for the policy carve-out seed.
- Write outcome + cargo-deny config hash back.

## Exit handoff

`v1-reviewer`.
