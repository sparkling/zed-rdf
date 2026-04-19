# Pin: Turtle directive terminator `.`

- **Diagnostic code:** `TTL-DIR-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Production:** `prefixID ::= '@prefix' PNAME_NS IRIREF '.'`;
  `base ::= '@base' IRIREF '.'` (§6.5 "Grammar").
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 Turtle §6.5:

> `prefixID ::= '@prefix' PNAME_NS IRIREF '.'`
> `base ::= '@base' IRIREF '.'`

The grammar requires every `@prefix` and `@base` directive to end
with `.` (U+002E FULL STOP). §4 "Turtle Grammar" also admits the
SPARQL-style directives `PREFIX` and `BASE` which are **not**
terminated by `.`; this pin covers the `@`-prefixed Turtle-native
forms only. The SPARQL-style forms are dispatched at a different
lexer branch.

## Reading chosen

Pure grammar reading. `TTL-DIR-001` is emitted when an `@prefix` or
`@base` directive is parsed to completion of its `IRIREF` argument
but the next significant token is not `.`. No ambiguity; the
reading is the W3C grammar as-is.

## Rationale

The `.` terminator is the one place Turtle's two directive families
(`@prefix` vs `PREFIX`) visibly diverge at the grammar level; a
distinct code catches the "user meant SPARQL-style but wrote
`@prefix`" confusion class without bundling it into
`TTL-SYNTAX-001`. W3C negative tests `turtle-syntax-bad-prefix-*`
exercise the branch.

## Diagnostic code

- **Code:** `TTL-DIR-001`
- **Emitted by:** `rdf-turtle` parser (see
  `crates/rdf-turtle/src/diag.rs:50` for the code definition).
- **Message template:**
  `TTL-DIR-001: expected '.' to terminate @prefix/@base directive at byte <N>`.
- **Fatal?** Yes.
