# Pin: N-Triples subject position accepts IRI or blank node only

- **Diagnostic code:** `NT-SUBJ-001`
- **Language / format:** N-Triples (RDF 1.1) and N-Quads.
- **Production:** `subject ::= IRIREF | BLANK_NODE_LABEL` (§2.3
  "Grammar").
- **Spec target:** RDF 1.1 N-Triples
  <https://www.w3.org/TR/n-triples/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 N-Triples §2.3 "Grammar":

> `subject ::= IRIREF | BLANK_NODE_LABEL`

No ambiguity: only an `IRIREF` (`<...>`) or a `BLANK_NODE_LABEL`
(`_:foo`) is valid at subject position. Literals, variables, `(`,
or anything else at subject position are parse errors.

## Reading chosen

Pure grammar reading. The parser rejects any token at subject
position that is not an `IRIREF` start (`<`) or a blank-node start
(`_:`). EOF at subject position is a fatal error. No ambiguity; the
reading is the W3C grammar as-is.

## Rationale

The N-Triples grammar is strict: unlike Turtle, subjects may not be
collections, and literals are never valid subjects under RDF 1.1
(RDF-star triples are a separate extension, not in scope for
Phase A). W3C negative tests `nt-syntax-bad-uri-*` and similar
cover the happy path of this rejection.

## Diagnostic code

- **Code:** `NT-SUBJ-001`
- **Emitted by:** `rdf-ntriples` at
  `crates/rdf-ntriples/src/lib.rs:358` (unexpected token at subject
  position) and `:363` (EOF at subject position).
- **Message template:**
  `NT-SUBJ-001: expected IRI or blank node at byte offset <N>, found <token>`
  or `NT-SUBJ-001: unexpected EOF at byte offset <N>`.
- **Fatal?** Yes.
