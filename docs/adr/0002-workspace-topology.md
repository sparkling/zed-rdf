# ADR-0002: Single-repo Cargo workspace topology

- **Status:** Accepted (2026-04-18)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Tags:** `policy`, `repo-layout`

## Context and Problem Statement

The project ships a Rust LSP, a handful of parser crates, a shared
vocabulary crate, a formatter crate, a conformance harness, and a
`wasm32-wasip2` Zed extension. These are tightly coupled: the LSP depends
on all parsers, tests use a shared harness, the extension launches the
LSP. We need a repo layout that keeps cross-crate refactors cheap and
enforces dependency hygiene.

## Decision Drivers

- **Atomic cross-crate changes.** Adding a diagnostic code type touches
  `rdf-diagnostics` and every parser; this should be one PR.
- **Reviewable diffs.** All affected crates visible in one place.
- **Dependency hygiene.** Cycles impossible; the extension must not pull
  in the native LSP as a Rust dependency (it spawns it as a subprocess).
- **Publishing.** ~12 crates; release tooling must scale.
- **Zed extension distinctness.** Different build target
  (`wasm32-wasip2`), different distribution channel (Zed registry), but
  same source tree.

## Considered Options

1. **Single Cargo workspace, polycrate.** One workspace root,
   `crates/*`, `extensions/zed-rdf/`.
2. **Multi-repo.** One repo per group.
3. **Nested workspaces** — root workspace for the engine crates, sub-
   workspace for the Zed extension.
4. **Workspace plus git submodules** for W3C test suites only
   (layered onto option 1 or 3).

## Decision

**Chosen option: Option 1 — a single Cargo workspace — with option 4
layered on top for external test fixtures.**

Layout as in [`03-architecture.md`](../sparc/03-architecture.md) §3.
Specifically:

- `crates/foundations/*`, `crates/syntax/*`, `crates/vocab/*`,
  `crates/format/*`, `crates/lsp/*`, `crates/testing/*` — all in-tree
  Rust crates.
- `extensions/zed-rdf/` is its own `Cargo.toml` with
  `crate-type = ["cdylib"]`. It has **no Rust dependency** on the
  in-tree engine crates; it only spawns `rdf-lsp` as a subprocess. This
  keeps the Wasm extension tiny.
- External test suites (`rdf-tests`, `sparql11-test-suite`, `json-ld-api`,
  `shexTest`) are **git submodules** under `external/tests/`.
- Tree-sitter grammars are **not** submodules; they are pinned by commit
  in `extension.toml` (Zed fetches them at install time). See ADR-0009.
- Cross-crate layering enforced by `cargo-deny`:
  - `crates/lsp/*` depends on `crates/syntax/*`, `crates/foundations/*`,
    `crates/vocab/*`, `crates/format/*`.
  - `crates/syntax/*` depends only on `crates/foundations/*`.
  - `crates/format/*` depends on `crates/syntax/*` and
    `crates/foundations/*`.
  - `extensions/zed-rdf/` depends on no in-tree crate.
- Publishing uses `cargo-release` at a workspace-level version unless a
  crate opts into independent versioning (rare; lock-step is default).

## Consequences

- **Positive**
  - Atomic refactors easy; one PR touches every crate it needs.
  - Dependency graph enforced in one place.
  - Single CI pipeline exercises the whole project.
  - Contributors see the full architecture by reading one tree.
- **Negative**
  - Full-workspace `cargo build` is slower than per-crate; `sccache` +
    layer caching in CI mitigates.
- **Neutral**
  - Extension is co-located with the engine, so a clone pulls everything;
    extension users install from the Zed registry, not by cloning.

## Validation

- `cargo-deny check` green in CI (layering respected).
- Any refactor that adds a new diagnostic across all parsers lands in a
  single PR.
- `cargo release` produces every crate's tag + changelog in one command.

## Links

- `docs/sparc/03-architecture.md` §3 crate topology.
- ADR-0003 DDD bounded contexts.
- ADR-0004 Third-party crate policy.
