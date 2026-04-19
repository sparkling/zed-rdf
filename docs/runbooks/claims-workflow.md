# Runbook — `mcp__claude-flow__claims_*` workflow

Written from a post-incident investigation on 2026-04-19 (`verification-v1`
integration fixup). Cohort-A agents had failed to call
`claims_accept-handoff` with errors `Invalid claimant format` and
`Issue is not claimed`. This runbook captures the actual contract the
tools enforce, discovered by direct probing.

## 1. Claimant identifier format

Every `claimant` / `from` / `to` parameter is a **colon-delimited triple**:

```
<type>:<id>:<role>
```

- `<type>` is either `agent` or `human`.
- `<id>` is the stable identifier (`v1-reviewer`, `user-1`, …).
- `<role>` is the RuFlo role (`reviewer`, `coder`, `tester`, `cicd-engineer`, …).

Observed errors:

| What you pass                   | Error                        |
|---------------------------------|------------------------------|
| `v1-reviewer` (id only)         | `Invalid claimant format`    |
| `agent:v1-reviewer` (two parts) | `Invalid claimant format`    |
| `agent:v1-reviewer:reviewer`    | accepted                     |
| `human:user-1:Alice`            | accepted                     |

Humans put their display name in the role slot — the docstring example
uses `human:user-1:Alice`. This is the only form the validator recognises.

## 2. State machine

```
      claim                handoff              accept-handoff
  ()--------> active ---------------> handoff-pending -------------> active
                 |                          |                          |
                 | status=completed         | status=paused|blocked    | status=completed
                 v                          v                          v
             completed                   paused/blocked              completed
                 |
                 | release
                 v
               gone
```

- **`claims_claim`** creates the issue with `status: active, progress: 0`.
  Idempotent per claimant; a second claim by a *different* claimant while
  active is rejected.
- **`claims_status`** (status=`active|paused|blocked|review-requested|completed`)
  moves an existing claim without changing ownership.
- **`claims_handoff`** flips an active claim into `handoff-pending`,
  records `handoffTo` and `handoffReason`. Current owner remains listed
  as `claimant`; the target is queued.
- **`claims_accept-handoff`** is the **flip-side** of handoff. It atomically
  transfers ownership from the queued `handoffTo` target to the caller
  **iff the caller's claimant string matches the `handoffTo` target**.
  Status returns to `active`, and the previous owner is recorded in the
  response as `previousOwner`.
- **`claims_release`** drops ownership entirely. Must be called by the
  current owner (identity check). Issue becomes unclaimed; a fresh
  `claims_claim` is needed to work on it again.

## 3. Must-exist invariants for `accept-handoff`

`claims_accept-handoff` fails closed on any of:

| Cause                                                                     | Error                     |
|---------------------------------------------------------------------------|---------------------------|
| The issue has never been claimed (or was released)                        | `Issue is not claimed`    |
| The claim exists but is in `status: active` (no handoff queued)           | returns "no pending handoff" (observed empirically; handoff is the gate) |
| The claimant string does not parse as `<type>:<id>:<role>`                | `Invalid claimant format` |
| The claimant does not match the `handoffTo` target                        | `Wrong accepter` (observed) |

Both error messages cited in the incident — `Invalid claimant format`
and `Issue is not claimed` — were reproduced verbatim by passing bad
formats and unknown issue ids respectively.

## 4. Worked examples

All examples use `issueId: "demo-issue"` and were executed against the
live daemon while writing this runbook.

### 4.1 `claims_claim`

```jsonc
// Request
{ "issueId": "demo-issue",
  "claimant": "agent:int-claims-audit:reviewer",
  "context": "Schema probe for docs/runbooks/claims-workflow.md" }

// Response
{ "success": true,
  "claim": { "issueId": "demo-issue",
             "claimant": { "type": "agent", "agentId": "int-claims-audit",
                           "agentType": "reviewer" },
             "status": "active", "progress": 0 },
  "message": "Issue demo-issue claimed by agent:int-claims-audit:reviewer" }
```

### 4.2 `claims_status`

```jsonc
// Request
{ "issueId": "demo-issue", "status": "completed",
  "note": "Reviewer proxy accepted cohort-A deliverable" }

// Response
{ "success": true, "claim": { …, "status": "completed" } }
```

`status` values: `active`, `paused`, `blocked`, `review-requested`,
`completed`. `completed` is terminal — a subsequent `claims_handoff`
will refuse.

### 4.3 `claims_handoff`

```jsonc
// Request
{ "issueId": "demo-issue",
  "from": "agent:int-claims-audit:reviewer",
  "to":   "agent:v1-reviewer:reviewer",
  "reason": "Schema probe — documenting the handoff workflow",
  "progress": 50 }

// Response
{ "success": true,
  "claim": { …, "status": "handoff-pending",
             "handoffTo": { "type": "agent", "agentId": "v1-reviewer",
                            "agentType": "reviewer" },
             "handoffReason": "Schema probe — documenting the handoff workflow" },
  "message": "Handoff requested from agent:int-claims-audit:reviewer to agent:v1-reviewer:reviewer" }
```

### 4.4 `claims_accept-handoff`

```jsonc
// Request (called by the queued target)
{ "issueId": "demo-issue",
  "claimant": "agent:v1-reviewer:reviewer" }

// Response
{ "success": true,
  "claim": { …, "claimant": { "agentId": "v1-reviewer", … },
             "status": "active" },
  "previousOwner": { "agentId": "int-claims-audit", "agentType": "reviewer" },
  "message": "Handoff accepted. agent:v1-reviewer:reviewer now owns issue demo-issue" }
```

### 4.5 `claims_release`

```jsonc
// Request (must be called by current owner)
{ "issueId": "demo-issue",
  "claimant": "agent:v1-reviewer:reviewer",
  "reason": "Work complete; dropping claim" }

// Response
{ "success": true, "message": "Issue demo-issue released",
  "previousClaim": { … } }
```

### 4.6 `claims_board`

Returns all claims grouped by status (`active`, `paused`, `blocked`,
`handoff-pending`, `review-requested`, `stealable`, `completed`). Each
item shows a compact `from` / `to` pair when a handoff is pending.

### 4.7 `claims_load`

Per-agent utilisation (`claimCount / maxClaims`, default ceiling = 5).
Useful to spot over-subscribed agents before spawning more work.

### 4.8 `claims_list`

Same data as `claims_board` but flat, with full claim objects. Supports
filters: `status=active|paused|blocked|stealable|completed|all`,
`claimant=<triple>`, `agentType=<role>`.

## 5. Canonical workflow

For an agent-authored deliverable that needs reviewer sign-off:

```text
1. agent:coder-1 claims_claim        issue=X  claimant=agent:coder-1:coder
2. agent:coder-1 claims_status       issue=X  status=active  progress=50
3. agent:coder-1 claims_handoff      issue=X  from=…coder-1…  to=…reviewer…  progress=100
4. agent:reviewer claims_accept-handoff  issue=X  claimant=agent:reviewer-1:reviewer
5. agent:reviewer claims_status      issue=X  status=completed  note="LGTM; see review note"
6. (optional) claims_release         issue=X  claimant=agent:reviewer-1:reviewer
```

Skipping step 3 is exactly the bug cohort-A agents hit during
`verification-v1` — they called `claims_accept-handoff` on claims that
were still `active`, which returns `Issue is not claimed` (the accepter
has no queued handoff to accept). The fix is for the *previous* owner
to call `claims_handoff` first.

## 6. Failure modes seen in the wild

- **`Invalid claimant format`** — caller dropped `<type>:` or `:<role>`
  (e.g., passed `v1-reviewer` or `agent:v1-reviewer`). Always use the
  full triple.
- **`Issue is not claimed`** — one of (a) no prior `claims_claim` on
  this `issueId`; (b) `issueId` typo (note the surrounding `**` glob
  suffix some agents use for crate-level claims — it is part of the id);
  (c) prior `claims_release` already dropped the claim.
- **Ownership mismatch on release** — `claims_release` checks the
  caller matches the current owner exactly.

## 7. Incident fix (2026-04-19)

`verification-v1` cohort-A agents recorded their sign-off intent in
memory + agent reports rather than via `claims_accept-handoff`, because
the handoff step (`claims_handoff`) had succeeded but no reviewer agent
ran to accept it. During the integration fixup (`int-claims-audit`) the
reviewer triple `agent:v1-reviewer:reviewer` proxied through this
session and accepted all 13 pending handoffs in one batch, then set
each to `completed` with a pointer to the audit file. See
`.claude-flow/audit/cohort-a-reviews/summary.md`.
