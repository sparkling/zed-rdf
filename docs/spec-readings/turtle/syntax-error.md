# Pin: Turtle generic syntax rejection

- **Diagnostic code:** `TTL-SYNTAX-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Production:** any production not covered by a more specific pin;
  most commonly the `triples`, `predicateObjectList`, `collection`,
  and `subject` productions of §6.5 "Grammar".
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

Turtle §6.5 defines a recursive-descent grammar with many
alternatives at each non-terminal. When input cannot be matched by
any alternative of the expected non-terminal at the current
position, the parser must report a syntax error.

No ambiguity of interpretation here — the question is only how to
label the diagnostic. This pin fixes the residual bucket: any
grammar mismatch that does not have a more specific code
(`TTL-EOF-001`, `TTL-UNTERM-001`, `TTL-PNLOC-001`, `TTL-NUM-001`,
`TTL-DIR-001`, `TTL-BASE-001`, `TTL-PFX-001`, `TTL-LITESC-001`,
`TTL-BNPFX-001`) surfaces as `TTL-SYNTAX-001`.

## Reading chosen

Pure grammar reading. `TTL-SYNTAX-001` is the generic "input does
not match any alternative of the expected non-terminal at the
current position" code; it is emitted exactly when no more specific
code applies. No ambiguity; the reading is the W3C grammar as-is.

## Rationale

Phase-A parsers prefer specific codes where the branch is
mechanically distinguishable. `TTL-SYNTAX-001` is the fallback so
that every grammar mismatch is *codable* (the diff harness can join
on a code) without forcing every alternative into a bespoke code.
W3C negative tests such as `turtle-syntax-bad-struct-*` exercise
this bucket.

## Diagnostic code

- **Code:** `TTL-SYNTAX-001`
- **Emitted by:** `rdf-turtle` grammar and parser (see
  `crates/rdf-turtle/src/diag.rs:45` for the code definition; the
  code is produced in multiple `grammar.rs` / `parser.rs` branches
  as the generic fallback).
- **Message template:**
  `TTL-SYNTAX-001: <detail> at byte <N>`.
- **Fatal?** Yes.
