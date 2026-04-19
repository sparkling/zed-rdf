# Adversary Veto Log — cross-namespace write (cohort B → shared)

Author: `v1-adv-veto` (cohort B)
Write permit: singular cross-namespace write per ADR-0020 §4
ADR authority: ADR-0019 §4

---

## Veto Event 2026-04-19

Agent: v1-adv-veto
Sweep: verification-v1
Timestamp: 2026-04-19

### Findings reviewed

Source: `docs/verification/adversary-findings/nt.md` (N-Triples / N-Quads brief)
Total finding hypotheses: 7

### Veto fires (4)

**NT-001** (High) — EOL handling CR/LF/CRLF
Parser: rdf-ntriples
Spec: NT §2 `EOL ::= [#xD#xA]+`
Basis: Spec is unambiguous. Bare `\r` and `\r\n` are valid terminators.
An implementation accepting only `\n` is non-conformant.
Veto: FIRED. rdf-ntriples merge blocked.

**NT-002** (High) — Relative IRI prohibition in NT context
Parser: rdf-ntriples
Spec: NT §2.1 absolute-IRI requirement
Basis: NT is a no-base-IRI format. Silent resolution via borrowed Turtle
logic is a spec violation that would corrupt parsed output silently.
Veto: FIRED. rdf-ntriples merge blocked.

**NT-004** (High) — Blank node label trailing dot
Parser: rdf-ntriples
Spec: NT §2.2 `BLANK_NODE_LABEL` grammar production
Basis: `_:b1.` is invalid; the trailing `.` is statement-terminator
punctuation not part of the label. A greedy label regex producing
wrong tokenisation is a parser correctness failure.
Veto: FIRED. rdf-ntriples merge blocked.

**NT-005** (High) — Literal datatype IRI absoluteness
Parser: rdf-ntriples
Spec: NT §2.4, IRIREF absoluteness uniform across all positions
Basis: A separate code path for datatype IRIs that applies Turtle-style
permissive resolution violates the spec's uniform absoluteness rule.
Silent acceptance of `"42"^^<integer>` is incorrect.
Veto: FIRED. rdf-ntriples merge blocked.

### Open (not vetoed at this time, but tracked)

**NT-003** (Medium) — Unicode escape case-sensitivity
Status: open — pending test evidence. Not vetoed yet; will veto if
cohort-A delivers without a failing test for this case.

**NT-006** (Medium) — Language tag case normalization (1.1 vs 1.2)
Status: open — requires spec version target clarification and a pin
in `docs/spec-readings/`. Not vetoed at this stage; spec pin must
precede merge.

**NT-007** (Medium) — Comment at EOF with no trailing newline
Status: open — pending test evidence. Not vetoed yet; will veto if
not covered before merge.

**Also reviewed: `docs/verification/adversary-findings/ttl.md` (Turtle / TriG brief)**
Total finding hypotheses in ttl.md: 9

### Additional veto fires from ttl.md (8)

**TTL-001** (High) — Prefix name leading digit in local part
Parser: rdf-turtle. Spec: Turtle §6.3 `PN_LOCAL`. W3C rdf-tests issue #90.
Veto: FIRED. rdf-turtle merge blocked.

**TTL-002** (High) — Percent-encoding in local part not decoded
Parser: rdf-turtle. Spec: Turtle §2.4 PLX pass-through.
Veto: FIRED. rdf-turtle merge blocked.

**TTL-003** (High) — Keyword `a`/`true`/`false` position sensitivity
Parser: rdf-turtle. Spec: Turtle §2.4 position-restricted grammar.
Veto: FIRED. rdf-turtle merge blocked.

**TTL-004** (High) — Empty collection must be rdf:nil
Parser: rdf-turtle. Spec: Turtle §2.8. W3C rdf-tests issue #115.
Veto: FIRED. rdf-turtle merge blocked.

**TTL-005** (High) — Short vs long string literal newline handling
Parser: rdf-turtle. Spec: Turtle §2.5.2 distinct lexer productions.
Veto: FIRED. rdf-turtle merge blocked.

**TTL-006** (High) — `BASE` directive scope and case sensitivity
Parser: rdf-turtle. Spec: Turtle §2.2 replacement semantics.
Veto: FIRED. rdf-turtle merge blocked.

**TTL-008** (High) — TriG blank node scope per graph block
Parser: rdf-turtle (TriG). Spec: TriG §2.2, §3 graph-scoped blank nodes.
Veto: FIRED. rdf-turtle merge blocked.

**TTL-009** (High) — Numeric literal type selection
Parser: rdf-turtle. Spec: Turtle §2.5.5 distinct numeric grammar tokens.
Veto: FIRED. rdf-turtle merge blocked.

### Open (not vetoed at this time, tracked)

**TTL-007** (Medium) — Trailing semicolon after last predicate-object pair
Status: open — W3C test suite has explicit coverage (`turtle-syntax-predicate-object-semicolon`);
severity reduced to medium by existing test suite coverage. Will veto if
cohort-A does not include that test.

**NT-003** (Medium) — Unicode escape case-sensitivity
Status: open — pending test evidence.

**NT-006** (Medium) — Language tag case normalization (1.1 vs 1.2)
Status: open — spec pin required.

**NT-007** (Medium) — Comment at EOF with no trailing newline
Status: open — pending test evidence.

### Cohort-A deliverables blocked

rdf-ntriples merge: BLOCKED
Condition for release: NT-001, NT-002, NT-004, NT-005 addressed by fix or
confirmed spurious by orchestrator triage. NT-003, NT-006, NT-007 also
require resolution or spurious classification before sign-off.

rdf-turtle merge: BLOCKED
Condition for release: TTL-001, TTL-002, TTL-003, TTL-004, TTL-005,
TTL-006, TTL-008, TTL-009 addressed by fix or confirmed spurious.
TTL-007 also requires resolution or spurious classification.

### Summary counts

Veto fires: 12
Spurious reclassifications: 0
Open findings: 16 (7 NT + 9 TTL)

---

## Veto Event 2026-04-19 (Extension: int-adv-veto-extend, cohort B)

Agent: int-adv-veto-extend
Sweep: verification-v1 integration pass (ADR-0020 §5)
Timestamp: 2026-04-19

### Findings reviewed

Source: `docs/verification/adversary-findings/iri.md` (IRI brief)
Total finding hypotheses: 8

### Veto fires (5) — IRI brief

**IRI-001** (High) — Remove-dots: above-root path clamping
Parser: rdf-iri (cross-cutting all RDF parsers)
Spec: RFC 3986 §5.2.4; IETF Errata 4005
Basis: The remove_dot_segments algorithm must clamp at root. Hand-rolled
resolvers that allow `../../../d` to escape the authority boundary produce
wrong IRIs silently. Cross-cutting impact across all serialization parsers.
Veto: FIRED. rdf-iri merge blocked.

**IRI-002** (High) — Pure fragment reference: base path preservation
Parser: rdf-iri (cross-cutting)
Spec: RFC 3986 §5.2.2
Basis: `#foo` must keep base scheme, authority, and path; only the
fragment is replaced. Implementations that discard the base path or
double-apply fragments produce wrong IRIs.
Veto: FIRED. rdf-iri merge blocked.

**IRI-003** (High) — Surrogate and private-use code point handling
Parser: rdf-iri (cross-cutting)
Spec: RFC 3987 §2.2; RFC 3987 Errata 3937
Basis: Surrogates (U+D800–U+DFFF) forbidden as scalar values; private-use
characters explicitly permitted. Both failure directions (admit surrogates;
reject private-use) are spec violations.
Veto: FIRED. rdf-iri merge blocked.

**IRI-005** (High) — Absoluteness check: authority-less schemes
Parser: rdf-iri (cross-cutting)
Spec: RFC 3986 §3; RFC 2141
Basis: `urn:`, `tag:`, `data:` are absolute IRIs without `://`. A check
for `://` incorrectly rejects conformant IRIs and causes silent data loss.
Veto: FIRED. rdf-iri merge blocked.

**IRI-006** (High) — Empty base path merge: slash insertion
Parser: rdf-iri (cross-cutting)
Spec: RFC 3986 §5.2.3 (Merge Paths)
Basis: Base `http://example` (empty path, authority present) + reference
`foo` must yield `http://example/foo`, not `http://examplefoo`. String
concatenation without merge-paths slash insertion corrupts the authority.
Veto: FIRED. rdf-iri merge blocked.

### Open (not vetoed at this time, but tracked) — IRI brief

**IRI-004** (Medium) — Percent-encoding case unification (`%2F` vs `%2f`)
Status: open — correctness failure manifests only when both forms appear
in the same dataset. Will veto if cohort-A delivers without a test.

**IRI-007** (Medium) — Host case-folding unifies distinct IRIs
Status: open — only manifests when both case variants appear in the same
dataset. Veto will fire if cohort-A lowercases hosts at parse time.

**IRI-008** (Medium) — NFC normalization unifies distinct IRIs
Status: open — runtime-dependent; depends on Unicode library behavior.
Will veto if cohort-A applies NFC at parse time without a guarding test.

---

Source: `docs/verification/adversary-findings/sparql.md` (SPARQL 1.1 brief)
Total finding hypotheses: 9

### Veto fires (7) — SPARQL brief

**SPARQL-001** (High) — OPTIONAL scoping: unbound variable in outer FILTER
Parser: sparql-syntax
Spec: SPARQL 1.1 §12.3.1, §17.3; Errata SE-2
Basis: Unbound `?x` in outer FILTER produces a type error under §17.3
(not `false` or `true`). Implementations that short-circuit to `false`
or `true` produce incorrect solution sets.
Veto: FIRED. sparql-syntax merge blocked.

**SPARQL-002** (High) — MINUS with no shared variables has no effect
Parser: sparql-syntax
Spec: SPARQL 1.1 §12.6
Basis: MINUS requires at least one shared variable to remove solutions.
Treating MINUS as NOT EXISTS incorrectly removes left-side solutions when
the right pattern matches but shares no variables.
Veto: FIRED. sparql-syntax merge blocked.

**SPARQL-003** (High) — CONSTRUCT blank node locally scoped per solution row
Parser: sparql-syntax
Spec: SPARQL 1.1 §10.3; Errata SE-1
Basis: Template blank node labels are local to each solution mapping.
A global blank node table across result rows merges graph nodes incorrectly.
Veto: FIRED. sparql-syntax merge blocked.

**SPARQL-005** (High) — BASE inside WHERE clause silently accepted
Parser: sparql-syntax
Spec: SPARQL 1.1 §3.1; grammar Prologue production
Basis: BASE is prologue-only. This fixture must be REJECTED with a parse
error. Turtle-aware parsers that let BASE slip through IRI hooks will
silently accept it and produce wrong IRIs. Identified in fixture README
as the highest-confidence real-divergence candidate.
Veto: FIRED. sparql-syntax merge blocked.

**SPARQL-006** (High) — GRAPH ?g includes default graph
Parser: sparql-syntax
Spec: SPARQL 1.1 §13.3, §18.5
Basis: `GRAPH ?g` iterates named graphs only; the default graph must not
be exposed. Implementations that include the default graph return spurious
solution rows.
Veto: FIRED. sparql-syntax merge blocked.

**SPARQL-008** (High) — INSERT DATA blank node scope per operation
Parser: sparql-syntax
Spec: SPARQL 1.1 Update §3.1.1
Basis: `_:b` occurrences within one INSERT DATA share the same blank node
(one scope per request). Minting a fresh node per occurrence corrupts
graph structure.
Veto: FIRED. sparql-syntax merge blocked.

**SPARQL-009** (High) — Inverse negated property path precedence
Parser: sparql-syntax
Spec: SPARQL 1.1 §9.3; grammar PathPrimary production
Basis: `^!(p)` means `^(!(p))` not `!(^p)`. Operator precedence
confusion in property path evaluation produces incorrect triple matches.
Veto: FIRED. sparql-syntax merge blocked.

### Open (not vetoed at this time, tracked) — SPARQL brief

**SPARQL-004** (Medium) — HAVING references SELECT aggregate alias
Status: open — evaluation order correctness; errata SE-3 confirms the
permitted form. Will veto if cohort-A evaluates HAVING before SELECT
projection.

**SPARQL-007** (Medium) — FILTER NOT EXISTS vs OPTIONAL/FILTER(!BOUND)
Status: open — optimization-correctness failure masked in simple cases.
Will veto if cohort-A rewrites FILTER NOT EXISTS as OPTIONAL/FILTER(!BOUND)
without correctness guards.

### Cohort-A deliverables blocked (updated totals)

rdf-ntriples merge: BLOCKED (NT-001, NT-002, NT-004, NT-005)
rdf-turtle merge: BLOCKED (TTL-001, TTL-002, TTL-003, TTL-004, TTL-005, TTL-006, TTL-008, TTL-009)
rdf-iri merge: BLOCKED (IRI-001, IRI-002, IRI-003, IRI-005, IRI-006)
sparql-syntax merge: BLOCKED (SPARQL-001, SPARQL-002, SPARQL-003, SPARQL-005, SPARQL-006, SPARQL-008, SPARQL-009)

### Updated summary counts

Veto fires: 24 (12 original + 5 IRI + 7 SPARQL)
Spurious reclassifications: 0
Open findings: 33 (7 NT + 9 TTL + 8 IRI + 9 SPARQL)

---
<!-- APPEND-ONLY -->
