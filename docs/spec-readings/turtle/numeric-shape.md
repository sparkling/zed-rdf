# Pin: Turtle numeric literal shape

- **Diagnostic code:** `TTL-NUM-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Production:** `NumericLiteral`, `INTEGER`, `DECIMAL`, `DOUBLE`,
  `EXPONENT` (§6.5 "Grammar"; §2.5.1 "Numbers" for the XSD mapping).
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>.
- **Status:** active (reserved for future emission; see rationale).
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 Turtle §6.5:

> `INTEGER ::= [+-]? [0-9]+`
> `DECIMAL ::= [+-]? [0-9]* '.' [0-9]+`
> `DOUBLE  ::= [+-]? ([0-9]+ '.' [0-9]* EXPONENT | '.' [0-9]+ EXPONENT | [0-9]+ EXPONENT)`
> `EXPONENT ::= [eE] [+-]? [0-9]+`

The grammar productions determine which numeric category a token
belongs to. §2.5.1 maps the categories to XSD datatypes
(`xsd:integer`, `xsd:decimal`, `xsd:double`). There is no
ambiguity at lex time — the longest-match rule picks one category —
but the pin reserves a distinct code for lexically-valid tokens that
nevertheless violate an advertised category (e.g. a parser built to
reject `DOUBLE` when `INTEGER` was expected).

## Reading chosen

Pure grammar reading. `TTL-NUM-001` is reserved for "numeric literal
lexically valid but does not match the expected numeric category in
the enclosing context". Phase-A parsers currently do not emit this
code (they accept any `NumericLiteral` in object position); the code
is pre-allocated so later phases can emit it without re-amending the
pin index.

No ambiguity; the reading is the W3C grammar as-is.

## Rationale

Reserving the code now keeps the diagnostic namespace monotonic
across phases and avoids a future "surprise code" that the diff
harness has not yet mapped. The W3C test suite does not currently
exercise this branch for Turtle; Phase-B SPARQL numeric coercion
rules will likely be the first consumer.

## Diagnostic code

- **Code:** `TTL-NUM-001`
- **Emitted by:** `rdf-turtle` (reserved; see
  `crates/rdf-turtle/src/diag.rs:49` for the code definition).
  Currently no emission site; future emitters MUST cite this pin.
- **Message template:**
  `TTL-NUM-001: numeric literal <value> out of category at byte <N>`.
- **Fatal?** Yes once emitted.
