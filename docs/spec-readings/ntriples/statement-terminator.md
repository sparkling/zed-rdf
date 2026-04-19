# Pin: N-Triples statement terminator `.`

- **Diagnostic code:** `NT-STMT-001`
- **Language / format:** N-Triples (RDF 1.1) and N-Quads.
- **Production:** `triple ::= subject predicate object '.'` (§2.3
  "Grammar").
- **Spec target:** RDF 1.1 N-Triples
  <https://www.w3.org/TR/n-triples/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 N-Triples §2.3 "Grammar":

> `triple ::= subject predicate object '.'`

No ambiguity: every triple ends with a literal `.` (U+002E FULL STOP)
token preceded by optional whitespace. End-of-input before the
terminator is a parse error.

## Reading chosen

Pure grammar reading. The parser requires a `.` at the end of every
triple; missing terminator (either a different non-whitespace token
or EOF) is a fatal parse error. No ambiguity; the reading is the
W3C grammar as-is.

## Rationale

No competing readings in the literature. The W3C N-Triples test
suite (`nt-syntax-bad-struct-01` and friends) establishes the
reading: any statement without a final `.` is rejected. Short pin;
serves as audit trail and diff-harness reference.

## Diagnostic code

- **Code:** `NT-STMT-001`
- **Emitted by:** `rdf-ntriples` at
  `crates/rdf-ntriples/src/lib.rs:311` (missing terminator, found
  other token) and `:318` (missing terminator at EOF).
- **Message template:**
  `NT-STMT-001: expected '.' at byte offset <N>, found <token>` or
  `NT-STMT-001: expected '.' at EOF (byte offset <N>)`.
- **Fatal?** Yes.
