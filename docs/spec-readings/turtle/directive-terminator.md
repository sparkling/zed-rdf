# Pin: Turtle directive terminator `.`

- **Diagnostic code:** `TTL-DIR-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Productions:**
  - `prefixID ::= '@prefix' PNAME_NS IRIREF '.'`
  - `base ::= '@base' IRIREF '.'`
  - `sparqlPrefix ::= "PREFIX" PNAME_NS IRIREF` (no `.`)
  - `sparqlBase ::= "BASE" IRIREF` (no `.`)
  - `directive ::= prefixID | base | sparqlPrefix | sparqlBase` (§6.5
    "Grammar").
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>.
- **Status:** active — strict reading in force since 2026-04-19.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 Turtle §6.5:

> `prefixID ::= '@prefix' PNAME_NS IRIREF '.'`
> `base ::= '@base' IRIREF '.'`
> `sparqlPrefix ::= "PREFIX" PNAME_NS IRIREF`
> `sparqlBase ::= "BASE" IRIREF`

The grammar requires every `@prefix` / `@base` directive to end
with `.` (U+002E FULL STOP). The SPARQL-style `PREFIX` / `BASE`
productions **do not** include a `.` terminator — a `.` appearing
immediately after `PREFIX <iri>` or `BASE <iri>` is therefore a
syntax error, not a legal termination.

## Reading chosen

Pure grammar reading, both directions:

- `TTL-DIR-001` is emitted when an `@prefix` or `@base` directive is
  parsed to completion of its `IRIREF` argument but the next
  significant token is **not** `.`.
- `TTL-DIR-001` is also emitted when a SPARQL-style `PREFIX` or
  `BASE` directive is parsed to completion of its `IRIREF` argument
  and the next significant token **is** `.` — the stray dot belongs
  to no legal production at that position.

The in-repo adversary fixture
`crates/testing/rdf-diff/tests/adversary-ttl/fm6-base-directive-replacement.ttl`
was originally authored with a stray `.` after the SPARQL-style
`BASE <…>` (i.e. `BASE <http://example/b/> .`). Because the
fixture's grammar claim is about directive *replacement* — not the
terminator — it was amended to drop the stray dot when this pin was
tightened to the strict reading (2026-04-19). W3C negative tests
`turtle-syntax-bad-base-03` / `turtle-syntax-bad-prefix-05` (and
the TriG peers) pin the strict rejection.

## Rationale

The `.` terminator is the one place Turtle's two directive families
(`@prefix` vs `PREFIX`) visibly diverge at the grammar level. The
strict reading in both directions keeps the accept/reject split
aligned with the W3C test corpus and avoids a tolerance knob that
would otherwise force an entry on the verification-v1 allow-list.

## Diagnostic code

- **Code:** `TTL-DIR-001`
- **Emitted by:** `rdf-turtle` parser (see
  `crates/rdf-turtle/src/diag.rs` for the code definition and
  `crates/rdf-turtle/src/grammar.rs` → `directive_prefix` /
  `directive_base` / `reject_dot` for the enforcement sites).
- **Message templates:**
  - `TTL-DIR-001: directive not terminated with '.'` — `@prefix` /
    `@base` missing the required dot.
  - `TTL-DIR-001: SPARQL-style PREFIX/BASE directive must not end
    with '.'` — stray dot after a SPARQL-style directive.
- **Fatal?** Yes.
