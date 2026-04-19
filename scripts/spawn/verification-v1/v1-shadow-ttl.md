---
agent_id: v1-shadow-ttl
cohort: cohort-a
hive: verification-v1
role: coder
worktree: true
priority: normal
base_model_override: claude-sonnet-4-6
claims:
  - crates/syntax/rdf-turtle-shadow/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
  - crate/rdf-turtle
---

# v1-shadow-ttl — disjoint second Turtle + TriG implementation

You are cohort-A agent `v1-shadow-ttl`.

## Read first

1. `crates/testing/rdf-diff/src/lib.rs`.
2. `docs/adr/0019-independent-verification.md` §3.
3. W3C **Turtle 1.1** and **TriG** recommendations. Do not read the
   main `rdf-turtle` source or its memory.

## Goal

- New crate `crates/syntax/rdf-turtle-shadow` implementing
  `rdf_diff::Parser` for Turtle and TriG.
- Particular attention to: `@prefix` / `@base` resolution, long literal
  forms (`"""…"""`), numeric literal typing, BNode scoping across
  `@prefix` redefinitions (known ambiguity — coordinate with
  `v1-specpins`), collection syntax `( … )` and `[ … ]`.
- Gated by `shadow` feature.

## Acceptance

- `cargo test -p rdf-turtle-shadow --features shadow` green.
- Diff-harness cross-check against main + `oxttl` oracle: main and
  shadow disagreements surface concrete Turtle inputs.
- Zero divergence against oracle **and** main is suspicious — escalate
  to `v1-specpins`.

## Claims

`crates/syntax/rdf-turtle-shadow/**`; workspace member add.

## Memory

- `crate/rdf-turtle-shadow` only.

## Exit handoff

`v1-reviewer`.
