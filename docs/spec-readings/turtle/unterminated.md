# Pin: Turtle unterminated IRI / string / collection / graph block

- **Diagnostic code:** `TTL-UNTERM-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Production:** `IRIREF`, `STRING_LITERAL_*`, `collection`,
  TriG `wrappedGraph`. See §6.5 "Grammar".
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>; RDF 1.1 TriG
  <https://www.w3.org/TR/trig/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

Every bracketed Turtle construct has an opening token paired with a
matching closing token:

- `IRIREF`: `<` … `>`
- short `STRING_LITERAL_QUOTE`: `"` … `"` (must not cross EOL)
- long `STRING_LITERAL_LONG_QUOTE`: `"""` … `"""` (may span EOLs)
- short `STRING_LITERAL_SINGLE_QUOTE`: `'` … `'`
- long `STRING_LITERAL_LONG_SINGLE_QUOTE`: `'''` … `'''`
- `collection`: `(` … `)`
- TriG graph block: `{` … `}`

The grammar is explicit that the opener must be followed by a
closer; it does not specify recovery. EOF without the closer is a
parse error.

## Reading chosen

Pure grammar reading. `TTL-UNTERM-001` is emitted when any of the
seven openers above is seen but no matching closer is found before
EOF. No ambiguity; the reading is the W3C grammar as-is. The
byte-offset in the message points at the **opener** (aiding user
triage), not the EOF position.

## Rationale

Keeping "unterminated" distinct from generic `TTL-SYNTAX-001` lets
the diff harness tell lexer truncation failures apart from
semantic-level grammar errors. Adversary inputs from
`docs/verification/adversary-findings/turtle.md` include truncated
strings and collections as a classic divergence surface; a stable
code short-circuits the triage path.

## Diagnostic code

- **Code:** `TTL-UNTERM-001`
- **Emitted by:** `rdf-turtle` lexer (see
  `crates/rdf-turtle/src/diag.rs:47` for the code definition; the
  code is raised from `lexer.rs` where opener/closer pairing is
  tracked).
- **Message template:**
  `TTL-UNTERM-001: unterminated <construct> starting at byte <N>`.
- **Fatal?** Yes.
