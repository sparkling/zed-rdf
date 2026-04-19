# Pin: N-Triples literal escapes — `\uXXXX` / `\UXXXXXXXX` hex case

- **Diagnostic code:** `NT-LITESC-001`
- **Language / format:** N-Triples (RDF 1.1) and N-Quads.
- **Production:** `UCHAR`, `STRING_LITERAL_QUOTE`, `ECHAR` (§2.3 and
  §2.4 of the RDF 1.1 N-Triples Recommendation).
- **Spec target:** RDF 1.1 N-Triples
  <https://www.w3.org/TR/n-triples/>; forward-compatibility note for
  RDF 1.2 draft <https://www.w3.org/TR/rdf12-n-triples/> (no conflict
  on this production).
- **Status:** active.
- **Author:** `v1-specpins` (cohort A).
- **Date:** 2026-04-19.

## Ambiguous clause

From RDF 1.1 N-Triples §2.3 "Grammar":

> `UCHAR ::= '\u' HEX HEX HEX HEX | '\U' HEX HEX HEX HEX HEX HEX HEX HEX`
> `HEX ::= [0-9] | [A-F] | [a-f]`

From §2.4 "Literals":

> "String literals are surrounded by double quote characters `"` and
> MAY contain ECHAR or UCHAR escape sequences."

The grammar is unambiguous that hex digits are case-insensitive for
**lexing** (the `HEX` production accepts both cases). It is silent on
whether the parser must **decode** the escape to a Unicode scalar
value before producing the canonical literal, or whether it may keep
the lexical escape in the stored literal. Two readings of "the
literal's value" are therefore superficially plausible:

- **Lexical-preserving:** treat `"\u00E9"` and `"\u00e9"` as two
  distinct RDF literals (different strings), matching the
  byte-for-byte equality posture RDF 1.1 Concepts §3.3 takes for
  literal simple strings.
- **Decode-at-parse:** treat both as the single-code-point string
  `U+00E9`, equal to the unescaped literal `"é"`.

## Reading chosen

**Decode-at-parse.** The parser MUST decode every `UCHAR` escape to
its Unicode scalar value before emitting the literal, and MUST treat
hex digits as case-insensitive during decoding. After decoding,
`"\u00E9"`, `"\u00e9"`, and `"é"` all denote the same simple literal
with lexical form `"é"` (a single USV `U+00E9`).

Consequences the parser must honour:

1. Post-decode, the `Fact::object` carries the decoded USV string;
   the original escape sequence is **not** preserved in canonical
   facts (it is lost, intentionally).
2. Surrogate code points decoded from a `UCHAR` escape (any
   `\uD800`–`\uDFFF` or `\U0000D800`–`\U0000DFFF`) are a parse error;
   the input is not a valid RDF 1.1 N-Triples document.
3. The encoding of the input file is fixed to UTF-8 by §2; the
   parser does **not** accept UTF-16 or any other encoding, and does
   not use `UCHAR` as an escape hatch for them.

## Rationale

- RDF 1.1 Concepts §3.3 defines a simple literal's lexical form as a
  Unicode string, not a byte sequence of escape characters. A parser
  that preserves the `\uXXXX` bytes would produce a literal whose
  lexical form is the six-character string `\u00E9`, not `é` — that
  contradicts §5.1 "Literals" which lists the lexical form as a
  Unicode string.
- The N-Triples test suite
  (<https://www.w3.org/2013/TurtleTests/>, shared grammar family)
  includes positive tests where a `UCHAR` and its decoded character
  must be equal (e.g. `literal_with_UTF8_boundaries.nt` and
  `literal_with_numeric_escape4.nt`). A lexical-preserving
  implementation would fail them.
- The surrogate rejection follows from RDF 1.1 Concepts §3.3 which
  requires the lexical form to be a Unicode string (i.e. a sequence
  of Unicode scalar values, and surrogates are not scalar values per
  Unicode 3.9). RFC 3987 errata ID 3937 affirms the same prohibition
  for IRIs; the literal side inherits the spirit.
- The cohort-B adversary brief `docs/verification/adversary-findings/nt.md`
  Failure Mode 3 calls out the case-sensitivity trap explicitly; the
  veto log marks it open pending this pin.

## Diagnostic code

- **Code:** `NT-LITESC-001`
- **Emitted by:** `rdf-ntriples`, `rdf-ntriples-shadow`, `oxttl`
  oracle adapter (when an oracle disagrees with the chosen reading
  the diff report will carry this code so the pin is the first hit
  in triage).
- **Message template:**
  `NT-LITESC-001: invalid UCHAR escape at byte <offset>: <detail>`
  or, for suspected divergence, `NT-LITESC-001: UCHAR hex-case
  decode divergence, see docs/spec-readings/ntriples/literal-escapes.md`.
- **Fatal?** Yes for surrogate decode or malformed escape; the code
  may also appear as a non-fatal warning prefix when a parser
  chooses to log the decoded-vs-lexical collapse.

## Forward references

- `crates/syntax/rdf-ntriples/SPEC.md` — TODO: add "Pinned readings"
  section citing `NT-LITESC-001` when Phase-A author lands that file.
- `crates/syntax/rdf-ntriples-shadow/src/lib.rs` — must emit the same
  code; verified by snapshot test `snapshot_ntriples_shadow_vs_main_smoke`
  in `crates/testing/rdf-diff/tests/snapshots.rs`.
- Adversary fixtures exercising this pin live under
  `tests/adversary-ntriples/` (owned by cohort B); filenames should
  carry the `nt-litesc-001` slug.
