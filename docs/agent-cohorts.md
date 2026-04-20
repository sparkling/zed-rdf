# Agent cohort registry — verification-v1 sweep

- **Instantiates:** ADR-0019 §3, §4, §6 (cohort separation).
- **Instantiates:** ADR-0020 §1.5 (pre-flight freeze of cohort assignments).
- **Status:** Frozen for the `verification-v1/*` tag. Changes require a new
  row at the bottom, not an in-place rewrite; history is part of the
  independence audit.
- **Audit window:** from `verification-v1/start` (this file's first commit)
  to `verification-v1/done`.

## Purpose

ADR-0019 §3 requires shadow implementations to be produced by a cohort with
a **disjoint prompt lineage** from the main implementation, ideally on a
**different base model**. ADR-0019 §4 requires the adversary hive to share
no framing with the implementers. ADR-0019 §6 requires cohort tags on
memory reads/writes.

This file is the source of truth for **which agent belongs to which
cohort**. Every prompt file under `scripts/spawn/verification-v1/` carries
its cohort tag; wrappers around `memory_store` / `memory_search` consult
this file (via `v1-memory-ttl`'s guard) to enforce no-cross-talk.

## Cohort definitions

A **cohort** is the tuple `(lineage_tag, base_model, seed_references)`.
Two cohorts are **disjoint** iff every element of the tuple differs or is
explicitly vetted as independent.

### Cohort A — implementing hive

- **Hive namespace:** `verification-v1`
- **Lineage tag:** `cohort-a`
- **Base model (default):** `claude-opus-4-7` (the orchestrator's model).
  Per-agent override recorded in the per-agent row below when used.
- **Seed references:** W3C spec text for each format, the frozen trait
  surface in `crates/testing/rdf-diff/src/lib.rs`, ADR-0019, ADR-0020,
  ADR-0006. **Does not** cite oracle crate APIs (`oxttl` etc.) beyond the
  adapter signatures authored by `v1-oracle-rust`.
- **May read memory namespaces:** `verification-v1`, `crate/rdf-diff`,
  `crate/*-shadow`, `verification/spec-readings`,
  `verification/cargo-deny`, `verification/memory-hygiene`.
- **May NOT read:** `verification-v1-adv`,
  `verification/adversary-findings`.

### Cohort B — adversary hive

- **Hive namespace:** `verification-v1-adv`
- **Lineage tag:** `cohort-b`
- **Base model (default):** `claude-sonnet-4-6`. Disjoint from cohort A's
  default on purpose; a genuinely different prior is the point.
- **Seed references:** W3C spec text, a red-team brief ("find inputs that
  expose divergent readings between the main parsers and the oracles
  listed in ADR-0019 §1"). **Does not** reference the implementing hive's
  prompts, memory, or worktrees.
- **May read memory namespaces:** `verification-v1-adv`,
  `verification/adversary-findings`, `verification/spec-readings` (pins
  are shared reference material, not implementation framing).
- **May NOT read:** `verification-v1`, any `crate/*-shadow` namespace.

### Cross-talk ban (mechanical)

`v1-memory-ttl` lands a guard in the `memory_store` / `memory_search`
wrapper that fails closed when a cohort-B caller queries a
`verification-v1` namespace, or vice versa. Until that guard lands,
discipline is prompt-level only and this registry is the backstop.

## Per-agent assignments

Every `Agent id` below matches the filename of the corresponding prompt
in `scripts/spawn/verification-v1/`. The prompt carries the cohort tag
in its frontmatter; prompts and this table must agree.

### Cohort A (`verification-v1`)

| Agent id           | RuFlo role          | Lineage  | Base model         | Notes                                                |
|--------------------|---------------------|----------|--------------------|------------------------------------------------------|
| `v1-diff-core`     | `coder`             | cohort-a | claude-opus-4-7    | Fills the frozen trait surface.                      |
| `v1-oracle-rust`   | `coder`             | cohort-a | claude-opus-4-7    | Adapter layer only; does not compose spec readings.  |
| `v1-oracle-jvm`    | `cicd-engineer`     | cohort-a | claude-opus-4-7    | CI job; no Rust path impact.                         |
| `v1-cargo-deny`    | `coder`             | cohort-a | claude-opus-4-7    | `[dev-dependencies]` carve-out enforcement.          |
| `v1-shadow-iri`    | `coder`             | cohort-a | claude-sonnet-4-6  | **Model override** for independence from main IRI.   |
| `v1-shadow-nt`     | `coder`             | cohort-a | claude-sonnet-4-6  | **Model override** for independence from main NT.    |
| `v1-shadow-ttl`    | `coder`             | cohort-a | claude-sonnet-4-6  | **Model override** for independence from main TTL.   |
| `v1-shadow-sparql` | `coder`             | cohort-a | claude-sonnet-4-6  | **Model override** for independence from main SPQL.  |
| `v1-specpins`      | `specification`     | cohort-a | claude-opus-4-7    | Pins are shared across cohorts.                      |
| `v1-memory-ttl`    | `memory-specialist` | cohort-a | claude-opus-4-7    | Lands the guard this table depends on.               |
| `v1-ci-wiring`     | `cicd-engineer`     | cohort-a | claude-opus-4-7    | PR gate wiring; no spec framing.                     |
| `v1-tester`        | `tester`            | cohort-a | claude-opus-4-7    | Property + snapshot tests only.                      |
| `v1-reviewer`      | `reviewer`          | cohort-a | claude-opus-4-7    | Engineering review; not spec arbitration.            |

> **Shadow-parser override rationale.** The four `v1-shadow-*` agents run
> on `claude-sonnet-4-6` rather than cohort A's default `claude-opus-4-7`.
> ADR-0019 §3 asks for "ideally a different base model". The main parsers
> are being written by `claude-opus-4-7` in Phase A; a different base for
> the shadow is the cheapest genuine-independence signal we have.

### Cohort B (`verification-v1-adv`)

| Agent id         | RuFlo role       | Lineage  | Base model         | Notes                                                    |
|------------------|------------------|----------|--------------------|----------------------------------------------------------|
| `v1-adv-redteam` | `reviewer`       | cohort-b | claude-sonnet-4-6  | Writes briefs; does not author fixtures itself.          |
| `v1-adv-nt`      | `tester`         | cohort-b | claude-sonnet-4-6  | Per-format adversary tester. Queued low if ceiling binds.|
| `v1-adv-ttl`     | `tester`         | cohort-b | claude-sonnet-4-6  | "                                                        |
| `v1-adv-iri`     | `tester`         | cohort-b | claude-sonnet-4-6  | "                                                        |
| `v1-adv-sparql`  | `tester`         | cohort-b | claude-sonnet-4-6  | "                                                        |
| `v1-adv-veto`    | `code-analyzer`  | cohort-b | claude-sonnet-4-6  | Veto flag per ADR-0019 §4.                               |

## Retirement trigger

On `git tag verification-v1/done` this registry becomes **historical**.
A follow-up sweep gets a new registry (`docs/agent-cohorts/vN.md` or a
successor ADR), not an edit of this one. See ADR-0020 Consequences
(Neutral).

## Phase B agents (ADR-0021)

**Audit window:** from `phase-b/start` to `phase-b/done`. Rules
are identical to the verification-v1 block above; only the namespaces
and agent IDs differ.

### Phase B — Cohort A (`phase-b`)

| Agent id                 | RuFlo role   | Lineage  | Base model        | Notes                                                       |
|--------------------------|--------------|----------|-------------------|-------------------------------------------------------------|
| `pb-rdf-xml`             | `coder`      | cohort-a | claude-opus-4-7   | Main RDF/XML parser.                                        |
| `pb-rdf-jsonld`          | `coder`      | cohort-a | claude-opus-4-7   | Main JSON-LD syntax parser.                                 |
| `pb-rdf-trix`            | `coder`      | cohort-a | claude-opus-4-7   | Main TriX parser.                                           |
| `pb-rdf-n3`              | `coder`      | cohort-a | claude-opus-4-7   | Main N3 parser.                                             |
| `pb-rdf-xml-main-tester` | `tester`     | cohort-a | claude-opus-4-7   | W3C rdfxml manifest tests.                                  |
| `pb-tester`              | `tester`     | cohort-a | claude-opus-4-7   | Cross-format adversary + snapshot wiring.                   |
| `pb-reviewer`            | `reviewer`   | cohort-a | claude-opus-4-7   | ADR-0017 §7 gates; audit at `.claude-flow/audit/phase-b-reviews/`. |
| `pb-shadow-rdfxml`       | `coder`      | cohort-a | claude-sonnet-4-6 | **Model override** for RDF/XML shadow independence.         |
| `pb-shadow-jsonld`       | `coder`      | cohort-a | claude-sonnet-4-6 | **Model override** for JSON-LD shadow independence.         |

### Phase B — Cohort B (`phase-b-adv`)

| Agent id        | RuFlo role      | Lineage  | Base model        | Notes                                           |
|-----------------|-----------------|----------|-------------------|-------------------------------------------------|
| `pb-adv-redteam`| `reviewer`      | cohort-b | claude-sonnet-4-6 | Red-team briefs; 3–10 failure modes per format. |
| `pb-adv-rdfxml` | `tester`        | cohort-b | claude-sonnet-4-6 | Adversary fixture corpus for rdfxml.            |
| `pb-adv-jsonld` | `tester`        | cohort-b | claude-sonnet-4-6 | Adversary fixture corpus for jsonld.            |
| `pb-adv-trix`   | `tester`        | cohort-b | claude-sonnet-4-6 | Adversary fixture corpus for trix.              |
| `pb-adv-n3`     | `tester`        | cohort-b | claude-sonnet-4-6 | Adversary fixture corpus for n3.                |
| `pb-adv-veto`   | `code-analyzer` | cohort-b | claude-sonnet-4-6 | Veto register; audit at `.claude-flow/audit/adversary-veto/register.md`. |

## Phase C agents (ADR-0022)

**Audit window:** from `phase-c/start` to `phase-c/done`.

### Phase C — Cohort A (`phase-c`)

| Agent id | RuFlo role | Lineage | Base model | Notes |
|----------|-----------|---------|-----------|-------|
| `pc-sparql-wiring` | `backend-dev` | cohort-a | claude-opus-4-7 | xtask/verify sparql + integration tests. |
| `pc-tester` | `tester` | cohort-a | claude-opus-4-7 | Adversary fixture un-ignore + snapshots. |
| `pc-reviewer` | `reviewer` | cohort-a | claude-opus-4-7 | ADR-0017 §7 gate review. |

## Phase D agents (ADR-0023)

**Audit window:** from `phase-d/start` to `phase-d/done`.

### Phase D — Cohort A (`phase-d`)

| Agent id | RuFlo role | Lineage | Base model | Notes |
|----------|-----------|---------|-----------|-------|
| `pd-shex-syntax` | `coder` | cohort-a | claude-opus-4-7 | ShEx compact syntax parser. |
| `pd-datalog-syntax` | `coder` | cohort-a | claude-opus-4-7 | Datalog syntax parser. |
| `pd-tester` | `tester` | cohort-a | claude-opus-4-7 | Integration + snapshot tests. |
| `pd-reviewer` | `reviewer` | cohort-a | claude-opus-4-7 | ADR-0017 §7 gate review. |

## Audit log

- 2026-04-19 — registry frozen during ADR-0020 pre-flight. Cohort A
  default `claude-opus-4-7`; cohort B default `claude-sonnet-4-6`. Four
  shadow-parser agents overridden to `claude-sonnet-4-6` for base-model
  disjointness per ADR-0019 §3. — Orchestrator, session
  `session-1776556770073`.
- 2026-04-20 — Phase B cohort rows appended during ADR-0021 pre-flight.
  9 cohort-A agents (2 with sonnet-4-6 model override) + 6 cohort-B
  adversary agents. Total 15 — at ADR-0017 ceiling. — Orchestrator,
  session `session-1776711885748`.
- 2026-04-20 — Phase C cohort rows appended. 3 cohort-A agents. sparql-syntax implementation pre-existing; Phase C work is test wiring + W3C conformance. — Orchestrator, session session-1776716816155.
- 2026-04-20 — Phase D cohort rows appended. 4 cohort-A agents. — Orchestrator, session session-1776716816155.
