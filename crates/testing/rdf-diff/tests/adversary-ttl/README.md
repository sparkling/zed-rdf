# Adversary Turtle / TriG Fixtures

Owner: `v1-adv-ttl` (cohort B, verification-v1 sweep).
Source brief: `docs/verification/adversary-findings/ttl.md`.
ADR references: ADR-0019 §4, ADR-0020 §6.5.

These fixtures target spec-compliance edges that are likely to cause
divergence between conformant and non-conformant Turtle / TriG parsers.
Each fixture file names the failure mode (`fm<N>`) it exercises, states
the divergence hypothesis, and carries the expected parse outcome in its
leading comment block.

---

## Fixture Index

| File | Finding | Hypothesis summary |
|------|---------|-------------------|
| `fm1-leading-digit-local.ttl` | FM1 | PN_LOCAL may start with `[0-9]`; XML-NCName parsers reject it |
| `fm1-prefix-redefinition.ttl` | FM1b | `@prefix` redefinition replaces earlier binding for same label |
| `fm2-percent-encoding-local.ttl` | FM2 | Percent-encoding in local part is NOT decoded before IRI concat |
| `fm3-keyword-scope.ttl` | FM3 | `a`/`true`/`false` are position-sensitive; must not leak to other positions |
| `fm4-empty-collection.ttl` | FM4 | Empty collection `()` is `rdf:nil`, not a fresh blank node |
| `fm4-nested-collection.ttl` | FM4 | Nested `(…)` must not be flattened; `rdf:first` of `((1))` is a bnode |
| `fm5-long-string-newline.ttl` | FM5 | Raw newline in `"""…"""` is valid content (not a parse error) |
| `fm5-short-string-newline-invalid.ttl` | FM5 | Raw newline in `"…"` is a parse error; lenient parsers accept it (wrong) |
| `fm6-base-directive-replacement.ttl` | FM6 | `BASE` (SPARQL-style) replaces active base; `@base` then `BASE` uses `BASE` |
| `fm6-chained-base.ttl` | FM6b | Chained `@base` directives each replace the previous; IRI resolution uses latest |
| `fm7-trailing-semicolon.ttl` | FM7 | Trailing `;` before `.` is valid; strict parsers reject it |
| `fm8-trig-bnode-scope.trig` | FM8 | TriG blank-node labels scoped per graph block, not per document |
| `fm9-numeric-literal-types.ttl` | FM9 | Integer/decimal/double type selection by token shape; `-0`, `+1` edge cases |

---

## Per-fixture Hypotheses

### FM1 — Leading digit in PN_LOCAL (`fm1-leading-digit-local.ttl`)

Turtle §6.3 `PN_LOCAL` explicitly allows `[0-9]` as the first character.
Implementations that validate local parts against XML NCName rules (which
prohibit a leading digit) will reject `ex:123` as invalid, producing an
`AcceptRejectSplit` divergence against a conformant parser.

Errata: w3c/rdf-tests issue #90.

### FM1b — Prefix redefinition (`fm1-prefix-redefinition.ttl`)

A second `@prefix ex:` declaration replaces the first. An implementation
that caches the first binding will produce wrong absolute IRIs for any
pname after the redefinition, observable as `FactOnlyIn` divergences.

### FM2 — Percent-encoding not decoded (`fm2-percent-encoding-local.ttl`)

`PLX` in the Turtle grammar passes percent-encoded sequences through to
the final IRI unchanged. An implementation that decodes `%C3%A9` to `é`
before concatenating with the prefix IRI produces a different IRI
(`<http://example/café>` vs `<http://example/caf%C3%A9>`), detectable as
`FactOnlyIn` or `ObjectMismatch`.

Spec: Turtle §2.4.

### FM3 — Keyword scope (`fm3-keyword-scope.ttl`)

`a` is syntactic sugar for `rdf:type` only in predicate position; `true`
and `false` are boolean literal shortcuts only in object position. An
over-eager keyword scanner may mis-tokenize `<http://example/a>` (an IRI
that happens to have the local part `a`) or misparse the subject position,
yielding parse errors or wrong node types.

Spec: Turtle §2.4; grammar `verb ::= predicate | 'a'`.

### FM4 — Empty collection is rdf:nil (`fm4-empty-collection.ttl`)

`()` in Turtle denotes the empty RDF list, which is the named IRI
`rdf:nil`, **not** a fresh blank node. An implementation that generates
`_:b <rdf:rest> <rdf:nil>` (or similar) instead of using `rdf:nil`
directly as the object of the outer triple will produce a fact that no
conformant parser emits.

Errata: w3c/rdf-tests issue #115.

### FM4 — Nested collections (`fm4-nested-collection.ttl`)

`(1)` generates `_:b0 rdf:first 1 . _:b0 rdf:rest rdf:nil .` and the
outer subject is `_:b0`. For `((1))`, the outer list's `rdf:first` must
be the blank node heading the inner list — not `1` directly. Flattening
implementations will produce a structurally different graph.

Spec: Turtle §2.8.

### FM5 — Long string raw newline (`fm5-long-string-newline.ttl`)

`"""…"""` (triple-quoted) strings may contain raw (unescaped) newlines.
An implementation that applies short-string lexer rules inside `"""…"""`
will reject valid input. This file is the positive (should-parse) case.

Spec: Turtle §6.3 `STRING_LITERAL_LONG_QUOTE`.

### FM5 — Short string raw newline invalid (`fm5-short-string-newline-invalid.ttl`)

A raw newline inside `"…"` (single-quoted) is explicitly excluded by the
`STRING_LITERAL_QUOTE` grammar production. An implementation sharing a
single lexer path for both string forms will accept this erroneously.
This file is the negative (should-reject) case; divergence is
`AcceptRejectSplit`.

Spec: Turtle §6.3 `STRING_LITERAL_QUOTE`.

### FM6 — BASE directive replacement (`fm6-base-directive-replacement.ttl`)

Both `@base` and the SPARQL-style `BASE` keyword update the active base
IRI, each replacing the previous value. An implementation that ignores
`BASE` or accumulates bases will resolve `<rel>` against the wrong IRI.

Spec: Turtle §2.2; RDF 1.1 Turtle added SPARQL-style `BASE`.

### FM6b — Chained @base (`fm6-chained-base.ttl`)

Multiple sequential `@base` directives each replace the previous active
base. Relative IRIs are resolved against the base in scope at their
position in the document, not against the document's first declared base.

Spec: Turtle §2.2.

### FM7 — Trailing semicolon (`fm7-trailing-semicolon.ttl`)

The `predicateObjectList` grammar allows an optional bare `;` at the end
of a predicate-object list (before the closing `.`). An implementation
that requires a verb and objectList after every `;` will reject this
valid syntax.

Spec: Turtle §2.5.1; grammar `predicateObjectList ::= verb objectList (';' (verb objectList)?)* `.
Errata: w3c/rdf-tests issue #152.

### FM8 — TriG blank-node scope (`fm8-trig-bnode-scope.trig`)

In TriG, blank-node labels are scoped per graph block. The same label
`_:b` in two different graph blocks denotes two different blank nodes.
An implementation that uses a document-level blank-node table will unify
them, producing incorrect quads.

Spec: TriG §2.2 / §3.

### FM9 — Numeric literal types (`fm9-numeric-literal-types.ttl`)

Token shape determines XSD datatype: no dot + no exponent → `xsd:integer`;
dot present, no exponent → `xsd:decimal`; exponent present → `xsd:double`.
Edge cases: `-0` and `+1` are `xsd:integer` (not decimal). An
implementation that strips fractional zeros or misidentifies the sign-only
prefix will assign the wrong datatype.

Spec: Turtle §2.5.5; productions `INTEGER`, `DECIMAL`, `DOUBLE`.

---

## Integration with `xtask verify`

The `xtask verify adversary-ttl` sub-command (wired by `v1-ci-wiring`)
runs each `.ttl` / `.trig` file in this directory through the registered
parser ensemble and emits a `DiffReport` per fixture. The sub-command
treats any `AcceptRejectSplit` or non-empty `divergences` as a potential
finding to forward to `v1-adv-veto`.

For fixtures marked `# Expected: parse FAILS` (negative cases), the
harness inverts the pass condition: `AcceptRejectSplit` where all
reference parsers reject is the expected clean outcome; a parser that
accepts is the divergence.

See `crates/testing/rdf-diff/tests/adversary_ttl.rs` for the Rust
integration test that exercises this directory.
