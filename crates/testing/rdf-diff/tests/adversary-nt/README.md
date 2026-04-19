# Adversary N-Triples / N-Quads fixtures

Owner: `v1-adv-nt` (cohort B, adversary hive, verification-v1 sweep).
Source brief: `docs/verification/adversary-findings/nt.md`.

Each fixture is a pair: an `.nt` or `.nq` input file and an `.expected`
side-car that states the predicted outcome plus the divergence hypothesis.

## Expected-file format

```
outcome: accept | reject | accept-with-warnings
fact-count: <N>          # ignored when outcome is reject
divergence-hypothesis: <free text>
spec-ref: <W3C spec citation>
```

The `xtask verify --adversary-nt` run reads each pair and compares every
registered parser's actual outcome against the predicted one. Any parser
that diverges from the prediction (or from any peer parser) is surfaced as
a `Divergence::AcceptRejectSplit` or `Divergence::ObjectMismatch` in the
`DiffReport`.

## Fixture index

| Fixture stem | Format | Failure mode | Hypothesis (one sentence) |
|---|---|---|---|
| `fm1-eol-bare-cr` | NT | FM1 — EOL variants | A parser that only splits on LF will reject a file whose line endings are bare CR, which the NT grammar permits via `EOL ::= [#xD#xA]+`. |
| `fm1-eol-crlf` | NT | FM1 — EOL variants | A parser that only splits on LF will reject CRLF-terminated lines, which the NT grammar explicitly permits. |
| `fm2-relative-iri-predicate` | NT | FM2 — relative IRI prohibition | A parser that silently borrows Turtle's base-IRI resolution will accept `<p>` as a predicate instead of rejecting it as the spec requires. |
| `fm2-relative-iri-graph` | NQ | FM2 — relative IRI prohibition | A parser that validates absoluteness for SPO but not the graph-name position will accept `<g>` as a graph name instead of rejecting it. |
| `fm3-unicode-escape-upper` | NT | FM3 — Unicode escape case | A parser that stores the raw escape `\u00E9` instead of decoding it to U+00E9 will diverge against a parser that decodes. |
| `fm3-unicode-escape-lower` | NT | FM3 — Unicode escape case | A parser that stores `\u00e9` raw will produce a different literal than one that decodes; both must produce U+00E9 per the case-insensitive HEX production. |
| `fm4-bnode-dot-middle` | NT | FM4 — blank-node trailing dot | `_:b.1` is a valid blank-node label (dot in middle is allowed by `PN_CHARS*`) and must be accepted with one fact. |
| `fm4-bnode-trailing-dot` | NT | FM4 — blank-node trailing dot | A greedy-regex parser will absorb the trailing `.` into the blank-node label `_:b1.` instead of treating it as the statement terminator, producing wrong output or silently mis-parsing. |
| `fm5-datatype-relative-iri` | NT | FM5 — datatype IRI absoluteness | A parser that enforces absoluteness for SPO IRIs but uses a laxer path for datatype IRIs will accept `"42"^^<integer>` instead of rejecting it. |
| `fm6-langtag-uppercase` | NT | FM6 — language-tag case (1.1 vs 1.2) | Under RDF 1.1 `"Hello"@EN` and `"Hello"@en` are distinct literals; under RDF 1.2 draft they normalise to `@en`; a parser mixing spec versions will produce wrong equality decisions. |
| `fm6-langtag-lowercase` | NT | FM6 — language-tag case (1.1 vs 1.2) | `"Hello"@en` is the canonical BCP47 form and must be accepted identically by all parsers; paired with `fm6-langtag-uppercase` to expose normalisation divergence. |
| `fm7-comment-no-final-newline` | NT | FM7 — comment at EOF with no newline | A line-oriented parser that requires a final `\n` will reject this file, which is valid because the NT grammar's `ntriplesDoc` production makes the trailing `EOL?` optional. |

## Spec references

- W3C RDF 1.1 N-Triples: <https://www.w3.org/TR/n-triples/>
- W3C RDF 1.1 N-Quads: <https://www.w3.org/TR/n-quads/>
- W3C RDF 1.2 N-Triples (draft): <https://www.w3.org/TR/rdf12-n-triples/>
- BCP47 language tags: <https://www.rfc-editor.org/rfc/rfc5646>
