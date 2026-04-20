# ADR-0025: Phase F execution plan — LSP core via hierarchical swarm

- **Status:** Proposed
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy)
- **Retires-on:** tag `phase-f/done`
- **Tags:** `process`, `execution`, `phase-f`, `lsp`, `rdf-lsp`

## Context and Problem Statement

Phase E closes at tag `phase-e/done` with `rdf-vocab` complete (11
vocabularies, >= 95 % term coverage) and per-format formatters green.
Phase F per [`../sparc/04-refinement.md`](../sparc/04-refinement.md)
§2 delivers `rdf-lsp`: a binary crate that implements a Language Server
Protocol server over stdin/stdout JSON-RPC, covering all 11 languages
supported by the parser stack.

LSP features required for Phase F:

- Initialisation handshake.
- `textDocument/didOpen`, `textDocument/didChange` with
  `textDocument/publishDiagnostics`.
- `textDocument/hover` (backed by `rdf-vocab` term definitions).
- `textDocument/completion`.
- `textDocument/definition` (goto-definition).
- `textDocument/documentSymbol`.
- `textDocument/formatting` (backed by Phase E formatters).

Languages: N-Triples, N-Quads, Turtle, TriG, RDF/XML, JSON-LD, TriX,
N3, SPARQL, ShEx, Datalog (11 total).

The LSP crate does not exist yet. A choice of LSP framework is required
before implementation begins (see Pre-flight §2 below). Risk R-6 in
`04-refinement.md` flags ecosystem churn for `tower-lsp` and similar
crates; the mitigation is thin LSP glue with feature services
decoupled, so a future framework swap touches only handler registration.

The architect must design the server structure and the feature-service
split before the two backend-dev agents are spawned, making the
hierarchical topology from ADR-0017 §4 appropriate.

## Decision Drivers

- **ADR-0017 compliance.** Phase F topology: hierarchical. Agents:
  1 architect + 2 backend-dev + 1 tester.
- **R-6 mitigation.** Feature services must be decoupled from the LSP
  transport layer to survive a future framework migration.
- **ADR-0004 compliance.** Any new LSP framework crate must be added to
  the allow-list before use.
- **Sequential gate discipline.** This ADR activates only after
  `phase-e/done` is tagged.

## Considered Options

1. **Single backend-dev agent builds the whole LSP binary.** Simpler
   spawn; likely becomes a context-budget problem for a 4–5 week crate.
2. **Hierarchical: architect + protocol agent + features agent.**
   Protocol agent owns the server binary, connection lifecycle, and
   `did*` handlers. Features agent owns hover, completion,
   goto-definition, documentSymbol, formatting. Clear claim boundary.
   Chosen.
3. **Per-language agents (11 agents).** Excessive parallelism for what
   is mostly a dispatch table; the feature logic is shared across
   languages.

## Decision

**Chosen option: 2 — hierarchical with architect pre-pass, protocol
and features split.**

### 1. Pre-flight (orchestrator, in-session)

Sequential; must complete before any spawn.

1. Confirm `phase-e/done` tag exists.
2. Author a new ADR for the LSP crate choice. Evaluate:
   - `tower-lsp` (async, `tower` middleware, widely used).
   - `lsp-types` + `async-lsp` (lower-level; more control).
   - `axum`-based JSON-RPC (bespoke; maximum control, maximum effort).
   Update ADR-0004 allow-list to include the chosen framework crate(s).
3. Create `crates/rdf-lsp/Cargo.toml` + `crates/rdf-lsp/src/main.rs`
   stub (binary crate). Add to workspace `[members]`.
4. Define the LSP integration test harness spec: stdin/stdout JSON-RPC
   driver, request/response fixtures per language per feature. Reference
   `docs/sparc/02-pseudocode.md` §9 for the completion context
   algorithm.
5. Amend `docs/agent-cohorts.md` with Phase F agent rows.
6. Seed memory namespace: `phase-f`, `crate/rdf-lsp`.
7. Write per-agent prompt files to `scripts/spawn/phase-f/`.

Exit: `cargo check --workspace --all-features` green with stub;
LSP framework ADR authored and allow-list updated; cohort registry
updated.

### 2. Worker roster — cohort A (hierarchical topology)

Namespace: `phase-f`. Topology: hierarchical. Base model:
`claude-opus-4-7`.

The architect runs first and delivers the server structure spec before
the two backend-dev agents and the tester are spawned.

| Agent id | Role | Worktree | Claims | Deliverable |
|---|---|---|---|---|
| `pf-architect` | `architecture` | no | read-only | LSP server structure; handler dispatch table; feature-service interface definitions (trait boundaries between protocol layer and feature modules); R-6 decoupling pattern. Writes spec to `docs/sparc/02-pseudocode.md` §9. Stores blueprint in `phase-f` memory namespace. |
| `pf-lsp-protocol` | `backend-dev` | yes | `crates/rdf-lsp/**` (excluding `src/features/**`) | LSP server binary: initialisation handshake, `didOpen`, `didChange`, `publishDiagnostics`; connection lifecycle; dispatcher wiring. |
| `pf-lsp-features` | `backend-dev` | yes | `crates/rdf-lsp/src/features/**` | Feature service implementations: hover, completion, goto-definition, documentSymbol, formatting; all 11 languages dispatched correctly. |
| `pf-tester` | `tester` | no | `crates/rdf-lsp/tests/**` | LSP integration harness (stdin/stdout JSON-RPC); happy-path tests for each of the 6 LSP features across all 11 languages. |

Concurrency: 3 agents after architect pre-pass.

### 3. Exit gate

- LSP integration harness green across all 11 languages for all 6
  features listed in §Context above.
- `cargo test --workspace --all-features` green.
- `cargo clippy --workspace --all-features -- -D warnings` clean.

### 4. Integration pass (orchestrator, in-session)

Triggered by the last agent completion callback.

1. Merge worktrees: `pf-lsp-protocol` first (establishes binary
   entry-point), then `pf-lsp-features`. Resolve any claim overlap on
   `crates/rdf-lsp/src/`.
2. Run `cargo test --workspace --all-features --no-fail-fast`;
   `cargo clippy --workspace --all-features -- -D warnings`;
   `cargo deny check`.
3. Run the LSP integration harness explicitly: confirm all 11-language
   × 6-feature combinations pass.
4. Flip this ADR to Accepted. Update `docs/sparc/04-refinement.md`
   Phase F retro. Tag `phase-f/done`.

### 5. Hard parallelism rules (restated from ADR-0017 §6)

1. Architect pre-pass completes and spec is committed before backend-dev
   agents are spawned.
2. Backend-dev + tester spawns go out in **one** Agent-tool message.
3. `run_in_background: true` on every parallel spawn.
4. Integration only in the orchestrator session; agents do not merge
   their own work.
5. `claims_claim` before every edit; `pf-lsp-protocol` and
   `pf-lsp-features` claims must not overlap.

## Consequences

### Positive
- Protocol/features split prevents a single agent from holding the
  entire `rdf-lsp` crate hostage for 4–5 weeks.
- Decoupled feature services (R-6 mitigation) make a future framework
  swap cheap.
- Integration harness written in Phase F validates all 11 languages and
  6 features before Phase G polish begins; regressions surface early.

### Negative
- Claim boundary between `pf-lsp-protocol` and `pf-lsp-features`
  requires careful interface freezing by the architect; a poorly drawn
  boundary stalls both agents.
- A new LSP framework ADR must be authored and accepted before pre-flight
  can exit; if the evaluation is contentious, it delays the spawn.
- RDF/XML and JSON-LD formatters may be absent (Phase E stretch goal);
  formatting handler must gracefully degrade for those two languages.

### Neutral
- The LSP framework choice ADR is a sibling to this plan, not contained
  within it; its number will follow the next available slot.
- This ADR becomes historical on tag `phase-f/done`.

## Validation

- Architect deliverable (`docs/sparc/02-pseudocode.md` §9) committed
  before backend-dev agents are spawned.
- LSP framework ADR Accepted and ADR-0004 allow-list updated before
  pre-flight exits.
- One spawn message for backend-dev + tester.
- Integration harness green (11 languages × 6 features) before ADR
  flips to Accepted.
- `cargo test --workspace --all-features` green on integration.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy
  (topology table §4, quality gates §7).
- [`0024-phase-e-execution-plan.md`](0024-phase-e-execution-plan.md)
  — predecessor phase.
- [`0004-third-party-crate-policy.md`](0004-third-party-crate-policy.md)
  — allow-list (LSP framework crate must be added here).
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 and risk
  R-6 — Phase F budget, exit gate, and framework-churn mitigation.
- [`../sparc/02-pseudocode.md`](../sparc/02-pseudocode.md) §9 — architect
  target section (completion context algorithm).
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
