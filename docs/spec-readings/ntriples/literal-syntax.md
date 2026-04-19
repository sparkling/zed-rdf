# Pin: N-Triples literal structural syntax

- **Diagnostic codes:** `NT-LIT-001`, `NT-LIT-002`, `NT-LIT-003`,
  `NT-LIT-004`, `NT-LIT-005`
- **Language / format:** N-Triples (RDF 1.1) and N-Quads.
- **Production:** `literal ::= STRING_LITERAL_QUOTE ('^^' IRIREF |
  LANGTAG)?` (§2.3 "Grammar"); `LANGTAG` per RFC 5646 §2.2.
- **Spec target:** RDF 1.1 N-Triples
  <https://www.w3.org/TR/n-triples/>.
- **Related pin:** `NT-LITESC-001` (UCHAR decoding and hex-case) —
  this pin covers the **structural** aspects of literal parsing; the
  sibling pin covers escape decoding.
- **Status:** active.
- **Author:** `cu-structural-pins`.
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 N-Triples §2.3:

> `STRING_LITERAL_QUOTE ::= '"' ([^#x22#x5C#xA#xD] | ECHAR | UCHAR)* '"'`
> `LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)*`

The structural readings carved out here are:

1. A literal's opening `"` must be matched by a closing `"` before
   EOF or EOL (forbidden raw-char class includes U+000A / U+000D).
2. Raw control characters U+0000..U+001F (and specifically newline
   CR/LF) are **forbidden** inside the literal; they may appear only
   via an `ECHAR` or `UCHAR` escape.
3. If `^^` follows a literal, an `IRIREF` must follow. No other
   continuation is accepted after `^^`.
4. If `@` follows a literal, the grammar requires at least one
   `[a-zA-Z]` subtag immediately; empty language tags and empty
   subtags after a `-` are rejected.
5. Any `\` inside a literal starts an `ECHAR`; unknown escape letters
   and truncated escapes are rejected.

These are all pure grammar readings, but the lexer emits distinct
codes so the diff harness can attribute divergence precisely.

## Reading chosen

Pure grammar reading, five branches:

- **`NT-LIT-001`** — unterminated literal (EOF or raw newline before
  closing `"`) is rejected.
- **`NT-LIT-002`** — raw control character (byte in `0x00..0x1F`) in
  the literal body is rejected; must be escaped.
- **`NT-LIT-003`** — datatype delimiter not of shape `^^` is
  rejected (e.g. a lone `^` or `^ ^`).
- **`NT-LIT-004`** — language tag empty (`@` followed by whitespace
  or `.`) or empty subtag after a hyphen is rejected.
- **`NT-LIT-005`** — string escape truncated at EOF, or `\<ch>` with
  `<ch>` not in the ECHAR set `{t, b, n, r, f, ", ', \\}` and not a
  `u`/`U` (those are handled by `NT-LITESC-001`), is rejected.

No ambiguity; the reading is the W3C grammar as-is.

## Rationale

The N-Triples grammar in §2.3 is fully determinate for these
branches. Splitting into five codes keeps the diagnostic signal
actionable for triage: "parser died because the string never closed"
is a fundamentally different failure from "parser died because the
datatype `^^` was malformed". The W3C test corpus
(`nt-syntax-bad-literal-01`..`03`, `nt-syntax-bad-lang-01`, etc.)
exercises each branch.

## Diagnostic codes

- **`NT-LIT-001`** — unterminated literal, emitted at
  `crates/rdf-ntriples/src/lib.rs:515`.
- **`NT-LIT-002`** — raw control character, emitted at
  `crates/rdf-ntriples/src/lib.rs:530`.
- **`NT-LIT-003`** — malformed `^^` datatype delimiter, emitted at
  `crates/rdf-ntriples/src/lib.rs:551`.
- **`NT-LIT-004`** — empty language tag / subtag, emitted at
  `crates/rdf-ntriples/src/lib.rs:583` and `:600`.
- **`NT-LIT-005`** — string escape error (truncated or unknown
  non-UCHAR escape), emitted at
  `crates/rdf-ntriples/src/lib.rs:677` and `:717`.
- **Fatal?** All five are fatal.
