# Pin: Turtle unexpected end-of-input

- **Diagnostic code:** `TTL-EOF-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Production:** any production — EOF encountered while a
  non-terminal is mid-production (§6.5 "Grammar").
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

The Turtle grammar is not explicit about EOF handling: every
production implicitly requires its terminal tokens, and EOF before
they arrive is an error. The pin distinguishes the "EOF mid-
production" branch from the broader `TTL-SYNTAX-001` fallback so the
diff harness can triage truncation / fragment cases quickly.

## Reading chosen

Pure grammar reading. `TTL-EOF-001` is emitted exactly when the
lexer or parser hits EOF while the current production still requires
at least one more token (e.g. subject with no predicate-object
list, `@prefix` directive with no IRI). No ambiguity.

## Rationale

Truncation is a common adversary case (partial-file replays) and
deserves a distinct code so the diff harness does not have to regex
a generic message. W3C negative tests such as
`turtle-syntax-bad-struct-07` cover the branch.

## Diagnostic code

- **Code:** `TTL-EOF-001`
- **Emitted by:** `rdf-turtle` parser (see
  `crates/rdf-turtle/src/diag.rs:46` for the code definition; the
  code is raised from parser entry points when the lookahead is
  EOF but the non-terminal is not complete).
- **Message template:**
  `TTL-EOF-001: unexpected end of input at byte <N>`.
- **Fatal?** Yes.
