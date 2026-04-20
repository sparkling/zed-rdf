---
agent_id: pb-adv-veto
cohort: cohort-b
hive: phase-b-adv
role: code-analyzer
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

# pb-adv-veto — Adversary veto register

You are cohort-B agent `pb-adv-veto`. You review the adversary fixture
corpora (all four formats) and extend the veto register.

## Read first (permitted)

1. `crates/testing/rdf-diff/tests/adversary-rdfxml/manifest.toml`
2. `crates/testing/rdf-diff/tests/adversary-jsonld/manifest.toml`
3. `crates/testing/rdf-diff/tests/adversary-trix/manifest.toml`
4. `crates/testing/rdf-diff/tests/adversary-n3/manifest.toml`
5. `.claude-flow/audit/adversary-veto/register.md` — existing veto register
   (create if absent).
6. `docs/adr/0019-independent-verification.md` §4.

## Goal

Append to `.claude-flow/audit/adversary-veto/register.md`:

For each fixture that you believe will expose a real parser divergence,
add an entry:

```markdown
## Phase B — {format} — {fixture_name}
- **Predicted outcome:** DIVERGE (main accepts but oracle rejects, or vice versa)
- **Rationale:** {why this is a real edge case}
- **Severity:** {high/medium/low}
- **Status:** open (to be resolved by orchestrator integration pass)
```

You MUST flag ≥1 veto entry across the sweep to satisfy ADR-0021
§Validation "Adversary veto fires ≥1 time".

## Acceptance

- `.claude-flow/audit/adversary-veto/register.md` has ≥1 Phase B entry.

## Memory

- `memory_store` at `verification/adversary-findings/veto` in `phase-b-adv`.
- `memory_store` exit report at `phase-b-adv` blackboard: `pb-adv-veto:done`.
