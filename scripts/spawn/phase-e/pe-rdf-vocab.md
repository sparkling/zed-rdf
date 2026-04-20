---
agent_id: pe-rdf-vocab
cohort: cohort-a
hive: phase-e
role: coder
model: claude-opus-4-7
worktree: true
claims:
  - crates/rdf-vocab/**
---

# pe-rdf-vocab — complete rdf-vocab crate with 11 vocabularies

You are cohort-A agent `pe-rdf-vocab`. Your job is to implement the
complete `rdf-vocab` crate with all 11 vocabulary namespaces. The crate
stub already exists at `crates/rdf-vocab/` (created in pre-flight).

## Read first

1. `.claude-flow/phase-e/arch-memo.md` — architect memo defining the term
   model, struct layout, and per-vocabulary term count targets. **Read this
   before writing any code.**
2. `docs/adr/0024-phase-e-execution-plan.md` — scope and exit gate (95%
   coverage bar per vocabulary).
3. `crates/rdf-vocab/src/lib.rs` — current stub; you will replace it with
   the full implementation.
4. `crates/rdf-vocab/Cargo.toml` — current dependencies (only `rdf-iri`).

## Goal

### 1. Implement all 11 vocabulary modules

Each module must follow the term model defined in the architect memo. At
minimum, each term definition must include:

- The full IRI as a `&'static str` constant.
- A label (short human-readable name).
- A comment (one-sentence description suitable for LSP hover-doc).

Vocabularies and minimum term counts for 95% coverage:

| Module | Namespace | Target term count |
|--------|-----------|------------------|
| `xsd`  | `http://www.w3.org/2001/XMLSchema#` | ≥ 42 of 44 |
| `rdf`  | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` | ≥ 27 of 28 |
| `rdfs` | `http://www.w3.org/2000/01/rdf-schema#` | ≥ 13 of 13 |
| `owl`  | `http://www.w3.org/2002/07/owl#` | ≥ 77 of 81 |
| `skos` | `http://www.w3.org/2004/02/skos/core#` | ≥ 34 of 35 |
| `sh`   | `http://www.w3.org/ns/shacl#` | ≥ 143 of 150 |
| `dcterms` | `http://purl.org/dc/terms/` | ≥ 53 of 55 |
| `dcat` | `http://www.w3.org/ns/dcat#` | ≥ 62 of 65 |
| `foaf` | `http://xmlns.com/foaf/0.1/` | ≥ 34 of 35 |
| `schema` | `https://schema.org/` | ≥ 76 of 80 (core terms only) |
| `prov` | `http://www.w3.org/ns/prov#` | ≥ 76 of 79 |

### 2. Write tests

Create `crates/rdf-vocab/tests/coverage.rs` with:

- One test per vocabulary that checks term coverage >= 95%.
- Each test iterates a reference list of canonical term IRIs and asserts
  that the corresponding constant is defined in the module (i.e., the IRI
  is present in the module's exported constants).
- A snapshot test that samples 5 terms per vocabulary and asserts that
  label and comment are both non-empty strings.

### 3. Acceptance

- `cargo test -p rdf-vocab` green.
- `cargo clippy -p rdf-vocab -- -D warnings` clean.
- All 11 modules present in `src/lib.rs` with the NS constant + typed
  term constants.
- Coverage tests in `tests/coverage.rs` all pass.

## Claims

Claim `crates/rdf-vocab/**` before editing. Release on completion.

## Memory

- `memory_store` at `implementation/vocab-modules` in `crate/rdf-vocab`
  namespace: list of modules implemented, term counts per module.
- `memory_store` exit report at `phase-e` blackboard:
  `pe-rdf-vocab:done` with per-vocabulary term counts and test pass status.

## Handoff

`claims_accept-handoff` → `pe-tester` when `cargo test -p rdf-vocab` and
`cargo clippy -p rdf-vocab` are both green.
