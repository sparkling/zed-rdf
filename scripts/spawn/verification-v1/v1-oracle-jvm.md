---
agent_id: v1-oracle-jvm
cohort: cohort-a
hive: verification-v1
role: cicd-engineer
worktree: true
priority: normal
claims:
  - .github/workflows/fact-oracles.yml
  - external/fact-oracles/**
forbidden_reads:
  - verification-v1-adv
  - verification/adversary-findings
---

# v1-oracle-jvm — materialise Jena + rdf4j fact corpora out-of-process

You are cohort-A agent `v1-oracle-jvm`. The JVM reference parsers do not
enter the Rust path. Instead, a CI job runs them over the W3C test
suites and emits a pinned JSON fact corpus that the diff harness loads.

## Read first

1. `docs/adr/0019-independent-verification.md` §1 (Jena / rdf4j
   paragraph).
2. `docs/adr/0020-verification-implementation-plan.md` §3 — your
   deliverable boundary.

## Goal

- `.github/workflows/fact-oracles.yml`:
  - Trigger: manual `workflow_dispatch` + weekly cron + on PR touching
    `external/tests/**` or `external/fact-oracles/**`.
  - Set up JDK 21 + Maven/Gradle toolchain.
  - Check out W3C suites at a pinned commit (see `external/tests/`
    vendoring done elsewhere — if missing, emit a clear error pointing
    to the phase-A prerequisite).
  - Run Jena + rdf4j over each suite, capture accept/reject + fact set,
    emit `external/fact-oracles/<lang>/<suite>-<commit>.json`.
  - Commit the JSON via PR (not push-to-main).
- `external/fact-oracles/README.md` — schema of the JSON files,
  consumable from the Rust side without a JSON-schema library.

## Acceptance

- CI job runs to completion in GitHub Actions on a smoke fixture.
- Output JSON conforms to the schema documented in the README.
- No Rust code touched.

## Claims

`.github/workflows/fact-oracles.yml`, `external/fact-oracles/**`,
`external/fact-oracles/README.md`.

## Memory

- `memory_store` at `schema/v1` in `verification/oracle-jvm` (create
  namespace if needed) with the JSON schema hash + commit pin.

## Exit handoff

`v1-reviewer`.
