# Pin: INSERT DATA / DELETE DATA — no variables, DELETE DATA no blank nodes

- **Diagnostic code:** `SPARQL-UPDATE-001`
- **Language / format:** SPARQL 1.1 Update.
- **Productions:** `InsertData ::= 'INSERT' 'DATA' QuadData`,
  `DeleteData ::= 'DELETE' 'DATA' QuadData`, `QuadData ::= '{'
  Quads '}'` (§19.8 productions [38], [39], [48]).
- **Spec target:** W3C SPARQL 1.1 Update
  <https://www.w3.org/TR/sparql11-update/#insertData>
  <https://www.w3.org/TR/sparql11-update/#deleteData>.
- **Status:** active.
- **Author:** `fe-phase-c-sparql`.
- **Date:** 2026-04-19.
- **Adversary reference:** Failure Mode 8 in
  `docs/verification/adversary-findings/sparql.md`.

## Ambiguous clause

SPARQL 1.1 Update §3.1.1 says:

> "The INSERT DATA operation adds some triples, given inline in the
> request, into the Graph Store. … Variables and blank nodes in
> patterns are not permitted in the INSERT DATA operation … Blank
> node labels are scoped to the request."

And §3.1.2:

> "The DELETE DATA operation removes some triples, given inline in
> the request, if the respective graphs contain those triples. …
> Variables and blank nodes in patterns are not permitted in the
> DELETE DATA operation."

The two are almost identical, but differ on blank nodes: INSERT DATA
allows them (they are freshly minted in the target graph, scoped to
the request); DELETE DATA rejects them because delete-by-blank-node-
reference has no well-defined semantics (blank node identity is not
shared across the request boundary).

## Reading chosen

At parse time, the parser enters a `data-block` mode when it sees
`INSERT DATA` or `DELETE DATA`. In that mode:

- Any `?var` or `$var` token in the quads body raises
  `SPARQL-UPDATE-001` (fatal).
- In DELETE DATA additionally, any `_:label` or `[]` anonymous blank
  node raises `SPARQL-UPDATE-001` (fatal).
- In INSERT DATA, blank nodes are accepted; multiple occurrences of
  the same `_:label` within one INSERT DATA refer to the same node.

The parser does not enforce the "scoped to the request" identity
rule at grammar level — that is an evaluator concern. It merely
preserves label text verbatim so the evaluator sees the author's
intent.

## Rationale

- §3.1.1 / §3.1.2 are prescriptive about variables.
- §3.1.2's specific blank-node prohibition is a known divergence
  hotspot (adversary FM8).
- The `within-one-operation same label = same node` property
  requires the encoder to preserve labels; any canonicalisation
  should happen post-parse.

## Non-goals

- This pin does NOT cover `DELETE WHERE` (§3.1.3), which allows
  patterns but has its own BNode constraints handled elsewhere.
- It does NOT cover the Modify operation's WHERE semantics.

## Diagnostic code

- **Code:** `SPARQL-UPDATE-001`
- **Emitted by:** `sparql-syntax`.
- **Message template:**
  - `SPARQL-UPDATE-001: variables are forbidden inside INSERT DATA /
    DELETE DATA (§3.1.1 / §3.1.2) at byte N`
  - `SPARQL-UPDATE-001: blank nodes are forbidden inside DELETE DATA
    (§3.1.2) at byte N`
- **Fatal?** Yes.
