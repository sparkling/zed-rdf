# ADR-0022: Phase C execution plan — SPARQL syntax via single-shot parallel swarm

- **Status:** Accepted
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy), [ADR-0019](0019-independent-verification.md) (independent
  verification)
- **Retires-on:** tag `phase-c/done`
- **Tags:** `process`, `execution`, `phase-c`, `agents`, `sparql`

## Context and Problem Statement

Phase B closed at tag `phase-b/done` with `rdf-xml`, `rdf-jsonld`,
`rdf-trix`, `rdf-n3` parsers, shadows, and adversary hive all passing
their respective exit gates.

Phase C per [`../sparc/04-refinement.md`](../sparc/04-refinement.md)
§2 lands **SPARQL syntax**. Exit gate: all `sparql11-test-suite` syntax
entries from `syntax-query/`, `syntax-update-1/`, `syntax-update-2/`
100% green (allow-list permitted with retirement plans).

Key pre-existing state:

- `crates/sparql-syntax/` — a 4562-line implementation (`grammar.rs`
  2033 lines, `lexer.rs` 901 lines, `ast.rs` 392 lines, `encode.rs`
  608 lines, `diag.rs` 131 lines, `lib.rs` 497 lines). Compiles clean.
- `crates/syntax/sparql-syntax-shadow/` — shadow already exists from
  `verification-v1`.
- `external/tests/sparql/` — W3C SPARQL test suite vendored:
  `manifest-sparql11-query.ttl`, `manifest-sparql11-update.ttl`,
  subdirs `syntax-query/`, `syntax-update-1/`, `syntax-update-2/`.

Phase C work is therefore narrowly scoped: wire the existing
implementation to the W3C manifests and achieve 100% pass. No new
parser, no new shadow, no new adversary hive.

## Decision Drivers

- **ADR-0017 compliance.** Phase-C kickoff per §10 checklist.
- **Minimal scope.** Parser is pre-existing and compiles; only test
  wiring and conformance fixes are needed.
- **Wall-clock over labour.** Three agents spawn concurrently; no
  sequential waves.
- **No shadow or adversary hive.** Both already exist from
  `verification-v1`; spawning them again would duplicate work without
  a fresh independence signal.

## Considered Options

1. **Single-shot parallel swarm (no shadow, no adversary hive).** Wire
   xtask + integration tests + reviewer concurrently. Fastest; shadow
   and adversary artefacts already present.
2. **Sequential: xtask wiring → tests → review.** Safest ordering, but
   serialises with no benefit given the parser already compiles.
3. **Full parallel swarm mirroring ADR-0021.** Adds shadow and
   adversary hive. Redundant — those exist; new runs would violate the
   independence signal (same corpus re-run).

## Decision

**Chosen option: 1 — single-shot parallel swarm, no shadow, no adversary
hive.**

### 1. Pre-flight (orchestrator, in-session, sequential)

1. **Confirm ADR-0007 Accepted.** Already done; hand-roll default
   applies to SPARQL syntax as it applies to every other format.
2. **Verify W3C suite vendored.** It is:
   `external/tests/sparql/manifest-sparql11-query.ttl` and
   `manifest-sparql11-update.ttl` present with subdirs.
3. **Extend xtask manifest runner.** Add `sparql-query` and
   `sparql-update` language cases to
   `xtask/verify/src/manifest.rs::parse_for_language`. SPARQL manifests
   use `mf:name`, `qt:query` (positive) / `ut:request` (update),
   `mf:result` (positive), and `mf:action` (negative syntax tests).
4. **Cohort registry update.** Append Phase C rows to
   `docs/agent-cohorts.md`.
5. **Memory namespace seeds.** `phase-c` (blackboard),
   `crate/sparql-syntax`,
   `verification/adversary-findings/sparql` (pre-existing from
   verification-v1; read-only reference).
6. **Per-agent prompt files.** Write to `scripts/spawn/phase-c/` before
   spawning.

Exit: xtask stubs present; cohort registry merged; prompt files on disk.

### 2. Single-shot parallel spawn — cohort A (`phase-c`)

**One Agent-tool message** containing all three workers below. All use
`run_in_background: true`. Editing agent (`pc-sparql-wiring`) uses
`isolation: "worktree"`; read-only agents do not.

| Agent id | Role | Worktree | Claims | Deliverable |
|----------|------|----------|--------|-------------|
| `pc-sparql-wiring` | `backend-dev` | yes | `crates/sparql-syntax/**`, `xtask/verify/**` | Extend xtask/verify sparql cases; create `crates/sparql-syntax/tests/w3c_syntax.rs`; fix any W3C test failures. |
| `pc-tester` | `tester` | no | `crates/sparql-syntax/tests/**` | Unit tests, snapshot tests, adversary fixture wiring (un-ignore `#[ignore]`-gated adversary-sparql tests). |
| `pc-reviewer` | `reviewer` | no | read-only | ADR-0017 §7 gate; audit at `.claude-flow/audit/phase-c-reviews/`. |

Concurrency: 3 agents.

No shadow cohort (exists from verification-v1).
No adversary hive (exists from verification-v1; `verification/adversary-findings/sparql` pre-populated).

### 3. Integration pass (orchestrator, in-session)

Triggered by the last cohort-A completion callback. Not polled.

1. Merge worktrees: `pc-sparql-wiring` worktree → main, then tester
   additions. Resolve any claim overlaps on workspace `Cargo.toml`.
2. Run `cargo test --workspace --all-features --no-fail-fast`;
   `cargo clippy --workspace --all-features -- -D warnings`;
   `cargo deny check`; `cargo run -p xtask -- verify sparql`.
3. **Phase-C exit gate check:**
   - `syntax-query/`: 100% of W3C positive/negative SPARQL 1.1 query
     syntax entries pass (allow-list permitted with cited retirement
     plans).
   - `syntax-update-1/`, `syntax-update-2/`: 100% of W3C SPARQL 1.1
     update syntax entries pass.
   - All previously-`#[ignore]`-gated adversary-sparql tests either
     pass or have documented retirement plans.
4. Flip this ADR to Accepted. Update
   `docs/sparc/04-refinement.md`'s Phase C retro. Tag `phase-c/done`.

### 4. Hard parallelism rules (restated from ADR-0017 §6)

1. All 3 spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. Dependent work resolves via frozen traits + stubs, not via spawn
   ordering.
4. Integration only in the orchestrator session.
5. `claims_claim` before every edit; schema `agent:<id>:<role>` per
   `docs/runbooks/claims-workflow.md`.
6. Cohort-tag enforcement via `scripts/memory-hygiene/cohort-guard.mjs`.

### 5. Agent prompt template

Every prompt carries:

- Cohort tag (`cohort-a`).
- Authoritative on-disk prompt path under `scripts/spawn/phase-c/`.
- Claim list.
- Forbidden read list (`phase-c-adv`,
  `verification/adversary-findings/sparql`).
- Frozen-API pointer (`crates/testing/rdf-diff/src/lib.rs`).
- Exit reporting into `crate/sparql-syntax` memory and `phase-c`
  blackboard.
- Handoff target (`pc-reviewer` for implementers).

## Consequences

- **Positive.**
  - Narrow scope: parser compiles, shadow exists, adversary corpus
    exists — only test wiring and conformance work remains.
  - 3-agent spawn is well inside the ADR-0017 ceiling (15).
  - Phase A infra (`xtask`, `rdf-diff`, oracles, `ALLOWLIST.md`) scales
    to SPARQL without bootstrap cost.
- **Negative.**
  - If the pre-existing implementation has spec gaps not caught by
    compilation, the tester or W3C runner will surface them and
    `pc-sparql-wiring` must fix them in the same sprint — no buffer
    agent.
  - No fresh adversary run; reliance on verification-v1 corpus.
- **Neutral.**
  - On tag `phase-c/done` this ADR becomes **historical** and is not
    consulted for future sweeps except as precedent.

## Validation

- **One spawn message** — orchestrator's tool-call log shows a single
  Agent-tool message with 3 entries.
- **W3C gate green** per §3 above before the ADR flips to Accepted.
- **Adversary fixtures un-ignored** — no `#[ignore]` remaining in
  adversary-sparql corpus without a documented retirement plan.
- **Wall-clock** target: ≤ 48 h from spawn to `phase-c/done` tag.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy.
- [`0019-independent-verification.md`](0019-independent-verification.md)
  — independence policy.
- [`0021-phase-b-execution-plan.md`](0021-phase-b-execution-plan.md)
  — prior single-shot parallel-swarm example.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 — Phase
  C budget + exit gate.
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
- Project root `CLAUDE.md` — Concurrency + Agent Orchestration
  (load-bearing).
