# Architecture Decision Records

This directory holds the **Architecture Decision Records (ADRs)** for
`zed-rdf`, written in the **[MADR 3.0](https://adr.github.io/madr/)**
format.

An ADR exists when a decision is architecturally significant enough that
later maintainers (including future us) will benefit from seeing the
*context* and *alternatives*, not just the outcome. Reach for an ADR when:

- the decision trades off between alternatives with real costs on both
  sides,
- the decision is hard to reverse,
- the decision touches more than one bounded context,
- people outside the codebase will ask *why*.

Small, local decisions live in code comments or PR descriptions; they do
not earn an ADR.

## Filing an ADR

1. Copy [`0000-madr-template.md`](0000-madr-template.md) to the next free
   number (`NNNN-kebab-case-title.md`).
2. Fill it in. Start in status **Proposed**.
3. Open a PR. Reviewers discuss in PR comments; the ADR body is edited as
   consensus forms.
4. Merge when status flips to **Accepted**.
5. If a later ADR supersedes this one, add a **Superseded by ADR-XXXX**
   line at the top of the old ADR and a **Supersedes ADR-YYYY** line in
   the new one.

ADRs are **immutable after acceptance** for their substantive content.
Fixes to typos and broken links are fine; changing the decision requires
a new ADR that supersedes the old one.

## Status lifecycle

| Status        | Meaning                                                       |
|---------------|---------------------------------------------------------------|
| `Proposed`    | In discussion; content may change.                            |
| `Accepted`    | Decision stands. Implementations must follow it.              |
| `Rejected`    | Discussed and declined. Kept for future readers.              |
| `Deprecated`  | No longer current; no replacement exists yet.                 |
| `Superseded`  | Replaced by a later ADR (see the superseded-by link).         |

## Numbering

Four-digit zero-padded, monotonically increasing. Do not renumber after
merge. Gaps from abandoned drafts are fine — keep them as signposts.

## Index

| #     | Title                                                                  | Status                |
|-------|------------------------------------------------------------------------|-----------------------|
| 0000  | [MADR template](0000-madr-template.md)                                 | —                     |
| 0001  | [Rust edition, toolchain, no MSRV commitment](0001-rust-toolchain.md)  | Accepted 2026-04-18   |
| 0002  | [Single-repo Cargo workspace topology](0002-workspace-topology.md)     | Accepted 2026-04-18   |
| 0003  | [DDD bounded contexts](0003-ddd-bounded-contexts.md)                   | Accepted 2026-04-18   |
| 0004  | [Third-party crate policy ("no forking" interpretation)](0004-third-party-crate-policy.md) | Accepted 2026-04-18 |
| 0005  | [Parser correctness scope boundary](0005-soundness-completeness-scope.md) | Accepted 2026-04-18 |
| 0006  | [Testing strategy](0006-testing-strategy.md)                           | Accepted 2026-04-18   |
| 0017  | [Implementation execution — ruflo-orchestrated parallel agent swarms](0017-execution-model.md) | Accepted 2026-04-18 |
| 0018  | [Phase A execution plan — parser foundations via ruflo-orchestrated parallel swarm](0018-phase-a-execution-plan.md) | Proposed 2026-04-18 |
| 0019  | [Independent verification against shared-prior failure modes](0019-independent-verification.md) | Proposed 2026-04-19 |
| 0020  | [Implementation plan for ADR-0019 — single-shot parallel swarm](0020-verification-implementation-plan.md) | Proposed 2026-04-19 |

### Reserved numbers (placeholders to be filled per phase)

| #     | Title                                                                  | Phase |
|-------|------------------------------------------------------------------------|-------|
| 0007  | Parser technology (`chumsky` vs `winnow` vs hand-written)              | A     |
| 0008  | Error model and diagnostic representation                              | A     |
| 0009  | Tree-sitter grammar policy (pin existing where possible; write our own for gaps) | A-H |
| 0010  | `rdf-format` formatter framework (shared vs per-format)                | E     |
| 0011  | LSP framework (`tower-lsp` vs `async-lsp` vs custom); stdio transport  | F     |
| 0012  | LSP capability matrix (which capabilities ship at v1.0)                | F-G   |
| 0013  | Incremental parsing strategy (re-parse vs rope-diff)                   | G     |
| 0014  | RDF 1.2 / SPARQL 1.2 default-on; `rdf-1-1-strict` / `sparql-1-1-strict` opt-out features | A, C |
| 0015  | Zed extension identifier + Marketplace publishing strategy             | H     |
| 0016  | Release engineering and semver policy                                  | I     |

Remove the row and replace with a numbered, filled ADR when ready.
