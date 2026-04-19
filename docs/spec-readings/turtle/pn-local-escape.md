# Pin: Turtle `PN_LOCAL_ESC` and percent-triplet shape in prefixed names

- **Diagnostic code:** `TTL-PNLOC-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Production:** `PN_LOCAL`, `PN_LOCAL_ESC`, `PERCENT` (§6.5
  "Grammar").
- **Spec target:** RDF 1.1 Turtle
  <https://www.w3.org/TR/turtle/>.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 Turtle §6.5:

> `PN_LOCAL_ESC ::= '\' ( '_' | '~' | '.' | '-' | '!' | '$' | '&' |
> "'" | '(' | ')' | '*' | '+' | ',' | ';' | '=' | '/' | '?' | '#' |
> '@' | '%' )`
>
> `PERCENT ::= '%' HEX HEX`

The grammar explicitly enumerates the 20 characters that may follow
a backslash in a prefixed-name local part; nothing else is valid
after the `\`. Similarly, `%` must be followed by exactly two hex
digits; any other continuation is a lex error.

## Reading chosen

Pure grammar reading. `TTL-PNLOC-001` is emitted when:

1. A `\` in `PN_LOCAL` is followed by any character outside the
   enumerated 20, or
2. A `%` in `PN_LOCAL` is not followed by two hex digits
   (truncated, non-hex character, or EOF).

No ambiguity; the reading is the W3C grammar as-is.

## Rationale

The `PN_LOCAL_ESC` set is closed; accepting e.g. `\n` inside a
prefixed name would be a silent grammar extension. Positive W3C
tests `turtle-syntax-pname-esc-*` cover the 20 valid forms;
negative tests cover the rejection.

## Diagnostic code

- **Code:** `TTL-PNLOC-001`
- **Emitted by:** `rdf-turtle` lexer (see
  `crates/rdf-turtle/src/diag.rs:48` for the code definition; the
  code is raised from `lexer.rs` when `PN_LOCAL` encounters a
  malformed `\`-escape or `%`-triplet).
- **Message template:**
  `TTL-PNLOC-001: invalid PN_LOCAL escape at byte <N>: <detail>`.
- **Fatal?** Yes.
