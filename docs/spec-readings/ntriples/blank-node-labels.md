# Pin: N-Triples blank-node label lexical syntax

- **Diagnostic codes:** `NT-BN-001`, `NT-BN-002`
- **Language / format:** N-Triples (RDF 1.1) and N-Quads.
- **Production:** `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9])
  ((PN_CHARS | '.')* PN_CHARS)?` (§2.3 "Grammar").
- **Spec target:** RDF 1.1 N-Triples
  <https://www.w3.org/TR/n-triples/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 N-Triples §2.3:

> `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`

Two small lexical decisions admit only one reading but are worth
pinning because they are the classic confusion points:

1. The `_:` prefix is mandatory; the grammar has no fallback.
2. The first label character after `_:` must match
   `PN_CHARS_U | [0-9]`; the label itself may be as short as one
   character but must not be empty.

## Reading chosen

Pure grammar reading:

- **`NT-BN-001`** — any label position that does not start with the
  two-byte prefix `_:` is rejected.
- **`NT-BN-002`** — after `_:`, the parser demands at least one
  character and it must satisfy `PN_CHARS_U | [0-9]`. An empty label
  (input ends at the `:` or transitions to whitespace) is rejected,
  as is a first character from any other set (e.g. `-`, a punctuator,
  or a character from `PN_CHARS_BASE`'s complement).
- **`NT-BN-002` (colon sub-clause)** — in N-Triples / N-Quads the
  `PN_CHARS_U` production is `PN_CHARS_BASE | '_'` (no `:`). This
  differs from Turtle, where `PN_CHARS_U` additionally permits `:`
  because Turtle has prefixed names. A `:` is therefore rejected both
  at the first-character position (`_::a` — W3C test
  `nt-syntax-bad-bnode-01`) and at any middle / trailing position
  (`_:abc:def` — W3C test `nt-syntax-bad-bnode-02`). The W3C
  manifest comment is verbatim: "Colon in bnode label not allowed
  (negative test)".

No ambiguity; the reading is the W3C grammar as-is.

## Rationale

- The `PN_CHARS_U` class is the Turtle-family "Unicode-aware
  identifier start" set, explicitly allowing `_` and the underscore-
  plus-digit opener. Accepting `-` or `.` as the first character
  would widen the grammar.
- Splitting the "prefix missing" case (001) from the "content
  invalid" case (002) lets a divergence report distinguish "parser
  never recognised the `_:` token" from "parser recognised the
  prefix but disagreed on the label contents".
- Tests: `nt-syntax-bad-uri-09` and the positive
  `nt-syntax-bnode-01`..`03` fixtures.

## Diagnostic codes

- **`NT-BN-001`** — emitted at
  `crates/rdf-ntriples/src/lib.rs:453`.
  Template: `NT-BN-001: expected '_:' at byte offset <N>`.
- **`NT-BN-002`** — emitted at
  `crates/rdf-ntriples/src/lib.rs:476` (illegal first character) and
  `:496` (empty label).
  Templates: `NT-BN-002: illegal first character in blank-node label at byte offset <N>` /
  `NT-BN-002: empty blank-node label at byte offset <N>`.
- **Fatal?** Both fatal.
