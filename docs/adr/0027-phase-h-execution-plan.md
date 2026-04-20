# ADR-0027: Phase H execution plan — Zed extension via mesh swarm

- **Status:** Proposed
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy)
- **Retires-on:** tag `phase-h/done`
- **Tags:** `process`, `execution`, `phase-h`, `zed`, `extension`

## Context and Problem Statement

Phase G closes at tag `phase-g/done` with `rdf-lsp` offering the
complete LSP feature set including rename, code actions, semantic tokens,
workspace symbols, and incremental parsing across all 11 languages.

Phase H per [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2
wraps that LSP server as a Zed extension:

- `extensions/zed-rdf/extension.toml` — manifest declaring supported
  languages and grammars.
- `extensions/zed-rdf/src/lib.rs` — extension entry point; the only
  job is launching `rdf-lsp` as the LSP server.
- Per-language `config.toml` files — language configuration for each of
  the 11 languages: NT, NQ, Turtle, TriG, RDF/XML, JSON-LD, TriX, N3,
  SPARQL, ShEx, Datalog.
- Tree-sitter `.scm` query files — highlight, injection, and locals
  queries for each language.
- Grammar pins — one `git` submodule or vendored snapshot per language
  grammar, pinned to a specific commit.

Active risks from `04-refinement.md` §4:

- **R-7 (`zed_extension_api` churn).** Mitigation: pin the
  `extension_api` version; the extension itself is thin (a launcher
  only), so migrations are cheap.
- **R-9 (tree-sitter query bitrot).** Mitigation: CI job running
  `tree-sitter query parse` against the pinned grammar on every PR that
  touches a grammar pin or a `.scm` file.

Languages in scope: NT, NQ, Turtle, TriG, RDF/XML, JSON-LD, TriX, N3,
SPARQL, ShEx, Datalog (11 total).

## Decision Drivers

- **ADR-0017 compliance.** Phase H topology: mesh. Agents: 1 coder
  (extension) + 1 coder (grammars) + 1 reviewer.
- **Sequential gate discipline.** This ADR activates only after
  `phase-g/done` is tagged.
- **Risk R-7 mitigation.** The `zed_extension_api` version must be
  pinned before coding begins; an ADR records the pin.
- **Risk R-9 mitigation.** The CI query-parse job is a hard gate; it
  must be green before `phase-h/done` is tagged.

## Considered Options

1. **Single coder for everything.** Serialises extension scaffold and 11
   grammar pin tasks; slow.
2. **Mesh: extension coder + grammar coder + reviewer.** Extension
   scaffold and grammar pinning are independent work streams. Chosen.
3. **Fold into Phase G.** Phase G is already 2–3 weeks and is focused on
   LSP internals, not Zed packaging; merging scopes bloats Phase G and
   conflates different bounded contexts.

## Decision

**Chosen option: 2 — mesh parallel swarm.**

### 1. Pre-flight (orchestrator, in-session)

Sequential; must complete before any spawn.

1. Confirm `phase-g/done` tag exists in the repository.
2. Run a `zed: install dev extension` dry-run (or inspect the current
   Zed extension API changelog) to identify the latest stable
   `zed_extension_api` version. Author a short ADR (or an amendment)
   recording the chosen version pin. Add the pin to
   `extensions/zed-rdf/Cargo.toml`.
3. For each of the 11 languages, identify the community or official
   tree-sitter grammar repository and the commit to pin. Record all 11
   grammar pins in `external/grammars/PINS.md` (create file if absent).
4. Create the `extensions/zed-rdf/` directory scaffold:
   `Cargo.toml`, `extension.toml`, `src/lib.rs` (empty stub launching
   `rdf-lsp`), `languages/` subdirectories for each language.
5. Amend `docs/agent-cohorts.md` with Phase H agent rows.
6. Seed memory namespaces: `phase-h`, `crate/extensions-zed-rdf`.
7. Write per-agent prompt files to `scripts/spawn/phase-h/`.

Exit: `zed_extension_api` version pinned and recorded; grammar pins
documented in `external/grammars/PINS.md`; extension scaffold compiles;
cohort registry updated.

### 2. Worker roster — cohort A (mesh topology)

Namespace: `phase-h`. Topology: mesh (peer). Base model:
`claude-opus-4-7`.

| Agent id | Role | Worktree | Claims | Deliverable |
|---|---|---|---|---|
| `ph-extension` | `coder` | yes | `extensions/zed-rdf/**` | Full Zed extension: `extension.toml`, `src/lib.rs`, per-language `config.toml` files, `.scm` highlight/injection/locals query files for all 11 languages, grammar pins wired into extension manifest. |
| `ph-grammars` | `coder` | yes | `external/grammars/**`, `extensions/zed-rdf/grammars/**` | Tree-sitter grammar pins (submodules or vendored snapshots); CI query-parse job (`.github/workflows/tree-sitter-queries.yml`). |
| `ph-reviewer` | `reviewer` | no | read-only | `zed: install dev extension` smoke test against all 11 languages; ADR-0017 §7 gate review; append-only audit at `.claude-flow/audit/phase-h-reviews/`. |

Concurrency: 3 agents.

### 3. Shadows

Not required. The Zed extension layer has no W3C conformance suite.
Per ADR-0019 §3, shadows apply to RDF parser formats only.

### 4. Integration pass (orchestrator, in-session)

Triggered by the last cohort-A completion callback. Not polled.

1. Merge worktrees: `ph-grammars` first (grammar pins must be present
   before extension manifest resolves them), then `ph-extension`.
   Apply reviewer findings.
2. Run `cargo build --workspace --all-features`;
   `cargo clippy --workspace --all-features -- -D warnings`;
   `cargo deny check`.
3. Run the tree-sitter CI job locally: `tree-sitter query parse` for
   each language's `.scm` files against the pinned grammar.
4. Phase-H exit gate check:
   - `zed: install dev extension` completes without error.
   - All 11 languages activate (syntax highlighting visible) in Zed.
   - Tree-sitter CI job green for all 11 language query files.
   - `ph-reviewer` sign-off recorded in the audit log.
5. Flip this ADR to Accepted. Update
   `docs/sparc/04-refinement.md` Phase H retro. Tag `phase-h/done`.

### 5. Hard parallelism rules (restated from ADR-0017 §6)

1. All spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. Grammar pins must exist before `ph-extension` merges its manifest;
   this ordering is enforced by the integration pass, not by spawn
   sequencing.
4. Integration only in the orchestrator session.
5. `claims_claim` before every edit.

## Consequences

### Positive
- Extension scaffold and grammar pinning proceed in parallel; wall-clock
  is the max of two short work streams, not their sum.
- The extension is intentionally thin (a launcher); the R-7 migration
  cost for any future `zed_extension_api` churn is low.
- The CI query-parse job (R-9 mitigation) is a hard gate that prevents
  silent query bitrot from accumulating after Phase H.

### Negative
- Tree-sitter grammars for some RDF languages (TriX, N3, ShEx, Datalog)
  may not exist in community repos and require authoring or forking.
  This is scoped to Phase H but may extend the timeline.
- Grammar pins are a maintenance burden: each Zed release that updates
  its bundled tree-sitter runtime may require grammar re-pins.

### Neutral
- This ADR becomes historical on tag `phase-h/done`.
- The `zed_extension_api` version pin ADR (authored in pre-flight)
  is a companion to this ADR; its lifecycle is independent.

## Validation

- One spawn message — orchestrator tool-call log shows a single
  Agent-tool message with 3 entries.
- Exit gate green per §4.4 above before ADR flips to Accepted.
- `external/grammars/PINS.md` lists all 11 grammar pins with commit
  hashes and repository URLs.
- Tree-sitter CI job present in `.github/workflows/` and green on
  the integration commit.
- No orphaned worktrees after integration.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy
  (topology table §4, quality gates §7).
- [`0019-independent-verification.md`](0019-independent-verification.md)
  — shadow scope rule (§3).
- [`0026-phase-g-execution-plan.md`](0026-phase-g-execution-plan.md)
  — predecessor phase.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2, §4 (R-7,
  R-9) — Phase H scope and risk register.
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
