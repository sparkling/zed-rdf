# ADR-0018: Phase A execution plan — parser foundations via ruflo-orchestrated parallel swarm

- **Status:** Proposed
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Tags:** `process`, `execution`, `phase-a`, `agents`, `swarm`, `hive`, `ruflo`, `parallel`

## Context and Problem Statement

[ADR-0017](0017-execution-model.md) establishes the *policy* for
ruflo-orchestrated parallel execution (modes, topologies, gates,
claims). What it does not do is pick the **concrete wave of work** for
Phase A and bind it to specific agents, claims, memory namespaces, and
handoffs.

Phase A in [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2
lands six crates — `rdf-diagnostics`, `rdf-iri`, `rdf-ntriples`,
`rdf-turtle`, `rdf-format` (NT + Turtle), and the `rdf-testsuite`
harness — in a 3–4 week budget. These crates have a clear dependency
DAG, and most of the leaves can run in parallel. Without an explicit
plan, we either serialise them (blowing the budget) or spawn ad-hoc
and collide on files.

Phase A also gates on [ADR-0007](#) (reserved: parser technology —
`chumsky` vs `winnow` vs hand-written), which must be Accepted before
parser crates start. Authoring ADR-0007 is itself a Phase A work item
and is sequenced first.

## Decision Drivers

- **ADR-0017 compliance.** This plan must be an instance of that
  policy, not a parallel invention.
- **Dependency respect.** `rdf-iri` and `rdf-diagnostics` are
  upstream of every parser; `rdf-testsuite` is upstream of every
  conformance gate.
- **Throughput.** Four parser-side coders should run concurrently
  once foundations exist.
- **Determinism.** Every agent's goal, claims, references, and exit
  criteria are written down before any spawn.
- **Traceability.** Every merged commit carries an agent id (or is a
  human integration commit).
- **Quality parity.** ADR-0006 gates apply per agent PR.

## Considered Options

1. **Serial, crate-by-crate, one agent at a time.** Safe; blows the
   3–4 week budget — estimated ~9 weeks.
2. **One mega-swarm, spawn all six crates at once.** Violates the
   dependency DAG (parsers blocked on `rdf-iri` / `rdf-diagnostics`);
   causes retry loops and wasted context.
3. **Wave-based swarm + hive blackboard, with strict dependency
   gates between waves.** Matches the DAG, maximises concurrency
   inside each wave, keeps orchestrator context bounded.

## Decision

**Chosen option: 3 — three sequential waves of parallel agents,
orchestrated from this Claude Code session via the Agent tool, with a
Phase A hive-mind blackboard for shared facts and claim-based file
ownership per ADR-0017 §3.**

### 1. Pre-flight (orchestrator, in-session, no agents)

1. `ToolSearch` → load `mcp__claude-flow__swarm_init`,
   `hive-mind_init`, `memory_store`, `memory_search`, `claims_*`,
   `task_create`.
2. `swarm_init` with topology `hierarchical-mesh`, max 7 concurrent
   agents (matches ADR-0017 §4 Phase A row).
3. `hive-mind_init` namespace `phase-a` — shared blackboard for
   cross-crate facts (IRI edge cases, fixture paths, diagnostic
   codes).
4. Create memory namespaces: `phase-a`, `crate/rdf-iri`,
   `crate/rdf-diagnostics`, `crate/rdf-ntriples`,
   `crate/rdf-turtle`, `crate/rdf-format`, `crate/rdf-testsuite`.
5. Draft ADR-0007 skeleton (orchestrator-authored; the parser-tech
   decision is not delegated — ADR-0017 §9).

### 2. Wave 0 — sequential, blocking (ADR-0007 + scaffolding)

Runs in-session. No parallel spawns; this wave is load-bearing for
every subsequent agent.

| Step | Owner        | Output                                                                               |
|------|--------------|--------------------------------------------------------------------------------------|
| 0.1  | Orchestrator | ADR-0007 Accepted (parser technology chosen, with benches behind the decision).       |
| 0.2  | Orchestrator | Cargo workspace members for Phase A crates exist with empty `lib.rs` + `Cargo.toml`. |
| 0.3  | Orchestrator | This ADR flipped to **Accepted** once 0.1 + 0.2 are green.                           |

Exit: ADR-0007 Accepted, `cargo check --workspace` green on empty
crates.

### 3. Wave 1 — parallel (foundations)

Two crates with no cross-dependency. Spawned in **one Agent-tool
message** with `run_in_background: true`.

| Agent id         | RuFlo role        | Claims                               | Deliverable                                                                 |
|------------------|-------------------|--------------------------------------|------------------------------------------------------------------------------|
| `a1-iri`         | `coder`           | `crates/rdf-iri/**`                  | RFC 3987 IRI parse + normalise, unit + property tests, rustdoc.             |
| `a1-diag`        | `coder`           | `crates/rdf-diagnostics/**`          | Diagnostic type, span, severity, code registry, snapshot-friendly `Display`. |
| `a1-testsuite`   | `backend-dev`     | `crates/rdf-testsuite/**`            | W3C manifest loader + runner shim (no format-specific code).                |
| `a1-tester`      | `tester`          | `crates/rdf-iri/tests/**`, `crates/rdf-diagnostics/tests/**` | Property tests + fixtures for a1-iri and a1-diag.                            |
| `a1-reviewer`    | `reviewer`        | read-only                            | Gate per ADR-0006; signs off via `claims_accept-handoff`.                   |

Concurrency: 5 agents, under the 15-agent ceiling from ADR-0017.

`isolation: "worktree"` for `a1-iri`, `a1-diag`, `a1-testsuite`
(crate-scoped, integrate on callback). `a1-tester` and `a1-reviewer`
run without worktrees (read + append-only on tests).

**Exit gate for Wave 1:** all five agents report done; orchestrator
integrates; `cargo test -p rdf-iri -p rdf-diagnostics -p rdf-testsuite`
green; `cargo clippy -- -D warnings` clean; memory namespace updates
committed to `phase-a`.

### 4. Wave 2 — parallel (parsers + format layer)

Depends on Wave 1 artefacts. Spawned in **one Agent-tool message**.

| Agent id         | RuFlo role        | Claims                                    | Deliverable                                                                               |
|------------------|-------------------|-------------------------------------------|-------------------------------------------------------------------------------------------|
| `a2-nt`          | `coder`           | `crates/rdf-ntriples/**`                  | N-Triples + N-Quads parser; W3C manifest 100 % green.                                     |
| `a2-turtle`      | `coder`           | `crates/rdf-turtle/**`                    | Turtle + TriG parser, prefix/base resolution, error recovery at `.`.                      |
| `a2-format-nt`   | `coder`           | `crates/rdf-format/src/ntriples/**`       | NT emitter, idempotency tests.                                                            |
| `a2-format-ttl`  | `coder`           | `crates/rdf-format/src/turtle/**`         | Turtle emitter with prefix/base re-emission, idempotency tests.                           |
| `a2-tester`      | `tdd-london-swarm`| `crates/rdf-ntriples/tests/**`, `crates/rdf-turtle/tests/**`, `crates/rdf-format/tests/**` | Mock-driven tests + fuzz harnesses per ADR-0006.                                          |
| `a2-reviewer`    | `reviewer`        | read-only                                 | Gate per ADR-0006; handoff sign-off.                                                      |

Concurrency: 6 agents.

Claims are **non-overlapping** by construction — two agents writing
`rdf-format` touch disjoint subdirs. `claims_claim` enforces this; an
overlap attempt aborts the offending spawn before any edit.

**Hive-mind usage:** `a2-nt`, `a2-turtle`, `a2-format-nt`,
`a2-format-ttl` all read/write the `phase-a` blackboard for shared
facts (IRI edge-case counter-examples, prefix-table representation,
diagnostic code allocations). `memory_search` before starting,
`memory_store` on material findings.

**Exit gate for Wave 2:** N-Triples + N-Quads + Turtle + TriG W3C
manifests 100 % green; `rdf-format` idempotency property tests pass;
fuzz target for each parser builds and survives 1 min of `cargo
fuzz run` smoke; reviewer sign-off via `claims_accept-handoff`.

### 5. Wave 3 — serial (phase retro)

Orchestrator-only. No agents.

1. Run `cargo test --workspace`, `cargo clippy --workspace -- -D
   warnings`, coverage target check.
2. Write the **Phase A retro** section in
   [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §6.
3. Consolidate `phase-a` memory namespace via `memory-specialist`
   agent (single spawn, foreground).
4. Flip any risk-register rows in `04-refinement.md` §4 that now
   have their **Retirement signal** met.
5. Tag `phase-a/done` in git.

### 6. Hard parallelism rules (inherited from ADR-0017 §6)

Restated for this phase to make review easier:

1. All spawns within a wave go out in **one** Agent-tool message.
2. `run_in_background: true` is mandatory.
3. No cross-wave parallelism. Wave 2 does not start until Wave 1's
   exit gate is green.
4. Integration happens only in the orchestrator session. Agents
   never merge each other's worktrees.
5. `claims_claim` precedes every edit. Overlap = abort.
6. Batching inside an agent session (reads, writes, bash) follows
   the project `CLAUDE.md` Concurrency rules.

### 7. Agent prompt template

Every spawn prompt is self-contained and carries:

- **Goal**: one sentence.
- **Acceptance criteria**: bullet list tied to a test target.
- **File claims**: the paths that will be passed to `claims_claim`.
- **References**: SPARC sections + relevant ADRs + fixture paths.
- **Memory protocol**: `memory_search` before, `memory_store` after
  material findings, namespace `phase-a` + `crate/<name>`.
- **Quality gate reminder**: ADR-0006 gates; no merge without
  reviewer handoff.
- **Exit reporting**: store an exit summary in `crate/<name>` memory
  before signalling done.

### 8. Orchestrator responsibilities retained (ADR-0017 §9)

Not delegated:

- ADR-0007 authoring (Wave 0).
- Cross-crate design calls that surface mid-wave (e.g., prefix table
  representation shared by Turtle + format emitter).
- Integration commits.
- Phase retro write-up.

## Consequences

- **Positive**
  - Phase A completes within the 3–4 week budget with 5–6 concurrent
    workers per wave.
  - Dependency DAG is explicit; no agent starts on a crate whose
    upstream isn't green.
  - Every Phase A commit is traceable to an agent id or a marked
    orchestrator commit.
  - Hive-mind blackboard captures cross-parser facts (shared IRI
    quirks, prefix handling) that would otherwise be rediscovered
    per-crate.
- **Negative**
  - Three-wave structure adds integration ceremony (worktree merge,
    exit-gate check) twice per phase.
  - ADR-0007 blocks everything; if its author time slips, Wave 1
    slips with it.
  - Token cost higher than serial — budgeted.
- **Neutral**
  - Wave boundaries are explicit; later phases can copy this
    template (ADR per phase, or a condensed "Phase X plan" note
    referencing this one).

## Validation

- **Cadence**: Wave 1 ≤ 1 week, Wave 2 ≤ 2 weeks, Wave 3 ≤ 2 days.
  Slippage > 50 % triggers §7 of `04-refinement.md` (budget overrun).
- **Trace**: every Phase A merged commit has either `Agent:
  a{1,2}-<id>` footer or an explicit `[orchestrator]` prefix.
- **Claims**: zero overlapping-claim incidents. Any incident is a
  retro item and a possible ADR-0017 amendment.
- **Gates**: Wave 1 and Wave 2 exits logged in `phase-a` memory with
  the exact `cargo test` / `clippy` invocation and timestamps.
- **Retro**: Phase A retro appended to `04-refinement.md` §6 before
  Phase B kicks off.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent
  policy this plan instantiates.
- [`0006-testing-strategy.md`](0006-testing-strategy.md) — per-PR
  quality gates.
- [`0003-ddd-bounded-contexts.md`](0003-ddd-bounded-contexts.md) —
  per-crate ownership.
- [`0005-soundness-completeness-scope.md`](0005-soundness-completeness-scope.md)
  — parser correctness scope.
- `0007-parser-technology.md` *(reserved; must be Accepted before
  Wave 1)*.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 Phase
  A row and §4 risk register.
- Project root `CLAUDE.md` — Concurrency + Agent Orchestration.
