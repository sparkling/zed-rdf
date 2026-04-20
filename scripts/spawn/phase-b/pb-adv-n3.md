---
agent_id: pb-adv-n3
cohort: cohort-b
hive: phase-b-adv
role: tester
model: claude-sonnet-4-6
worktree: true
claims:
  - crates/testing/rdf-diff/tests/adversary-n3/**
forbidden_reads:
  - phase-b
  - crates/rdf-n3
---

# pb-adv-n3 — Adversary fixture corpus for N3

You are cohort-B agent `pb-adv-n3`. You create adversary test fixtures for N3.

## Read first (permitted)

1. `scripts/spawn/phase-b/adv-briefs.md` — the `n3` section (or from
   `verification/adversary-findings/n3` in `phase-b-adv` memory).
2. N3: <https://www.w3.org/TeamSubmission/n3/>
3. `docs/adr/0019-independent-verification.md` §4.

## Goal

Create fixture files under `crates/testing/rdf-diff/tests/adversary-n3/`:
- At least one `.n3` file per failure mode.
- `.expected` companion (ACCEPT or REJECT).
- `manifest.toml`.

## Acceptance

- ≥3 adversary fixtures. `manifest.toml` valid.

## Claims

Claim `crates/testing/rdf-diff/tests/adversary-n3/**`. Release on completion.

## Memory

- `memory_store` exit report at `phase-b-adv`: `pb-adv-n3:done`.

## Handoff

`claims_accept-handoff` → `pb-adv-veto`.
