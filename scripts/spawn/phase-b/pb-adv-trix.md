---
agent_id: pb-adv-trix
cohort: cohort-b
hive: phase-b-adv
role: tester
model: claude-sonnet-4-6
worktree: true
claims:
  - crates/testing/rdf-diff/tests/adversary-trix/**
forbidden_reads:
  - phase-b
  - crates/rdf-trix
---

# pb-adv-trix — Adversary fixture corpus for TriX

You are cohort-B agent `pb-adv-trix`. You create adversary test fixtures
for TriX.

## Read first (permitted)

1. `scripts/spawn/phase-b/adv-briefs.md` — the `trix` section (or from
   `verification/adversary-findings/trix` in `phase-b-adv` memory).
2. TriX: <https://www.hpl.hp.com/techreports/2004/HPL-2004-56.html>
3. `docs/adr/0019-independent-verification.md` §4.

## Goal

Create fixture files under `crates/testing/rdf-diff/tests/adversary-trix/`:
- At least one `.trix` (XML) file per failure mode.
- `.expected` companion (ACCEPT or REJECT).
- `manifest.toml`.

## Acceptance

- ≥3 adversary fixtures. `manifest.toml` valid.

## Claims

Claim `crates/testing/rdf-diff/tests/adversary-trix/**`. Release on completion.

## Memory

- `memory_store` exit report at `phase-b-adv`: `pb-adv-trix:done`.

## Handoff

`claims_accept-handoff` → `pb-adv-veto`.
