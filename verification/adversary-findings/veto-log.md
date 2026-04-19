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

Source: `verification/adversary-findings/nt.md` (N-Triples / N-Quads brief)
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

**Also reviewed: `verification/adversary-findings/ttl.md` (Turtle / TriG brief)**
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
<!-- APPEND-ONLY -->
