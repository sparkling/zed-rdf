# Pin: Property path precedence — inverse wraps over negated set

- **Diagnostic code:** `SPARQL-PATH-001`
- **Language / format:** SPARQL 1.1 Query.
- **Productions:** `Path ::= PathAlternative`, `PathAlternative ::=
  PathSequence ('|' PathSequence)*`, `PathSequence ::= PathEltOrInverse
  ('/' PathEltOrInverse)*`, `PathEltOrInverse ::= PathElt | '^' PathElt`,
  `PathElt ::= PathPrimary PathMod?`, `PathPrimary ::= iri | 'a' |
  '!' PathNegatedPropertySet | '(' Path ')'` (§19.8 productions
  [88]–[94]), `PathNegatedPropertySet ::= PathOneInPropertySet | '('
  (PathOneInPropertySet ('|' PathOneInPropertySet)*)? ')'`.
- **Spec target:** W3C SPARQL 1.1 Query
  <https://www.w3.org/TR/sparql11-query/#propertypaths> §9.
- **Status:** active.
- **Author:** `fe-phase-c-sparql`.
- **Date:** 2026-04-19.
- **Adversary reference:** Failure Mode 9 in
  `docs/verification/adversary-findings/sparql.md`. Public-sparql-dev
  thread "inverse negated property paths" (2012).

## Ambiguous clause

The grammar unambiguously fixes inverse (`^`) at the `PathEltOrInverse`
level — above `PathElt`, which is `PathPrimary PathMod?`, which itself
contains `!` `PathNegatedPropertySet`. Thus:

```text
^ ! ( p )
└┬┘
 PathEltOrInverse
   └── '^' PathElt
           └── PathPrimary = '!' PathNegatedPropertySet
```

Reading this concretely:

- `^!(p)` = `^(!(p))` = "inverse of the (negated p) property set", i.e.
  match `?o` as the subject and any predicate that is not `p`.
- `!(^p)` is a **different** expression and is illegal by grammar
  because `PathNegatedPropertySet`'s atoms are only forward IRIs or
  single-atom inverse IRIs — it accepts `!(^p)` with exactly one atom
  `^p`, but not an outer `^` wrapping a negated set expressed as an
  independent prefix operator.

Implementations that re-parse `^!(p)` as `!(^p)` have conflated atom
inversion (inside the negated property set) with whole-path inversion
(outside, binding the entire PathElt). That is a known divergence.

## Reading chosen

The parser produces `Path::Inverse(Path::Negated(...))` for the input
`^!(p)`, and `Path::Negated([NegatedAtom::Inv(p)])` for `!(^p)`. The
encoder renders these distinctly in the AST-as-Facts payload so the
diff harness can distinguish them. Equivalently:

- `^!(p)` — inverse binds the whole `PathElt`.
- `!(^p)` — atom-level inverse inside the negated set.

The adversary test fixture (`fm9-inverse-negated-property-path.sparql`)
combines both forms plus `!(p|q)` to maximise exposure.

## Rationale

- The grammar is the authority; any implementation must follow its
  derivation tree.
- The distinction is evaluator-relevant because `^!(p)` binds
  `(?o, any-but-p, ?s)` triples while `!(^p)` binds
  `(?s, any-non-inverse-p, ?o)` triples — different edge directions.
- Public-sparql-dev discussion "inverse negated property paths" (2012)
  documents the mistake.

## Non-goals

- This pin does NOT enforce any evaluation semantics (rewriting,
  optimisation). It is a structural / encoding commitment only.
- It does NOT constrain how zero-or-more / zero-or-one modifiers
  interact with inversion; those productions are straightforwardly
  composed in the AST.

## Diagnostic code

- **Code:** `SPARQL-PATH-001`
- **Emitted by:** `sparql-syntax` (structural-only — the code is
  reserved for future use should precedence diagnostics become
  desirable; currently never fatal).
- **Fatal?** No (structural encoding contract only).
