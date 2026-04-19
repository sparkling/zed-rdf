# ADR-0019: Independent verification against shared-prior failure modes

- **Status:** Proposed
- **Date:** 2026-04-19
- **Deciders:** Henrik Pettersen
- **Supersedes:** —
- **Amends:** ADR-0004 (adds `[dev-dependencies]` carve-out for reference oracles)
- **Tags:** `policy`, `testing`, `quality`, `agents`, `verification`

## Context and Problem Statement

ADR-0017 establishes AI-agent-orchestrated parallel execution as the
project's default. ADR-0006's test pyramid (unit → property → fuzz →
snapshot → W3C → LSP-integration) is the correctness evidence. ADR-0005
claims "accept exactly the spec language, verified by W3C manifests at
100 %."

Under the AI-team execution model, parsers, tests, fuzzers, and the
reviewer agent all draw from **the same LLM priors** about what each
W3C spec means. A Round-2 adversarial review of the ADR set surfaced
the resulting failure mode:

> Green CI proves agent-generated tests agree with agent-generated
> code — not that either matches the spec. Under-specified productions
> (literal escapes, IRI percent-encoding, BOM handling, BNode scoping
> across `@prefix` redefs) receive a **confidently consistent wrong
> reading**. More agent labour makes this worse, not better.

W3C manifests are finite and do not exhaustively cover the spec's edge
cases; they cannot serve as the sole oracle. ADR-0006 adds breadth of
evidence but not *independence* of evidence.

This ADR decides how the project breaks the circularity.

## Decision Drivers

- **Oracle independence.** The thing judging correctness must not share
  a prior with the thing being judged.
- **Agent-team compatibility.** Countermeasures must be executable by
  agents, not require scarce human attention.
- **ADR-0004 fidelity.** "No forking" was about runtime parser deps,
  not test oracles. The interpretation needs to be explicit.
- **Continuous, not ceremonial.** Independent verification must run
  every PR, not at release only.
- **Detectability over prevention.** We accept that divergent-reading
  drift will occur; we insist it is caught.

## Considered Options

1. **Status quo** — W3C manifests + agent-authored tests. Cheapest,
   preserves ADR-0004 strictly. Leaves the circularity open; the
   fatal flaw of the current design remains.
2. **Human spot-check** — human reviews a sample of parser edge
   cases. Reintroduces the human as a labour bottleneck; does not
   scale across 9 languages.
3. **Differential testing against external reference parsers** as
   `[dev-dependencies]` only — `oxttl`, `oxrdfxml`, `oxjsonld`,
   `oxsparql`, and (out-of-tree) Jena / rdf4j via a pre-materialised
   fact corpus. Automated, continuous, cheap under agent labour.
4. **Independent second implementation** per critical parser from a
   disjoint agent cohort (different prompt lineage, ideally different
   base model), cross-diffed. Strongest independence signal;
   highest token cost.
5. **Adversarial-review hive with veto** — a review hive distinct
   from the implementing hive, spawned from a disjoint prompt
   lineage, empowered to block merge on correctness grounds.

## Decision

**Chosen option: a layered combination of 3, 4, and 5.** Each layer
targets a different independence axis; together they break the
circularity at tractable cost.

### 1. `[dev-dependencies]` oracle carve-out (amends ADR-0004)

ADR-0004's ban on RDF/SPARQL parser crates applies to **runtime
`[dependencies]` only**. The following crates are permitted as
**`[dev-dependencies]`** for differential / oracle testing **and
nowhere else**:

| Crate          | Role                                                      |
|----------------|-----------------------------------------------------------|
| `oxttl`        | Reference parser for N-Triples, N-Quads, Turtle, TriG.    |
| `oxrdfxml`     | Reference parser for RDF/XML.                             |
| `oxjsonld`     | Reference parser for JSON-LD surface syntax.              |
| `oxsparql-syntax` | Reference parser for SPARQL 1.1.                       |
| `sophia_*`     | Secondary reference for Turtle family (optional).         |

`cargo-deny` is configured to fail the build if any of the above
appears in `[dependencies]`, `[build-dependencies]`, or the dependency
closure of a non-test binary. An explicit regression test asserts the
separation.

Jena and rdf4j are **out of process**. A CI job materialises a fact
corpus from each on the W3C suites and stores the result as JSON in
`external/fact-oracles/` (pinned by suite commit). Our parsers'
outputs are diffed against this corpus; no JVM runs in the Rust test
path.

### 2. Differential test harness (`crates/testing/rdf-diff/`)

A new crate in the existing `crates/testing/` tree. Responsibilities:

- For each format in scope, feed the union of W3C manifests + fuzz
  corpora + a curated edge-case corpus (`external/tests/edge/*`)
  into **our parser** and the **reference parser(s)** above.
- Normalise both outputs to a canonical `Facts` form (triples /
  quads / graph shape, prefix-free, BNode-canonicalised).
- Diff. Any divergence — accept/reject, fact count, literal
  datatype, language tag, BNode scope, IRI resolution — is a CI
  failure with a minimal reproducer emitted.
- Allow-list entries permitted only when **both** our parser and ≥2
  reference oracles disagree with the spec; entry must cite the
  upstream issue and is subject to ADR-0006's `ALLOWLIST.md`
  expiration rules.

### 3. Independent second implementation for load-bearing parsers

`rdf-iri`, `rdf-ntriples`, `rdf-turtle`, and `sparql-syntax` each
carry a **shadow implementation** in a `-shadow` sibling crate
(`crates/syntax/rdf-turtle-shadow/` etc.). Shadow crates:

- Are implemented by a **disjoint agent cohort** — different prompt
  lineage, different seed references, ideally a different base
  model (recorded in the ADR-0017 `Agent:` footer).
- Exist solely to feed the diff harness; they are **not published**
  and are behind a `shadow` Cargo feature.
- Must be produced in parallel with the main implementation, not
  later.
- Shadow ≠ main output on any input is a CI failure.

Formats where a shadow implementation is disproportionate
(`rdf-trix`, `rdf-n3`, `datalog-syntax`) rely on layers 1 and 2
only; rationale recorded in their `SPEC.md`.

### 4. Adversarial-review hive with veto (amends ADR-0017)

Every parser PR requires sign-off from an **adversarial-review hive**
distinct from the implementing hive:

- Spawned with a prompt lineage that does **not** share the
  implementation prompt's framing of the spec.
- Loaded with the spec text + a red-team brief ("find inputs that
  expose divergent readings").
- Has **veto authority** — merges block on its flag until the diff
  harness or the implementer addresses the finding.
- Sign-off recorded alongside the ADR-0017 reviewer handoff:
  `Adversary: <agent-id>`.

### 5. Spec-reading pin records

Every ambiguous spec production gets a versioned pin in
`docs/spec-readings/<lang>/<production>.md` **before** any parser
encodes it. Pins carry: the ambiguous clause, the reading chosen,
references (spec §, errata, mailing-list threads), and the date.
Agents cite the pin in the diagnostic code they emit for the
production. Divergence across parsers on the same pin is an
automatic ADR-level escalation.

### 6. Memory-poisoning countermeasures (amends ADR-0017 §8)

The ruflo `phase-<letter>` blackboard gets:

- **TTL** on non-pinned facts (default 7 days).
- **Falsification hooks**: every `memory_store` entry carries a test
  id; if the test later fails or is invalidated, the memory entry is
  quarantined.
- **Cohort tagging**: entries are tagged with the authoring agent's
  cohort; the diff harness and adversary hive must not read from
  the implementing cohort's namespace.

## Consequences

- **Positive**
  - Breaks the oracle-circularity fatal flaw identified in the Round 2
    adversarial ADR review.
  - `[dev-dependencies]` oracles give a near-free differential signal
    the project is currently forgoing.
  - Shadow implementations + adversary hive make "green CI" mean more
    than "agents agree with themselves".
  - TTL + falsification hooks stop wrong priors from compounding
    across waves.
- **Negative**
  - Token cost rises: every parser lands with a shadow + an
    adversarial review. Budgeted as verification cost, not waste.
  - ADR-0004 is no longer clean — `[dev-dependencies]` carve-out must
    be policed. `cargo-deny` handles the mechanical case.
  - Shadow crates add workspace members (4 now, maybe more later).
    `cargo check --workspace` time rises; mitigated by feature flag.
- **Neutral**
  - Fact-corpus pre-materialisation introduces a Java toolchain at
    CI-image-build time. Contained to one job; no Rust-path impact.

## Validation

- **Oracle separation** — `cargo-deny` gate on runtime deps is green;
  a regression test asserts no `ox*` / `sophia_*` crate appears in any
  `rdf-*` crate's runtime dependency closure.
- **Differential signal is live** — `crates/testing/rdf-diff` produces
  at least one *material* divergence finding per parser before that
  parser's conformance gate is declared green. Zero divergences on
  first run is treated as suspicious and triggers a prompt-lineage
  audit.
- **Shadow independence** — shadow-vs-main diff harness is green on
  the W3C suites; prompt-lineage records confirm disjoint cohorts.
- **Adversary veto fires** — in the first 3 parsers shipped, the
  adversary hive produces at least one non-trivial finding each.
  Zero findings across all 3 is a smell, not a success.
- **Spec pins exist** — every `SPEC.md` cross-references its ambiguous
  productions to a `docs/spec-readings/*.md` pin.
- **Memory TTL works** — an audit query finds no `phase-<letter>`
  blackboard entry older than 7 days without an explicit pin.

## Links

- [`0004-third-party-crate-policy.md`](0004-third-party-crate-policy.md)
  — amended by §1 above.
- [`0005-soundness-completeness-scope.md`](0005-soundness-completeness-scope.md)
  — this ADR provides the missing *independence* leg of the correctness
  claim.
- [`0006-testing-strategy.md`](0006-testing-strategy.md) — differential
  harness is a new formal layer between property tests and W3C
  manifests.
- [`0017-execution-model.md`](0017-execution-model.md) — amended in §4
  (adversary hive) and §6 (memory hygiene).
- [`0018-phase-a-execution-plan.md`](0018-phase-a-execution-plan.md) —
  Phase A parsers must ship with their shadow + adversary sign-off or
  slip to Phase B.
- Round-2 adversarial ADR review (2026-04-19, hive transcript).
