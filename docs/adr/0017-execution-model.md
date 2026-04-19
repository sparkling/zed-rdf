# ADR-0017: Implementation execution — ruflo-orchestrated parallel agent swarms

- **Status:** Accepted (2026-04-18)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Tags:** `process`, `execution`, `agents`, `orchestration`, `ruflo`

## Context and Problem Statement

The plan in [`../sparc/04-refinement.md`](../sparc/04-refinement.md)
spans ~12 crates across 9 phases with many independently implementable
work items. Serial, single-session execution by one human engineer is
too slow for a realistic v1.0 schedule; unstructured parallelism is
unsafe (races, drift, silently overlapping file edits).

The repository was initialised with `ruflo init --full --force
--with-embeddings` on 2026-04-18. Daemon, memory (hybrid backend,
HNSW-indexed, Poincaré embeddings), and a 15-agent swarm
(`hierarchical-mesh`) are live. We need an explicit written policy for
**how implementation work is parcelled, spawned, coordinated,
integrated, and gated** — so ruflo is used consistently rather than
improvised per-phase.

## Decision Drivers

- **Throughput** on crate-independent work.
- **Determinism** in how agents are spawned and coordinated; repeatable
  across phases.
- **Traceability** — every committed line attributable to either the
  human orchestrator or a named agent run.
- **Context hygiene** — the orchestrating Claude Code session must not
  drown in agent output; use `run_in_background: true` and status
  checkpoints.
- **CLAUDE.md compliance** (project root) — Concurrency and Agent
  Orchestration sections are load-bearing.
- **Quality parity** — agent-produced code passes the same gates as
  human-written code (ADR-0006).

## Considered Options

1. **Serial Claude Code.** One task at a time, no agent spawning.
   Simple; too slow for the plan.
2. **Ad-hoc parallel Agents.** Spawn when it feels right, no rules.
   Fast when it works; high rate of wasted runs, merge conflicts,
   and drift.
3. **Ruflo-orchestrated swarms + hives + persistent memory with
   explicit per-phase topology and claim-based coordination.**
4. **Autonomous CI-driven agents** (auto-merging bots, scheduled cron
   agents). Premature for current scale.

## Decision

**Chosen option: 3 — ruflo-orchestrated parallel execution with
explicit topology per phase, claim-based file ownership, and
memory-backed cross-session state.**

### 1. Execution modes

| Mode       | When to use                                                | Mechanism                                                                  |
|------------|------------------------------------------------------------|----------------------------------------------------------------------------|
| **A Direct** | 1–2 files, trivial fix, doc tweak                         | Orchestrator works in-session. No agent.                                   |
| **B Swarm**  | 3+ independent files or independent crates                | `swarm_init` (if not already) + **one** Agent-tool message with all spawns |
| **C Hive**   | Multi-phase work with shared blackboard / memory           | `hive-mind_init`; spawn via Agent tool; shared memory namespace            |
| **D Phase**  | Whole SPARC phase kick-off                                 | `swarm_init` with chosen topology + one spawn message per phase            |

Mode choice is made **at the start of a task**, not during.

### 2. Agent-tool conventions (Claude Code)

- **Agent tool** is the **only** spawning primitive. CLI swarm commands
  manage infrastructure; the Agent tool creates workers.
- `run_in_background: true` is **mandatory** for parallel spawns.
- **All spawns for a phase or swarm go out in ONE message.** This is
  the hard rule from CLAUDE.md "Concurrency". Serialised spawns waste
  wall-clock time.
- After spawning: **do not poll.** Wait for completion callbacks. If
  visibility is required, call `mcp__claude-flow__swarm_status` **once**;
  never in a loop.
- Each spawn carries a **self-contained prompt** with: goal, acceptance
  criteria, file-claim list, references to SPARC sections + relevant
  ADRs, test fixture paths, reminder to store results in ruflo memory.
- Long crate-level work uses `isolation: "worktree"` so the agent works
  in an isolated git worktree. The orchestrator integrates on
  completion.

### 3. Coordination primitives

Loaded via `ToolSearch` before use.

| Tool                                              | Purpose                                                                 |
|---------------------------------------------------|-------------------------------------------------------------------------|
| `mcp__claude-flow__swarm_init` / `swarm_status`   | Swarm lifecycle + visibility.                                           |
| `mcp__claude-flow__agent_spawn` / `agent_list`    | Used rarely (Agent tool preferred); available for non-Claude workers.   |
| `mcp__claude-flow__hive-mind_init` / `…_broadcast` | Multi-agent shared blackboard.                                         |
| `mcp__claude-flow__memory_store` / `memory_search` | Cross-agent, cross-session facts; HNSW-indexed.                       |
| `mcp__claude-flow__claims_claim` / `claims_release` / `claims_accept-handoff` | Exclusive work ownership; hands off work at review. |
| `mcp__claude-flow__hooks_pre-task` / `hooks_post-task` | Lifecycle; auto-feeds outcomes to the learner.                    |
| `mcp__claude-flow__task_create` / `task_update`   | Long-lived ruflo-level task tracking (distinct from Claude's
TaskCreate). |

Daemon workers (`map`, `audit`, `optimize`, `consolidate`, `testgaps`,
`preload`) run in background maintenance mode; they **do not replace**
spawned agents.

### 4. Per-phase topology

Chosen per the nature of each phase. Numbers are concurrent agents.

| Phase | Topology           | Agents (rough plan)                                                                                                                       |
|-------|--------------------|-------------------------------------------------------------------------------------------------------------------------------------------|
| **A** | hierarchical-mesh  | 1 architect (orchestrator-side) + 4 coders (`rdf-diagnostics`, `rdf-iri`, `rdf-ntriples`, `rdf-turtle`) + 1 tester + 1 reviewer            |
| **B** | mesh (peer)        | 4 coders (`rdf-xml`, `rdf-jsonld`, `rdf-trix`, `rdf-n3`) + 1 tester + 1 reviewer                                                          |
| **C** | hierarchical       | 1 specification + 1 backend-dev (`sparql-syntax`) + 1 tester + 1 reviewer                                                                 |
| **D** | mesh               | 2 coders (`shex-syntax`, `datalog-syntax`) + 1 tester + 1 reviewer                                                                        |
| **E** | hierarchical       | 1 architect + 2 coders (`rdf-vocab`, `rdf-format`) + 1 tester                                                                              |
| **F** | hierarchical-mesh  | 1 architect + 2 backend-dev (LSP protocol + feature services) + 1 tester                                                                  |
| **G** | mesh               | 3 coders (rename, code actions, semantic tokens) + 1 tester                                                                               |
| **H** | mesh               | 1 coder (Wasm extension shim) + 1 coder (tree-sitter `.scm` assets per language) + 1 reviewer                                             |
| **I** | hierarchical-mesh  | 1 release-manager + 1 performance-engineer + 1 security-auditor                                                                           |

Concurrent workers per phase kept at 3–6 to balance throughput against
orchestrator context load. Hard ceiling: 15 concurrent agents (ruflo
default).

### 5. Agent-role catalogue

Prefer specialised RuFlo agents over the generic `coder`:

| Role               | RuFlo agent name                                   | Used for                                                      |
|--------------------|----------------------------------------------------|---------------------------------------------------------------|
| Specification      | `specification`                                    | SPARC-01 deltas per feature                                   |
| Pseudocode         | `pseudocode`                                       | SPARC-02 algorithm sketches                                   |
| Architecture       | `architecture` / `system-architect`                | SPARC-03 deltas per phase                                     |
| Implementation     | `coder`                                            | Default coder                                                 |
| LSP server code    | `backend-dev`                                      | `rdf-lsp` and protocol layer                                  |
| TDD                | `tester` / `tdd-london-swarm`                      | Test-first code generation                                    |
| Review             | `reviewer` / `code-review-swarm`                   | PR-level review before integration                            |
| Quality            | `code-analyzer`                                    | Post-hoc quality pass                                         |
| Security           | `security-auditor` / `pii-detector`                | Defence-in-depth (even for v1.0 without network surface)      |
| Performance        | `performance-engineer` / `perf-analyzer`           | Before each phase exit                                        |
| Release            | `release-manager`                                  | Phase I                                                       |
| Memory             | `memory-coordinator` / `memory-specialist`         | Cross-phase state, pattern distillation                       |
| Orchestrator-side  | `goal-planner` / `task-orchestrator` / `sparc-coord` | Used from the orchestrator session, not spawned               |

### 6. Hard parallelism rules

1. **One message per wave of spawns.** All independent spawns go out
   together. No serialised "Agent, then Agent, then Agent".
2. **`run_in_background: true` is mandatory** on every parallel spawn.
3. **Dependent work is sequential.** If agent B needs agent A's output,
   A finishes (callback received), its artefact is stored in ruflo
   memory, then B is spawned.
4. **Integration is always done by the orchestrator** (this Claude
   Code session). Agents do not merge their own work. Integration =
   worktree merge, full local CI, phase retro.
5. **File claims before edit.** Agents call `claims_claim` with the
   path list they intend to edit. Two agents cannot hold overlapping
   claims.
6. **Batching everywhere.** Independent file reads/writes/edits and
   independent Bash commands go in one message (orchestrator and
   agents both).

### 7. Quality gates per agent PR

An agent's work is not merged until **all** of the following hold, per
ADR-0006:

- Tests green, including W3C manifests for the crate.
- `cargo clippy -- -D warnings` clean.
- Coverage target met for the crate.
- Fuzz + property targets exist for parser crates.
- No decision taken outside an existing ADR without authoring a new
  one in the same PR.
- Reviewer agent signs off via `claims_accept-handoff`.
- Orchestrator does the final integration commit (footer: `Agent:
  <agent-id> / Reviewer: <reviewer-id>`).

### 8. Memory discipline

- Facts that outlive the phase go into ruflo memory (embedded +
  HNSW-indexed).
- Namespaces:
  - `phase-<letter>` — per-phase blackboard.
  - `crate/<name>` — per-crate facts (spec-clause mappings, gotchas,
    fixture paths).
  - `global/decisions` — accepted ADRs (pointer-only; ADR markdown is
    source of truth).
- Agents call `memory_search` **before** starting and `memory_store`
  after material findings.
- `hooks_session-end` emits a consolidated summary at session close.
- Pattern distillation (via `memory-specialist` / ruflo learner) runs
  nightly via the daemon's `consolidate` worker.
- **Memory-hygiene contract (amendment 2026-04-19, landed by
  `v1-memory-ttl` under ADR-0020).** Non-pinned entries in `phase-*` and
  `verification-*` namespaces expire after **7 days** (TTL sweep at
  `scripts/memory-hygiene/ttl-sweep.mjs`, audit log at
  `.claude-flow/audit/memory-hygiene/<date>.json`). A **falsification
  hook** quarantines entries tagged with a test id when that test fails
  (`scripts/memory-hygiene/falsification-hook.mjs`). A **cohort-tag
  guard** (`scripts/memory-hygiene/cohort-guard.mjs`) wraps
  `memory_store` / `memory_search`, consults `docs/agent-cohorts.md`, and
  fails-closed on cross-cohort reads. Runbook in
  `scripts/memory-hygiene/README.md`. See ADR-0019 §6 for the
  original requirement.

### 9. What the orchestrator still does by hand

- Architecture decisions that reshape the bounded-context map.
- Cross-phase trade-offs.
- ADR authoring at the significant-decision level.
- Final integration and release cutting.
- Conflict resolution when claims or agents disagree.

Everything else is delegated.

### 10. Kick-off checklist per phase

Before the first spawn of a phase:

1. Fill the relevant section of
   [`02-pseudocode.md`](../sparc/02-pseudocode.md).
2. Ensure ADRs the phase depends on are Accepted.
3. `mcp__claude-flow__swarm_init` with the phase's topology (if the
   existing swarm's topology is wrong).
4. Create a ruflo memory namespace for the phase.
5. Draft per-crate prompts (goal / acceptance / claims / references).
6. Spawn all workers in one message with `run_in_background: true`.
7. Wait. Integrate on callbacks. Run phase retro into memory + this
   ADR register.

## Consequences

- **Positive**
  - 3–6× wall-clock speed-up on crate-independent work (estimated);
    matches the phase budgets in `04-refinement.md`.
  - Every commit is traceable to a specific agent run or to a human
    orchestrator commit.
  - Persistent memory turns per-session findings into durable learning.
  - Explicit topology per phase prevents "just spawn agents and hope".
- **Negative**
  - Token cost higher than serial execution; budgeted as part of
    tooling cost.
  - Risk of subtle conflicts if claims are misused; mitigated by
    orchestrator integration + `cargo-deny`-style CI + reviewer agent.
  - Daemon + memory + swarm add operational overhead (log watching,
    restart after crashes); documented in a RUNBOOK (to be authored in
    phase A).
- **Neutral**
  - Serial execution remains possible for tiny changes (mode A); the
    ADR does not force parallelism for trivia.

## Validation

- **Cadence:** phase A completes within the 3–4 week budget in
  `04-refinement.md` §2 using 4 parallel coder agents + 1 tester + 1
  reviewer.
- **Trace:** every merged commit either has an `Agent: <id>` footer
  referencing a completed agent run, or is a manual orchestrator
  commit (explicitly marked in the message).
- **Claims:** zero merge conflicts caused by two agents editing
  overlapping files without a handoff. Any violation triggers an ADR
  amendment to tighten rules.
- **Memory:** every phase exit writes a consolidated summary to the
  `phase-<letter>` namespace; referenced from the phase retro in
  `04-refinement.md` under a `## Phase <X> retro` heading.

## Links

- Project root `CLAUDE.md` — Concurrency + Agent Orchestration sections
  (load-bearing).
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §6 engineering
  workflow.
- [`0006-testing-strategy.md`](0006-testing-strategy.md) — agent PRs
  must pass the same quality gates as human PRs.
- [`0003-ddd-bounded-contexts.md`](0003-ddd-bounded-contexts.md) — per-crate
  ownership maps naturally onto per-crate agent claims.
- RuFlo CLI docs (`ruflo daemon`, `ruflo memory`, `ruflo swarm`).
