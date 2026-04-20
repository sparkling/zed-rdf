# Adversary TriX fixtures

Owner: `pb-adv-trix` (cohort B, adversary hive, Phase B).
ADR references: ADR-0019 §4, ADR-0020 §6.5.

These fixtures target spec-compliance edges that are likely to cause divergence
between conformant and non-conformant TriX parsers. Each fixture names the
failure mode (`fm<N>`) it exercises, states the divergence hypothesis, and
carries the expected parse outcome in its companion `.expected` sidecar.

TriX spec: Carroll & Klyne, "Triples in XML", HP Labs Technical Report
HPL-2004-56, 2004.
URL: https://www.hpl.hp.com/techreports/2004/HPL-2004-56.html

Note: `rdf-trix` relies on layers 1 and 2 of ADR-0019 (reference-oracle
differential testing and adversary hive review); no shadow implementation is
required per ADR-0019 §3 rationale recorded in `crates/rdf-trix/SPEC.md`.

---

## Expected-file format

```
outcome: accept | reject | implementation-defined
fact-count: <N>          # ignored when outcome is reject
divergence-hypothesis: <free text>
spec-ref: <citation>
```

The `xtask verify --adversary-trix` run reads each pair and compares every
registered parser's actual outcome against the predicted one. Any parser that
diverges from the prediction (or from any peer parser) is surfaced as a
`Divergence::AcceptRejectSplit` or `Divergence::ObjectMismatch` in the
`DiffReport`. Fixtures marked `implementation-defined` produce informational
divergences only and do not block CI by themselves.

---

## Fixture index

| File | FM | Outcome | Hypothesis summary |
|------|----|---------|-------------------|
| `wrong-namespace.trix` | FM1 | reject | Incorrect namespace URI (no trailing slash) must be rejected |
| `bnode-as-graph.trix` | FM2 | impl-defined | `<bnode>` in graph-name slot is outside spec; behaviour varies |
| `triple-wrong-arity.trix` | FM3 | reject | `<triple>` with 4 children violates the exactly-3 rule |

---

## Per-fixture hypotheses

### FM1 — Wrong namespace URI (`wrong-namespace.trix`)

The canonical TriX namespace is `http://www.w3.org/2004/03/trix/trix-1/`
(with trailing slash). This document uses
`http://www.w3.org/2004/03/trix/trix-1` (without trailing slash). The two
URIs are different identifiers; a conformant namespace-aware parser must reject
the document because the root element is not in the required namespace.

A namespace-oblivious parser that matches only the local name `TriX` will
accept the document and produce 1 fact, creating an `AcceptRejectSplit`
divergence against a conformant parser.

Spec: TriX §2 — "The document element must be `<TriX>` in the namespace
`http://www.w3.org/2004/03/trix/trix-1/`."

### FM2 — Blank node as graph name (`bnode-as-graph.trix`)

TriX §3 specifies that a named graph's graph identifier is given by a `<uri>`
child element. This document places a `<bnode>` element in the graph-name
position, which is not covered by the spec. Three divergent outcomes are
plausible:

- **Reject** — strict schema validation fails; the document is rejected.
- **Accept as bnode-named graph** — the parser treats `<bnode>g1</bnode>` as
  an anonymous graph with blank-node name `_:g1` (or a fresh blank node).
- **Accept into default graph** — the parser ignores the malformed graph name
  and places the triple in the default graph.

All three constitute an `AcceptRejectSplit` or `ObjectMismatch` on the graph
field. The fixture is marked `implementation-defined`; divergence is
informational.

Spec: TriX §3 — graph element structure.

### FM3 — Wrong-arity `<triple>` (`triple-wrong-arity.trix`)

TriX §3 specifies that a `<triple>` element contains exactly three RDF-term
children (subject, predicate, object). This document provides four children.
A conformant parser must reject the document. A lenient parser may silently
discard the extra child and accept the first three, producing an
`AcceptRejectSplit` divergence.

Spec: TriX §3 — "A triple element has exactly three children."

---

## Spec references

- TriX — Triples in XML: https://www.hpl.hp.com/techreports/2004/HPL-2004-56.html
- W3C RDF 1.1 Concepts: https://www.w3.org/TR/rdf11-concepts/
