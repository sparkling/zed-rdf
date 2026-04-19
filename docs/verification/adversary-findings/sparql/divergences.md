# SPARQL main-vs-shadow divergences (verification-v1 sweep)

Agent: `fe-phase-c-sparql` (Phase C, ADR-0017 §4 / ADR-0018).
Source parsers:

- Main — `crates/sparql-syntax` (`SparqlParser`).
- Shadow — `crates/syntax/sparql-syntax-shadow` (`SparqlShadowParser`).

Both implement `rdf_diff::Parser` and are compared by the diff harness
per ADR-0020 §1.4. The two codebases are independent (ADR-0019 §3):
neither was read while developing the other.

## Class 1 — independent AST-as-Facts encoding

Each crate chose its own fact encoding for the AST. The diff harness
compares the canonical `Facts` set; where encodings differ, divergence
surfaces as `FactOnlyIn` on both sides (every main predicate is missing
on the shadow side, and vice versa).

- **Main encoding.** Predicates live under `<urn:x-sparql-syntax:*>`
  (keys `kind`, `form`, `projection`, `where`, `modify-where`,
  `insert-data`, …). See `crates/sparql-syntax/README.md` for the
  full schema.
- **Shadow encoding.** The shadow uses its own namespace and
  granularity (it chose a different predicate namespace and may split
  structures that the main collapses, and vice versa).

Because the sweep's `Fact` canonicalisation is byte-for-byte on the
predicate and object strings, the two encodings do not interoperate.
The adversary tests accordingly gate on **accept/reject agreement**
only for FM1–FM4, FM6–FM10, FM11, FM12; fact diff is reported for
triage but does not fail the test. Full fact-level agreement is a
Phase-C+1 goal and requires either:

1. a shared AST-as-Facts schema bolted into `rdf-diff` (future ADR), or
2. an AST-comparison adapter that maps both encodings through a
   normaliser.

Until then, the **accept/reject contract is the hard gate** at the
harness layer, matching ADR-0019 §2 "any *behavioural* divergence is
a CI failure".

## Class 2 — FM5 (BASE mid-query)

- **Main.** Rejects with `SPARQL-PROLOGUE-001`, per §4.1 Prologue
  production. This is the spec-compliant behaviour.
- **Shadow.** May accept silently. If so, this is a documented
  divergence: the adversary test gates on main-rejection and emits a
  `stderr` note when the shadow diverges. The shadow is welcome to
  fix in a follow-up; the harness surfaces the split.

## Class 3 — FM11b (BIND scoping violation)

- **Main.** Rejects with `SPARQL-BIND-001`, per §18.2.1.
- **Shadow.** May accept with lenient scoping. Test gate is
  main-rejection; shadow acceptance is documented.

## Class 4 — FM8 (GRAPH block inside INSERT DATA)

- **Main.** Accepts `INSERT DATA { GRAPH <g> { … } }` per §3.1.1's
  QuadData → Quads production (`Quads ::= TriplesTemplate?
  (QuadsNotTriples '.'? TriplesTemplate?)*`).
- **Shadow.** Rejects with `"syntax error near 'GRAPH': expected RDF
  term"`. The shadow's Update grammar appears narrower than the spec.
  Hard gate is main-accepts; shadow divergence is recorded here.

## Class 5 — FM9 (negated property set)

- **Main.** Accepts `!(p)`, `!(p|q)`, and `^!(p)` per §9.3. The
  encoding distinguishes `^!(p)` from `!(^p)`.
- **Shadow.** Rejects `!` in path primary position: `"syntax error
  near '!': expected IRI or ( in property path"`. The main-accepts
  gate is retained; shadow divergence is recorded.

## Class 6 — Semantics-only failure modes

FM1, FM3, FM4, FM6, FM7, FM12 all parse successfully in both crates
(grammar-level). Their divergence is specification-semantic (FILTER
error-on-unbound vs false, HAVING alias resolution order, etc.), out
of scope for Phase C. Those tests surface no accept/reject split.

## Reopening criteria

A test in this file may be tightened from "accept/reject only" to
"fact-clean" once a shared AST encoding lands. At that point the
corresponding entry here is removed and the test assertion swaps to
`assert!(report.is_clean(), …)`.
