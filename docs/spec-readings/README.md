# Spec-reading pins

- **Instantiates:** ADR-0019 §5 (spec-reading pin records).
- **Cohort scope:** shared reference material. Both cohort A
  (`verification-v1`) and cohort B (`verification-v1-adv`) may read.
- **Author:** `v1-specpins` (cohort A) for the `verification-v1` sweep.
- **Status:** initial bootstrap. New pins added as adversary findings
  surface; closed pins are amended, never deleted.

## Why these exist

Under the agent-team execution model (ADR-0017) every parser, every
test, and the review hive can share an LLM prior about what an
under-specified production means. ADR-0019 breaks the resulting
oracle-circularity by, among other things, pinning the chosen reading
of each ambiguous production **before** any parser encodes it. Pins are
the project's arbitration record; shadow implementations, main parsers,
adversary fixtures, and `DiffReport` triage all cite the same pin.

## Scope

A pin is required when all of the following hold:

1. The W3C spec wording admits more than one plausible mechanical
   reading (not merely theoretical; adversary hypotheses in
   `docs/verification/adversary-findings/*.md` are evidence of plausibility).
2. A parser in-tree (`crates/syntax/*` or a shadow) must make a
   deterministic choice for that production.
3. Divergence across parsers on this production would silently corrupt
   the emitted RDF or query result, rather than producing a loud parse
   error.

Productions that are unambiguous (e.g. the grammar's literal escape
table when it names a single reading) do not need pins.

## Layout

```
docs/spec-readings/
├── README.md               (this file — index + methodology)
├── <lang>/<production>.md  (per-pin record)
```

`<lang>` is one of `ntriples`, `turtle`, `iri`, `any`, `sparql`,
`jsonld`. `any/` is the cross-cutting bucket (e.g. BOM handling, which
applies to every textual format).

## Pin record template

Every pin file has the same five sections, in order:

1. **Header** — `diagnostic_code`, `lang`, `production`, spec target,
   status, author, date.
2. **Ambiguous clause** — verbatim quote from the spec, with §.
3. **Reading chosen** — the single mechanical reading our parsers will
   implement. Stated as a rule, not a recommendation.
4. **Rationale** — errata, mailing-list threads, W3C test-suite
   interpretation, RFC cross-references; enough evidence that a
   reader can audit the choice without re-doing the research.
5. **Diagnostic code** — the token parsers emit in
   `Diagnostics::messages` (per the frozen trait surface at
   `crates/testing/rdf-diff/src/lib.rs`) when they exercise the pin.
   The code acts as the grep key from a failing diff back to this
   document.

Pins are amended by appending an **Amendment** sub-section dated and
attributed; earlier content is preserved verbatim for audit.

## Diagnostic-code namespace

Codes are ASCII, hyphen-separated, and form `<LANG>-<PROD>-<NNN>`.
`<LANG>` tags:

- `NT` — N-Triples / N-Quads.
- `TTL` — Turtle / TriG.
- `IRI` — cross-cutting IRI resolution (RFC 3986 / 3987).
- `ANY` — cross-cutting textual format (BOM, EOL run at top of file, …).
- `SPARQL` — SPARQL 1.1 query and update.
- `JSONLD` — JSON-LD 1.1 surface.

`<PROD>` is a 3-6 letter mnemonic for the production (`LITESC`,
`BNPFX`, `PCT`, `BOM`, `LITCMP`, `KWALIAS`, …). `<NNN>` is a three-
digit running number within the `(LANG, PROD)` pair. The index below
is the authoritative code map; **do not** emit a code that is not in
this table without amending the index in the same PR.

Coordination with `v1-diff-core`: the `Diagnostics` type is opaque to
the diff harness (`Diagnostics::messages: Vec<String>`), so codes are
message prefixes, not a typed enum. `v1-diff-core` treats a code
prefix match as the join key between a `DiffReport` divergence and
its pin; the pin path is derivable as
`docs/spec-readings/<lang>/<production>.md` from the `LANG`/`PROD`
tags. This is recorded in memory namespace
`verification/spec-readings/pins` so the diff-core agent can
mechanically consume it.

## Pin index

| Diagnostic code    | Pin file                                                           | Status  |
|--------------------|--------------------------------------------------------------------|---------|
| `NT-LITESC-001`    | `ntriples/literal-escapes.md`                                      | active  |
| `TTL-LITESC-001`   | `turtle/literal-escapes.md`                                        | active  |
| `TTL-BNPFX-001`    | `turtle/bnode-prefix-rescope.md`                                   | active  |
| `IRI-PCT-001`      | `iri/percent-encoding-3986-vs-3987.md`                             | active  |
| `ANY-BOM-001`      | `any/bom-handling.md`                                              | active  |
| `SPARQL-LITCMP-001`| `sparql/literal-comparison.md`                                     | active  |
| `JSONLD-KWALIAS-001`| `jsonld/keyword-aliasing.md`                                      | active  |

New pins must add a row here in the same commit that adds the file.

## Cross-reference policy

Each format's main `SPEC.md` (authored in Phase A) cross-references
its pins under a "Pinned readings" section. Where no `SPEC.md` exists
yet, the pin leaves a forward-reference TODO so the Phase-A author
picks it up. Adversary fixtures that exercise a pin cite the
diagnostic code in their filename or header comment; that is how the
diff harness wires back to the pin at triage time.

## Amendment procedure

1. Append an `## Amendment — YYYY-MM-DD — <agent-id>` section to the
   pin.
2. State what changed (clause wording in spec errata; test-suite update;
   previously-overlooked mailing-list decision) and the new reading.
3. Bump the code's `NNN` only if the reading itself changes; a
   clarification keeps the code stable.
4. Update the pin index status column if the pin becomes `superseded`
   or `withdrawn`. Superseded pins are kept for audit.

## Handoff

On completion: `claims_accept-handoff` → `v1-reviewer`.
