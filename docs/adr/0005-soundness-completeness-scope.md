# ADR-0005: Parser correctness scope boundary

- **Status:** Accepted (2026-04-18)
- **Date:** 2026-04-18
- **Deciders:** Henrik Pettersen
- **Supersedes:** earlier v1 of this ADR (which tried to define
  soundness/completeness for reasoning engines that are now out of scope)
- **Tags:** `policy`, `semantics`

## Context and Problem Statement

The project scope is a Zed extension + LSP, not a reasoning engine. The
brief phrase "sound and complete" still matters — we want our parsers to
**accept exactly what the spec defines as valid, and reject everything
else** — but it no longer applies to OWL reasoning, SHACL validation, or
Datalog evaluation, because we do not do any of those.

This ADR pins down what correctness means at LSP scope.

## Decision Drivers

- **Truth in advertising.** We claim only what we can verify.
- **Test-suite-first.** Claims map directly to W3C test manifests.
- **LSP usability.** Error-tolerant parsers must still produce a valid
  CST on broken input so LSP features work.

## Decision

**Our correctness claim per module, at v1.0:**

| Module / crate           | Claim                                                                                                                                                                                                                                               |
|--------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `rdf-iri`                | RFC 3987 IRI parse / normalise / relative resolution is sound and complete per the RFC, verified by the RFC's own ABNF tests.                                                                                                                     |
| `rdf-ntriples`           | Accepts exactly the language of RDF 1.1 N-Triples / N-Quads (+ RDF 1.2 extensions behind the `rdf-star` feature). Verified by `w3c/rdf-tests` → `nt`, `nq` at 100 %.                                                                                 |
| `rdf-turtle`             | Accepts exactly the language of RDF 1.1 Turtle / TriG (+ RDF 1.2 behind flag). Verified by `w3c/rdf-tests` → `turtle`, `trig` at 100 %.                                                                                                              |
| `rdf-xml`                | Accepts exactly the language of RDF 1.1 XML Syntax. Verified by `w3c/rdf-tests` → `rdf-xml` at 100 %.                                                                                                                                                 |
| `rdf-jsonld`             | Accepts exactly the JSON-LD 1.1 **surface syntax** and verifies **`@context` well-formedness** per the spec. We do **not** implement expand/compact algorithms; queries requiring semantic JSON-LD equivalence are out of scope. Verified by the syntax subset of `w3c/json-ld-api`. |
| `rdf-trix`               | Accepts the TriX XML schema; no official W3C manifest exists — covered by a curated corpus.                                                                                                                                                          |
| `rdf-n3`                 | Accepts Notation3 **surface syntax** (formulas, rules, quoted graphs). We do **not** implement N3 reasoning or built-ins. Covered by a curated corpus seeded from W3C Team Submission examples and TimBL's n3 repository.                         |
| `sparql-syntax`          | Accepts exactly SPARQL 1.1 Query + Update grammar (+ SPARQL 1.2 additions behind `sparql-1-2`). Verified by `w3c/rdf-tests` → `sparql11` syntax manifests at 100 %.                                                                                  |
| `shex-syntax`            | Accepts ShExC + ShExJ surface syntax. Verified by `shexSpec/shexTest` syntax-only entries at 100 %.                                                                                                                                                  |
| `datalog-syntax`         | Accepts our chosen Datalog surface syntax (ADR-TBD). No W3C suite exists. Covered by a curated fixture corpus. Semantics (evaluation) are out of scope.                                                                                              |
| `rdf-vocab`              | Curated vocabulary database. Correctness = "entries match the specs they claim to come from". Covered by snapshot tests against each vocabulary's canonical document.                                                                               |
| `rdf-format`             | Formatters are **idempotent** (format → format produces identical output) and **round-trip preserving** (parse → format → parse yields identical facts). Verified by property tests on the W3C corpus.                                            |
| `rdf-lsp`                | LSP spec conformance: LSP 3.17 methods we implement behave per the protocol. Verified by LSP integration harness.                                                                                                                                  |

**What we do not claim:**

- No semantic correctness of SPARQL queries (we do not execute them).
- No RDFS / OWL / SHACL / ShEx / Datalog **reasoning** or **validation**
  guarantees — those engines are not in scope.
- No correctness across remote endpoints — we do not talk to the network.

**Error tolerance requirement.** Every parser must produce a CST even on
malformed input, alongside the appropriate `Diagnostic`s. Correctness
here is a *two-pronged* claim:

- **On well-formed input**: the parser's fact output is spec-correct.
- **On malformed input**: the parser emits the right diagnostics per an
  agreed table (ADR-0008), recovers at statement boundaries, and does
  not panic. Verified by fuzz + snapshot tests.

## Consequences

- **Positive**: honest, tractable, test-suite-anchored claims.
- **Negative**: users wanting "run this SPARQL" or "validate against this
  SHACL shape" hit the scope wall. Documented; revisit only via an ADR
  amendment.
- **Neutral**: future growth into validation/reasoning would require new
  crates + new ADRs; the current scope leaves room for that without
  coupling to it.

## Validation

- Every parser crate ships a `SPEC.md` mapping features to spec clauses.
- Conformance gate (see [`05-completion.md`](../sparc/05-completion.md)
  §2) is green before release.
- A `ScopeBoundary` integration test exists: requesting a feature we do
  not implement (e.g., "run this query") fails with a documented,
  user-friendly message (where such a surface exists).

## Links

- `docs/sparc/01-specification.md` §2 hard scope boundaries.
- `docs/sparc/05-completion.md` §2 conformance gate.
- ADR-0006 testing strategy.
