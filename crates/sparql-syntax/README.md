# sparql-syntax

Main SPARQL 1.1 Query + Update grammar parser for the zed-rdf workspace.

Referenced by:

- ADR-0017 §4 (Phase workflow).
- ADR-0018 Phase C ("SPARQL 1.1 full + 1.2 behind feature").
- ADR-0020 §1.4 (verification-v1 integration contract).

The shadow peer is `crates/syntax/sparql-syntax-shadow`. Both crates
implement `rdf_diff::Parser` and are compared by the diff harness.

## Scope

Grammar-only. This crate produces a structural AST and encodes it as
`rdf_diff::Fact`s. It does **not** evaluate SPARQL, apply dataset
semantics, or verify algebraic equivalence. Where the adversary brief
identifies grammar-level static checks (FM5, FM8, FM11b), those are
enforced at parse time with the diagnostic codes listed below.

### Covered productions

- Query forms: `SELECT`, `CONSTRUCT`, `ASK`, `DESCRIBE`.
- Update ops: `INSERT DATA`, `DELETE DATA`, `DELETE WHERE`,
  `DELETE`/`INSERT` Modify (with optional `WITH`), `LOAD`, `CLEAR`,
  `CREATE`, `DROP`, `COPY`, `MOVE`, `ADD`.
- Group graph pattern elements: triples blocks with property / object
  lists, `OPTIONAL`, `MINUS`, `UNION`, nested groups, `FILTER`,
  `BIND`, `VALUES`, `SERVICE` (with `SILENT`), `GRAPH`, sub-SELECT.
- Property paths: sequence `/`, alternative `|`, inverse `^`,
  zero-or-one `?`, zero-or-more `*`, one-or-more `+`, negated
  property set `!(p | ^q | …)`. Grouping with `( … )`.
- Expressions: logical `||` / `&&`, comparisons `= != < <= > >=`,
  `IN` / `NOT IN`, arithmetic `+ - * /`, unary `! + -`, function
  calls (built-ins and IRI-valued), `EXISTS` / `NOT EXISTS`,
  aggregates (`COUNT`, `SUM`, `MIN`, `MAX`, `AVG`, `SAMPLE`,
  `GROUP_CONCAT` with `SEPARATOR=`), with optional `DISTINCT`.
- Solution modifiers: `GROUP BY` (variables / expressions /
  `(expr AS ?v)`), `HAVING`, `ORDER BY` (`ASC` / `DESC`), `LIMIT`,
  `OFFSET`.
- Prologue: `BASE <iri>` and `PREFIX name: <iri>` (SPARQL-style).

### Grammar-level static checks

- **`SPARQL-PROLOGUE-001`** (FM5): `BASE` or `PREFIX` tokens inside a
  group graph pattern are rejected. The Prologue is a prefix-only
  region per §4.1 `Prologue ::= (BaseDecl | PrefixDecl)*`.
- **`SPARQL-BIND-001`** (FM11b): `BIND(expr AS ?x)` is rejected when
  `?x` is already in scope in the enclosing group graph pattern
  (§18.2.1).
- **`SPARQL-UPDATE-001`** (FM8): `INSERT DATA` / `DELETE DATA` reject
  variables; `DELETE DATA` additionally rejects blank nodes
  (§3.1.1 / §3.1.2).
- **`SPARQL-PATH-001`** (FM9): inverse binds over a whole PathElt, so
  `^!(p)` parses as `^(!(p))`, never as `!(^p)`. The encoding renders
  the inverse wrapping `Path::Inverse(Path::Negated(…))` so the
  difference from `!(^p)` is visible in the fact payload.

## AST-as-Facts encoding contract

Every parse produces a multiset of facts with a shared sentinel subject
and predicates under the `urn:x-sparql-syntax:` namespace. The diff
harness compares the canonical `Facts` set, so the specific payload
rendering below is **this crate's** choice; the shadow is free to use
a different encoding. Divergences that are purely encoding-shape
artefacts are documented in
`docs/verification/adversary-findings/sparql/divergences.md` and do
not count as correctness bugs.

### Fact shape

```text
subject   = <urn:x-sparql-syntax:request>
predicate = <urn:x-sparql-syntax:{key}>
object    = "{payload}"       (plain literal; lexical form verbatim)
graph     = None              (request-scoped, not dataset-scoped)
```

### Keys

| key | meaning |
|---|---|
| `kind` | `"query"` or `"update"`. |
| `base` | Prologue `BASE` IRI (literal IRI, no angle brackets). |
| `prefix` | `"name -> iri"` per Prologue `PREFIX`. |
| `form` | Query form: `SELECT`, `CONSTRUCT`, `ASK`, or `DESCRIBE`. |
| `select-modifier` | `DISTINCT` or `REDUCED`. |
| `projection` | `"*"` or `"?var"` or `"(expr AS ?var)"`. |
| `construct-template` | One fact per triple in the CONSTRUCT template, plus `"<short-form>"` sentinel for short CONSTRUCT. |
| `dataset` | `"FROM <iri>"` / `"FROM NAMED <iri>"`. |
| `where` | A single fact containing the serialised group graph pattern (braces delimit). |
| `group-by` | One fact per group condition. |
| `having` | One fact per HAVING expression. |
| `order-by` | One fact per ORDER BY condition. |
| `limit`, `offset` | Decimal integer. |
| `values` | Serialised VALUES block (variable list + rows). |
| `describe-target` | Target variable or IRI. |
| `op` | Update operation keyword form. |
| `insert-data`, `delete-data`, `delete-where` | One fact per quad group. |
| `modify-with`, `modify-delete`, `modify-insert`, `modify-using` | Modify operation parts. |
| `modify-where` | Modify WHERE pattern (single fact). |

The WHERE and modify-where payload renders as `"{ ELEM1 . ELEM2 . … }"`
where each element is its own serialisation. Nested group graph
patterns recurse.

### Independence note

This encoding was authored without reference to the shadow's encoding.
Some divergences are expected:

1. The shadow may split WHERE into per-element facts rather than a
   single serialised payload. The diff harness will report
   `FactOnlyIn` on both sides; triage is against the fixture's
   grammar-level expectation, not against identical fact shape.
2. The shadow may choose a different namespace prefix. That surfaces
   as predicate-namespace divergences.

Both classes are enumerated in
`docs/verification/adversary-findings/sparql/divergences.md`.

## Diagnostic codes

See `docs/spec-readings/sparql/` for the pins. The codes emitted by
this crate are:

- `SPARQL-LITCMP-001` (pin exists; not emitted in syntax phase).
- `SPARQL-PROLOGUE-001` (FM5).
- `SPARQL-BIND-001` (FM11b).
- `SPARQL-UPDATE-001` (FM8).
- `SPARQL-PATH-001` (FM9; structural encoding only, never fatal).
- `SPARQL-AGG-001` (aggregate scope; reserved, not yet emitted).
- `SPARQL-SYNTAX-001`, `SPARQL-EOF-001`, `SPARQL-UNTERM-001`,
  `SPARQL-UTF8-001`, `SPARQL-IRI-001`, `SPARQL-BASE-001`,
  `SPARQL-PFX-001`, `SPARQL-LITESC-001` (grammar infrastructure).

## Non-goals

- Full RFC 3987 IRI resolution at parse time. Relative IRIs are
  preserved as-is; `BASE` binding is recorded in the Prologue fact
  stream but not mechanically applied. `rdf-iri` is a hard
  dependency for future resolution work — the edge is present so
  the crate does not need restructuring when resolution lands.
- SPARQL evaluation, algebra construction, dataset semantics.
- SPARQL 1.2 extensions (scope-gated for a future `1.2` feature).
