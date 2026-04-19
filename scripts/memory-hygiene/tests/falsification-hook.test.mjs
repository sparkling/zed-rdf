#!/usr/bin/env node
// Unit test for falsification-hook.findTaggedEntries, plus an end-to-end
// invocation against a synthetic failure.

import { findTaggedEntries } from '../falsification-hook.mjs';
import { readFileSync, mkdtempSync, existsSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

const HERE = dirname(fileURLToPath(import.meta.url));
const fixturePath = resolve(HERE, '..', 'fixtures', 'falsification.fixture.json');
const hookPath = resolve(HERE, '..', 'falsification-hook.mjs');
const entries = JSON.parse(readFileSync(fixturePath, 'utf8'));

let failed = 0;
function expect(name, cond) {
  if (cond) console.log(`ok  ${name}`);
  else { failed++; console.log(`FAIL ${name}`); }
}

// Unit: tag filter by test id.
const tagged = findTaggedEntries(entries, 'turtle-escape-regression');
expect('finds entry tagged via test:<id>',
  tagged.some((e) => e.key === 'reading-draft-turtle-escape'));
expect('finds entry with explicit test_id field',
  tagged.some((e) => e.key === 'followup-note'));
expect('skips untagged entries',
  !tagged.some((e) => e.key === 'unrelated-fact'));

// End-to-end: run the script in a temp audit dir with a synthetic failure.
const auditDir = mkdtempSync(join(tmpdir(), 'falsification-'));
const proc = spawnSync(process.execPath, [
  hookPath,
  `--fixture=${fixturePath}`,
  `--test-id=turtle-escape-regression`,
  `--audit-dir=${auditDir}`,
  '--date=2026-04-19',
  '--quiet',
], { encoding: 'utf8' });

expect('hook runs cleanly', proc.status === 0);

const quarantineDir = join(auditDir, 'quarantine', '2026-04-19');
expect('quarantine dir created', existsSync(quarantineDir));

const manifests = existsSync(quarantineDir) ? readdirSync(quarantineDir) : [];
expect('manifest file written', manifests.length === 1);

if (manifests.length === 1) {
  const manifest = JSON.parse(readFileSync(join(quarantineDir, manifests[0]), 'utf8'));
  expect('manifest carries test_id', manifest.test_id === 'turtle-escape-regression');
  expect('manifest count matches tagged entries', manifest.count === tagged.length);
}

if (failed > 0) { console.error(`\n${failed} test(s) failed`); process.exit(1); }
console.log('\nall falsification-hook tests passed');
