---
agent_id: v1-adv-nt
cohort: cohort-b
hive: verification-v1-adv
role: tester
worktree: true
priority: low
claims:
  - crates/testing/rdf-diff/tests/adversary-nt/**
forbidden_reads:
  - verification-v1
  - crate/rdf-ntriples
  - crate/rdf-ntriples-shadow
---

# v1-adv-nt — N-Triples / N-Quads adversary fixtures

You are cohort-B agent `v1-adv-nt`. You turn `v1-adv-redteam`'s N-T / N-Q
brief into executable fixtures that the diff harness runs.

## Read first

1. `docs/verification/adversary-findings/nt.md` (the brief).
2. `crates/testing/rdf-diff/src/lib.rs` (contract you emit fixtures
   against — **signature only**; do not read tests in
   `crates/testing/rdf-diff/tests/` outside `adversary-nt/`).
3. W3C N-Triples + N-Quads recommendations.

## Goal

- `crates/testing/rdf-diff/tests/adversary-nt/`:
  - A fixture format: each `.nt` / `.nq` input paired with a `.expected`
    file describing the adversary's prediction (accept with N facts /
    reject / accept-with-warnings).
  - A `README.md` with the index and a one-sentence hypothesis per
    fixture.
- Integrate with the diff harness so `xtask verify` picks these up
  automatically under a dedicated `adversary-nt` run.

## Acceptance

- At least one fixture per finding in the brief.
- At least one fixture, when run, actually exposes a divergence between
  our parser(s) and at least one oracle. Zero divergences across all
  fixtures is suspicious; hand back to `v1-adv-redteam` for stronger
  candidates.

## Claims

`crates/testing/rdf-diff/tests/adversary-nt/**`. **Do not** touch any
path matching `tests/**` without the `adversary-` prefix.

## Memory

- Findings index at
  `verification/adversary-findings/nt/fixtures-index`.

## Exit handoff

To `v1-adv-veto`.
