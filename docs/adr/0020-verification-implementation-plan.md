# ADR-0020: Implementation plan for ADR-0019 — single-shot parallel swarm

- **Status:** Proposed
- **Date:** 2026-04-19
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** ADR-0017 (execution model), ADR-0019 (independent
  verification)
- **Tags:** `process`, `execution`, `agents`, `swarm`, `hive`, `ruflo`,
  `verification`, `parallel`

## Context and Problem Statement

[ADR-0019](0019-independent-verification.md) decides *what* independent
verification infrastructure must exist. [ADR-0017](0017-execution-model.md)
decides *how* work is parcelled across ruflo-orchestrated agents.
Neither picks the concrete swarm shape, agent roster, claims, or
wave structure for landing 0019.

Under the AI-team premise (see the Round-2 adversarial review,
2026-04-19), human labour is not the bottleneck. Phase-A's three-wave
structure (ADR-0018) was appropriate there because Phase A has a real
dependency DAG — foundations before parsers. ADR-0019's work items do
**not** share that DAG: most of them are independent infrastructure
that can be built against stub interfaces and integrated later.

This ADR decides: **everything in 0019 launches in a single orchestrated
sweep**, with claim-partitioned parallel agents, a `verification-v1`
hive-mind blackboard, and an adversarial-review hive that runs
*concurrently* with the implementing hive rather than gating it.

## Decision Drivers

- **Labour is cheap, wall-clock is scarce.** Minimise sequential
  dependencies; maximise concurrent work.
- **No artificial waves.** Waves exist in ADR-0018 because of a real
  dependency DAG. ADR-0019's work does not have that DAG and must not
  simulate one.
- **Claim-partitioned safety.** Concurrency without collisions requires
  disjoint claims, enforced by `claims_claim` before edit.
- **Cohort separation (ADR-0019 §3, §4).** Shadow implementations and
  adversary hive must draw from **disjoint prompt lineages and,
  ideally, different base models**. Cohort discipline is part of the
  plan, not an afterthought.
- **Stub-first interfaces.** Agents writing shadow parsers, the diff
  harness, and the adversary red-team must not block on each other;
  they integrate against a frozen `rdf-diff` trait surface authored
  in pre-flight.
- **ADR-0017 compliance.** This is an instance of that policy, not a
  parallel invention.

## Considered Options

1. **Three sequential waves mirroring ADR-0018.** Scaffolding →
   infrastructure → shadows+adversary. Safe, familiar, but
   unnecessarily serialises work with no real dependency.
2. **One mega-spawn with claim-partitioned agents against stub
   interfaces + cohort-separated implementing and adversary hives
   running concurrently.**
3. **Per-format mini-waves** — one wave per parser. Fragments the
   orchestrator's attention and hides systemic issues (e.g., trait
   surface mistakes) until repeated once per wave.

## Decision

**Chosen option: 2 — single-shot parallel swarm.** One pre-flight
block (orchestrator, in-session) to freeze interfaces and spawn
topology, then **one Agent-tool message** spawning the full worker
set with `run_in_background: true`, with an adversarial hive
running concurrently on the same work.

### 1. Pre-flight (orchestrator, in-session, ≤ 30 min)

Sequential because these artefacts are what every agent reads.

1. **Load tools** via `ToolSearch`:
   `mcp__claude-flow__swarm_init`, `hive-mind_init`, `memory_store`,
   `memory_search`, `claims_claim`, `claims_release`,
   `claims_accept-handoff`, `task_create`.
2. **Swarm init** with topology `hierarchical-mesh`, max 15 concurrent
   agents (the ADR-0017 ceiling).
3. **Hive-mind init** two namespaces:
   - `verification-v1` — implementing-hive blackboard.
   - `verification-v1-adv` — adversary-hive blackboard (**cohort-tag
     enforced**; implementing agents cannot read from it; adversary
     agents cannot read from implementing namespaces per ADR-0019 §6).
4. **Freeze `rdf-diff` trait surface.** Orchestrator authors the
   public API of `crates/testing/rdf-diff/src/lib.rs`:
   `trait Parser { fn parse(&self, input: &[u8]) -> Result<Facts,
   Diagnostics>; }`, `struct Facts`, `fn diff(a: &Facts, b: &Facts)
   -> DiffReport`. Stub bodies `todo!()`. Committed before any spawn.
   This is the integration contract shadow, oracle, and main parsers
   all implement.
5. **Freeze cohort registry.** `docs/agent-cohorts.md` pins each
   agent's prompt lineage and base model. Implementing and adversary
   cohorts are disjoint here; subsequent spawns read this file.
6. **Create memory namespaces** (beyond the two hive-minds):
   `crate/rdf-diff`, `crate/*-shadow`, `verification/spec-readings`,
   `verification/cargo-deny`, `verification/memory-hygiene`,
   `verification/adversary-findings`.
7. **Draft per-agent prompts** (§4 below) and write them to
   `scripts/spawn/verification-v1/` as files — agents read their
   prompt from disk so the orchestrator's Agent-tool message stays
   small.

Exit: trait surface compiles (`cargo check -p rdf-diff` green on
stubs); namespaces exist; cohort registry merged.

### 2. Single-shot parallel spawn

**One Agent-tool message** containing every worker below. All use
`run_in_background: true`. All use `isolation: "worktree"` unless
otherwise noted (read-only / append-only agents need no worktree).

No further orchestrator spawns until the final integration pass (§5).

### 3. Worker roster — implementing hive (cohort A)

Namespace: `verification-v1`. Prompt lineage: `cohort-a` (recorded in
cohort registry).

| Agent id           | RuFlo role        | Worktree | Claims                                                                 | Deliverable                                                                                 |
|--------------------|-------------------|----------|-----------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| `v1-diff-core`     | `coder`           | yes      | `crates/testing/rdf-diff/**`                                          | Fills stubs: `Facts` canonicalisation (BNode-canonical, prefix-free), `diff`, `DiffReport`. |
| `v1-oracle-rust`   | `coder`           | yes      | `crates/testing/rdf-diff-oracles/**` (new)                            | `[dev-dependencies]` adapters for `oxttl`, `oxrdfxml`, `oxjsonld`, `oxsparql-syntax`.       |
| `v1-oracle-jvm`    | `cicd-engineer`   | yes      | `.github/workflows/fact-oracles.yml`, `external/fact-oracles/**`      | CI job: materialises Jena + rdf4j fact corpora from W3C suites; commits pinned JSON.        |
| `v1-cargo-deny`    | `coder`           | yes      | `deny.toml`, `crates/testing/deny-regression/**`                      | `cargo-deny` carve-out; regression test asserting `ox*` / `sophia_*` never leak to runtime. |
| `v1-shadow-iri`    | `coder`           | yes      | `crates/syntax/rdf-iri-shadow/**`                                     | Second RFC 3987 impl, disjoint prompt lineage, `shadow` feature only.                       |
| `v1-shadow-nt`     | `coder`           | yes      | `crates/syntax/rdf-ntriples-shadow/**`                                | Second N-Triples/N-Quads impl.                                                              |
| `v1-shadow-ttl`    | `coder`           | yes      | `crates/syntax/rdf-turtle-shadow/**`                                  | Second Turtle/TriG impl.                                                                    |
| `v1-shadow-sparql` | `coder`           | yes      | `crates/syntax/sparql-syntax-shadow/**`                               | Second SPARQL 1.1 syntax impl (grammar-only, no execution).                                 |
| `v1-specpins`      | `specification`   | yes      | `docs/spec-readings/**`                                               | Pin records for ambiguous productions (literal escapes, IRI pct-enc, BOM, BNode @prefix scope). |
| `v1-memory-ttl`    | `memory-specialist` | yes    | `.claude-flow/memory/**`, `scripts/memory-hygiene/**`                 | TTL + falsification hooks + cohort tagging for `phase-*` and `verification-*` namespaces.   |
| `v1-ci-wiring`     | `cicd-engineer`   | yes      | `.github/workflows/verification.yml`, `xtask/verify/**`               | Wires diff harness + shadow-vs-main + cargo-deny + oracle job into PR gate.                 |
| `v1-tester`        | `tester`          | no       | `crates/testing/rdf-diff/tests/**`, `crates/syntax/*-shadow/tests/**` | Property + snapshot tests for diff harness and each shadow.                                 |
| `v1-reviewer`      | `reviewer`        | no       | read-only on all above                                                | Engineering review per ADR-0006; handoff sign-off via `claims_accept-handoff`.              |

Concurrency: 13 agents. Under the 15-agent ceiling.

Cohort-A prompt seed: emphasises spec *text*, manifests, and the frozen
`rdf-diff` trait. Does not reference `oxttl`'s API beyond adapter
signatures.

### 4. Worker roster — adversary hive (cohort B)

Namespace: `verification-v1-adv`. Prompt lineage: `cohort-b`. **Must
not read from `verification-v1`**; TTL + cohort-tag guards enforce.

Runs **concurrently** with cohort A — adversary does not wait for
implementation to finish. Early findings flow into
`verification/adversary-findings` and become targets for cohort A in
the same run.

| Agent id           | RuFlo role                 | Claims                                                  | Deliverable                                                                                 |
|--------------------|----------------------------|---------------------------------------------------------|---------------------------------------------------------------------------------------------|
| `v1-adv-redteam`   | `reviewer` (cohort-b seed) | read-only on `crates/syntax/**`                         | Red-team input generator per format; writes findings to `verification/adversary-findings`.  |
| `v1-adv-nt`        | `tester` (cohort-b seed)   | `crates/testing/rdf-diff/tests/adversary-nt/**`         | Inputs that expose divergent readings between our NT parser and oracles.                    |
| `v1-adv-ttl`       | `tester` (cohort-b seed)   | `crates/testing/rdf-diff/tests/adversary-ttl/**`        | Same, Turtle/TriG.                                                                          |
| `v1-adv-iri`       | `tester` (cohort-b seed)   | `crates/testing/rdf-diff/tests/adversary-iri/**`        | Same, IRI normalisation + resolution.                                                       |
| `v1-adv-sparql`    | `tester` (cohort-b seed)   | `crates/testing/rdf-diff/tests/adversary-sparql/**`     | Same, SPARQL syntax.                                                                        |
| `v1-adv-veto`      | `code-analyzer`            | read-only                                               | Holds the veto flag per ADR-0019 §4; merges block until cleared.                            |

Concurrency: 6 agents. Combined with cohort A, **19 concurrent
agents** — above ADR-0017's 15-agent ceiling. Mitigation: the 4
per-format adversary testers are spawned with a `priority: low` flag
so the scheduler queues them if the ceiling binds; all 6 adversary
agents still complete within the same sweep.

### 5. Integration pass (orchestrator, in-session)

Triggered by the **last** cohort-A agent completion callback — not
polled. Not a separate "wave"; it is the closing act of the single
sweep.

1. Merge all worktrees in dependency-safe order: trait + oracles →
   shadows → tests → CI → cargo-deny → memory hygiene → spec pins.
2. Run `cargo test --workspace`, `cargo clippy --workspace -- -D
   warnings`, `cargo-deny check`, `cargo llvm-cov` targets.
3. Run the **diff harness** against the current Phase A parsers (NT,
   Turtle) + shadows. **Expect non-zero divergences on first run**
   (ADR-0019 §Validation treats "zero" as suspicious).
4. Triage adversary findings: each finding is either (a) a confirmed
   divergence → ticket + cohort-A follow-up, or (b) a spurious
   cohort-B reading → pin in `docs/spec-readings/` and closed.
5. Flip ADR-0019 from Proposed to Accepted. Flip this ADR (0020) to
   Accepted.
6. Update ADR-0004 with a pointer to the `[dev-dependencies]`
   carve-out defined in 0019 §1.
7. Amend ADR-0017 §8 with the memory-hygiene contract landed by
   `v1-memory-ttl`.
8. Write the sweep retro to
   [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §6 under
   a `## Verification sweep retro` heading.
9. `git tag verification-v1/done`.

### 6. Hard parallelism rules (from ADR-0017 §6, restated)

1. All 19 spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. Dependent work is resolved via the frozen `rdf-diff` trait surface,
   not by sequencing spawns.
4. Integration only in the orchestrator session.
5. `claims_claim` precedes every edit. Adversary-hive tester crates
   live under `crates/testing/rdf-diff/tests/adversary-*/**`;
   implementing-hive tests live under
   `crates/testing/rdf-diff/tests/` (no `adversary-` prefix) and per-crate
   `tests/`. No overlap by construction.
6. Cohort-tag enforcement in `memory_search` / `memory_store` wrappers
   — the first deliverable of `v1-memory-ttl` is the guard that all
   other agents rely on; it lands early in the sweep but does not
   block spawn.

### 7. Agent prompt template (delta on ADR-0018 §7)

Every prompt additionally carries:

- **Cohort tag** — `cohort-a` or `cohort-b`. Determines which memory
  namespaces are readable.
- **Trait contract reference** — frozen API in
  `crates/testing/rdf-diff/src/lib.rs`.
- **No-cross-talk rule** — cohort B may not read `verification-v1`
  (and vice versa). Enforced at memory-wrapper level after
  `v1-memory-ttl` lands; prompt-level discipline before that.
- **Exit reporting into `crate/<name>` memory** per ADR-0018 §7.

### 8. Orchestrator responsibilities retained

- Freezing the trait surface.
- Pinning cohort registry.
- Final integration and ADR flips.
- Adversary-finding triage when cohort A and cohort B disagree on
  the *correct* reading.
- Release sign-off (ADR-0019 §Validation: human attribution of the
  cut tag).

## Consequences

- **Positive**
  - One coherent sweep, no artificial waves; wall-clock dominated by
    the slowest single agent, not by N sequential waves.
  - Adversary hive produces findings **concurrently** with
    implementation — bugs surface before integration, not after.
  - Cohort separation baked into pre-flight, not retrofitted.
  - Every deliverable in ADR-0019 maps to exactly one claim-holder;
    no orphan work.
- **Negative**
  - 19 concurrent agents exceeds the 15-agent ceiling; relies on
    priority-queueing for the 4 adversary-per-format agents. Real
    overflow falls back to a short second spawn, not a wave.
  - Frozen trait surface is a commitment made before implementation
    experience. If it is wrong, cohort A's worktrees have to be
    re-based. Mitigated by keeping the surface small (four items).
  - Token cost of concurrent adversary hive ≈ 1.5× implementing
    hive. Budgeted as verification cost.
- **Neutral**
  - This ADR is itself a candidate for the rot-risk flagged by the
    Round-2 review (ADR rot). It carries its own retirement trigger:
    on tag `verification-v1/done` it becomes **historical** and is
    not consulted for future sweeps except as precedent.

## Validation

- **One spawn message** — the orchestrator's tool-call log shows a
  single Agent-tool message with 19 entries (or ≤ 15 + an overflow
  pair, see above).
- **Cohort separation holds** — no cohort-B agent called
  `memory_search` against a `verification-v1` namespace; no
  cohort-A agent called it against `verification-v1-adv`. Logged
  via the `memory-specialist`'s audit.
- **Diff harness produces non-zero divergences on first Phase-A
  run** — per ADR-0019 §Validation. Zero is suspicious.
- **Adversary veto fires at least once** in this sweep. Zero is
  suspicious.
- **All 19 agents complete** without orchestrator polling; the
  integration pass starts on the final completion callback.
- **Wall-clock** ≤ 48 h from spawn to `verification-v1/done` tag.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent
  policy.
- [`0018-phase-a-execution-plan.md`](0018-phase-a-execution-plan.md) —
  prior worked example (wave-based, because of its real DAG).
- [`0019-independent-verification.md`](0019-independent-verification.md)
  — *what* is being implemented here.
- [`0004-third-party-crate-policy.md`](0004-third-party-crate-policy.md)
  — amended by 0019 §1, applied here.
- [`0006-testing-strategy.md`](0006-testing-strategy.md) — diff
  harness fills a new layer in the pyramid.
- Round-2 adversarial ADR review (2026-04-19, hive transcript).
- Project root `CLAUDE.md` — Concurrency + Agent Orchestration
  (load-bearing).
