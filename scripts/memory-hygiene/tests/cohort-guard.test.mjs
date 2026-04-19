#!/usr/bin/env node
// Unit tests for cohort-guard. Acceptance cases from v1-memory-ttl prompt:
//   1. cohort-B caller reading `verification-v1`     → rejected
//   2. cohort-A caller reading `verification-v1-adv` → rejected
//   3. cohort-A caller reading `verification/spec-readings` → permitted
//
// Runs standalone: `node scripts/memory-hygiene/tests/cohort-guard.test.mjs`
// Exits 0 on pass, 1 on fail.

import { guard } from '../cohort-guard.mjs';

let failed = 0;
function check(name, result, expectAllow, reasonIncludes) {
  const ok = result.allow === expectAllow
    && (!reasonIncludes || (result.reason || '').includes(reasonIncludes));
  if (ok) {
    console.log(`ok  ${name}`);
  } else {
    failed++;
    console.log(`FAIL ${name}: got ${JSON.stringify(result)}`);
  }
}

// 1. Cross-cohort B → verification-v1 must be denied.
check(
  'cohort-B read verification-v1 → deny',
  guard({ cohort: 'cohort-b', action: 'read', namespace: 'verification-v1' }),
  false,
  'deny-listed for cohort-b',
);

// 2. Cohort-A read of adversary namespace must be denied.
check(
  'cohort-A read verification-v1-adv → deny',
  guard({ cohort: 'cohort-a', action: 'read', namespace: 'verification-v1-adv' }),
  false,
  'deny-listed for cohort-a',
);

// 3. Shared spec-readings allowed for cohort-A.
check(
  'cohort-A read verification/spec-readings → allow',
  guard({ cohort: 'cohort-a', action: 'read', namespace: 'verification/spec-readings' }),
  true,
);

// Extra: shared spec-readings also allowed for cohort-B.
check(
  'cohort-B read verification/spec-readings → allow',
  guard({ cohort: 'cohort-b', action: 'read', namespace: 'verification/spec-readings' }),
  true,
);

// Extra: cohort-A read own hive allowed.
check(
  'cohort-A read verification-v1 → allow',
  guard({ cohort: 'cohort-a', action: 'read', namespace: 'verification-v1' }),
  true,
);

// Extra: agent-id resolution — v1-adv-nt is cohort-b.
check(
  'agent=v1-adv-nt read verification-v1 → deny',
  guard({ agent: 'v1-adv-nt', action: 'read', namespace: 'verification-v1' }),
  false,
);

// Extra: agent-id resolution — v1-diff-core is cohort-a.
check(
  'agent=v1-diff-core read verification/spec-readings → allow',
  guard({ agent: 'v1-diff-core', action: 'read', namespace: 'verification/spec-readings' }),
  true,
);

// Fail-closed: unknown cohort.
check(
  'unknown cohort → deny',
  guard({ cohort: 'cohort-z', action: 'read', namespace: 'verification-v1' }),
  false,
  'unknown cohort',
);

// Unknown agent → deny.
check(
  'unknown agent → deny',
  guard({ agent: 'v1-nonexistent', action: 'read', namespace: 'verification-v1' }),
  false,
  'not in cohort registry',
);

// Cross-cohort write into other hive → deny.
check(
  'cohort-B write verification-v1 → deny',
  guard({ cohort: 'cohort-b', action: 'write', namespace: 'verification-v1' }),
  false,
);

if (failed > 0) {
  console.error(`\n${failed} test(s) failed`);
  process.exit(1);
}
console.log('\nall cohort-guard tests passed');
