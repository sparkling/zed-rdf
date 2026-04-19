# Turtle — W3C Manifest Divergences

Owner: `pa-w3c-vendor`. Captures divergences surfaced by running the
`xtask verify` harness against the vendored W3C `rdf-turtle` suite
(see `external/tests/PINS.md`).

## Current state (post-triage, 2026-04-19 — allow-list closed)

- Corpus root: `external/tests/ttl/`
- Files discovered: 433
- Harness mode: **live** (`rdf-turtle::TurtleParser` driving `rdf-diff`).
- `xtask verify` outcome: 626 entries, **626 pass / 0 divergences / 0 allow-listed** → strict pass-rate **100 %**.
- Workspace: `cargo test --workspace --all-features --no-fail-fast` green; `cargo clippy --workspace --all-features -- -D warnings` clean.

## Class taxonomy

The original 54-divergence set resolved to six underlying causes. Five
were parser bugs fixed in this pass; the sixth is a harness gap that is
captured in the allow-list pending a `mf:assumedTestBase` wiring change
in `xtask/verify`.

### B. Bare blank-node-property-list as statement subject — **FIXED**

Turtle §2.5 grammar:

```
triples ::= subject predicateObjectList
          | blankNodePropertyList predicateObjectList?
```

The parser required a `predicateObjectList` after every subject,
including the `blankNodePropertyList` branch where it must be optional.
A `[ :p :o ] .` statement was therefore rejected with
`TTL-SYNTAX-001 expected verb`.

Fix: `Parser::parse_triple_stmt` now tracks the subject kind and skips
the `predicateObjectList` call when the subject was a
`blankNodePropertyList` AND the lookahead is not a verb (`a` /
`IRIREF` / `pname`). See `grammar.rs` → `SubjectKind`.

Member test-ids (TTL):

- `turtle-syntax-bnode-08`
- `turtle-syntax-bnode-09`
- `turtle-syntax-bnode-10`
- `sole_blankNodePropertyList`
- `nested_blankNodePropertyLists`
- `blankNodePropertyList_containing_collection`

### E. Nested-collection bnode label ordering — **FIXED**

`turtle-eval-lists-05` (`(1 2 (1 2)) :p (("a") "b" :o) .`) produced a
fact-set isomorphic to the expected N-Triples but with a bnode
relabelling that the diff harness's bnode-blind canonicaliser could
not fold into a perfect match. The outer-collection head received a
larger label than the inner-collection heads because `collection()`
minted the head *after* all items were parsed.

Fix: mint the cons-cell bnode *before* parsing the item so outer cells
always precede inner cells in label-allocation order. See
`grammar.rs` → `collection()`.

Member test-ids: `turtle-eval-lists-05`.

### F. `@prefix`/SPARQL keyword lexical ambiguity — **FIXED**

The lexer's `lex_pname` returned `(prefix, local="")` for both bare
keywords (`a`, `PREFIX`, `BASE`, `GRAPH`, `true`, `false`) and empty-
local pnames (`a:`, `prefix:`). When the keyword dispatch fired it
silently "ate" a trailing `:` that was never present, with the result
that `@prefix a: <…>` was tokenised as `@prefix a <…>` and rejected
with `expected 'prefix:' name`.

Fix: `lex_pname` now returns `(prefix, local, end, had_colon)`; the
keyword arms only fire when `had_colon == false`. A bare identifier
that matches neither a keyword nor a pname is a hard syntax error.

Member test-ids: `turtle-subm-02` (and by cascade the TriG mirror).

### G. TTL-BASE-001 relative IRIs without a harness-supplied base — **CLOSED**

Thirty-four TTL entries (17 unique, seen once per manifest pass) use
relative IRIs and rely on the manifest's `mf:assumedTestBase` to serve
as the in-scope base IRI. The `xtask verify` harness previously passed
the raw action bytes to `rdf-turtle::TurtleParser` without a synthetic
base, so the parser correctly rejected per the pin in
`docs/spec-readings/turtle/base-undeclared.md` (TTL-BASE-001).

Closed on 2026-04-19 by:

1. Adding `TurtleParser::parse_with_base` / `TriGParser::parse_with_base`
   as inherent methods that seed the parser's base-IRI slot before
   `parse_document()` runs. The `rdf_diff::Parser::parse` contract is
   unchanged — it still means "no external base".
2. Threading `mf:assumedTestBase` through `xtask/verify/src/manifest.rs`:
   `extract_assumed_test_base` pulls the triple off the manifest during
   manifest parse, and `run_entry` concats it with the action filename
   (via `concat_retrieval_url`) before passing to `parse_for_language`.
   Only positive-syntax and eval kinds receive the seed — negative-*
   tests are deliberately malformed and must not be given free relative-
   IRI resolution.

Additional bug surfaced and fixed during the closure: W3C
`turtle-syntax-number-11` (`123.E+1 .`) exercised the Turtle §6.5
`DOUBLE ::= [0-9]+ '.' [0-9]* EXPONENT` branch with zero digits after
the dot. The lexer's `lex_number` only consumed a `.` when followed by
a digit, mis-classifying the `.` as a statement terminator. Extended
the predicate to also consume `.` when the lookahead is an
`e`/`E` and at least one leading digit has been seen.

Member test-ids (TTL, 17):

- `turtle-subm-01`, `turtle-subm-27`
- `turtle-syntax-datatypes-01`, `turtle-syntax-datatypes-02`
- `turtle-syntax-kw-01`, `turtle-syntax-kw-02`
- `turtle-syntax-number-01` … `turtle-syntax-number-11` (11)

### H. Tolerant trailing `.` after SPARQL PREFIX/BASE — **CLOSED**

Turtle §6.5 says SPARQL-style `PREFIX` / `BASE` do **not** take a `.`
terminator; W3C tests `turtle-syntax-bad-base-03` and
`turtle-syntax-bad-prefix-05` exercise this as negative syntax. The
parser previously accepted a stray `.` in this position for backward
compatibility with the in-repo adversary fixture
`crates/testing/rdf-diff/tests/adversary-ttl/fm6-base-directive-replacement.ttl`,
which was authored with a trailing dot.

Closed on 2026-04-19 by:

1. Amending the fm6 fixture to drop the stray `.` after `BASE <…>` —
   the fixture's grammar claim is about directive *replacement*, not
   the terminator.
2. Replacing the tolerant `consume_if_dot` call in `grammar.rs`'s
   `directive_prefix` / `directive_base` with a new `reject_dot`
   helper that emits `DirectiveTerminator` when the next token is `.`
   after a SPARQL-style directive.
3. Updating the spec-reading pin in
   `docs/spec-readings/turtle/directive-terminator.md` to document the
   strict reading.

Member test-ids: `turtle-syntax-bad-base-03`, `turtle-syntax-bad-prefix-05`.

## Tally

| class | count | status                                  |
| ----- | ----- | --------------------------------------- |
| B     | 6     | fixed                                   |
| E     | 2     | fixed                                   |
| F     | 2     | fixed                                   |
| G     | 34    | closed (harness + number-lexer fix)     |
| H     | 4     | closed (fixture + strict grammar)       |

## Deferred

None. The verification-v1 Phase-A exit gate reports zero divergences
and zero allow-listed entries for TTL.
