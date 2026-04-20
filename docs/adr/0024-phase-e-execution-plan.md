# ADR-0024: Phase E execution plan — vocab + formatters via hierarchical swarm

- **Status:** Proposed
- **Date:** 2026-04-20
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Instantiates:** [ADR-0017](0017-execution-model.md) (execution
  policy)
- **Retires-on:** tag `phase-e/done`
- **Tags:** `process`, `execution`, `phase-e`, `vocab`, `formatters`

## Context and Problem Statement

Phase D closes at tag `phase-d/done` with `shex-syntax` and
`datalog-syntax` green. Phase E per
[`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 lands:

- `rdf-vocab` complete: vocabulary terms for xsd, rdf, rdfs, owl, skos,
  sh (SHACL), dcterms, dcat, foaf, schema.org (schema:), and prov. Each
  term carries a label and comment for hover-doc rendering in the LSP
  (Phase F). This crate does not exist yet and must be created.
- Per-format formatters in the existing `rdf-format` crate: Turtle
  formatter (idempotent, round-trips), N-Triples formatter, N-Quads
  formatter. RDF/XML and JSON-LD formatters are stretch goals within
  Phase E scope.

The two work streams have a shallow dependency: the architect must
define the `rdf-vocab` term model and the formatter API surface before
the two coders begin. This internal sequencing makes the hierarchical
topology from ADR-0017 §4 appropriate.

## Decision Drivers

- **ADR-0017 compliance.** Phase E topology: hierarchical. Agents:
  1 architect + 2 coders + 1 tester.
- **Architect-first sequencing.** The term model and formatter API must
  be stable before coder spawns; architect runs in the orchestrator
  session or as a short-lived prior spawn.
- **Sequential gate discipline.** This ADR activates only after
  `phase-d/done` is tagged.
- **LSP dependency.** `rdf-vocab` term coverage determines Phase F hover
  quality; the 95 % coverage bar is set here.

## Considered Options

1. **Parallel mesh (skip architect).** Both coders draft their own APIs
   independently. Risk: incompatible term models between `rdf-vocab` and
   the formatter's namespace handling.
2. **Hierarchical: architect then parallel coders.** Architect freezes
   the API; coders work in parallel. Chosen.
3. **Sequential: vocab then formatters.** Wastes concurrency; the two
   coders have no blocking dependency once the architect finishes.

## Decision

**Chosen option: 2 — hierarchical with architect pre-pass.**

### 1. Pre-flight (orchestrator, in-session)

Sequential; must complete before any spawn.

1. Confirm `phase-d/done` tag exists.
2. Audit `crates/rdf-format/` current state: identify which formatters
   exist (even as stubs) versus which are absent.
3. Create `crates/rdf-vocab/Cargo.toml` + `crates/rdf-vocab/src/lib.rs`
   stub if the crate does not exist. Add to workspace `[members]`.
4. Amend `docs/agent-cohorts.md` with Phase E agent rows.
5. Seed memory namespaces: `phase-e`, `crate/rdf-vocab`,
   `crate/rdf-format`.
6. Write per-agent prompt files to `scripts/spawn/phase-e/`.

Exit: `cargo check --workspace --all-features` green with the new stub;
cohort registry updated.

### 2. Worker roster — cohort A (hierarchical topology)

Namespace: `phase-e`. Topology: hierarchical. Base model:
`claude-opus-4-7`.

The architect runs first (in-session or as a short prior spawn) and
delivers its spec before the two coders are spawned.

| Agent id | Role | Worktree | Claims | Deliverable |
|---|---|---|---|---|
| `pe-architect` | `architecture` | no | read-only | Design `rdf-vocab` term model (struct layout, label/comment fields, namespace enums); formatter API surface (trait + method signatures). Writes spec to `docs/sparc/02-pseudocode.md` §3. Stores blueprint in `phase-e` memory namespace. |
| `pe-rdf-vocab` | `coder` | yes | `crates/rdf-vocab/**` | Complete vocabulary crate: 11 namespaces (xsd, rdf, rdfs, owl, skos, sh, dcterms, dcat, foaf, schema, prov); term definitions with labels and comments; hover-doc snapshots. |
| `pe-formatters` | `coder` | yes | `crates/rdf-format/**` | Turtle formatter (idempotent), N-Triples formatter, N-Quads formatter; idempotency tests (`format(format(x)) == format(x)` for all fixtures). RDF/XML and JSON-LD formatters are stretch goals. |
| `pe-tester` | `tester` | no | `crates/rdf-vocab/tests/**`, `crates/rdf-format/tests/**` | Snapshot tests for hover-doc coverage; idempotency property tests for formatters. |

Concurrency: 3 agents after architect pre-pass (architect may overlap
with the orchestrator session; not counted toward the per-phase ceiling
if run in-session).

### 3. Exit gate

- Hover-doc snapshots locked: term coverage >= 95 % for each of the
  11 vocabularies (label + comment present for >= 95 % of terms).
- Formatter idempotency tests green: `format(format(x)) == format(x)`
  holds for all Turtle, N-Triples, and N-Quads fixtures.
- `cargo test --workspace --all-features` green.

### 4. Integration pass (orchestrator, in-session)

Triggered by the last coder / tester completion callback.

1. Merge worktrees: `pe-rdf-vocab` → `pe-formatters`. Resolve workspace
   `Cargo.toml` overlap.
2. Run `cargo test --workspace --all-features --no-fail-fast`;
   `cargo clippy --workspace --all-features -- -D warnings`;
   `cargo deny check`.
3. Verify exit gate metrics per §3 above.
4. Flip this ADR to Accepted. Update `docs/sparc/04-refinement.md`
   Phase E retro. Tag `phase-e/done`.

### 5. Hard parallelism rules (restated from ADR-0017 §6)

1. Architect pre-pass completes before coders are spawned.
2. Coder + tester spawns go out in **one** Agent-tool message.
3. `run_in_background: true` on every parallel spawn.
4. Integration only in the orchestrator session.
5. `claims_claim` before every edit.

## Consequences

### Positive
- Architect pre-pass prevents API divergence between vocab term model
  and formatter namespace handling.
- 95 % vocab coverage bar sets a concrete floor for Phase F hover
  quality before LSP work starts.
- Formatter idempotency is locked in Phase E rather than discovered as
  a regression in Phase F.

### Negative
- Architect pre-pass adds a sequential step; total elapsed time is
  slightly longer than a pure mesh would be.
- RDF/XML and JSON-LD formatters are stretch goals and may not land in
  Phase E; Phase F must tolerate their absence.

### Neutral
- `rdf-vocab` is a new crate; workspace topology and dependency graph
  require a one-time registration step in pre-flight.
- This ADR becomes historical on tag `phase-e/done`.

## Validation

- Architect deliverable (`docs/sparc/02-pseudocode.md` §3) exists and
  is committed before coders are spawned.
- One spawn message for coders + tester.
- Exit gate metrics (§3) verified before ADR flips to Accepted.
- `cargo test --workspace --all-features` green on integration.

## Links

- [`0017-execution-model.md`](0017-execution-model.md) — parent policy
  (topology table §4, quality gates §7).
- [`0023-phase-d-execution-plan.md`](0023-phase-d-execution-plan.md)
  — predecessor phase.
- [`../sparc/04-refinement.md`](../sparc/04-refinement.md) §2 — Phase E
  budget and exit gate.
- [`../sparc/02-pseudocode.md`](../sparc/02-pseudocode.md) §3 — architect
  target section.
- [`../runbooks/claims-workflow.md`](../runbooks/claims-workflow.md)
  — claimant schema.
