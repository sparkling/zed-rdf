# Adversary Brief: Turtle / TriG

Cohort: verification-v1-adv (cohort B)
Format: Turtle, TriG
Spec references: W3C RDF 1.1 Turtle https://www.w3.org/TR/turtle/
                 W3C RDF 1.1 TriG https://www.w3.org/TR/trig/
                 RDF 1.2 Turtle draft https://www.w3.org/TR/rdf12-turtle/
Errata: W3C Turtle errata https://www.w3.org/2001/sw/wiki/Turtle/Errata
        Turtle test suite errata (W3C GitHub w3c/rdf-tests, issues #90, #115, #152, #200)
        public-rdf-comments: "Re: Turtle prefix name edge cases" (2013, 2022 revisit)

---

## Failure Mode 1: Prefix name allows leading digit in local part

Spec: Turtle §6.3 grammar, `PN_LOCAL ::= (PN_CHARS_U | ':' | [0-9] | PLX) ((PN_CHARS | '.' | ':' | PLX)* (PN_CHARS | ':' | PLX))?`. The local part MAY start with a digit.

Sketch:
```turtle
@prefix ex: <http://example/> .
ex:123 ex:p ex:o .
```

Divergence hypothesis: An implementation that follows XML NCName rules (which forbid a leading digit) will reject `ex:123` as a syntactically invalid prefixed name. The Turtle grammar deliberately deviates from XML here. This is a common trap for implementers who reuse XML namespace libraries.

Rationale: The Turtle grammar explicitly allows `[0-9]` at the start of `PN_LOCAL`. W3C test manifest includes `turtle-syntax-pname-esc-01` through `_06` covering escapes; leading-digit coverage is in `turtle-syntax-number-*`. See errata note in w3c/rdf-tests issue #90.

---

## Failure Mode 2: Percent-encoding in prefixed name local parts

Spec: Turtle §6.3: `PLX ::= PERCENT | PN_LOCAL_ESC`. `PERCENT ::= '%' HEX HEX`. A percent-encoded character in a local part is NOT decoded by the Turtle parser; it is kept as-is in the resulting IRI.

Sketch:
```turtle
@prefix ex: <http://example/> .
ex:caf%C3%A9 ex:p ex:o .
```

Divergence hypothesis: An implementation that percent-decodes the local part before appending it to the prefix IRI will produce `<http://example/café>` (a non-ASCII IRI), whereas the spec requires the IRI to retain the percent-encoding: `<http://example/caf%C3%A9>`. These are different IRIs under strict comparison.

Rationale: Turtle §2.4 states that percent-encoded sequences are passed through unchanged. An IRI with and without percent-encoding of the same Unicode character are NOT equivalent in RDF (IRI equality is character-for-character).

---

## Failure Mode 3: Predicate `a` and `true`/`false` are not general keywords

Spec: Turtle §2.4: `a` is syntactic sugar for `rdf:type` only in the predicate position. `true` and `false` are boolean literal shortcuts only as object values.

Sketch:
```turtle
<http://example/s> a <http://example/C> .
<http://example/a> <http://example/p> <http://example/o> .   # 'a' as IRI local part via prefixed name is fine
<http://example/s> <http://example/p> true .
```

Divergence hypothesis: An implementation that over-eagerly tokenizes `a` as a keyword in subject or object position, or `true`/`false` in subject or predicate position, will produce incorrect parse errors or wrong node types. The grammar restricts these tokens to specific syntactic positions.

Rationale: Grammar productions `verb ::= predicate | 'a'` and `object ::= ... | BooleanLiteral` are position-sensitive; the keyword tokens must not leak into other positions. Implementations using a greedy keyword scanner before position context will misparse.

---

## Failure Mode 4: Nested collection (RDF list) blank node identity

Spec: Turtle §2.8 defines collection syntax `(e1 e2 e3)`. Each element generates a fresh blank node. Crucially, `()` (the empty collection) is `rdf:nil`, a named IRI, not a blank node.

Sketch:
```turtle
<http://example/s> <http://example/p> () .
<http://example/s> <http://example/p> (1) .
<http://example/s> <http://example/p> ((1)) .
```

Divergence hypothesis:
- `()` must produce exactly `rdf:nil` as the object; an implementation that generates a fresh blank node for empty collections is wrong.
- `(1)` must produce two triples: `_:b0 rdf:first "1"^^xsd:integer . _:b0 rdf:rest rdf:nil .` with the subject of the outer triple being `_:b0`.
- `((1))` nests a list inside a list; the outer list's `rdf:first` is another blank node that heads an inner list. Implementations that flatten or confuse the nesting will produce wrong graphs.

Errata reference: W3C rdf-tests issue #115 ("empty list should be rdf:nil not blank node").

---

## Failure Mode 5: String escape `\n` vs literal newline

Spec: Turtle §2.5.2: short string literals use `\"...\"`; long string literals use `"""..."""`. Inside short strings, a literal (unescaped) newline is a parse error. Inside long strings, it is valid content.

Sketch:
```turtle
<s> <p> "line1
line2" .
```
vs
```turtle
<s> <p> """line1
line2""" .
```

Divergence hypothesis: An implementation that uses the same lexer rule for short and long strings may accept the first form (embedding a raw newline in a short string), producing a literal that should be a parse error. Conversely, an implementation that applies short-string escaping rules inside long strings will reject valid `"""` content containing unescaped newlines.

Rationale: Turtle §6.3 grammar `STRING_LITERAL_LONG_QUOTE` and `STRING_LITERAL_QUOTE` have distinct character sets; sharing a code path between them is a common bug.

---

## Failure Mode 6: `@base` vs `BASE` directive scope and case sensitivity

Spec: Turtle §2.2: `@base` is case-sensitive and takes effect immediately. `BASE` (SPARQL-style, no `@` prefix) is also valid in Turtle 1.1. Both update the active base IRI from their point in the document onward. They are NOT cumulative — each replaces the previous base.

Sketch:
```turtle
@base <http://example/a/> .
BASE <http://example/b/> .
<rel> <p> <o> .
```

Here `<rel>` must resolve against `<http://example/b/>`, not `<http://example/a/>`.

Divergence hypothesis: An implementation that only supports `@base` and ignores `BASE` will fail to update the base IRI on the `BASE` directive, resolving `<rel>` against the wrong base. Alternatively, one that accumulates bases rather than replacing them will also produce wrong output.

Rationale: Turtle §2.2 states "The base URI is updated for each occurrence of a directive." The SPARQL-style keywords were added in RDF 1.1 Turtle; pre-1.1 code paths may not handle them.

---

## Failure Mode 7: Semicolon after last predicate-object pair

Spec: Turtle §2.5.1 grammar: `predicateObjectList ::= verb objectList (';' (verb objectList)?)* `. The trailing `?` after the second `(';' ...)` allows a bare trailing semicolon.

Sketch:
```turtle
<http://example/s> <http://example/p> <http://example/o> ; .
```

Divergence hypothesis: An implementation that requires a predicate-object pair to follow every `;` will reject this valid triple. The grammar's trailing `?` makes the additional `(verb objectList)` after `;` optional.

Errata reference: W3C rdf-tests issue #152 ("trailing semicolon in predicate-object list"); test `turtle-syntax-predicate-object-semicolon` was added to the official test suite to cover this case.

---

## Failure Mode 8: TriG default graph vs named graph blank-node scope

Spec: TriG §2.2: Blank node labels are scoped per graph. The same label `_:b` in two different graph blocks refers to two different blank nodes.

Sketch (TriG):
```trig
{ _:b <p> <o> . }
<http://example/g1> { _:b <p> <o2> . }
```

Divergence hypothesis: An implementation that scopes blank nodes per document (as in Turtle, which has a single implicit graph) rather than per graph block will unify the two `_:b` nodes, producing incorrect quads. This is a direct conflict between Turtle blank-node scoping semantics and TriG's extension.

Rationale: TriG §3 explicitly states "blank node identifiers are local to a graph." Reusing Turtle's document-level blank-node table without resetting it per graph block is a plausible implementation mistake.

---

## Failure Mode 9: Numeric literal type selection

Spec: Turtle §2.5.5: integer literals (no `.`, no exponent) are `xsd:integer`; decimal literals (`.` present, no exponent) are `xsd:decimal`; double literals (exponent present) are `xsd:double`. The production `INTEGER ::= [+-]? [0-9]+` does NOT match `1.0` (that is DECIMAL).

Sketch:
```turtle
<s> <p> 1 .
<s> <p> 1.0 .
<s> <p> 1.0e0 .
<s> <p> +1 .
<s> <p> -0 .
```

Divergence hypothesis: An implementation that parses `1.0` as integer (by stripping the fractional zero) will assign the wrong datatype. `-0` is a valid `xsd:integer` with value negative zero by xsd:integer lexical mapping (which maps it to 0); an implementation that rejects it as invalid integer syntax is wrong. `+1` must be `xsd:integer`, not `xsd:decimal`.

Rationale: The Turtle grammar uses exact token boundaries; any post-parsing "cleanup" of numeric literals risks reassigning their datatype.

---

## Summary of Divergence Hypotheses

| # | Area | Likely miss |
|---|------|-------------|
| 1 | Leading digit in local part | Reject valid pname via XML NCName rules |
| 2 | Percent-encoding in local part | Decode before IRI concatenation |
| 3 | `a`/`true`/`false` keyword scope | Over-eager keyword tokenization |
| 4 | Empty collection = rdf:nil | Generate blank node for `()` |
| 5 | Short vs long string newlines | Share lexer path, accept/reject incorrectly |
| 6 | `BASE` directive replacement | Ignore SPARQL-style BASE or accumulate bases |
| 7 | Trailing semicolon | Require predicate after `;` |
| 8 | TriG blank node scope per graph | Use document-level blank node table |
| 9 | Numeric literal type | Wrong datatype for `-0`, `1.0`, `+1` |
