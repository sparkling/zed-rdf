# SPARC-02 — Pseudocode

> **Supersedes** the engine-scoped v1. Rewritten 2026-04-18 for the
> Zed-extension + LSP scope.

This phase is **filled just-in-time**, one refinement phase ahead of
implementation. The goal is not to pre-design every algorithm before
writing code; it is to make sure non-trivial ones are thought through at
a level above Rust *before* we commit to types and ownership.

## Rules of engagement

1. **Only the algorithms that deserve it.** A lexer rule does not need
   pseudocode. Error-tolerant Turtle recovery, SPARQL scope resolution,
   JSON-LD context validation, incremental re-parse, idempotent
   formatting — those do.
2. **Each sketch lives next to the phase it unblocks.**
3. **Pseudocode is not code.** Typed Rust-ish prose: function
   signatures, invariants, recovery rules, references to spec clauses.
4. **Every sketch cites the spec clause it implements.**
5. **Error-recovery strategy** is part of every parser sketch, not an
   afterthought.

## Planned sections (filled per phase)

- **§1 IRI parsing and relative resolution** (phase A): RFC 3987 ABNF
  implementation; base + relative resolution per RFC 3986 §5.
- **§2 Turtle error-tolerant parser** (phase A): `.`-boundary resync;
  prefix and base state machine; CST representation (`rowan`/`cstree`).
- **§3 Turtle formatter** (phase A/E): prefix reuse policy,
  indentation, line-wrapping of long lists, blank-node anonymisation
  rules. Idempotency proof sketch.
- **§4 RDF/XML event-based parser** (phase B): streaming `quick-xml`
  consumer; mapping RDF/XML patterns to facts; handling
  `rdf:parseType=Collection`, `Resource`, `Literal`.
- **§5 JSON-LD context validation** (phase B): `@context` value rules
  without full expand/compact; scoped contexts; type-scoped rules.
- **§6 SPARQL scope resolution** (phase C): prefix map; variable
  binding sites; subquery boundaries; aggregate scope.
- **§7 ShEx deterministic recognition** (phase D): triple expression
  determinism check (for highlighting / authoring aid, not for
  validation).
- **§8 Incremental parsing** (phase G): rope → changed-range → smallest
  re-parsed subtree (where cheap); worst-case full re-parse.
- **§9 LSP completion context detection** (phase F): token stream +
  cursor position → completion kind (keyword / prefix / local name /
  snippet / vocab term).
- **§10 Rename scope computation** (phase G): per-language rules for
  which occurrences of a symbol are the same symbol.

Sections are added **before** the corresponding crate or feature begins.
