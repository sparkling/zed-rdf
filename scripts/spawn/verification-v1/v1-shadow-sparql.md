---
agent_id: v1-shadow-sparql
cohort: cohort-a
hive: verification-v1
role: coder
worktree: true
priority: normal
base_model_override: claude-sonnet-4-6
claims:
  - crates/syntax/sparql-syntax-shadow/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
  - crate/sparql-syntax
---

# v1-shadow-sparql — disjoint second SPARQL 1.1 syntax parser (no execution)

You are cohort-A agent `v1-shadow-sparql`.

## Read first

1. `crates/testing/rdf-diff/src/lib.rs`.
2. `docs/adr/0019-independent-verification.md` §3.
3. **SPARQL 1.1 Query Language** recommendation. Do not read the main
   `sparql-syntax` source.

## Goal

- New crate `crates/syntax/sparql-syntax-shadow` implementing
  `rdf_diff::Parser` over SPARQL query strings.
- **Grammar only** — no query execution, no dataset evaluation.
- Emits a canonical AST-as-Facts mapping that the diff harness can
  compare across implementations (a stable fact-encoding of the parse
  tree; document it in the crate's README).
- Handles: update syntax (DELETE/INSERT/LOAD/CLEAR), federated query
  (SERVICE), property paths, literal comparison rules at the lexical
  level.

## Acceptance

- `cargo test -p sparql-syntax-shadow --features shadow` green.
- Diff against `oxsparql-syntax` oracle on SPARQL 1.1 conformance
  manifests is non-empty on first run; clean output is suspicious.

## Claims

`crates/syntax/sparql-syntax-shadow/**`; workspace member add.

## Memory

- `crate/sparql-syntax-shadow` only.

## Exit handoff

`v1-reviewer`.
