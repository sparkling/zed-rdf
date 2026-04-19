# Pin: N-Triples object position accepts IRI, blank node, or literal

- **Diagnostic code:** `NT-OBJ-001`
- **Language / format:** N-Triples (RDF 1.1) and N-Quads.
- **Production:** `object ::= IRIREF | BLANK_NODE_LABEL | literal`
  (§2.3 "Grammar").
- **Spec target:** RDF 1.1 N-Triples
  <https://www.w3.org/TR/n-triples/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 N-Triples §2.3 "Grammar":

> `object ::= IRIREF | BLANK_NODE_LABEL | literal`

No ambiguity: object position accepts exactly `<IRI>`, `_:label`, or
a `"..."` literal (optionally with language tag or datatype).

## Reading chosen

Pure grammar reading. The parser rejects any token at object
position that is not an `IRIREF` start (`<`), a blank-node start
(`_:`), or a literal start (`"`). EOF at object position is a fatal
error. No ambiguity; the reading is the W3C grammar as-is.

## Rationale

W3C negative tests such as `nt-syntax-bad-base-01` and the positive
test corpus exhaustively cover the three admitted object forms;
divergence from this reading would mean accepting tokens (bare
identifiers, variables, parentheses) that the grammar explicitly
excludes.

## Diagnostic code

- **Code:** `NT-OBJ-001`
- **Emitted by:** `rdf-ntriples` at
  `crates/rdf-ntriples/src/lib.rs:375` (unexpected token at object
  position) and `:380` (EOF at object position).
- **Message template:**
  `NT-OBJ-001: expected IRI, blank node, or literal at byte offset <N>, found <token>`
  or `NT-OBJ-001: unexpected EOF at byte offset <N>`.
- **Fatal?** Yes.
