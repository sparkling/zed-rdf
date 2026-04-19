# Adversary Brief: N-Triples / N-Quads

Cohort: verification-v1-adv (cohort B)
Format: N-Triples (NT) and N-Quads (NQ)
Spec references: W3C RDF 1.1 N-Triples https://www.w3.org/TR/n-triples/
                 W3C RDF 1.2 N-Triples (draft) https://www.w3.org/TR/rdf12-n-triples/
                 RDF 1.1 N-Quads https://www.w3.org/TR/n-quads/
Errata/ML: W3C public-rdf-comments mailing list; RDF 1.2 WG errata tracker (none published as of April 2026 for NT specifically, but see divergence notes below).

---

## Failure Mode 1: EOL handling — CR, LF, CRLF

Spec: NT §2 grammar, production `EOL ::= [#xD#xA]+ ` (RDF 1.1) vs the RDF 1.2 draft which clarifies `[#xD#xA]+` can match bare CR.

Sketch:
```
<s> <p> "o" .\r<s2> <p2> "o2" .\n
```

Divergence hypothesis: An implementation that only accepts `\n` as a line terminator will reject bare `\r` or `\r\n` sequences that are technically permitted. Conversely, an implementation accepting any `[#xD#xA]+` run may concatenate two logical lines if it treats the sequence as a single EOL. This is a classic source of off-by-one in token boundaries.

Rationale: Section 2 grammar says EOL is `[#xD#xA]+`; a `+` quantifier means `\r\n` is one EOL, and two `\n\n` are also one EOL (blank line). Implementations that split on exactly one newline character may mis-count lines or reject valid files.

---

## Failure Mode 2: IRI vs relative IRI prohibition

Spec: NT §2.1: "IRIs in N-Triples must be absolute IRIs." No base IRI resolution is defined.

Sketch:
```
<http://example/s> <p> <o> .
```
where `<p>` has no scheme.

Divergence hypothesis: A parser that silently resolves relative IRIs against a default base (e.g. `file:///`) will accept the above without error, producing a different IRI than one that rejects it. The implementing hive may borrow IRI-resolution logic from Turtle and accidentally apply it in the NT parser path.

Rationale: NT is explicitly a no-base-IRI format. The grammar production `IRIREF` in NT requires an absolute IRI; any colon-less token in angle brackets is a parse error.

---

## Failure Mode 3: Unicode escape case-sensitivity

Spec: NT §2.3: `UCHAR ::= '\u' HEX HEX HEX HEX | '\U' HEX HEX HEX HEX HEX HEX HEX HEX`. `HEX ::= [0-9] | [A-F] | [a-f]`.

Sketch:
```
<http://example/s> <http://example/p> "\u00E9" .
<http://example/s> <http://example/p> "\u00e9" .
```

Divergence hypothesis: Both lines are valid (hex is case-insensitive per the grammar) and must produce the same RDF literal value (U+00E9, LATIN SMALL LETTER E WITH ACUTE). An implementation that stores the raw escape rather than decoding it will treat them as distinct literals. An implementation that normalizes only to uppercase or only to lowercase during comparison will break round-tripping.

Rationale: The HEX production is case-insensitive; decoding to the Unicode code point is mandatory, not optional.

---

## Failure Mode 4: Blank node label restrictions

Spec: NT §2.2: `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`. Notably, a label must NOT end in `.` and must NOT contain certain Unicode categories outside PN_CHARS.

Sketch:
```
_:b.1 <http://example/p> <http://example/o> .
_:b1. <http://example/p> <http://example/o> .
```

Divergence hypothesis: `_:b.1` is valid (`.` in the middle is allowed by `PN_CHARS*`). `_:b1.` is invalid (the label ends in `.` which the grammar forbids via the trailing `PN_CHARS` constraint). An implementation using a greedy regex like `[A-Za-z0-9._-]+` will accept both, producing a blank node whose label includes a trailing period that should have been treated as statement-terminator punctuation.

Rationale: The trailing-dot restriction is a known sharp edge shared with Turtle; it is easy to overlook when writing the NT parser independently.

---

## Failure Mode 5: Literal datatype IRI must be absolute

Spec: NT §2.4: datatype IRIs in typed literals must be absolute IRIs (same constraint as subject/predicate/object IRIs).

Sketch:
```
<http://example/s> <http://example/p> "42"^^<integer> .
```

Divergence hypothesis: A parser that only validates subject/predicate/object IRIs for absoluteness, but skips the datatype IRI, will accept the above. The implementing hive may validate datatype IRIs in a separate code path that re-uses Turtle's (more permissive) IRI handling.

Rationale: The absoluteness constraint applies uniformly to all IRIREF tokens in NT; the grammar is identical in all positions.

---

## Failure Mode 6: Language tag case normalization (RDF 1.2 change)

Spec: RDF 1.1 NT §2.4 required language tags to be well-formed BCP47 but did NOT mandate case normalization. RDF 1.2 (draft, §2.4) mandates that language tags MUST be stored in canonical lower-case form.

Sketch:
```
<http://example/s> <http://example/p> "Hello"@EN .
<http://example/s> <http://example/p> "Hello"@en .
```

Divergence hypothesis: Under RDF 1.1, these are distinct literals (different language tags). Under RDF 1.2, they may be required to normalize to `@en`, making them equal. An implementing hive targeting 1.1 semantics but using 1.2 grammar rules (or vice versa) will produce wrong equality decisions. The errata and WG discussion on this point were contentious; see public-rdf-comments thread "Language tag case" (2023).

Errata reference: W3C RDF 1.2 Working Group issue tracker, issue #rdf-canon-langtag (no formal errata number assigned as of April 2026; WG resolution recorded in minutes 2024-06-12).

---

## Failure Mode 7: Comment handling at end-of-file

Spec: NT §2 grammar allows a comment (`#` to end-of-line) as an optional suffix on any line, including the last line. A file may legally end with a comment and no trailing newline.

Sketch (no trailing newline after the comment):
```
<s> <p> <o> . # terminal comment
```
(file ends immediately after `t`)

Divergence hypothesis: A parser that requires a final `\n` after the last triple will reject this valid file. A parser that reads until EOF and treats the absence of EOL as an error in comment parsing will also fail. The NT grammar's production for `ntriplesDoc` is `triple? (EOL triple?)* EOL?`; the final `EOL?` makes the trailing newline optional.

Rationale: Many hand-written parsers implement the "read a line, parse it" pattern and fail when the final line has no newline; the spec explicitly makes the trailing EOL optional.

---

## Summary of Divergence Hypotheses

| # | Area | Likely miss |
|---|------|-------------|
| 1 | EOL variants | Reject bare CR or miscount lines |
| 2 | Relative IRIs in NT | Silent resolution instead of error |
| 3 | Unicode escape case | Store raw instead of decode; treat as distinct |
| 4 | Blank node trailing dot | Greedy label regex includes punctuation |
| 5 | Datatype IRI absoluteness | Skip validation in datatype branch |
| 6 | Language tag case (1.1 vs 1.2) | Wrong equality under version mismatch |
| 7 | Final comment with no EOL | Require trailing newline, reject valid file |
