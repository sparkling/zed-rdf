# Pin: Turtle literal escapes — short vs long strings, `\uXXXX` decode, newline rules

- **Diagnostic code:** `TTL-LITESC-001`
- **Language / format:** Turtle (RDF 1.1) and TriG.
- **Productions:** `STRING_LITERAL_QUOTE`, `STRING_LITERAL_SINGLE_QUOTE`,
  `STRING_LITERAL_LONG_QUOTE`, `STRING_LITERAL_LONG_SINGLE_QUOTE`,
  `ECHAR`, `UCHAR` (Turtle §6.3, §2.5.2).
- **Spec target:** RDF 1.1 Turtle <https://www.w3.org/TR/turtle/>;
  forward-compatibility note for RDF 1.2 Turtle draft
  <https://www.w3.org/TR/rdf12-turtle/>.
- **Status:** active.
- **Author:** `v1-specpins` (cohort A).
- **Date:** 2026-04-19.

## Ambiguous clause

From Turtle §2.5.2 "Literals":

> "A literal written in Turtle as a single- or double-quoted string
> MUST NOT contain an unescaped literal linefeed, line separator,
> carriage return or NEL character. Literals written as triple-quoted
> strings MAY contain such characters literally."

From Turtle §6.3 grammar:

> `STRING_LITERAL_QUOTE ::= '"' ([^#x22#x5C#xA#xD] | ECHAR | UCHAR)* '"'`
> `STRING_LITERAL_LONG_QUOTE ::= '"""' (('"' | '""')? ([^"\] | ECHAR | UCHAR))* '"""'`
> `ECHAR ::= '\' [tbnrf"'\\]`
> `UCHAR ::= '\u' HEX HEX HEX HEX | '\U' HEX HEX HEX HEX HEX HEX HEX HEX`

Three questions the plain grammar does not answer head-on:

1. Which character set exactly is forbidden as an **unescaped literal**
   inside a short string? §2.5.2 names "linefeed, line separator,
   carriage return or NEL" (U+000A, U+2028, U+000D, U+0085) but the
   grammar production only negates `#x22 #x5C #xA #xD`.
2. When both a `UCHAR` and the corresponding literal character are
   accepted, do they produce the same RDF literal value?
3. Are `\b` and `\f` mandatory for parsers to decode to U+0008 and
   U+000C, or may a parser refuse them as "unusual"?

## Reading chosen

The Turtle parser MUST implement all three of:

1. **Forbidden unescaped characters in short strings.** The parser
   rejects a short `STRING_LITERAL_QUOTE` or
   `STRING_LITERAL_SINGLE_QUOTE` that contains any of U+000A
   (LF), U+000D (CR), U+2028 (LINE SEPARATOR), or U+0085 (NEL) as a
   literal character. The grammar's `[^#x22#x5C#xA#xD]` covers LF and
   CR; LS (U+2028) and NEL (U+0085) rejection is mandated by §2.5.2
   prose and the parser MUST honour the prose, not just the negated
   character class. Long (triple-quoted) strings accept all of these
   literally.
2. **UCHAR decode equivalence.** Every `UCHAR` and `ECHAR` escape is
   decoded to its Unicode scalar value at parse time. After decode,
   `"\u00E9"`, `"\u00e9"`, `"\U000000E9"` and the literal `"é"`
   produce the same RDF literal. Surrogate code points decoded from
   `UCHAR` are a parse error (mirrors `NT-LITESC-001`).
3. **Full ECHAR table mandatory.** The parser MUST decode the full
   `ECHAR ::= '\' [tbnrf"'\\]` table, including `\b` (U+0008) and
   `\f` (U+000C). Refusing `\b`/`\f` is non-conformant; emitting them
   as literal `\b`/`\f` two-character strings is also non-conformant.

Canonical form after decode matches `NT-LITESC-001`: `Fact::object`
carries the decoded USV string; escape sequences do not survive
canonicalisation.

## Rationale

- §2.5.2 prose is normative (MUST NOT). The grammar's negated
  character class is a minimum; the prose sentence enumerates four
  characters, and RDF 1.1 Concepts §3.3 requires the lexical form to
  be a Unicode string free of line-break ambiguity for interchange.
  Accepting LS or NEL literally inside a short string creates a
  cross-implementation equality hazard that ADR-0019 §5 calls out as
  a pin-worthy ambiguity.
- The decode-equivalence reading matches `NT-LITESC-001` and the W3C
  Turtle test suite. Tests such as
  `literal_with_numeric_escape4.ttl` and
  `literal_with_UTF8_boundaries.ttl` assert equivalence across escape
  and literal forms.
- The full `ECHAR` table is listed in the grammar. A parser may not
  pick and choose which escapes to honour; the cohort-B adversary
  brief `docs/verification/adversary-findings/ttl.md` Failure Mode 5
  raises the short-vs-long lexer confusion and the veto log records
  this as blocking (`TTL-005`).
- RDF 1.2 Turtle draft §2.5.2 preserves the same prose verbatim; this
  pin carries forward without amendment when 1.2 ships.

## Diagnostic code

- **Code:** `TTL-LITESC-001`
- **Emitted by:** `rdf-turtle`, `rdf-turtle-shadow`, `oxttl` oracle
  adapter.
- **Message template:**
  `TTL-LITESC-001: unescaped <char-name> (<U+XXXX>) in short string at byte <offset>`
  or `TTL-LITESC-001: surrogate UCHAR decode <U+XXXX> at byte <offset>`
  or `TTL-LITESC-001: unknown ECHAR escape '\\<ch>' at byte <offset>`.
- **Fatal?** Yes — each of the three cases is a parse error.

## Forward references

- `crates/syntax/rdf-turtle/SPEC.md` — TODO: add "Pinned readings"
  citing `TTL-LITESC-001` and `TTL-BNPFX-001`.
- Shadow crate `crates/syntax/rdf-turtle-shadow/` must emit the same
  code; enforced by `snapshot_turtle_shadow_vs_main_smoke`.
- Adversary fixtures: `tests/adversary-turtle/ttl-litesc-001-*.ttl`
  (naming convention for cohort B).
