# adversary-sparql — SPARQL 1.1 Grammar Adversary Fixtures

Agent: `v1-adv-sparql` (cohort B, adversary hive, verification-v1 sweep)
ADR references: ADR-0019 §4, ADR-0020 §1.4

These fixtures target grammar-level and specification-semantic divergences in
SPARQL 1.1 parsers. Execution semantics are out of scope; each fixture is a
valid (or intentionally invalid) SPARQL document whose correct parse outcome is
precisely stated. The `xtask verify --adversary-sparql` target runs the Rust
integration module `adversary_sparql.rs` against each fixture.

## Fixture Index

| File | Finding | Expected outcome |
|------|---------|-----------------|
| fm1-optional-filter-unbound.sparql | FM1: OPTIONAL + outer FILTER sees unbound ?x | Accept (parse ok); divergence at evaluation |
| fm2-minus-no-shared-variable.sparql | FM2: MINUS with zero shared variables has no effect | Accept |
| fm3-construct-bnode-per-row.sparql | FM3: CONSTRUCT blank node is fresh per solution row | Accept |
| fm4-having-select-alias.sparql | FM4: HAVING references SELECT aggregate alias ?cnt | Accept |
| fm5-base-mid-query.sparql | FM5: BASE inside WHERE clause — grammar violation | **Reject (parse error)** |
| fm6-graph-variable-default-graph.sparql | FM6: GRAPH ?g does not match default graph | Accept |
| fm7-filter-not-exists-vs-optional.sparql | FM7: FILTER NOT EXISTS differs from OPTIONAL/FILTER(!BOUND) | Accept |
| fm8-insert-data-bnode-scope.sparql | FM8: INSERT DATA blank node scope within one operation | Accept (as SPARQL Update) |
| fm9-inverse-negated-property-path.sparql | FM9: ^!(p) means ^(!(p)), not !(^p) — precedence | Accept |
| fm10-service-nesting.sparql | FM10: SERVICE nested inside SERVICE | Accept per spec grammar |
| fm11-bind-scoping.sparql | FM11: BIND defines ?x, used in later triple | Accept |
| fm11b-bind-scoping-violation.sparql | FM11b: ?x used before BIND that defines it | **Reject (query error)** |
| fm12-subquery-projection.sparql | FM12: Outer query references var not in subquery SELECT | Accept; ?internal unbound |

## Per-Fixture Hypotheses

### FM1 — OPTIONAL + FILTER on unbound variable (fm1-optional-filter-unbound.sparql)

Spec: §12.3.1 + §17.3. The outer `FILTER(?x > 3)` references `?x` which is
only bound inside the OPTIONAL branch. When the OPTIONAL does not match, `?x`
is unbound and `?x > 3` raises a type error under §17.3. A type error in
FILTER discards the solution — this is different from evaluating to `false`
(which would also discard) or evaluating to `true` (which would retain).
Errata SE-2. **Surfaces real divergence**: implementations that short-circuit
unbound as `false` are indistinguishable from correct ones in discard outcome,
but implementations that treat it as `true` over-include.

### FM2 — MINUS with disjoint variable sets (fm2-minus-no-shared-variable.sparql)

Spec: §12.6. The MINUS right-hand side introduces `?x` and `?y`, which do not
appear on the left (`?s`, `?o`). Per §12.6 the shared-variable set is empty,
so MINUS has no effect. An implementation treating MINUS as NOT EXISTS removes
left-side solutions when the right pattern has any solutions — wrong.

### FM3 — CONSTRUCT blank node per solution row (fm3-construct-bnode-per-row.sparql)

Spec: §10.3, errata SE-1. `_:b` in the CONSTRUCT template is locally scoped
to each solution mapping. Two result rows produce two distinct blank nodes for
`_:b`. An implementation that treats `_:b` as one global node across all rows
merges the graph incorrectly.

### FM4 — HAVING references SELECT aggregate alias (fm4-having-select-alias.sparql)

Spec: §11.4, §11.4.2, errata SE-3. `?cnt` is the alias for `COUNT(?o)` in
SELECT. HAVING is allowed to reference it per §11.4.2. Implementations that
evaluate HAVING before projecting SELECT aliases cannot resolve `?cnt`.

### FM5 — BASE inside WHERE clause (fm5-base-mid-query.sparql)

Spec: §3.1, grammar `Prologue ::= (BaseDecl | PrefixDecl)*`. BASE is a
prologue-only declaration; it is not a valid group-graph-pattern element.
This fixture MUST be rejected with a parse error. It is the only fixture in
this set that exercises an accept/reject split at the grammar level. This is
the highest-confidence real-divergence candidate: Turtle-aware parsers that
let BASE slip through IRI-resolution hooks will silently accept it.

### FM6 — GRAPH ?g does not include default graph (fm6-graph-variable-default-graph.sparql)

Spec: §13.3, §18.5. `GRAPH ?g { ... }` iterates named graphs only. If the
dataset has no named graphs, the result is empty. An implementation that
exposes the default graph via `GRAPH ?g` returns spurious solutions.

### FM7 — FILTER NOT EXISTS vs OPTIONAL/FILTER(!BOUND) (fm7-filter-not-exists-vs-optional.sparql)

Spec: §12.5. When the NOT EXISTS inner pattern shares all variables with the
outer pattern, the OPTIONAL/FILTER(!BOUND) rewrite is not equivalent. The
rewrite leaves `?o` already bound before the OPTIONAL, so `!BOUND(?o)` is
always false and the filter discards everything — same outcome as NOT EXISTS
here, but for the wrong reason. In the general case an optimiser using this
rewrite produces different results.

### FM8 — INSERT DATA blank node scope (fm8-insert-data-bnode-scope.sparql)

Spec: SPARQL Update §3.1.1. Both `_:b` occurrences within one INSERT DATA
refer to the same blank node (one-scope-per-operation). An implementation
that mints a fresh blank node per occurrence splits the intended single node
into two disconnected nodes.

### FM9 — Inverse negated property path precedence (fm9-inverse-negated-property-path.sparql)

Spec: §9.3, grammar `PathPrimary ::= '^' PathElt`. `^!(rdf:type)` is parsed
as `^(!(rdf:type))`: invert the direction of the whole negated-property-set
path. Parsing it as `!(^rdf:type)` applies negation to the inverse path —
a different expression with different match semantics. Public-sparql-dev
discussion (2012).

### FM10 — Nested SERVICE (fm10-service-nesting.sparql)

Spec: SPARQL 1.1 Federated §4. The grammar allows SERVICE anywhere a
GroupGraphPattern is allowed, including inside another SERVICE block.
Implementations with hard-coded depth limits reject valid queries.

### FM11 / FM11b — BIND variable scoping (fm11-bind-scoping.sparql, fm11b-bind-scoping-violation.sparql)

Spec: §18.2.1. A BIND target variable must not appear in the same group
graph pattern before the BIND. FM11 tests the legal form (BIND then use);
FM11b tests the illegal form (use before BIND) which must be rejected.

### FM12 — Subquery projection-list scope (fm12-subquery-projection.sparql)

Spec: §12.3.2. Variables defined inside a subquery but not in its SELECT
list are invisible to the outer query. `?internal` is defined inside the
subquery via BIND but is not projected; the outer SELECT referencing it
must treat it as unbound.

## Integration with xtask verify

The test module `crates/testing/rdf-diff/tests/adversary_sparql.rs` reads
this directory at test time (via `collect_fixtures("sparql")` in
`snapshots.rs`) and feeds each `.sparql` file to the registered parsers.
The `xtask verify --adversary-sparql` target runs these tests and reports
any `AcceptRejectSplit` or `ObjectMismatch` divergences.

Fixtures annotated "Reject (parse error)" are expected to produce
`Divergence::AcceptRejectSplit` between parsers — that is the divergence
signal they are designed to surface.
