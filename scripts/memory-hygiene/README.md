# memory-hygiene

ADR-0019 §6 implementation — three guards that together stop cohort
cross-talk and memory-poisoning in the `verification-v1` sweep.

| File                     | Role                                                          |
|--------------------------|---------------------------------------------------------------|
| `cohort-guard.mjs`       | Pre-memory-op guard. Fails **closed** on cross-cohort reads.  |
| `cohort-registry.mjs`    | Parses `docs/agent-cohorts.md`; shared dependency.            |
| `ttl-sweep.mjs`          | Purges non-pinned entries >7 days in `phase-*`/`verification-*`. |
| `falsification-hook.mjs` | Quarantines memory tagged with a failing test id.             |
| `hooks.toml`             | ruflo hook registration consumed by `v1-ci-wiring`.           |
| `tests/`                 | Unit tests: three suites, one per guard.                      |

## Running tests

```
./scripts/memory-hygiene/run-tests.sh
```

Each suite is standalone — use it as a smoke test when editing any
individual script.

## Cohort guard

Every `memory_store` / `memory_search` call is expected to pass through
`cohort-guard check` before reaching the backend. The runner supplies
`--agent=<id>` (resolved against the registry) or `--cohort=<c>`
directly. Exit code:

- `0` — allowed (reason on stdout)
- `2` — denied (reason on stderr)
- `3` — input error (treated as deny)

The allow/deny matrix lives in `docs/agent-cohorts.md` under each
cohort's "May read" / "May NOT read" block. Shared namespaces —
`verification/spec-readings` and `verification/memory-hygiene` — are
readable from either cohort. **Fail-closed** behaviour is load-bearing:
an unknown cohort, missing agent, or unparseable registry all deny.

## TTL sweep

Default retention is 7 days; pinned entries (tag `pinned` or
`pinned: true`) are exempt. Scope patterns: `/^phase-/` and
`/^verification-/`. Namespaces outside that — e.g. `crate/*`,
`verification/spec-readings` — are skipped with reason
`out-of-scope`. The sweep never mutates memory directly; it emits a
purge manifest that the ruflo hook runner consumes.

### Manual purge

1. Export the current memory listing to JSON (runner-side):

   ```
   npx claude-flow memory list --json \
     --namespace='phase-*,verification-*' \
     > /tmp/memory-listing.json
   ```

2. Dry-run to inspect:

   ```
   node scripts/memory-hygiene/ttl-sweep.mjs \
     --fixture=/tmp/memory-listing.json --dry-run
   ```

3. Apply (runner executes the manifest):

   ```
   node scripts/memory-hygiene/ttl-sweep.mjs \
     --fixture=/tmp/memory-listing.json --dry-run=false
   ```

   This writes
   `.claude-flow/audit/memory-hygiene/<date>.purge-manifest.json`;
   the runner issues `memory_delete` for each entry and records the
   outcome in the audit log.

### Audit log format

`.claude-flow/audit/memory-hygiene/<YYYY-MM-DD>.json`:

```json
{
  "version": 1,
  "run_id": "2026-04-19-12345",
  "ttl_days": 7,
  "dry_run": true,
  "scope_patterns": ["^phase-", "^verification-"],
  "counts": { "purge": 3, "keep": 42, "skipped": 8 },
  "purge": [{ "namespace": "...", "key": "...", "age_ms": 864000000 }],
  "keep":  [{ "namespace": "...", "key": "...", "reason": "pinned" }],
  "skipped": [{ "namespace": "...", "key": "...", "reason": "out-of-scope" }]
}
```

## Falsification hook

Triggered on test-failure events. The runner supplies the failing test
id (`${RUFLO_TEST_ID}`) and the current memory listing on stdin. Entries
tagged with `test:<id>` or carrying `test_id: "<id>"` are written to a
quarantine manifest:

`.claude-flow/audit/memory-hygiene/quarantine/<YYYY-MM-DD>/<test-id>.json`

The runner is responsible for the live `memory_delete` (or soft-move to
a `quarantine/*` namespace). The hook itself is non-destructive so a
dry-run on a fixture is always safe.

## Inspecting quarantine

```
ls .claude-flow/audit/memory-hygiene/quarantine/
jq '.entries[] | .namespace + "/" + .key' \
  .claude-flow/audit/memory-hygiene/quarantine/2026-04-19/*.json
```

To re-instate a quarantined entry: re-run the originating task (which
will re-store it) or copy the value out of the manifest and
`memory_store` it back with a pin tag. Re-instatement must be logged as
an ADR-level decision if it crosses a falsified-test boundary.

## Inspecting cross-cohort violations

`cohort-guard` logs every denial to the runner's hook log. To audit:

```
grep 'memory-hygiene.cohort-guard.*deny' .claude-flow/logs/hooks.log
```

A spike in denials means either a misconfigured agent or an attempt to
bypass cohort separation. Both are escalations.

## Wiring

`hooks.toml` in this directory is the **owner's** copy. `v1-ci-wiring`
mirrors it into the active ruflo hook config (under
`.claude-flow/hooks/`). The three hooks register at:

- `pre-memory-op` → `cohort-guard` (fail-closed)
- `test-failure` → `falsification-hook`
- `daily` → `ttl-sweep` (03:00 UTC)

Changes here MUST be reflected by a companion PR touching the active
hook config; the cohort-guard entry especially is load-bearing — until
it is live, cohort separation is discipline-only.

## Relationship to the cohort registry

`docs/agent-cohorts.md` is the **source of truth**. The guard parses it
at every invocation (it's small and the read is cheap). Do not cache,
do not fork. If the table's format changes, update
`cohort-registry.mjs` in the same commit.
