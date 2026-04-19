---
agent_id: v1-shadow-nt
cohort: cohort-a
hive: verification-v1
role: coder
worktree: true
priority: normal
base_model_override: claude-sonnet-4-6
claims:
  - crates/syntax/rdf-ntriples-shadow/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
  - crate/rdf-ntriples
---

# v1-shadow-nt — disjoint second N-Triples + N-Quads implementation

You are cohort-A agent `v1-shadow-nt`, running on a different base model
from the main `rdf-ntriples` implementer.

## Read first

1. `crates/testing/rdf-diff/src/lib.rs`.
2. `docs/adr/0019-independent-verification.md` §3.
3. W3C **N-Triples** and **N-Quads** recommendations. Do not read the
   main `rdf-ntriples` source or its `crate/rdf-ntriples` memory.

## Goal

- New crate `crates/syntax/rdf-ntriples-shadow` with an N-Triples
  parser and an N-Quads parser implementing `rdf_diff::Parser`.
- Gated by `shadow` feature.
- Emit structured `Diagnostics` for rejection cases, matching the
  frozen trait's contract.
- Particular attention to: Unicode escapes (`\uXXXX` /
  `\UXXXXXXXX`), BOM handling, line-terminator variants, literal
  lexical form preservation (no trimming).

## Acceptance

- `cargo test -p rdf-ntriples-shadow --features shadow` green.
- W3C N-Triples + N-Quads test suites pass when run via the shadow
  (once `v1-oracle-jvm` materialises the fact corpus, cross-check).
- Non-zero divergence vs main on first diff-harness run; zero is
  suspicious.

## Claims

`crates/syntax/rdf-ntriples-shadow/**`; workspace member add.

## Memory

- `crate/rdf-ntriples-shadow` only.

## Exit handoff

`v1-reviewer`.
