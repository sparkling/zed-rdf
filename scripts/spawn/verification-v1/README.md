# Spawn bundle — verification-v1 sweep (ADR-0020)

Every `*.md` in this directory is a **self-contained prompt** intended to
be loaded from disk into an `Agent` tool call at spawn time. The
orchestrator's spawn message should read each prompt and paste it into
the `prompt:` field — it must **not** summarise or paraphrase, because
the prompt files are the audit trail for what cohort-A and cohort-B
agents were told.

## Sweep contract

- **ADR:** [`docs/adr/0020-verification-implementation-plan.md`](../../../docs/adr/0020-verification-implementation-plan.md)
- **Parent ADR:** [`docs/adr/0019-independent-verification.md`](../../../docs/adr/0019-independent-verification.md)
- **Policy:** [`docs/adr/0017-execution-model.md`](../../../docs/adr/0017-execution-model.md)
- **Cohorts:** [`docs/agent-cohorts.md`](../../../docs/agent-cohorts.md)
- **Frozen trait:** [`crates/testing/rdf-diff/src/lib.rs`](../../../crates/testing/rdf-diff/src/lib.rs)
- **Ceiling:** 15 concurrent agents (ADR-0017 §4). Overflow (4 adversary
  testers) queued with `priority: low`.
- **Background:** every spawn MUST carry `run_in_background: true`.
- **Worktrees:** every editing agent MUST carry `isolation: "worktree"`
  unless this README explicitly says "no worktree".

## Spawn roster (single Agent-tool message)

### Cohort A (13 agents, hive `verification-v1`)

| Agent id           | subagent_type         | Worktree | Priority |
|--------------------|-----------------------|----------|----------|
| `v1-diff-core`     | `coder`               | yes      | normal   |
| `v1-oracle-rust`   | `coder`               | yes      | normal   |
| `v1-oracle-jvm`    | `cicd-engineer`       | yes      | normal   |
| `v1-cargo-deny`    | `coder`               | yes      | normal   |
| `v1-shadow-iri`    | `coder`               | yes      | normal   |
| `v1-shadow-nt`     | `coder`               | yes      | normal   |
| `v1-shadow-ttl`    | `coder`               | yes      | normal   |
| `v1-shadow-sparql` | `coder`               | yes      | normal   |
| `v1-specpins`      | `specification`       | yes      | normal   |
| `v1-memory-ttl`    | `memory-specialist`   | yes      | normal   |
| `v1-ci-wiring`     | `cicd-engineer`       | yes      | normal   |
| `v1-tester`        | `tester`              | no       | normal   |
| `v1-reviewer`      | `reviewer`            | no       | normal   |

### Cohort B (6 agents, hive `verification-v1-adv`)

| Agent id         | subagent_type     | Worktree | Priority |
|------------------|-------------------|----------|----------|
| `v1-adv-redteam` | `reviewer`        | no       | normal   |
| `v1-adv-nt`      | `tester`          | yes      | low      |
| `v1-adv-ttl`     | `tester`          | yes      | low      |
| `v1-adv-iri`     | `tester`          | yes      | low      |
| `v1-adv-sparql`  | `tester`          | yes      | low      |
| `v1-adv-veto`    | `code-analyzer`   | no       | normal   |

## Hard parallelism rules (restated from ADR-0020 §6)

1. All 19 spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. Dependent work resolves via the frozen trait surface, not via spawn
   ordering.
4. Integration happens only in the orchestrator session, triggered by
   the **last** cohort-A completion callback.
5. `claims_claim` precedes every edit.
6. Cohort-tag enforcement lives in the `v1-memory-ttl` wrapper; until
   that lands, prompt-level discipline is the only guard.

## What the orchestrator does NOT do after the spawn

- Does **not** poll `swarm_status` in a loop.
- Does **not** spawn additional workers until the integration pass.
- Does **not** read cohort-B memory as cohort-A, or vice versa.
