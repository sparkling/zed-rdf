# fact-oracles — JVM-materialised reference corpora

This directory is produced **out-of-process** by
[`.github/workflows/fact-oracles.yml`](../../.github/workflows/fact-oracles.yml).
It holds accept/reject + fact-set JSON emitted by Apache Jena and Eclipse
rdf4j over the pinned W3C RDF test suites, one file per
`(language, parser, suite-commit)` triple.

The Rust diff harness (`crates/testing/rdf-diff/`) loads these files as
static oracles at test time. **No JVM ever runs in the Rust test path.**
This separation is the ADR-0019 §1 "JVM out of process" boundary.

## Why this exists

ADR-0019 breaks the Round-2 oracle-circularity flaw by adding *independent*
evidence that our parsers agree with implementations built from a different
prior. Two of those implementations — Jena and rdf4j — are JVM-only. We do
not embed a JVM in the Rust build. Instead, CI materialises a pinned JSON
corpus that any Rust process can load without dependencies.

## Directory layout

```
external/fact-oracles/
    README.md                           <-- this file
    tools/                              <-- Java CLI + pom.xml (workflow use only)
    fixtures/smoke/<lang>/*.{nt,nq,...} <-- smoke fixture for workflow acceptance
    <lang>/<parser>-<suite-commit-12>.json
```

Languages (`<lang>`): `nt`, `nq`, `ttl`, `trig`, `rdfxml`.
Parsers (`<parser>`): `jena`, `rdf4j`.

## JSON schema (v1.0.0)

The shape below is the contract between this workflow and the Rust diff
harness. It is intentionally flat and string-typed so the Rust side can
parse it with `serde_json::Value` or hand-rolled field access **without a
JSON-schema crate**.

### Top-level object

| field              | type     | description                                                                                  |
|--------------------|----------|----------------------------------------------------------------------------------------------|
| `schema_version`   | `string` | Semver. Major version `1` guarantees backward-compatible field additions only.               |
| `lang`             | `string` | One of `nt`, `nq`, `ttl`, `trig`, `rdfxml`.                                                  |
| `parser`           | `string` | One of `jena`, `rdf4j`.                                                                      |
| `parser_version`   | `string` | Maven coordinate version of the parser (e.g. `4.10.0` for Jena, `5.0.2` for rdf4j).          |
| `suite_commit`     | `string` | Commit SHA (full or `"smoke-fixture"` for the inline smoke corpus).                          |
| `generated_at_utc` | `string` | ISO-8601 UTC timestamp of generation.                                                        |
| `cases`            | `array`  | Array of case objects, sorted by `id` ascending for deterministic diffs.                     |

### Case object (each element of `cases`)

| field           | type      | required             | description                                                                                           |
|-----------------|-----------|----------------------|-------------------------------------------------------------------------------------------------------|
| `id`            | `string`  | always               | Stable id = path relative to the suite root, extension stripped, forward-slash separators.            |
| `input_path`    | `string`  | always               | Path relative to the suite root (forward-slash separators), extension intact.                         |
| `input_sha256`  | `string`  | always               | Lowercase hex SHA-256 of the input bytes. Corpus comparability pin.                                   |
| `accepted`      | `boolean` | always               | `true` if the parser accepted the input without a fatal error; `false` otherwise.                     |
| `error_class`   | `string`  | when `accepted=false`| Fully qualified Java class of the raised error (e.g. `org.eclipse.rdf4j.rio.RDFParseException`).      |
| `error_message` | `string`  | when `accepted=false`| First-line error message, trimmed. Diagnostic only; diff harness does not compare message text.       |
| `facts`         | `array`   | always               | Array of canonical fact lines (see below). Empty array for rejected inputs and for empty documents.   |
| `fact_count`    | `integer` | always               | `== facts.length`. Redundant but emitted so the Rust side can sanity-check before parsing the array.  |

### Fact-line canonical form

Each element of `facts` is a single-line string in an N-Triples-shaped
serialisation chosen so `jena` and `rdf4j` entries for the same input
**match byte-for-byte on agreeing cases**. Divergences are the signal the
diff harness is looking for.

For triple-bearing languages (`nt`, `ttl`, `rdfxml`):

    <iri> <iri> object .

For quad-bearing languages (`nq`, `trig`), the graph name is appended
**unless it is the default graph**, in which case the triple form is used:

    <iri> <iri> object <iri> .
    <iri> <iri> object .          # default graph

Where `object` is:

| kind            | form                                                     |
|-----------------|----------------------------------------------------------|
| IRI             | `<` *escaped-iri* `>`                                    |
| blank node      | `_:b0`, `_:b1`, ... assigned by first-appearance order   |
| plain literal   | `"` *escaped-lex* `"` (only when datatype is `xsd:string`) |
| lang literal    | `"` *escaped-lex* `"@` *lowercase-BCP47*                 |
| typed literal   | `"` *escaped-lex* `"^^<` *escaped-datatype-iri* `>`      |

#### IRI escape rules

Characters `<`, `>`, `"`, `{`, `}`, `|`, `^`, `` ` ``, `\`, and any code
point ≤ `0x20` are replaced with `\uXXXX`. All other characters pass
through unchanged. **IRIs are not percent-re-encoded here** — that is a
spec-reading pin (see `docs/spec-readings/`), not a serialisation
decision.

#### Literal lexical-form escape rules

`\` → `\\`, `"` → `\"`, `\n` → `\n`, `\r` → `\r`, `\t` → `\t`, any other
code point below `0x20` → `\uXXXX`. Everything else passes through.

#### Blank-node labelling

Blank nodes are renamed `_:b<N>` where `N` is the zero-based index of
first appearance in the file as yielded by the parser's triple/quad
stream. **This is a deterministic relabelling; it is not spec-level
bnode canonicalisation** (see ADR-0019 §2 — that job belongs to the Rust
diff harness).

### Ordering

- `cases` is sorted ascending by `id`.
- Within a case, `facts` is sorted lexicographically ascending.

The Rust diff harness does not re-sort; it relies on this invariant.

## Consumption from Rust

Minimal shape, no JSON-schema crate required:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Corpus {
    pub schema_version: String,
    pub lang: String,
    pub parser: String,
    pub parser_version: String,
    pub suite_commit: String,
    pub generated_at_utc: String,
    pub cases: Vec<Case>,
}

#[derive(Deserialize)]
pub struct Case {
    pub id: String,
    pub input_path: String,
    pub input_sha256: String,
    pub accepted: bool,
    #[serde(default)]
    pub error_class: String,
    #[serde(default)]
    pub error_message: String,
    pub facts: Vec<String>,
    pub fact_count: usize,
}
```

Consumers MUST reject files where `schema_version` does not start with
`"1."`.

## Pinning policy

- **Suite pin** — `W3C_RDF_TESTS_COMMIT` in the workflow. Bumping the
  commit is an ADR-level decision; diff harness baselines regenerate in
  the same PR.
- **Parser pin** — `JENA_VERSION`, `RDF4J_VERSION` in the workflow.
  Bumped only with an accompanying divergence report.
- **Tool pin** — `external/fact-oracles/tools/pom.xml` carries defaults
  that the workflow overrides via `-D` properties; the workflow is the
  source of truth.

## Refresh cadence

- **Weekly** cron (Monday 04:17 UTC).
- **On demand** via `workflow_dispatch`. Setting `smoke_fixture: true`
  runs the workflow against the inline fixture under `fixtures/smoke/`
  rather than the vendored W3C suites; use this for pre-merge smoke of
  the workflow itself.
- **On PR** touching `external/tests/**` or `external/fact-oracles/**`
  or the workflow file. In this mode the workflow only regenerates the
  corpus if `external/tests/` is vendored; otherwise it fails with a
  clear pointer to the phase-A prerequisite, as PR-time upstream fetches
  would make the diff non-reproducible.

Refreshes land as PRs against `main`, never as direct pushes (ADR-0019
§1 and project policy).

## What this does *not* do

- No bnode canonicalisation (rdf-canon). That is the Rust diff harness's
  job and lives behind the frozen `rdf-diff` trait surface.
- No percent-encoding normalisation of IRIs. Pinned spec readings in
  `docs/spec-readings/` decide what the "right" reading is; the oracle
  reports whatever the parser produced.
- No JSON-LD. ADR-0019 §1 routes JSON-LD to `oxjsonld` as a Rust
  `[dev-dependencies]` oracle rather than the JVM path.
- No SPARQL. SPARQL-syntax oracles live with the Rust oracle crate.
