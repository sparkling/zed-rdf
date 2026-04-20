# ADR-0021: Phase B execution plan — remaining RDF syntax via single-shot parallel swarm

- **Status:** Proposed
- **Date:** 2026-04-19
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy), [ADR-0019](0019-independent-verification.md) (independent
  verification)
- **Retires-on:** tag `phase-b/done`
- **Tags:** `process`, `execution`, `phase-b`, `agents`, `swarm`,
  `hive`, `ruflo`, `parallel`

## Context and Problem Statement

[Phase A](0018-phase-a-execution-plan.md) closed at tag `phase-a/done`
with `rdf-diagnostics`, `rdf-iri`, `rdf-ntriples`, `rdf-turtle`,
`rdf-format`, the `xtask verify` W3C manifest runner, and the
verification-v1 infra (diff harness, oracles, shadows, adversary
hive). Every NT / N-Quads / Turtle / TriG W3C manifest entry passes
strict (0 allow-list).

Phase B per [`../sparc/04-refinement.md`](../sparc/04-refinement.md)
§2 lands the **remaining RDF syntax**: `rdf-xml`, `rdf-jsonld` (syntax
+ `@context` well-formedness, not expand/compact), `rdf-trix`,
`rdf-n3`. 4–6 week budget. Exit gate: RDF/XML + JSON-LD syntax suites
100 % green; snapshot coverage for TriX + N3 (W3C has no conformance
suite for those two).

Unlike Phase A, Phase B has **almost no internal dependency DAG**
once the Phase A foundations are in place:

- `rdf-xml` uses `quick-xml` as an infrastructure dep (already
  allow-listed by ADR-0004) plus `rdf-iri` + `rdf-diagnostics`. No
  downstream Phase-B dep.
- `rdf-jsonld` uses `serde_json` + `rdf-iri` + `rdf-diagnostics`. No
  downstream Phase-B dep.
- `rdf-trix` is an XML wrapper around N-Triples content; it may call
  `rdf-xml`'s public parser OR ship its own tiny streaming XML
  tokeniser. Either way it is a small crate.
- `rdf-n3` is a superset of Turtle; it consumes the `rdf-turtle`
  grammar and adds N3-specific productions (`@keywords`, reification,
  quoted formulas). It path-depends on `rdf-turtle` but only at the
  grammar level — it doesn't extend the Parser trait shape.

The independence / shadow policy from ADR-0019 §3 already records
that **TriX and N3 get layers 1 + 2 only** (oracle + diff harness;
no shadow). Only `rdf-xml` and `rdf-jsonld` ship with shadows.

## Decision Drivers

- **ADR-0017 compliance.** Phase-B kickoff per §10 checklist.
- **Cohort separation (ADR-0019 §3).** Shadow implementations for
  `rdf-xml` + `rdf-jsonld` must draw from a disjoint cohort.
- **Wall-clock over labour.** Under the AI-team premise, four main
  parsers + two shadows + adversary hive all spawn concurrently.
- **No dependency waves.** Phase B's four mains are independent;
  wave structure (ADR-0018's shape) would serialise without benefit.
- **Reuse the landed infra.** `xtask verify`, `rdf-diff-oracles`,
  the adversary hive pattern, and `docs/spec-readings/` all extend
  to the new formats without bootstrap.

## Considered Options

1. **Wave-based plan mirroring ADR-0018.** Foundations → parsers →
   tests. Phase A needed this because of its DAG; Phase B does not,
   so waves here are theatre.
2. **Single-shot parallel swarm with pre-flight freeze.** One
   Agent-tool message spawning all main + shadow + adversary agents
   against frozen traits. Mirrors the successful verification-v1
   shape (ADR-0020).
3. **Per-format mini-sweeps.** Land `rdf-xml` first, then
   `rdf-jsonld`, then TriX, then N3. Sequential; wastes concurrency;
   hides systemic issues (e.g. oracle adapter shape) until the
   fourth crate.

## Decision

**Chosen option: 2 — single-shot parallel swarm.** Pre-flight freeze
of trait surface + oracle adapter contract, then one spawn with every
worker, with an adversary hive running concurrently on cohort-B.

### 1. Pre-flight (orchestrator, in-session, ≤ 45 min)

Sequential because these artefacts are what every agent reads.

1. **Confirm ADR-0007 (parser technology).** If not yet Accepted,
   author or flip it — Phase B coders need to know whether to
   hand-roll or use `chumsky` / `winnow`.
2. **Verify vendored W3C suites.** `external/tests/rdfxml/` is
   already populated from the verification-v1 sweep. `external/tests/`
   needs a `jsonld/` subdir (W3C JSON-LD test suite is at
   `https://github.com/w3c/json-ld-api` — different repo). Vendor
   it via `git subtree add` at a pinned commit.
3. **Extend oracle adapters.** `crates/testing/rdf-diff-oracles`
   already wraps `oxrdfxml` + `oxjsonld` behind `oracle-oxrdfxml` /
   `oracle-oxjsonld` features. Confirm adapters compile; if they
   need a minor update to match a Phase-B API delta, patch in-session.
4. **Extend xtask manifest runner.** Add `rdfxml` and `jsonld`
   language cases to `xtask/verify/src/manifest.rs::parse_for_language`
   (currently stubs for both). Main + shadow + oracle plumbing
   mirrors the Phase A pattern.
5. **Cohort registry update.** Amend `docs/agent-cohorts.md` with the
   Phase-B agent rows. Shadow agents (`pb-shadow-xml`,
   `pb-shadow-jsonld`) keep the `claude-sonnet-4-6` base-model
   override; all others default to `claude-opus-4-7`.
6. **Memory namespace seeds.** `crate/rdf-xml`, `crate/rdf-jsonld`,
   `crate/rdf-trix`, `crate/rdf-n3`, `crate/rdf-xml-shadow`,
   `crate/rdf-jsonld-shadow`, `phase-b` (blackboard),
   `verification/adversary-findings/{rdfxml,jsonld,trix,n3}`.
7. **Per-agent prompt files.** Write to `scripts/spawn/phase-b/` as
   files — the orchestrator's Agent-tool message reads from disk.

Exit: ADR-0007 Accepted; `cargo check --workspace --all-features`
green; `external/tests/jsonld/` populated; cohort registry merged.

### 2. Single-shot parallel spawn

**One Agent-tool message** containing every worker below. All use
`run_in_background: true`. All editing agents use
`isolation: "worktree"`; read-only agents (reviewer, adversary-veto)
do not.

### 3. Worker roster — implementing cohort A

Namespace: `phase-b`. Prompt lineage: `cohort-a`. Base model:
`claude-opus-4-7` (orchestrator default).

| Agent id            | RuFlo role   | Worktree | Claims                                               | Deliverable                                                   |
|---------------------|--------------|----------|------------------------------------------------------|---------------------------------------------------------------|
| `pb-rdf-xml`        | `coder`      | yes      | `crates/rdf-xml/**`                                  | RDF/XML parser (`XmlParser`), `rdf_diff::Parser` impl.        |
| `pb-rdf-jsonld`     | `coder`      | yes      | `crates/rdf-jsonld/**`                               | JSON-LD parser + `@context` well-formedness; `rdf_diff::Parser`. |
| `pb-rdf-trix`       | `coder`      | yes      | `crates/rdf-trix/**`                                 | TriX parser (XML wrapper around N-Triples content).           |
| `pb-rdf-n3`         | `coder`      | yes      | `crates/rdf-n3/**`                                   | N3 parser (Turtle superset).                                  |
| `pb-rdf-xml-main-tester` | `tester` | no       | `crates/rdf-xml/tests/**`                            | Inline + integration tests, runs W3C rdfxml manifest.        |
| `pb-tester`         | `tester`    | no       | `crates/testing/rdf-diff/tests/**`                   | Cross-format adversary + snapshot wiring; un-ignore per pattern. |
| `pb-reviewer`       | `reviewer`  | no       | read-only                                            | ADR-0017 §7 gates; append-only audit at `.claude-flow/audit/phase-b-reviews/`. |

Concurrency: 7 agents.

### 4. Worker roster — shadow cohort (cohort-A, model-override)

Namespace: `phase-b`. Prompt lineage: `cohort-a`. Base model:
`claude-sonnet-4-6` (ADR-0019 §3 disjointness).

| Agent id                | RuFlo role | Worktree | Claims                                  | Deliverable                                  |
|-------------------------|------------|----------|-----------------------------------------|----------------------------------------------|
| `pb-shadow-rdfxml`      | `coder`    | yes      | `crates/syntax/rdf-xml-shadow/**`       | Independent RDF/XML parser, `shadow` feature-gated. |
| `pb-shadow-jsonld`      | `coder`    | yes      | `crates/syntax/rdf-jsonld-shadow/**`    | Independent JSON-LD syntax parser, `shadow`-gated. |

Concurrency: +2 agents (9 total so far).

### 5. Worker roster — adversary hive (cohort B)

Namespace: `phase-b-adv`. Prompt lineage: `cohort-b`. Base model:
`claude-sonnet-4-6`. **Must not read `phase-b` memory** per
verification-v1 guard.

| Agent id            | RuFlo role      | Worktree | Claims                                              | Deliverable                                           |
|---------------------|-----------------|----------|-----------------------------------------------------|-------------------------------------------------------|
| `pb-adv-redteam`    | `reviewer`      | no       | read-only on spec                                   | Briefs for `rdfxml`, `jsonld`, `trix`, `n3` (3–10 failure modes per format). |
| `pb-adv-rdfxml`     | `tester`        | yes      | `crates/testing/rdf-diff/tests/adversary-rdfxml/**` | Fixture corpus from brief.                            |
| `pb-adv-jsonld`     | `tester`        | yes      | `crates/testing/rdf-diff/tests/adversary-jsonld/**` | "                                                     |
| `pb-adv-trix`       | `tester`        | yes      | `crates/testing/rdf-diff/tests/adversary-trix/**`   | "                                                     |
| `pb-adv-n3`         | `tester`        | yes      | `crates/testing/rdf-diff/tests/adversary-n3/**`     | "                                                     |
| `pb-adv-veto`       | `code-analyzer` | no       | read-only                                           | Veto register extension; audit at `.claude-flow/audit/adversary-veto/register.md`. |

Concurrency: +6 agents (15 total — at the ADR-0017 ceiling exactly).

### 6. Integration pass (orchestrator, in-session)

Triggered by the **last** cohort-A completion callback. Not polled.

1. Merge worktrees in dependency-safe order: shadows → mains → tests
   → adversary fixtures. Resolve claim overlaps on workspace `Cargo.toml`.
2. Run `cargo test --workspace --all-features --no-fail-fast`;
   `cargo clippy --workspace --all-features -- -D warnings`;
   `cargo deny check`; `xtask verify`.
3. **Phase-B exit gate check** — per-language:
   - `rdfxml`: 100 % of W3C rdfxml positive/negative syntax + eval
     tests pass (allow-list permitted with cited retirement plans).
   - `jsonld`: 100 % of W3C JSON-LD syntax suite (expand/compact
     explicitly NOT required — those are Phase E).
   - `trix`: snapshot corpus green; no W3C suite exists.
   - `n3`: snapshot corpus green; no W3C suite exists.
4. Triage adversary findings — close or allow-list with retirement
   plans, matching the verification-v1 pattern.
5. Flip this ADR to Accepted. Update
   `docs/sparc/04-refinement.md`'s Phase B retro. Tag
   `phase-b/done`.

### 7. Hard parallelism rules (restated)

1. All 15 spawns in **one** Agent-tool message.
2. `run_in_background: true` on every spawn.
3. Dependent work resolves via frozen traits + stubs, not via spawn
   ordering.
4. Integration only in the orchestrator session.
5. `claims_claim` before every edit; schema
   `agent:<id>:<role>` per `docs/runbooks/claims-workflow.md`.
6. Cohort-tag enforcement via `scripts/memory-hygiene/cohort-guard.mjs`.

### 8. Agent prompt template

Every prompt carries:

- Cohort tag (`cohort-a` / `cohort-b`).
- Base-model override (for shadows + adversary cohort).
- Authoritative on-disk prompt path under `scripts/spawn/phase-b/`.
- Claim list.
- Forbidden read list (cross-cohort namespaces).
- Frozen-API pointer (`crates/testing/rdf-diff/src/lib.rs`).
- Exit reporting into `crate/<name>` memory.
- Handoff target (`pb-reviewer` for implementers, `pb-adv-veto` for
  adversaries).

## Consequences

- **Positive.**
  - Throughput: 4 mains + 2 shadows + adversary hive in one spawn,
    15 agents at the ceiling — wall-clock dominated by the slowest
    single agent.
  - Independence preserved: shadow model-override + cohort-B
    adversary hive unchanged from verification-v1.
  - Phase A infra (`xtask`, `rdf-diff`, oracles, `ALLOWLIST.md`,
    spec-readings) scales: no bootstrap cost.
- **Negative.**
  - 15 agents exactly meets the ceiling; one additional adversary
    agent (if a per-format fixture gap surfaces) overflows into a
    short second spawn.
  - `rdf-trix` and `rdf-n3` have no shadow (per ADR-0019 §3); the
    differential signal is oracle-only for those two formats.
  - JSON-LD `@context` well-formedness bleeds toward the boundary of
    "syntax" vs "semantics". We pin the scope: no expand, no compact,
    no normalize. Anything more is an ADR amendment.
- **Neutral.**
  - This ADR is itself a candidate for the "ADR rot" risk from the
    Round-2 review (see ADR-0020 Consequences). It carries the same
    retirement trigger: on tag `phase-b/done` it becomes **historical**
    and is not consulted for future sweeps except as precedent.

## Validation

- **One spawn message** — orchestrator's tool-call log shows a single
  Agent-tool message with 15 entries.
- **Cohort separation holds** — no cohort-B agent queries
  `phase-b`; no cohort-A agent queries `phase-b-adv`.
  `cohort-guard.mjs` audit empty of cross-cohort violations.
- **Shadow divergence is non-zero on first run** for `rdf-xml` and
  `rdf-jsonld`. Zero is suspicious (ADR-0019 §Validation); audit
  prompt lineages.
- **Adversary veto fires** ≥ 1 time across the sweep.
- **W3C gate green** per §6.3 above before the ADR flips to Accepted.
- **Wall-clock** ≤ 72 h from spawn to `phase-b/done` tag.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy.
- [`0018-phase-a-execution-plan.md`](0018-phase-a-execution-plan.md) —
  prior wave-based worked example.
- [`0019-independent-verification.md`](0019-independent-verification.md)
  — independence policy (shadow + oracle + adversary).
- [`0020-verification-implementation-plan.md`](0020-verification-implementation-plan.md)
  — the single-shot parallel-swarm template we reuse.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 — Phase
  B budget + exit gate.
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
- Project root `CLAUDE.md` — Concurrency + Agent Orchestration
  (load-bearing).
