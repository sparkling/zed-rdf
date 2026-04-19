# Pin: SPARQL literal comparison — `=`, `!=`, and term equality

- **Diagnostic code:** `SPARQL-LITCMP-001`
- **Language / format:** SPARQL 1.1 Query and Update.
- **Productions:** `FILTER` expression evaluation (§17.3),
  `RDFterm-equal` (§17.4.1.7), `sameTerm` (§17.4.1.8), `COMPARE`
  infra (§17.3.1), `GROUP BY`/`DISTINCT` value-equality semantics.
- **Spec target:** W3C SPARQL 1.1 Query
  <https://www.w3.org/TR/sparql11-query/>; RDF 1.1 Concepts §3.3;
  XPath/XQuery Functions and Operators 3.0 §10 for numeric/string
  comparison.
- **Status:** active.
- **Author:** `v1-specpins` (cohort A).
- **Date:** 2026-04-19.

## Ambiguous clause

From SPARQL 1.1 §17.3 "Operator Mapping":

> "SPARQL … uses the XPath functions and operators for many of its
> comparison operations. … Comparison operations return an error if
> the operands are not comparable."

From §17.4.1.7 "RDFterm-equal":

> "Returns `TRUE` if term1 and term2 are the same RDF term as defined
> in Resource Description Framework (RDF): Concepts and Abstract
> Syntax [RDF-CONCEPTS]; produces a type error if the arguments are
> both literal but are not the same RDF term; returns `FALSE`
> otherwise."

From §17.4.1.8 "sameTerm":

> "Returns `TRUE` if term1 and term2 are the same RDF term; returns
> `FALSE` otherwise."

The tension is how `=` (the operator in a FILTER) dispatches between
value-based comparison (XPath numeric/string/boolean equality) and
term-based comparison (`RDFterm-equal`). When comparing two literals
with datatypes SPARQL does not recognise as comparable (e.g. a
custom datatype IRI), the result is an **error**, not `FALSE`; the
error then propagates through `FILTER` and discards the solution.
Three readings are seen in the wild:

1. "Same RDF term or bust" — `"1"^^xsd:integer = "01"^^xsd:integer`
   is `false` (different lexical forms, different RDF terms).
2. "Value equality always" — treat `=` as XPath `op:numeric-equal`
   whenever both sides are numeric; the above is `true`.
3. "Value equality for recognised types, error otherwise" — the
   spec's intended reading.

## Reading chosen

The parser-and-evaluator MUST implement the **spec's intended
reading**, which combines three rules:

1. **Operand mapping (§17.3).** `=` between two literals is
   evaluated as follows:
   - If both literals' datatypes fall in the "SPARQL recognised
     numeric" set (xsd:integer, xsd:decimal, xsd:float, xsd:double,
     and their derived types), use XPath `op:numeric-equal`. Thus
     `"1"^^xsd:integer = "01"^^xsd:integer` is `true` (same numeric
     value).
   - If both are plain simple literals, use XPath
     `op:string-equal`. (Recall RDF 1.1 Concepts §3.3: simple
     literals are typed `xsd:string`.)
   - If both are `xsd:boolean`, use `op:boolean-equal`.
   - If both are `xsd:dateTime`, use `op:dateTime-equal`.
   - If both are language-tagged strings: equal iff both the
     lexical form AND the language tag match (case-insensitive on
     the tag per BCP47, see `NT-LITESC-001` discussion of the RDF
     1.2 canonicalisation note which does **not** yet apply here
     for RDF 1.1 semantics).
2. **Otherwise error (§17.4.1.7).** For any other literal pair —
   different unrecognised datatypes, or one recognised numeric and
   the other unrecognised — `=` raises a type error. Inside a
   `FILTER`, a type error discards the solution (not accepts, not
   rejects as `FALSE`). Inside `BIND`, a type error leaves the
   variable unbound.
3. **`sameTerm` is always exact (§17.4.1.8).** `sameTerm(x, y)` is
   strictly term equality: same lexical form, same datatype IRI
   (byte-for-byte per `IRI-PCT-001`), same language tag. No value
   coercion. `sameTerm("1"^^xsd:integer, "01"^^xsd:integer)` is
   `false`.

Corollary used by the evaluator:

- **`DISTINCT` and `GROUP BY`** use `RDFterm-equal` / value-based
  equality where applicable, not `sameTerm`. `"1"^^xsd:integer` and
  `"01"^^xsd:integer` collapse under `DISTINCT`.
- **Solution-mapping equality** inside the algebra (used by MINUS
  §12.6, subqueries) is `sameTerm`, not `RDFterm-equal`.

## Rationale

- §17.3's operator-mapping table is prescriptive, not advisory.
  Implementations that treat `=` as `sameTerm` will fail the W3C
  SPARQL 1.1 test suite entry
  `rdfTermEquality` (see `data-sparql11/cast/` and
  `data-sparql11/functions/` manifests).
- §17.4.1.7's "produces a type error if the arguments are both
  literal but are not the same RDF term" is the narrow clause that
  makes FILTER discard solutions instead of keeping them with
  `FALSE`. public-sparql-dev mailing-list thread "Filter scope in
  OPTIONAL" (2011) and SPARQL errata SE-2 both reinforce that
  errors in FILTER discard (see the related pin area for
  OPTIONAL; not the subject of this pin but adjacent).
- The cohort-B adversary brief
  `docs/verification/adversary-findings/sparql.md` Failure Mode 1
  highlights the unbound-variable error case in FILTER (§12.3.1);
  this pin covers the closely-related literal-comparison case. Both
  flow through the "error → discard" rule.
- The `oxsparql-syntax` oracle handles syntax only; semantic
  comparison is verified against Jena / rdf4j fact-corpus outputs
  per ADR-0019 §1. Those engines implement this reading;
  divergence is a cohort-A correctness bug.

## Non-goals

- This pin does **not** cover SPARQL ordering (`ORDER BY`) which
  has its own total-order rules in §15.1; a separate pin will be
  added if adversary findings flag divergence.
- It does not cover the RDF 1.2 literal-direction proposal
  (`rdf:dirLangString`) which is a 1.2-era addition.

## Diagnostic code

- **Code:** `SPARQL-LITCMP-001`
- **Emitted by:** `sparql-syntax`, `sparql-syntax-shadow`,
  `oxsparql-syntax` oracle adapter (syntax only), and — once the
  query evaluator lands in a later Phase — the evaluator crate.
- **Message template (syntax phase):**
  `SPARQL-LITCMP-001: literal comparison semantics deferred to evaluator; syntax only`
  (non-fatal trace, used so the diff harness does not mis-attribute
  differences to the syntax layer).
- **Message template (evaluator phase):**
  `SPARQL-LITCMP-001: literal comparison error at <loc>: <detail>`
- **Fatal?** No in the syntax layer (it is a semantic pin);
  evaluator disagreement surfaces as `ObjectMismatch` or
  `AcceptRejectSplit` with this code in the triage hint.

## Forward references

- `crates/syntax/sparql-syntax/SPEC.md` — TODO: cite
  `SPARQL-LITCMP-001` under "Pinned readings" and forward to
  the future evaluator crate's SPEC.
- `crates/syntax/sparql-syntax-shadow/` — the syntax layer is the
  immediate consumer; semantic enforcement lands later.
- Adversary fixtures:
  `tests/adversary-sparql/sparql-litcmp-001-numeric-coerce.rq`,
  `tests/adversary-sparql/sparql-litcmp-001-unknown-datatype.rq`,
  `tests/adversary-sparql/sparql-litcmp-001-sameterm-vs-eq.rq`.
