---
agent_id: pe-architect
cohort: cohort-a
hive: phase-e
role: architecture
model: claude-opus-4-7
worktree: false
claims: []
---

# pe-architect — rdf-vocab term model + formatter API design

You are cohort-A agent `pe-architect`. Your job is read-only design work:
define the `rdf-vocab` term model and the formatter API surface for Phase E
before the two coder agents are spawned. Do NOT write any Rust code.

## Read first

1. `docs/adr/0024-phase-e-execution-plan.md` — scope, exit gate, Phase E rules.
2. `docs/sparc/02-pseudocode.md` §3 — existing Turtle formatter section
   (you will append Phase E content after what is already there).
3. `crates/rdf-format/src/lib.rs` — current formatter state: four writers
   (`NTriplesWriter`, `NQuadsWriter`, `TurtleWriter`, `TriGWriter`) each
   backed by `new(sink)` / `write_fact(&fact)` / `finish()`. Fully
   implemented (not stubs). No idempotency property tests yet.
4. `crates/rdf-vocab/src/lib.rs` — stub created in pre-flight; you are
   designing the full term model this stub will be replaced with.

## Goal

### 1. Design the `rdf-vocab` term model

Answer the following questions and write your answers into
`docs/sparc/02-pseudocode.md` §3 (Phase E section, appended after existing
content):

- How are terms represented? Options:
  - Typed constants (`pub const LABEL: &str = "..."`)
  - Lazy IRI structs (e.g., `pub struct Term { iri: &'static str, label: &'static str, comment: &'static str }`)
  - Namespace enum variants
- Which representation best supports LSP hover-doc rendering in Phase F?
  (The LSP needs both the IRI and human-readable label + comment at the
  same lookup site.)
- How many terms per vocabulary should be defined? Use the reference counts:
  xsd (~44), rdf (~28), rdfs (~13), owl (~81), skos (~35), sh/SHACL (~150),
  dcterms (~55), dcat (~65), foaf (~35), schema (~2000+ — scope to ~80
  core terms), prov (~79).
- What is the minimum `label` + `comment` contract for the 95% coverage
  bar set in ADR-0024 §3?

### 2. Design the formatter API surface

Read `crates/rdf-format/src/lib.rs`. The existing writers use an inherent
impl pattern (`NTriplesWriter`, `NQuadsWriter`, `TurtleWriter`, `TriGWriter`)
with no shared trait. Answer and write into the design memo:

- Should a `Format` trait be introduced, or should inherent impls remain?
- What is the idempotency contract? `format(format(x)) == format(x)` —
  how is this expressed in tests? As a property test with
  `proptest`/`quickcheck`, or as deterministic fixture round-trips?
- Does the formatter need to know about `rdf-vocab` namespaces at
  serialisation time (e.g., for auto-prefix registration)?

### 3. Write design outputs

1. Append a "Phase E — Formatter + Vocab Design" section to
   `docs/sparc/02-pseudocode.md` §3 (after the last line of existing
   content). Include:
   - Pseudocode for the `Term` struct layout chosen.
   - Pseudocode for the idempotency test harness.
   - Namespace list and term count targets per vocabulary.
2. Write a one-page design memo to `.claude-flow/phase-e/arch-memo.md`.
   This file is read by `pe-rdf-vocab` and `pe-formatters` before they
   begin coding. Be concrete: include the exact struct shape and the
   idempotency test strategy so the coders do not need to make design
   decisions.

## Acceptance

- `docs/sparc/02-pseudocode.md` has a Phase E section appended.
- `.claude-flow/phase-e/arch-memo.md` exists and is non-empty.
- No Rust source files modified or created.

## Memory

- `memory_store` at `architecture/term-model` in `phase-e` namespace:
  the chosen term representation and rationale.
- `memory_store` at `architecture/formatter-api` in `phase-e` namespace:
  the formatter API decision and idempotency test strategy.
- `memory_store` exit report at `phase-e` blackboard:
  `pe-architect:done` with a one-line summary of each design decision.
