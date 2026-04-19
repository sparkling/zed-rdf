---
agent_id: v1-oracle-rust
cohort: cohort-a
hive: verification-v1
role: coder
worktree: true
priority: normal
claims:
  - crates/testing/rdf-diff-oracles/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
---

# v1-oracle-rust — Rust-side oracle adapter crate

You are cohort-A agent `v1-oracle-rust`. Create the new crate
`crates/testing/rdf-diff-oracles` that wraps the permitted reference
parsers as `[dev-dependencies]` only, exposing them through the frozen
`rdf_diff::Parser` trait.

## Read first

1. `crates/testing/rdf-diff/src/lib.rs` — the `Parser` trait you will
   implement.
2. `docs/adr/0019-independent-verification.md` §1 — the permitted list
   of oracle crates.
3. `docs/adr/0004-third-party-crate-policy.md` — the rule being amended.

## Goal

New crate `rdf-diff-oracles` with:

- `[dev-dependencies]`: `oxttl`, `oxrdfxml`, `oxjsonld`,
  `oxsparql-syntax`, `sophia_*` (optional). Do not add these to
  `[dependencies]` under any circumstance.
- `pub mod oxttl_adapter`, `oxrdfxml_adapter`, `oxjsonld_adapter`,
  `oxsparql_adapter`, `sophia_adapter`, each implementing
  `rdf_diff::Parser`.
- Each adapter translates the upstream's native AST into the frozen
  `Facts` form via `Facts::canonicalise`. Adapters do **not** invent
  semantics; if the upstream rejects input, return
  `Err(Diagnostics { fatal: true, messages: … })`.
- Adapters are behind per-oracle Cargo features (`oracle-oxttl`,
  `oracle-oxrdfxml`, `oracle-oxjsonld`, `oracle-oxsparql`,
  `oracle-sophia`) so downstream harness tests can isolate which
  oracles to run.

## Acceptance

- `cargo check -p rdf-diff-oracles --all-features` green.
- `cargo tree -p rdf-diff-oracles --target all -e normal` shows **zero**
  `ox*` / `sophia_*` under normal edges.
- Smoke test: round-trips a trivial Turtle document through each
  adapter and asserts the resulting `Facts` are canonically equal.

## Claims

`crates/testing/rdf-diff-oracles/**` and add the crate as a workspace
member in `Cargo.toml`. Coordinate the workspace-root edit with other
agents by claiming `Cargo.toml` briefly, appending your member line,
and releasing.

## Memory

- `memory_store` at `adapters/layout` in `crate/rdf-diff-oracles`
  describing which oracle owns which format.

## Exit handoff

`claims_accept-handoff` → `v1-reviewer`.
