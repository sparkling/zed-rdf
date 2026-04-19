---
agent_id: v1-memory-ttl
cohort: cohort-a
hive: verification-v1
role: memory-specialist
worktree: true
priority: normal
claims:
  - .claude-flow/memory/**
  - scripts/memory-hygiene/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
---

# v1-memory-ttl — TTL + falsification hooks + cohort-tag guard

You are cohort-A agent `v1-memory-ttl`. Your guard is **load-bearing**
for every other cohort-A agent: until it ships, the no-cross-talk rule
in the cohort registry is discipline-only. Land it early in the sweep.

## Read first

1. `docs/adr/0019-independent-verification.md` §6.
2. `docs/adr/0017-execution-model.md` §8.
3. `docs/agent-cohorts.md` — the truth table your guard enforces.
4. `verification/memory-hygiene/ttl/defaults` in memory.

## Goal

- `scripts/memory-hygiene/` with:
  - `ttl-sweep.ts` (or `.sh`) that walks `phase-*` and
    `verification-*` namespaces, purges non-pinned entries older than
    7 days, and emits an audit log to
    `.claude-flow/audit/memory-hygiene/<date>.json`.
  - `falsification-hook.ts` invoked from ruflo hooks on test-failure
    events; quarantines memory entries tagged with the failing test
    id.
  - `cohort-guard.ts` — a wrapper around `memory_store` /
    `memory_search` that consults `docs/agent-cohorts.md` and rejects
    cross-cohort reads. Fails closed.
- A ruflo hook registration (in the project's hook config) wiring all
  three into the pre-task / post-task lifecycle.
- A markdown **runbook** under `scripts/memory-hygiene/README.md`:
  how to purge manually, how to inspect quarantine, how to audit
  cross-cohort violations.

## Acceptance

- Unit tests against the guard prove: cohort-B caller reading
  `verification-v1` is rejected; cohort-A caller reading
  `verification-v1-adv` is rejected; cohort-A reading
  `verification/spec-readings` is permitted.
- TTL sweep dry-run emits the expected purge list on a seeded
  fixture.
- Falsification hook, triggered by a synthetic failure, correctly
  quarantines.

## Claims

`.claude-flow/memory/**`, `scripts/memory-hygiene/**`, and the hook
config file (coordinate with `v1-ci-wiring` if overlapping).

## Memory

- Write the guard version to
  `verification/memory-hygiene/guard/version`.
- Do not read `verification-v1-adv` (even though you're building the
  guard that would allow it — practise the rule you are codifying).

## Exit handoff

`v1-reviewer`.
