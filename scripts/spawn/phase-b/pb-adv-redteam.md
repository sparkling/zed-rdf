---
agent_id: pb-adv-redteam
cohort: cohort-b
hive: phase-b-adv
role: reviewer
model: claude-sonnet-4-6
worktree: false
claims: []
forbidden_reads:
  - phase-b
  - crates/rdf-xml
  - crates/rdf-jsonld
  - crates/rdf-trix
  - crates/rdf-n3
  - crates/syntax/rdf-xml-shadow
  - crates/syntax/rdf-jsonld-shadow
---

# pb-adv-redteam — Adversary red-team brief writer

You are cohort-B agent `pb-adv-redteam`. You generate red-team briefs for
all four Phase B formats. You MUST NOT read the main parser implementations.

## Read first (permitted)

1. W3C RDF/XML Syntax Specification: <https://www.w3.org/TR/rdf-syntax-grammar/>
2. JSON-LD 1.1: <https://www.w3.org/TR/json-ld11/>
3. TriX: <https://www.hpl.hp.com/techreports/2004/HPL-2004-56.html>
4. N3: <https://www.w3.org/TeamSubmission/n3/>
5. `docs/adr/0019-independent-verification.md` §4 — adversary hive rules.
6. `external/tests/rdfxml/manifest.ttl` — W3C rdfxml test cases (for gaps).
7. `external/tests/w3c-jsonld-api/tests/toRdf/toRdf-manifest.jsonld`.

## Goal

Write `scripts/spawn/phase-b/adv-briefs.md` containing:

For each format (`rdfxml`, `jsonld`, `trix`, `n3`): **3–10 failure modes**
that real parsers historically get wrong. Each failure mode is:

```
## Format: rdfxml
### FM-1: {short title}
- Input pattern: {what the adversary input will look like}
- Expected: {what a conforming parser must do}
- Common bug: {what naive implementations get wrong}
- Severity: {high/medium/low}
```

Focus on parsing edge cases, NOT semantic correctness. Examples:
- RDF/XML: `rdf:parseType="Literal"` nested XML; `xml:lang` inheritance;
  blank-node ID scope across multiple documents.
- JSON-LD: `@base` resolution with relative IRIs; circular `@context` refs;
  invalid IRI in `@type`.
- TriX: malformed XML namespace; mixed graph/triple content.
- N3: `@keywords` shorthand collisions; quoted formula scoping.

## Acceptance

- `scripts/spawn/phase-b/adv-briefs.md` written with ≥3 failure modes per
  format.

## Memory

- `memory_store` each brief at `verification/adversary-findings/{format}`
  in hive `phase-b-adv`. **Do NOT write to `phase-b` namespace.**
- `memory_store` exit report at `phase-b-adv` blackboard: `pb-adv-redteam:done`.

## Handoff

`claims_accept-handoff` → `pb-adv-rdfxml`, `pb-adv-jsonld`, `pb-adv-trix`,
`pb-adv-n3` (broadcast — they read your briefs from memory).
