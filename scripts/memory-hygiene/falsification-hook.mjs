#!/usr/bin/env node
// falsification-hook.mjs — ADR-0019 §6 falsification hook.
//
// Invoked from ruflo's test-failure hook. Quarantines any memory entries
// tagged with the failing test id. Quarantine is a non-destructive move:
// entries are written to a `quarantine/<date>/` manifest and (in live
// mode) deleted from their origin namespace.
//
// Input: either CLI arg `--test-id=<id>` plus `--fixture=<path>` to a
// JSON list of entries, or stdin JSON:
//   { "test_id": "<id>", "entries": [ {namespace, key, tags, ...}, ... ] }
//
// Tag convention: an entry is linked to a test id via
//   - tags: [ "test:<id>" ]  (preferred), or
//   - a top-level `test_id` field.

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, '..', '..');
const DEFAULT_AUDIT_DIR = resolve(REPO_ROOT, '.claude-flow', 'audit', 'memory-hygiene');

function parseArgs(argv) {
  const out = { _: [] };
  for (const a of argv) {
    if (a.startsWith('--')) {
      const [k, v] = a.slice(2).split('=');
      out[k] = v === undefined ? true : v;
    } else out._.push(a);
  }
  return out;
}

export function findTaggedEntries(entries, testId) {
  const needle = `test:${testId}`;
  return entries.filter((e) => {
    if (e.test_id === testId) return true;
    const tags = e.tags || [];
    return Array.isArray(tags) && tags.includes(needle);
  });
}

function readStdin() {
  return new Promise((resolveP, reject) => {
    let buf = '';
    process.stdin.setEncoding('utf8');
    process.stdin.on('data', (c) => (buf += c));
    process.stdin.on('end', () => resolveP(buf));
    process.stdin.on('error', reject);
  });
}

function todayStamp(date) {
  const d = date ? new Date(date) : new Date();
  const y = d.getUTCFullYear();
  const m = String(d.getUTCMonth() + 1).padStart(2, '0');
  const day = String(d.getUTCDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const dryRun = args['dry-run'] !== false; // default true
  const auditDir = args['audit-dir'] ? resolve(args['audit-dir']) : DEFAULT_AUDIT_DIR;

  let testId = args['test-id'];
  let entries;

  if (args.fixture) {
    entries = JSON.parse(readFileSync(resolve(args.fixture), 'utf8'));
  } else {
    const raw = await readStdin();
    if (!raw.trim()) {
      console.error('falsification-hook: no input on stdin and no --fixture');
      process.exit(3);
    }
    const payload = JSON.parse(raw);
    testId = testId || payload.test_id;
    entries = payload.entries || [];
  }

  if (!testId) {
    console.error('falsification-hook: missing test_id');
    process.exit(3);
  }

  const quarantined = findTaggedEntries(entries, testId);
  const date = todayStamp(args['date']);
  const quarantineDir = resolve(auditDir, 'quarantine', date);
  mkdirSync(quarantineDir, { recursive: true });

  const manifestFile = resolve(quarantineDir, `${testId.replace(/[^\w.-]/g, '_')}.json`);
  const manifest = {
    version: 1,
    test_id: testId,
    dry_run: dryRun,
    created_at: new Date().toISOString(),
    count: quarantined.length,
    entries: quarantined,
  };
  writeFileSync(manifestFile, JSON.stringify(manifest, null, 2) + '\n', 'utf8');

  if (!args.quiet) {
    console.log(`falsification-hook ${dryRun ? '(dry-run)' : '(LIVE)'}: test=${testId} quarantined=${quarantined.length}`);
    console.log(`manifest: ${manifestFile}`);
    for (const q of quarantined) console.log(`  QUARANTINE ${q.namespace}/${q.key}`);
  }

  // Live deletion is delegated to the ruflo hook runner, which owns MCP
  // access. The manifest is the contract between this script and the
  // runner.
}

const isMain = (() => {
  try { return import.meta.url === `file://${process.argv[1]}`; } catch { return false; }
})();

if (isMain) {
  main().catch((err) => {
    console.error(`falsification-hook: ${err.message}`);
    process.exit(1);
  });
}
