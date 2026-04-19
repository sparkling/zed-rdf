#!/usr/bin/env node
// Unit test for ttl-sweep classification against a seeded fixture.

import { classifyEntries } from '../ttl-sweep.mjs';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(HERE, '..', 'fixtures', 'ttl-sweep.fixture.json');
const entries = JSON.parse(readFileSync(fixturePath, 'utf8'));

// The fixture is anchored: `now` is 2026-04-19T12:00:00Z. Entries older
// than 7 days (stored_at < 2026-04-12T12:00:00Z) without `pinned` should
// appear in purge.
const now = Date.parse('2026-04-19T12:00:00Z');
const { purge, keep, skipped } = classifyEntries(entries, { now, ttlDays: 7 });

const purgeKeys = new Set(purge.map((p) => `${p.namespace}/${p.key}`));
const keepKeys = new Set(keep.map((k) => `${k.namespace}/${k.key}`));
const skippedKeys = new Set(skipped.map((s) => `${s.namespace}/${s.key}`));

let failed = 0;
function expect(name, cond) {
  if (cond) console.log(`ok  ${name}`);
  else { failed++; console.log(`FAIL ${name}`); }
}

expect('old non-pinned verification-v1 entry is purged',
  purgeKeys.has('verification-v1/stale-fact'));
expect('recent verification-v1 entry kept',
  keepKeys.has('verification-v1/fresh-fact'));
expect('spec-readings namespace out of TTL scope',
  skippedKeys.has('verification/spec-readings/turtle-bnode-scope'));
expect('old pinned entry in phase-a kept',
  keepKeys.has('phase-a/pinned-decision'));
expect('old phase-a entry purged',
  purgeKeys.has('phase-a/old-note'));
expect('out-of-scope namespace skipped',
  skippedKeys.has('crate/rdf-turtle/design-note'));

console.log(`\npurge=${purge.length} keep=${keep.length} skipped=${skipped.length}`);
if (failed > 0) { console.error(`\n${failed} test(s) failed`); process.exit(1); }
console.log('all ttl-sweep tests passed');
