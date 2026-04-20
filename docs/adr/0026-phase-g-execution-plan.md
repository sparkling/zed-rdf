# ADR-0026: Phase G execution plan — LSP polish via mesh swarm

- **Status:** Proposed
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy)
- **Retires-on:** tag `phase-g/done`
- **Tags:** `process`, `execution`, `phase-g`, `lsp`, `polish`

## Context and Problem Statement

Phase F closes at tag `phase-f/done` with `rdf-lsp` covering the core
LSP feature set: `didOpen`/`didChange`/`publishDiagnostics`, hover,
completion, goto-definition, documentSymbol, and formatting across all
11 RDF/SPARQL languages.

Phase G per [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2
lands the polish layer on top of that core:

- **Rename**: per-language rename rules determining which occurrences of
  a symbol constitute the same entity. Described in
  [`../sparc/02-pseudocode.md`](../sparc/02-pseudocode.md) §10.
- **Code actions**: extract prefix, add missing prefix, sort prefixes.
- **Semantic tokens**: per-language semantic token provider classifying
  every token for syntax highlighting. Shared legend defined in
  pre-flight.
- **Workspace symbols**: cross-file symbol index.
- **Incremental parsing**: changed-range to smallest re-parsed subtree,
  as specified in [`../sparc/02-pseudocode.md`](../sparc/02-pseudocode.md)
  §8.

Performance targets from `04-refinement.md` §5:

- LSP cold-open 10k-line Turtle: highlight ≤ 100 ms.
- First diagnostics ≤ 500 ms.

Phase G has no intra-phase DAG once the shared semantic-token legend and
incremental-parse interface are frozen in pre-flight. Rename + code
actions, semantic tokens, and incremental parsing are fully independent
of each other. The mesh (peer) topology from ADR-0017 §4 is appropriate.

## Decision Drivers

- **ADR-0017 compliance.** Phase G topology: mesh. Agents: 3 coders +
  1 tester.
- **Sequential gate discipline.** This ADR activates only after
  `phase-f/done` is tagged.
- **Performance accountability.** The ≤ 100 ms cold-open target is a
  hard criterion bench gate, not a guideline.
- **Shared contract freeze.** The semantic-token legend and incremental
  parse interface must be pinned before any coder begins, so agents do
  not diverge on the shared surface.

## Considered Options

1. **Sequential: rename → code actions → semantic tokens →
   incremental.** Serialises independent work; wastes concurrency.
2. **Mesh parallel swarm with pre-flight freeze.** Three coders spawn
   simultaneously after the legend and interface contract are frozen.
   Chosen.
3. **Fold incremental parsing into Phase F.** Phase F is already 4–5
   weeks; adding incremental parse inflates scope and blocks Phase G
   entry.

## Decision

**Chosen option: 2 — mesh parallel swarm with pre-flight freeze.**

### 1. Pre-flight (orchestrator, in-session)

Sequential; must complete before any spawn.

1. Confirm `phase-f/done` tag exists in the repository.
2. Define the semantic-token legend shared across all 11 languages.
   Write to `crates/rdf-lsp/src/semantic_tokens.rs` as a frozen public
   constant (`pub const LEGEND: SemanticTokensLegend`). Every language's
   token provider must index into this legend without modifying it.
3. Write the incremental parse interface specification as pseudocode in
   `docs/sparc/02-pseudocode.md` §8 (if not already present). The
   interface must expose: `fn update(&mut self, change: TextDocumentContentChangeEvent)
   -> ChangedRange` and `fn reparse(&mut self, range: ChangedRange) -> &Ast`.
4. Amend `docs/agent-cohorts.md` with Phase G agent rows.
5. Seed memory namespaces: `phase-g`, `crate/rdf-lsp`.
6. Write per-agent prompt files to `scripts/spawn/phase-g/`.

Exit: semantic-token legend committed; incremental parse interface
pseudocode present; `cargo check --workspace --all-features` green;
cohort registry updated.

### 2. Worker roster — cohort A (mesh topology)

Namespace: `phase-g`. Topology: mesh (peer). Base model:
`claude-opus-4-7`.

| Agent id | Role | Worktree | Claims | Deliverable |
|---|---|---|---|---|
| `pg-rename-actions` | `coder` | yes | `crates/rdf-lsp/src/rename.rs`, `crates/rdf-lsp/src/code_actions.rs` | Rename + code actions per language (extract prefix, add missing prefix, sort prefixes). |
| `pg-sem-tokens` | `coder` | yes | `crates/rdf-lsp/src/semantic_tokens.rs` | Semantic token provider for all 11 languages, indexing into the frozen legend. |
| `pg-incremental` | `coder` | yes | `crates/rdf-lsp/src/incremental.rs` | Incremental parse pipeline per pseudocode §8. |
| `pg-tester` | `tester` | no | `crates/rdf-lsp/tests/g/**` | Integration tests per feature; criterion bench verifying ≤ 100 ms cold-open highlight on 10k-line Turtle. |

Concurrency: 4 agents.

### 3. Shadows

Not required. Phase G targets the LSP layer, which does not carry a W3C
conformance suite. Per ADR-0019 §3, shadows apply to parser formats with
W3C suites only.

### 4. Integration pass (orchestrator, in-session)

Triggered by the last cohort-A completion callback. Not polled.

1. Merge worktrees: `pg-rename-actions` → `pg-sem-tokens` →
   `pg-incremental` → apply tester fixtures. Resolve any workspace
   `Cargo.toml` overlap.
2. Run `cargo test --workspace --all-features --no-fail-fast`;
   `cargo clippy --workspace --all-features -- -D warnings`;
   `cargo deny check`; `xtask verify`.
3. Phase-G exit gate check:
   - Rename: per-language integration tests green.
   - Code actions: extract-prefix, add-missing-prefix, sort-prefixes
     tests green.
   - Semantic tokens: all 11 language providers pass snapshot tests.
   - Incremental parse: round-trip correctness tests green.
   - **Perf gate**: criterion bench on 10k-line Turtle: highlight
     ≤ 100 ms; first diagnostics ≤ 500 ms. CI fails on breach.
4. Flip this ADR to Accepted. Update
   `docs/sparc/04-refinement.md` Phase G retro. Tag `phase-g/done`.

### 5. Hard parallelism rules (restated from ADR-0017 §6)

1. All spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. Dependent work resolves via frozen legend + interface, not spawn
   ordering.
4. Integration only in the orchestrator session.
5. `claims_claim` before every edit.

## Consequences

### Positive
- Three independent coder agents land rename/actions, semantic tokens,
  and incremental parsing in parallel; wall-clock dominated by the
  slowest, not their sum.
- Shared semantic-token legend frozen pre-flight eliminates the risk of
  per-language token-type divergence.
- Criterion bench in the exit gate makes the ≤ 100 ms target a hard
  CI invariant, not a soft aspiration.

### Negative
- Incremental parse adds surface area in `rdf-lsp`; any future parser
  API change must update the incremental pipe.
- Workspace symbols (cross-file index) require a VFS or file-watching
  integration; if the LSP crate's VFS is not settled at Phase F exit,
  this sub-feature may need to be descoped to a follow-on PR.

### Neutral
- This ADR becomes historical on tag `phase-g/done`.
- Phase G does not introduce new crates; all deliverables land in the
  existing `rdf-lsp` crate.

## Validation

- One spawn message — orchestrator tool-call log shows a single
  Agent-tool message with 4 entries.
- Exit gate green per §4.3 above before ADR flips to Accepted.
- Criterion bench output recorded in `bench/baselines/` and committed.
- No orphaned worktrees after integration.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy
  (topology table §4, quality gates §7).
- [`0019-independent-verification.md`](0019-independent-verification.md)
  — shadow scope rule (§3).
- [`0025-phase-f-execution-plan.md`](0025-phase-f-execution-plan.md)
  — predecessor phase.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 and §5 —
  Phase G scope, performance targets.
- [`../sparc/02-pseudocode.md`](../sparc/02-pseudocode.md) §8 and §10 —
  incremental parse and rename pseudocode.
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
