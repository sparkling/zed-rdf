# ADR-0028: Phase I execution plan — publish + harden via hierarchical-mesh swarm

- **Status:** Proposed
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy)
- **Retires-on:** tag `phase-i/done` (= `v1.0`)
- **Tags:** `process`, `execution`, `phase-i`, `release`, `publish`

## Context and Problem Statement

Phase H closes at tag `phase-h/done` with the Zed extension working
end-to-end across all 11 languages. Phase I per
[`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 is the
final hardening and release phase:

- **Crate publish**: `rdf-diagnostics`, `rdf-iri`, `rdf-ntriples`,
  `rdf-turtle`, `rdf-format`, `rdf-xml`, `rdf-jsonld`, `rdf-trix`,
  `rdf-n3`, `sparql-syntax`, `shex-syntax`, `datalog-syntax`, `rdf-vocab`,
  `rdf-lsp` — published to crates.io in dependency order.
- **Extension publish**: `extensions/zed-rdf` published to the Zed
  extension registry.
- **Fuzz 24h clean**: `cargo-fuzz` targets per parser crate run to
  completion without finding new crashes.
- **Docs + examples**: `rustdoc` complete across all public APIs;
  runnable examples in `examples/`.
- **Tag `v1.0`**.

The three agents in Phase I have distinct, non-overlapping concerns:
release sequencing, performance/fuzz hardening, and security audit.
The hierarchical-mesh topology from ADR-0017 §4 is appropriate: the
release-manager acts as a soft hub (publishing sequence must be
serialised), while the performance engineer and security auditor work
in parallel.

## Decision Drivers

- **ADR-0017 compliance.** Phase I topology: hierarchical-mesh. Agents:
  1 release-manager + 1 performance-engineer + 1 security-auditor.
- **Sequential gate discipline.** This ADR activates only after
  `phase-h/done` is tagged.
- **Fuzz as a hard gate.** 24h fuzz clean is non-negotiable for v1.0;
  it is not advisory.
- **`cargo deny check` clean.** Advisories, bans, licenses, and sources
  must all pass before publish.
- **Security audit.** Input validation at all system boundaries; no
  `unsafe` outside justified, documented usages; `cargo audit` clean.

## Considered Options

1. **Sequential: release → fuzz → security.** Serialises independent
   work; delays v1.0 unnecessarily.
2. **Hierarchical-mesh: release-manager hub + perf + security in
   parallel.** Fuzz and security audit are independent of each other and
   of most release tasks. Chosen.
3. **Single agent does everything.** Too much context for one agent;
   fuzz + audit + publish are domain-distinct tasks.

## Decision

**Chosen option: 2 — hierarchical-mesh swarm.**

### 1. Pre-flight (orchestrator, in-session)

Sequential; must complete before any spawn.

1. Confirm `phase-h/done` tag exists in the repository.
2. Run `cargo publish --dry-run` for each crate in dependency order.
   Fix any packaging issues (missing `description`, `license`,
   `repository` fields; undeclared features; etc.) before spawning.
3. Verify `CHANGELOG.md` is present and complete. Confirm semver:
   all crates start at `0.1.0` per workspace `version.workspace`;
   no crate may publish with a `0.0.x` version.
4. Run `cargo deny check` (advisories + bans + licenses + sources).
   Resolve any outstanding issues. Phase I does not ship with a dirty
   deny report.
5. Confirm `cargo-fuzz` targets exist for each parser crate. If a
   target is missing, create a stub under `fuzz/fuzz_targets/` before
   spawning the performance engineer.
6. Amend `docs/agent-cohorts.md` with Phase I agent rows.
7. Seed memory namespaces: `phase-i`, `release/v1.0`.
8. Write per-agent prompt files to `scripts/spawn/phase-i/`.

Exit: `cargo publish --dry-run` clean for all 14 crates; `cargo deny
check` clean; fuzz targets present; cohort registry updated.

### 2. Worker roster — cohort A (hierarchical-mesh topology)

Namespace: `phase-i`. Topology: hierarchical-mesh. Base model:
`claude-opus-4-7`.

| Agent id | Role | Worktree | Claims | Deliverable |
|---|---|---|---|---|
| `pi-release` | `release-manager` | yes | `Cargo.toml`, `CHANGELOG.md`, `examples/**` | Crate publish sequence (dependency order); extension publish to Zed registry; `v1.0` tag; changelog complete; runnable examples in `examples/`. |
| `pi-perf` | `performance-engineer` | yes | `benches/**`, `.github/workflows/**` | Fuzz 24h clean for all parser crates; criterion baselines committed to `bench/baselines/`; CI bench job present; perf targets from §5 verified (N-Triples ≥ 200 MB/s, Turtle ≥ 80 MB/s, SPARQL ≥ 1000 queries/s, LSP cold-open ≤ 100 ms). |
| `pi-security` | `security-auditor` | no | read-only | Security audit: input validation at system boundaries; no undocumented `unsafe`; `cargo audit` clean; `npx @sparkleideas/cli@latest security scan` output reviewed. |

Concurrency: 3 agents.

### 3. Shadows

Not applicable. Phase I is a release/hardening phase with no new parser
crates. Per ADR-0019 §3, shadows are scoped to parser formats.

### 4. Integration pass (orchestrator, in-session)

Triggered by the last cohort-A completion callback. Not polled.

1. Review security audit report from `pi-security`. Resolve any
   blocking findings before proceeding.
2. Confirm fuzz report from `pi-perf`: 24h clean for all parser crates.
   If any crash is found, triage, fix, and re-fuzz (may require a
   short follow-on spawn; ADR amendment not required for a scope-bounded
   fix).
3. Confirm `pi-release` worktree: `cargo publish --dry-run` still clean;
   changelog finalized; examples compile and run.
4. Run final pre-release checks:
   - `cargo test --workspace --all-features --no-fail-fast`
   - `cargo clippy --workspace --all-features -- -D warnings`
   - `cargo deny check`
   - `xtask verify`
5. Publish crates to crates.io in dependency order (orchestrator
   executes; `pi-release` provides the ordering spec).
6. Publish extension to Zed registry.
7. Tag `v1.0` and `phase-i/done`.
8. Flip this ADR to Accepted. Update
   `docs/sparc/04-refinement.md` Phase I retro.

### 5. Hard parallelism rules (restated from ADR-0017 §6)

1. All spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. `pi-release` does not execute `cargo publish` — it produces the
   sequence spec and changelog. The orchestrator executes the actual
   publish in the integration pass.
4. Integration only in the orchestrator session.
5. `claims_claim` before every edit.

## Consequences

### Positive
- Fuzz and security audit run in parallel with release preparation;
  v1.0 wall-clock is dominated by the 24h fuzz window, not by the sum
  of all tasks.
- Security audit is independent of release mechanics; findings cannot
  be silently skipped in the rush to tag.
- All crates enter crates.io at `0.1.0` with a clean deny report and
  a fuzz-clean baseline — a strong foundation for post-v1.0 semver.

### Negative
- A fuzz crash found late in the 24h window restarts the clock for the
  affected crate. This is the primary schedule risk for Phase I.
- `cargo publish` is not reversible; a botched publish (wrong version,
  missing feature) requires a semver-bump re-publish. Pre-flight dry-run
  is the only mitigation.
- The Zed extension registry publish process is new; the orchestrator
  may encounter undocumented friction on first publish.

### Neutral
- This ADR becomes historical on tag `phase-i/done` (= `v1.0`).
- Post-v1.0 maintenance follows the normal PR workflow; no phase
  machinery is needed.

## Validation

- One spawn message — orchestrator tool-call log shows a single
  Agent-tool message with 3 entries.
- Exit gate green per §4 above before ADR flips to Accepted.
- `git tag v1.0` present on the release commit.
- All 14 crates visible on crates.io with version `0.1.0`.
- Extension visible in the Zed extension registry.
- Fuzz reports (one per parser crate) committed to
  `.claude-flow/audit/fuzz-v1.0/`.
- Security audit report committed to
  `.claude-flow/audit/security-v1.0/`.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy
  (topology table §4, quality gates §7).
- [`0019-independent-verification.md`](0019-independent-verification.md)
  — shadow scope rule (§3).
- [`0027-phase-h-execution-plan.md`](0027-phase-h-execution-plan.md)
  — predecessor phase.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 and §5 —
  Phase I scope and benchmarking discipline.
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
