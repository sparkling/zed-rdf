# Pin: N-Triples IRIREF lexical syntax

- **Diagnostic codes:** `NT-IRI-001`, `NT-IRI-002`, `NT-IRI-003`,
  `NT-IRI-004`, `NT-IRI-005`
- **Language / format:** N-Triples (RDF 1.1) and N-Quads.
- **Production:** `IRIREF ::= '<' ([^#x00-#x20<>"{}|^\`\\] | UCHAR)*
  '>'` (§2.3 "Grammar").
- **Spec target:** RDF 1.1 N-Triples
  <https://www.w3.org/TR/n-triples/>; RFC 3987 §2.2 (IRI syntax) for
  cross-reference.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 N-Triples §2.3 the `IRIREF` production forbids a fixed
set of bytes raw (the control-char block `#x00..#x20`, plus
`<`, `>`, `"`, `{`, `}`, `|`, `^`, backtick, backslash). Those code
points may only enter an IRI via a `UCHAR` escape.

N-Triples §2.4 adds that an empty IRI (`<>`) and a bare relative
reference are **not** accepted in N-Triples itself — the format
requires absolute IRIs per §3 and the surrounding grammar composed
with `IRIREF`.

The residual ambiguity is narrow and is already covered by
`IRI-PCT-001` for percent-encoding and `NT-LITESC-001` for UCHAR
decoding. The cases pinned here are the five loud-failure branches
of the lexer; they document the structural reading.

## Reading chosen

Pure grammar reading, five branches:

1. **`NT-IRI-001` — missing `<`.** An IRI position that does not
   start with `<` is rejected.
2. **`NT-IRI-002` — unterminated.** An `<` that never meets a closing
   `>` before EOF is rejected; the parser does not attempt recovery
   by scanning forward across newlines.
3. **`NT-IRI-003` — illegal raw character.** Any character in the
   forbidden set listed above that appears **unescaped** between the
   opening `<` and closing `>` is rejected. UCHAR-encoded forms of
   the same code points remain legal (and are decoded per
   `NT-LITESC-001`).
4. **`NT-IRI-004` — empty IRI.** An `<>` (zero-length IRI) is
   rejected in the main parser even though the `IRIREF` production
   permits zero characters; the RDF-term layer requires a non-empty
   absolute IRI.
5. **`NT-IRI-005` — relative IRI.** N-Triples requires absolute IRIs.
   Any `IRIREF` content that lacks a scheme (or starts with a
   hier-part marker such as `//` without a scheme) is rejected.

No ambiguity beyond the split above; the reading is the W3C grammar
composed with the N-Triples absolute-IRI requirement.

## Rationale

- The forbidden-raw-character set is taken verbatim from the ABNF
  in §2.3.
- The absolute-IRI requirement follows from §3 "IRI Terms"
  ("`IRIREF` production matches IRIs as defined in RFC 3987") and
  the semantic requirement that RDF subjects/predicates/objects be
  absolute IRIs per RDF 1.1 Concepts §3.2.
- Splitting into five codes (rather than a single catch-all) gives
  the diff harness enough signal to triage lexer divergence from
  semantic divergence without requiring a full structural diff.
- Cohort-B adversary brief `docs/verification/adversary-findings/nt.md`
  (Failure Modes around IRI lexing) motivates the explicit
  unterminated-vs-illegal-char split (codes 002 vs 003).

## Diagnostic codes

- **`NT-IRI-001`** — emitted at
  `crates/rdf-ntriples/src/lib.rs:403`.
  Template: `NT-IRI-001: expected '<' at byte offset <N>`.
- **`NT-IRI-002`** — emitted at
  `crates/rdf-ntriples/src/lib.rs:414`.
  Template: `NT-IRI-002: unterminated IRI starting at byte offset <N>`.
- **`NT-IRI-003`** — emitted at
  `crates/rdf-ntriples/src/lib.rs:435`.
  Template: `NT-IRI-003: illegal character <ch> in IRI at byte offset <N>`.
- **`NT-IRI-004`** — emitted at
  `crates/rdf-ntriples/src/lib.rs:751`.
  Template: `NT-IRI-004: empty IRI at byte offset <N>`.
- **`NT-IRI-005`** — emitted at
  `crates/rdf-ntriples/src/lib.rs:759`, `:770`, `:776`.
  Template: `NT-IRI-005: relative IRI reference at byte offset <N>`.
- **Fatal?** All five are fatal.
