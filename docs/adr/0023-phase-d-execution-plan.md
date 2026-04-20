# ADR-0023: Phase D execution plan — ShEx + Datalog syntax via mesh swarm

- **Status:** Proposed
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy)
- **Retires-on:** tag `phase-d/done`
- **Tags:** `process`, `execution`, `phase-d`, `shex`, `datalog`

## Context and Problem Statement

Phase C closes at tag `phase-c/done` with `sparql-syntax` covering
SPARQL 1.1 full syntax + the 1.2 feature-flag additions. Phase D per
[`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 lands two
utility syntax crates:

- `shex-syntax`: ShEx 2.x compact syntax (ShExC) parser. Goal is
  highlighting and authoring aid, not validation. Hand-rolled per
  ADR-0007. The ShEx test suite (`shexSpec/shexTest`) provides syntax
  entries; if the suite cannot be vendored, a hand-written fixture
  corpus is used instead.
- `datalog-syntax`: Datalog syntax with recursive rules. No W3C suite
  exists; the corpus is hand-written fixtures only.
- SHACL recognition is handled via `rdf-vocab` vocabulary terms detected
  in a Turtle AST — it is **not** a new parser and does not land in
  Phase D.

Phase D has no intra-phase DAG: `shex-syntax` and `datalog-syntax` are
fully independent of each other and of any other active crate. The mesh
(peer) topology from ADR-0017 §4 is appropriate.

## Decision Drivers

- **ADR-0017 compliance.** Phase D topology: mesh. Agents: 2 coders +
  1 tester + 1 reviewer.
- **ADR-0007 compliance.** Both parsers are hand-rolled; no external
  parser-combinator dependency.
- **No W3C shadow requirement.** ADR-0019 §3 restricts shadows to
  parser formats with W3C conformance suites. ShEx and Datalog are
  utility parsers; shadow implementations are not required.
- **Sequential gate discipline.** This ADR activates only after
  `phase-c/done` is tagged.

## Considered Options

1. **Sequential: ShEx first, then Datalog.** Wastes concurrency for
   two crates with zero dependency between them.
2. **Mesh parallel swarm.** Both coders spawn simultaneously. Minimal
   coordination overhead; matches ADR-0017 §4. Chosen.
3. **Merge Phase D into Phase C.** Would violate the sequential gate
   policy and bloat the Phase C scope.

## Decision

**Chosen option: 2 — mesh parallel swarm.**

### 1. Pre-flight (orchestrator, in-session)

Sequential; must complete before any spawn.

1. Confirm `phase-c/done` tag exists in the repository.
2. Create crate stubs:
   - `crates/shex-syntax/Cargo.toml` + `crates/shex-syntax/src/lib.rs`
     implementing the `rdf_diff::Parser` trait as a stub.
   - `crates/datalog-syntax/Cargo.toml` +
     `crates/datalog-syntax/src/lib.rs` implementing `rdf_diff::Parser`
     as a stub.
   Add both to the workspace `Cargo.toml` `[members]` list.
3. Attempt to vendor the ShEx test suite via `git subtree add` at a
   pinned commit from `https://github.com/shexSpec/shexTest` into
   `external/tests/shex/`. If unavailable or not vendorable, fall back
   to a hand-written fixture corpus under
   `crates/shex-syntax/tests/fixtures/`.
4. Extend `xtask/verify/src/manifest.rs` to add `shex` and `datalog`
   language cases (mirror the pattern for `rdfxml` / `jsonld`).
5. Amend `docs/agent-cohorts.md` with Phase D agent rows.
6. Seed memory namespaces: `phase-d`, `crate/shex-syntax`,
   `crate/datalog-syntax`.
7. Write per-agent prompt files to `scripts/spawn/phase-d/`.

Exit: `cargo check --workspace --all-features` green; crate stubs
present; cohort registry updated.

### 2. Worker roster — cohort A (mesh topology)

Namespace: `phase-d`. Topology: mesh (peer). Base model:
`claude-opus-4-7`.

| Agent id | Role | Worktree | Claims | Deliverable |
|---|---|---|---|---|
| `pd-shex-syntax` | `coder` | yes | `crates/shex-syntax/**` | ShEx 2.x compact syntax parser; 100 % of ShEx test suite syntax-only entries green, or hand-written corpus green if suite is unavailable. |
| `pd-datalog-syntax` | `coder` | yes | `crates/datalog-syntax/**` | Datalog parser (recursive rules); snapshot fixture corpus green. |
| `pd-tester` | `tester` | no | `crates/shex-syntax/tests/**`, `crates/datalog-syntax/tests/**` | Integration tests and snapshot tests for both crates. |
| `pd-reviewer` | `reviewer` | no | read-only | ADR-0017 §7 gate; append-only audit at `.claude-flow/audit/phase-d-reviews/`. |

Concurrency: 4 agents.

### 3. Shadows

Not required. Per ADR-0019 §3, only parser formats with W3C conformance
suites receive independent shadow implementations. ShEx and Datalog are
utility parsers without W3C suites.

### 4. Integration pass (orchestrator, in-session)

Triggered by the last cohort-A completion callback. Not polled.

1. Merge worktrees in order: coder worktrees → tester fixtures.
   Resolve any workspace `Cargo.toml` overlap.
2. Run `cargo test --workspace --all-features --no-fail-fast`;
   `cargo clippy --workspace --all-features -- -D warnings`;
   `cargo deny check`; `xtask verify`.
3. Phase-D exit gate check:
   - `shex-syntax`: 100 % of ShEx test suite syntax-only entries pass,
     or hand-written corpus fully green if suite was not vendored.
   - `datalog-syntax`: hand-written snapshot fixture corpus green.
4. Flip this ADR to Accepted. Update
   `docs/sparc/04-refinement.md` Phase D retro. Tag `phase-d/done`.

### 5. Hard parallelism rules (restated from ADR-0017 §6)

1. All spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. Dependent work resolves via frozen traits + stubs, not via spawn
   ordering.
4. Integration only in the orchestrator session.
5. `claims_claim` before every edit.

## Consequences

### Positive
- Two parsers land in parallel; wall-clock dominated by the slower of
  the two rather than their sum.
- No shadow overhead: Phase D stays at 4 agents, well inside the
  15-agent ceiling.
- Hand-written Datalog corpus is self-contained and not subject to
  external suite availability.

### Negative
- If the ShEx test suite cannot be vendored, the conformance signal is
  limited to the hand-written corpus; wider coverage deferred.
- No adversary hive planned for Phase D (utility parsers, lower
  correctness stakes than RDF/XML or JSON-LD). This is a conscious
  scope reduction.

### Neutral
- SHACL recognition is deferred to Phase E (`rdf-vocab`) and does not
  affect Phase D scope.
- This ADR becomes historical on tag `phase-d/done`.

## Validation

- One spawn message — orchestrator tool-call log shows a single
  Agent-tool message with 4 entries.
- Exit gate green per §4.3 above before ADR flips to Accepted.
- `cargo test --workspace --all-features` green on integration.
- No orphaned worktrees after integration.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy
  (topology table §4, quality gates §7).
- [`0019-independent-verification.md`](0019-independent-verification.md)
  — shadow scope rule (§3).
- [`0021-phase-b-execution-plan.md`](0021-phase-b-execution-plan.md)
  — prior parallel-swarm worked example.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 — Phase D
  budget and exit gate.
- [`0007-parser-technology.md`](0007-parser-technology.md) — hand-roll
  mandate.
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
